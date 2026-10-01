//! Single-owner native search actor processing commands between bounded slices.
//! Only this adapter owns wall-clock deadlines and protocol output.

use super::{
    command::Command,
    go::{Go, PlayState},
    options::{IDENTIFICATION, ProtocolOptions},
    output,
};
use crate::NativeTablebases;
use gwaymaegyi_search::TablebaseProbe;
use gwaymaegyi_search::{Engine, Parameter, SearchTuning, SharedTable};
use std::fmt::Write as _;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::{
    sync::mpsc::{Receiver, SyncSender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
struct SoftLimitInput {
    started: Instant,
    original_opt: u64,
    max_ms: u64,
    best_move_nodes: u64,
    nodes: u64,
    stability: u32,
    score_delta: i32,
    tuning: SearchTuning,
}

#[derive(Debug)]
struct Active {
    started: Instant,
    deadline: Option<Instant>,
    hard: Option<Instant>,
    original_opt: u64,
    max_ms: u64,
    adaptive: bool,
    state: PlayState,
    last_depth: u8,
    last_nodes: u64,
    stability: u32,
    prev_best: Option<gwaymaegyi_core::Move>,
    score_hist: [i32; 3],
    score_count: u8,
    helper_stop: Arc<AtomicBool>,
    helper_nodes: Arc<AtomicU64>,
    helpers: Vec<JoinHandle<()>>,
}
#[derive(Debug)]
struct Worker {
    engine: Engine,
    options: ProtocolOptions,
    active: Option<Active>,
    output: SyncSender<String>,
    tablebases: Option<Arc<NativeTablebases>>,
    tablebase_path: Option<String>,
    last_fen: String,
    last_moves: Vec<String>,
    shared_table: Option<Arc<SharedTable>>,
}

pub(super) fn run(input: &Receiver<Command>, output: &SyncSender<String>) {
    let Ok(engine) = Engine::new() else {
        let _ = output.send("info string engine initialization failed".into());
        return;
    };
    let mut worker = Worker {
        engine,
        options: ProtocolOptions::default(),
        active: None,
        output: output.clone(),
        tablebases: None,
        tablebase_path: None,
        last_fen: gwaymaegyi_core::START_FEN.to_owned(),
        last_moves: Vec::new(),
        shared_table: None,
    };
    loop {
        for command in input.try_iter().take(32) {
            if !worker.command(command) {
                return;
            }
        }
        if worker.engine.searching() {
            if !worker.advance() {
                return;
            }
        } else if let Ok(command) = input.recv() {
            if !worker.command(command) {
                return;
            }
        } else {
            worker.finish();
            return;
        }
    }
}
impl Worker {
    fn send(&self, line: String) -> bool {
        self.output.send(line).is_ok()
    }
    fn rejected(&self, error: &str) -> bool {
        self.send(format!(
            "info string rejected: {}",
            error.replace(['\r', '\n'], " ")
        ))
    }
    fn command(&mut self, command: Command) -> bool {
        match command {
            Command::Uci => {
                let mut lines = IDENTIFICATION.trim_end_matches("uciok").to_owned();
                for behavior in gwaymaegyi_search::Behavior::ALL {
                    if writeln!(
                        lines,
                        "option name Behavior {} type check default true",
                        behavior.name()
                    )
                    .is_err()
                    {
                        return false;
                    }
                }
                for spec in Parameter::SPECS {
                    if writeln!(
                        lines,
                        "option name {} type spin default {} min {} max {}",
                        spec.name, spec.default, spec.min, spec.max
                    )
                    .is_err()
                    {
                        return false;
                    }
                }
                lines.push_str("uciok");
                return self.send(lines);
            }
            Command::Ready => return self.send("readyok".into()),
            Command::NewGame => {
                let result = self.engine.new_game();
                if let Err(error) = result {
                    return self.rejected(&error.to_string());
                }
                if let Some(shared) = &self.shared_table {
                    shared.clear();
                }
                self.last_fen = gwaymaegyi_core::START_FEN.to_owned();
                self.last_moves.clear();
                self.active = None;
            }
            Command::Option { name, value } => {
                if name.eq_ignore_ascii_case("SyzygyPath") {
                    return self.set_tablebase_path(&value);
                }
                match self.options.set(&mut self.engine, &name, &value) {
                    Ok(()) => {
                        self.shared_table = None;
                        self.active = None;
                    }
                    Err(error) => return self.rejected(&error),
                }
            }
            Command::Position { fen, moves } => {
                let borrowed: Vec<_> = moves.iter().map(String::as_str).collect();
                match self.engine.set_position(&fen, &borrowed) {
                    Ok(()) => {
                        self.last_fen = fen;
                        self.last_moves = moves;
                        self.active = None;
                    }
                    Err(error) => return self.rejected(&error.to_string()),
                }
            }
            Command::Go(go) => {
                if let Err(error) = self.start(&go) {
                    return self.rejected(&error);
                }
            }
            Command::Bench { depth, count } => {
                self.finish();
                let started = Instant::now();
                let report =
                    match gwaymaegyi_search::run_benchmark(depth, count, self.engine.options()) {
                        Ok(report) => report,
                        Err(error) => return self.rejected(&error.to_string()),
                    };
                let raw_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                let elapsed_ms = raw_ms.max(1);
                let nps = report.total_nodes.saturating_mul(1_000) / elapsed_ms;
                let msg = format!(
                    "info string bench positions {} depth {} nodes {} nps {nps}",
                    report.positions, report.depth, report.total_nodes
                );
                return self.send(msg);
            }
            Command::PrintParams => {
                let mut lines = String::new();
                for spec in Parameter::SPECS {
                    let step = ((spec.max - spec.min) / 20).max(1);
                    if writeln!(
                        lines,
                        "{}, int, {}, {}, {}, {step}, 0.002",
                        spec.name, spec.default, spec.min, spec.max
                    )
                    .is_err()
                    {
                        return false;
                    }
                }
                return self.send(lines.trim_end().to_owned());
            }
            Command::Stop => self.finish(),
            Command::PonderHit => {
                if let Some(active) = self.active.as_mut() {
                    if active.state == PlayState::Ponder {
                        active.state = PlayState::Normal;
                        active.started = Instant::now();
                        active.deadline = if active.original_opt > 0 {
                            active
                                .started
                                .checked_add(Duration::from_millis(active.original_opt))
                        } else {
                            None
                        };
                        active.hard = if active.max_ms > 0 {
                            active
                                .started
                                .checked_add(Duration::from_millis(active.max_ms))
                        } else {
                            None
                        };
                    }
                }
                if !self.engine.searching() {
                    self.finish();
                }
            }
            Command::Quit => {
                self.engine.stop();
                return false;
            }
            Command::EndInput => {
                self.finish();
                return false;
            }
            Command::Invalid(error) => return self.rejected(&error),
            Command::Ignore => {}
        }
        true
    }
    fn set_tablebase_path(&mut self, path: &str) -> bool {
        let path = path.trim();
        if path.is_empty() {
            self.engine.set_tablebase(None);
            self.tablebases = None;
            self.tablebase_path = None;
            self.active = None;
            return self.send("info string Syzygy tablebases disabled".into());
        }
        if self.tablebase_path.as_deref() == Some(path) {
            return self.send(format!("info string Syzygy path unchanged: {path}"));
        }
        if self.tablebases.is_some() {
            return self.rejected("disable the current SyzygyPath before selecting another");
        }
        match NativeTablebases::open(path) {
            Ok(tablebases) => {
                let max = tablebases.max_pieces();
                let tablebases = Arc::new(tablebases);
                self.engine
                    .set_tablebase(Some(tablebases.clone() as Arc<dyn TablebaseProbe>));
                self.tablebases = Some(tablebases);
                self.tablebase_path = Some(path.to_owned());
                self.active = None;
                self.send(format!(
                    "info string Syzygy WDL enabled for up to {max} pieces; root DTZ uses matching .rtbz files"
                ))
            }
            Err(error) => self.rejected(&error),
        }
    }

    fn start(&mut self, go: &Go) -> Result<(), String> {
        self.finish();
        let started = Instant::now();
        let helper_stop = Arc::new(AtomicBool::new(false));
        let helper_nodes = Arc::new(AtomicU64::new(0));
        let helpers = if self.options.threads > 1 {
            let shared = self.ensure_shared_table()?;
            shared.next_search();
            self.engine.set_shared_table(Some(Arc::clone(&shared)));
            self.spawn_helpers(go, &shared, &helper_stop, &helper_nodes)?
        } else {
            self.engine.set_shared_table(None);
            Vec::new()
        };
        let roots: Vec<_> = go.roots.iter().map(String::as_str).collect();
        self.engine
            .start_moves(go.limits, &roots)
            .map_err(|error| error.to_string())?;
        let time = if go.state == PlayState::Infinite {
            None
        } else {
            go.time_control(
                self.engine.game().board().side_to_move() as usize,
                self.options.overhead,
            )
        };
        let (max_ms, original_opt) = time.unwrap_or((0, 0));
        let (deadline, hard) = if go.state == PlayState::Normal {
            (
                if original_opt > 0 {
                    started.checked_add(Duration::from_millis(original_opt))
                } else {
                    None
                },
                if max_ms > 0 {
                    started.checked_add(Duration::from_millis(max_ms))
                } else {
                    None
                },
            )
        } else {
            (None, None)
        };
        self.active = Some(Active {
            started,
            deadline,
            hard,
            original_opt,
            max_ms,
            adaptive: go.adaptive_time(),
            state: go.state,
            last_depth: 0,
            last_nodes: 0,
            stability: 1,
            prev_best: None,
            score_hist: [0; 3],
            score_count: 0,
            helper_stop,
            helper_nodes,
            helpers,
        });
        if !self.engine.searching() && go.state == PlayState::Normal {
            self.finish();
        }
        Ok(())
    }

    fn ensure_shared_table(&mut self) -> Result<Arc<SharedTable>, String> {
        if let Some(existing) = &self.shared_table {
            return Ok(Arc::clone(existing));
        }
        let table = SharedTable::new(self.engine.options().hash_mib())
            .map_err(|error| error.to_string())?;
        let shared = Arc::new(table);
        self.shared_table = Some(Arc::clone(&shared));
        Ok(shared)
    }

    fn spawn_helpers(
        &self,
        go: &Go,
        shared: &Arc<SharedTable>,
        stop: &Arc<AtomicBool>,
        nodes: &Arc<AtomicU64>,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let mut handles = Vec::with_capacity(usize::from(self.options.threads.saturating_sub(1)));
        let mut base_options = self.engine.options();
        base_options
            .set_hash_mib(1)
            .map_err(|error| error.to_string())?;
        for worker_id in 1..self.options.threads {
            let shared_ref = Arc::clone(shared);
            let stop_ref = Arc::clone(stop);
            let nodes_ref = Arc::clone(nodes);
            let fen = self.last_fen.clone();
            let moves = self.last_moves.clone();
            let roots = go.roots.clone();
            let limits = go.limits;
            let tb = self
                .tablebases
                .as_ref()
                .map(|probe| Arc::clone(probe) as Arc<dyn TablebaseProbe>);
            handles.push(thread::spawn(move || {
                let Ok(mut helper) = Engine::new() else {
                    return;
                };
                helper.set_worker_id(worker_id);
                helper.set_tablebase(tb);
                if helper.configure(base_options).is_err() {
                    return;
                }
                helper.set_shared_table(Some(shared_ref));
                let move_refs: Vec<_> = moves.iter().map(String::as_str).collect();
                let root_refs: Vec<_> = roots.iter().map(String::as_str).collect();
                if helper.set_position(&fen, &move_refs).is_err()
                    || helper.start_moves(limits, &root_refs).is_err()
                {
                    return;
                }
                let mut prev_nodes = 0_u64;
                while helper.searching() && !stop_ref.load(Ordering::Relaxed) {
                    let Ok(rep) = helper.step(256) else {
                        break;
                    };
                    let delta = rep.nodes.saturating_sub(prev_nodes);
                    prev_nodes = rep.nodes;
                    nodes_ref.fetch_add(delta, Ordering::Relaxed);
                }
            }));
        }
        Ok(handles)
    }
    fn advance(&mut self) -> bool {
        let Ok(mut report) = self.engine.step(256) else {
            self.finish();
            return self.rejected("search failed");
        };
        let Some(active) = self.active.as_mut() else {
            return self.rejected("missing active search");
        };
        report.nodes = report
            .nodes
            .saturating_add(active.helper_nodes.load(Ordering::Relaxed));
        let elapsed = active.started.elapsed().as_millis();
        let tuning = self.engine.options().tuning();
        let publish = report.depth > active.last_depth;
        if publish {
            active.last_depth = report.depth;
            active.last_nodes = report.nodes;
            if let Some(best) = report.best_move {
                if active.prev_best == Some(best) {
                    active.stability = active.stability.saturating_add(1);
                } else {
                    active.stability = 1;
                }
                active.prev_best = Some(best);
            }
            let score = report.variations.first().map_or(0, |line| line.score_cp);
            let prev = if active.score_count > 0 {
                let n = usize::from(active.score_count).clamp(1, 3);
                active.score_hist[..n].iter().sum::<i32>() / i32::try_from(n).unwrap_or(1)
            } else {
                0
            };
            active.score_hist[usize::from(active.score_count) % 3] = score;
            active.score_count = active.score_count.saturating_add(1);
            if active.adaptive {
                if let Some(max_ms) = active.hard.and_then(|hard| {
                    hard.duration_since(active.started)
                        .as_millis()
                        .try_into()
                        .ok()
                }) {
                    active.deadline = Self::soft_deadline(SoftLimitInput {
                        started: active.started,
                        original_opt: active.original_opt,
                        max_ms,
                        best_move_nodes: report.best_move_nodes,
                        nodes: report.nodes,
                        stability: active.stability,
                        score_delta: prev - score,
                        tuning,
                    });
                }
            }
        }
        let now = Instant::now();
        let expired = active.deadline.is_some_and(|deadline| now >= deadline)
            || active.hard.is_some_and(|hard| now >= hard);
        let normal = active.state == PlayState::Normal;
        if publish {
            for line in output::info(
                &report,
                self.engine.options().chess960(),
                elapsed,
                self.engine.options().multi_pv(),
            ) {
                if !self.send(line) {
                    return false;
                }
            }
        }
        if expired {
            self.engine.stop();
        }
        if !self.engine.searching() && normal {
            self.finish();
        }
        true
    }
    /// Reference soft-limit adjustment: scale the original opt by the
    /// best-move node share, best-move stability, and a smoothed score drop.
    #[must_use]
    fn soft_deadline(input: SoftLimitInput) -> Option<Instant> {
        let soft_ms = input.tuning.soft_limit_ms(
            input.original_opt,
            input.max_ms,
            input.best_move_nodes,
            input.nodes,
            input.stability,
            input.score_delta,
        )?;
        input.started.checked_add(Duration::from_millis(soft_ms))
    }

    fn finish(&mut self) {
        if let Some(active) = self.active.take() {
            active.helper_stop.store(true, Ordering::Relaxed);
            self.engine.stop();
            for handle in active.helpers {
                let _ = handle.join();
            }
            let mut report = self.engine.report();
            report.nodes = report
                .nodes
                .saturating_add(active.helper_nodes.load(Ordering::Relaxed));
            if report.nodes != active.last_nodes || report.depth == 0 {
                for line in output::info(
                    &report,
                    self.engine.options().chess960(),
                    active.started.elapsed().as_millis(),
                    self.engine.options().multi_pv(),
                ) {
                    if !self.send(line) {
                        return;
                    }
                }
            }
            let _ = self.send(output::best(
                &report,
                self.engine.options().chess960(),
                self.options.ponder,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SoftLimitInput, Worker};
    use gwaymaegyi_search::{Parameter, SearchTuning};
    use std::time::Instant;

    #[test]
    fn native_soft_limit_uses_validated_tuning_and_respects_hard_cap()
    -> Result<(), Box<dyn std::error::Error>> {
        let started = Instant::now();
        let mut tuning = SearchTuning::default();
        let default = Worker::soft_deadline(SoftLimitInput {
            started,
            original_opt: 1000,
            max_ms: 3000,
            best_move_nodes: 250,
            nodes: 1000,
            stability: 1,
            score_delta: 0,
            tuning,
        })
        .ok_or("finite budget")?;
        tuning.set(Parameter::BmFactor1, 100)?;
        let reduced = Worker::soft_deadline(SoftLimitInput {
            started,
            original_opt: 1000,
            max_ms: 3000,
            best_move_nodes: 250,
            nodes: 1000,
            stability: 1,
            score_delta: 0,
            tuning,
        })
        .ok_or("finite budget")?;
        assert!(reduced < default);
        let capped = Worker::soft_deadline(SoftLimitInput {
            started,
            original_opt: 1000,
            max_ms: 100,
            best_move_nodes: 0,
            nodes: 1000,
            stability: 1,
            score_delta: 1000,
            tuning,
        })
        .ok_or("finite budget")?;
        assert!(capped.duration_since(started).as_millis() <= 100);
        Ok(())
    }
}

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
use gwaymaegyi_search::{Engine, Parameter, SearchTuning};
use std::fmt::Write as _;
use std::sync::Arc;
use std::{
    sync::mpsc::{Receiver, SyncSender},
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
}
#[derive(Debug)]
struct Worker {
    engine: Engine,
    options: ProtocolOptions,
    active: Option<Active>,
    output: SyncSender<String>,
    tablebases: Option<Arc<NativeTablebases>>,
    tablebase_path: Option<String>,
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
                self.active = None;
            }
            Command::Option { name, value } => {
                if name.eq_ignore_ascii_case("SyzygyPath") {
                    return self.set_tablebase_path(&value);
                }
                match self.options.set(&mut self.engine, &name, &value) {
                    Ok(()) => self.active = None,
                    Err(error) => return self.rejected(&error),
                }
            }
            Command::Position { fen, moves } => {
                let borrowed: Vec<_> = moves.iter().map(String::as_str).collect();
                match self.engine.set_position(&fen, &borrowed) {
                    Ok(()) => self.active = None,
                    Err(error) => return self.rejected(&error.to_string()),
                }
            }
            Command::Go(go) => {
                if let Err(error) = self.start(&go) {
                    return self.rejected(&error);
                }
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
        let started = Instant::now();
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
        });
        if !self.engine.searching() && go.state == PlayState::Normal {
            self.finish();
        }
        Ok(())
    }
    fn advance(&mut self) -> bool {
        let Ok(report) = self.engine.step(256) else {
            self.finish();
            return self.rejected("search failed");
        };
        let Some(active) = self.active.as_mut() else {
            return self.rejected("missing active search");
        };
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
            self.engine.stop();
            let report = self.engine.report();
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

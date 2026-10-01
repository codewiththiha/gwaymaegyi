//! Single-owner native search actor processing commands between bounded slices.
//! Only this adapter owns wall-clock deadlines and protocol output.

use super::{
    command::Command,
    go::{Go, PlayState},
    options::{IDENTIFICATION, ProtocolOptions},
    output,
};
use gwaymaegyi_core::START_FEN;
use gwaymaegyi_search::Engine;
use std::fmt::Write as _;
use std::{
    sync::mpsc::{Receiver, SyncSender},
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Active {
    started: Instant,
    deadline: Option<Instant>,
    hard: Option<Instant>,
    original_opt: u64,
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
                for spec in gwaymaegyi_search::Parameter::SPECS {
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
                let result = self.engine.set_position(START_FEN, &[]);
                if let Err(error) = result {
                    return self.rejected(&error.to_string());
                }
                self.active = None;
            }
            Command::Option { name, value } => {
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
                    if active.state == PlayState::Ponder && active.original_opt > 0 {
                        active.state = PlayState::Normal;
                        let now = Instant::now();
                        active.deadline = now.checked_add(Duration::from_millis(active.original_opt));
                        active.hard = active.deadline;
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
    fn start(&mut self, go: &Go) -> Result<(), String> {
        let roots: Vec<_> = go.roots.iter().map(String::as_str).collect();
        self.engine
            .start_moves(go.limits, &roots)
            .map_err(|error| error.to_string())?;
        let time = go.time_control(
            self.engine.game().board().side_to_move() as usize,
            self.options.overhead,
        );
        let started = Instant::now();
        let (deadline, hard, original_opt) = match time {
            Some((max, opt)) => (
                started.checked_add(Duration::from_millis(opt)),
                started.checked_add(Duration::from_millis(max)),
                opt,
            ),
            None => (None, None, 0),
        };
        self.active = Some(Active {
            started,
            deadline,
            hard,
            original_opt,
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
            if let Some(max_ms) = active.hard.and_then(|hard| {
                hard.duration_since(active.started)
                    .as_millis()
                    .try_into()
                    .ok()
            }) {
                active.deadline = Self::soft_deadline(
                    active.started,
                    active.original_opt,
                    max_ms,
                    report.best_move_nodes,
                    report.nodes,
                    active.stability,
                    prev - score,
                );
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
    #[expect(
        clippy::cast_precision_loss,
        reason = "Node ratios need no precision beyond 53 bits; clamped downstream."
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "The value is clamped to max_ms, a u64, before conversion."
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "The value is clamped to at least 1.0 before conversion."
    )]
    fn soft_deadline(
        started: Instant,
        original_opt: u64,
        max_ms: u64,
        best_move_nodes: u64,
        nodes: u64,
        stability: u32,
        score_delta: i32,
    ) -> Option<Instant> {
        if original_opt == 0 {
            return None;
        }
        let fract = best_move_nodes as f64 / nodes.max(1) as f64;
        let factor = (1.49 - fract) * 1.77;
        let bm_factor = f64::from(stability).mul_add(-0.06, 1.52);
        let score_factor = (1.0 + f64::from(score_delta) / 540.0).clamp(0.90, 1.18);
        let soft = (original_opt as f64 * factor)
            .mul_add(bm_factor, 0.0)
            .mul_add(score_factor, 0.0);
        let soft = soft.clamp(1.0, max_ms as f64).round() as u64;
        started.checked_add(Duration::from_millis(soft))
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

use super::{
    command::Command,
    go::{Go, PlayState},
    options::{IDENTIFICATION, ProtocolOptions},
    output,
};
use gwaymaegyi_core::START_FEN;
use gwaymaegyi_search::Engine;
use std::{
    sync::mpsc::{Receiver, SyncSender},
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Active {
    started: Instant,
    deadline: Option<Instant>,
    budget: Option<u64>,
    state: PlayState,
    last_depth: u8,
    last_nodes: u64,
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
            Command::Uci => return self.send(IDENTIFICATION.into()),
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
                    if active.state == PlayState::Ponder {
                        active.state = PlayState::Normal;
                        active.deadline = active.budget.and_then(|time| {
                            Instant::now().checked_add(Duration::from_millis(time))
                        });
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
        let budget = go.budget_ms(
            self.engine.game().board().side_to_move() as usize,
            self.options.overhead,
        );
        let started = Instant::now();
        let deadline = if go.state == PlayState::Normal {
            budget.and_then(|time| started.checked_add(Duration::from_millis(time)))
        } else {
            None
        };
        self.active = Some(Active {
            started,
            deadline,
            budget,
            state: go.state,
            last_depth: 0,
            last_nodes: 0,
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
        active.last_depth = report.depth;
        if publish {
            active.last_nodes = report.nodes;
        }
        let expired = active
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline);
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
            let _ = self.send(output::best(&report, self.engine.options().chess960()));
        }
    }
}

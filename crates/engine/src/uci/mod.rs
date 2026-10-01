//! Bounded-channel UCI session orchestration with explicit input/output ownership.

#![expect(
    clippy::missing_errors_doc,
    reason = "I/O failures are documented in plain prose."
)]

mod command;
mod go;
mod options;
mod output;
mod worker;

use command::Command;
use std::{
    error::Error,
    io::{BufRead, Read, Write},
    sync::mpsc,
    thread,
};

/// Runs a bounded-channel UCI session; EOF cancels active work and returns its legal fallback.
pub fn run_uci(
    input: impl BufRead + Send + 'static,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let (commands, receiver) = mpsc::sync_channel(128);
    let (responses, results) = mpsc::sync_channel(64);
    let reader = thread::spawn(move || read_commands(input, &commands));
    let search = thread::spawn(move || worker::run(&receiver, &responses));
    for response in results {
        writeln!(output, "{response}")?;
        output.flush()?;
    }
    reader.join().map_err(|_| "UCI reader failed")?;
    search.join().map_err(|_| "UCI worker failed")?;
    Ok(())
}

fn read_commands(mut input: impl BufRead, sender: &mpsc::SyncSender<Command>) {
    loop {
        let mut line = String::new();
        match (&mut input).take(16_385).read_line(&mut line) {
            Ok(0) => break,
            Ok(_) if line.len() > 16_384 => {
                let _ = sender.send(Command::Invalid("command exceeds 16 KiB".into()));
                break;
            }
            Ok(_) => {
                let command = Command::parse(&line);
                let quit = matches!(command, Command::Quit);
                if sender.send(command).is_err() || quit {
                    return;
                }
            }
            Err(error) => {
                let _ = sender.send(Command::Invalid(error.to_string()));
                break;
            }
        }
    }
    let _ = sender.send(Command::EndInput);
}

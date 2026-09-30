//! Native process boundary; chess rules live in the portable library.

mod cli;

use std::{io::{self, Write}, process::ExitCode};

fn main() -> ExitCode {
    match cli::run(&mut io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "gwaymaegyi: {error}");
            ExitCode::FAILURE
        }
    }
}

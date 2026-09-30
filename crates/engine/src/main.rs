//! Native process boundary; chess rules live in the portable library.

use std::{
    env,
    io::{self, Write},
    process::ExitCode,
};

fn main() -> ExitCode {
    match gwaymaegyi::run(env::args().skip(1), &mut io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "gwaymaegyi: {error}");
            ExitCode::FAILURE
        }
    }
}

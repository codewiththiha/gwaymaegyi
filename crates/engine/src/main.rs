//! Process boundary selecting UCI or utility commands and mapping exit failures.

#[cfg(not(target_family = "wasm"))]
use std::{
    env,
    io::{self, Write},
    process::ExitCode,
};

#[cfg(not(target_family = "wasm"))]
fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    let mut output = io::stdout().lock();
    let result = if args.is_empty() || args == ["uci"] {
        gwaymaegyi::run_uci(io::BufReader::new(io::stdin()), &mut output)
    } else {
        gwaymaegyi::run(args.into_iter(), &mut output)
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "gwaymaegyi: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(target_family = "wasm")]
fn main() {}

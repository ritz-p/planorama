mod classification;
mod cli;
mod layout;
mod model;
mod plan;
mod semantic;
mod svg;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("planorama: {error}");
            ExitCode::FAILURE
        }
    }
}

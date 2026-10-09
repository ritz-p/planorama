#[cfg(test)]
mod benchmarks;
mod classification;
mod cli;
mod icons;
mod layout;
mod model;
mod plan;
mod provider;
mod semantic;
mod svg;
mod view;

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

use std::process::ExitCode;

fn main() -> ExitCode {
    match planorama::run_cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("planorama: {error}");
            ExitCode::FAILURE
        }
    }
}

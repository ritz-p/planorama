use std::process::ExitCode;

fn main() -> ExitCode {
    match planorama::run_cli_with_status() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("planorama: {error}");
            ExitCode::FAILURE
        }
    }
}

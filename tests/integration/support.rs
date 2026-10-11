use std::io::{ErrorKind, Write};
use std::process::{Command, Output, Stdio};

pub fn run(input: &[u8]) -> Output {
    run_with_args(input, &[])
}

pub fn run_with_args(input: &[u8], args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_planorama"))
        .args(["-", "-o", "-"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Err(error) = child.stdin.take().unwrap().write_all(input) {
        // Invalid arguments can close stdin early; callers still check the exit status and output.
        assert_eq!(error.kind(), ErrorKind::BrokenPipe, "{error}");
    }
    child.wait_with_output().unwrap()
}

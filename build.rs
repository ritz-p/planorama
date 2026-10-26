use std::{env, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
}

fn revision(root: &Path) -> Option<String> {
    let top = git(root, &["rev-parse", "--show-toplevel"])?;
    if std::fs::canonicalize(top.trim()).ok()? != std::fs::canonicalize(root).ok()? {
        return None;
    }
    let sha = git(root, &["rev-parse", "--verify", "HEAD"])?;
    let sha = sha.trim();
    if ![40, 64].contains(&sha.len()) || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let status = git(root, &["status", "--porcelain", "--untracked-files=normal"])?;
    Some(format!(
        "{sha}{}",
        if status.is_empty() { "" } else { "-dirty" }
    ))
}

fn main() {
    let root = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    // Recheck Git even when only HEAD, the index, or untracked files changed.
    println!("cargo:rerun-if-changed=build-provenance-always-recheck");
    println!(
        "cargo:rustc-env=PLANORAMA_REVISION={}",
        revision(Path::new(&root)).unwrap_or_default()
    );
}

use super::support::run_with_args;

#[test]
fn typed_cloudflare_matches_are_explicitly_enabled_and_configurable() {
    let input = include_bytes!("../fixtures/cloudflare-imported-plan.json");
    let ordinary = run_with_args(input, &[]);
    for format in ["svg", "html"] {
        let output = run_with_args(input, &["--cloudflare-value-matches", "--format", format]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("data-evidence=\"configured-value\"")
        );
    }
    let disabled = run_with_args(
        input,
        &[
            "--cloudflare-value-matches",
            "--no-cloudflare-value-matches",
        ],
    );
    assert_eq!(ordinary.stdout, disabled.stdout);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("recovery.toml");
    std::fs::write(&path, "[generation]\ncloudflare-value-matches = true\n").unwrap();
    let output = run_with_args(input, &["--config", path.to_str().unwrap()]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("configured-value"));
}

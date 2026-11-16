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

#[test]
fn cloudflare_scope_config_cli_overrides_and_intersections() {
    let input = include_bytes!("../fixtures/cloudflare-imported-plan.json");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scope.toml");
    std::fs::write(&path, "[generation]\ncloudflare-accounts = ['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa']\ncloudflare-zones = ['bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ncloudflare-groups = true\n").unwrap();
    for format in ["svg", "html"] {
        let output = run_with_args(
            input,
            &["--config", path.to_str().unwrap(), "--format", format],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = String::from_utf8(output.stdout).unwrap();
        assert!(output.contains("data-cloudflare-management-member"));
        assert!(!output.contains("data-terraform-address=\"cloudflare_workers_route.api\""));
        let unknown = run_with_args(
            input,
            &[
                "--config",
                path.to_str().unwrap(),
                "--cloudflare-account",
                "unknown",
                "--no-cloudflare-groups",
            ],
        );
        assert!(unknown.status.success());
        let unknown = String::from_utf8(unknown.stdout).unwrap();
        assert!(unknown.contains("data-terraform-address=\"cloudflare_workers_route.api\""));
        assert!(!unknown.contains("data-cloudflare-management-member"));
    }
    assert_eq!(
        run_with_args(input, &[]).stdout,
        run_with_args(
            input,
            &[
                "--config",
                path.to_str().unwrap(),
                "--no-cloudflare-accounts",
                "--no-cloudflare-zones",
                "--no-cloudflare-groups"
            ]
        )
        .stdout
    );
    let focused = run_with_args(
        input,
        &[
            "--cloudflare-zone",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "--focus",
            "cloudflare_workers_route.api",
            "--focus-depth",
            "0",
            "--changed-only",
        ],
    );
    assert!(focused.status.success());
    assert_eq!(
        String::from_utf8(focused.stdout)
            .unwrap()
            .matches("data-terraform-address=")
            .count(),
        1
    );
}

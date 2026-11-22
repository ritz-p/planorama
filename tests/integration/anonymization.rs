use super::support::run_with_args;
const INPUT: &[u8] = include_bytes!("../fixtures/anonymization-plan.json");

#[test]
fn anonymized_exports_remove_identifiers_from_all_exported_bytes() {
    for format in ["svg", "html", "json"] {
        let ordinary = run_with_args(INPUT, &["--format", format]);
        assert!(ordinary.status.success());
        assert!(String::from_utf8_lossy(&ordinary.stdout).contains("private_table_sentinel"));
        let output = run_with_args(INPUT, &["--anonymize", "--format", format]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        for secret in [
            "sentinel",
            "198.51.100.0/24",
            "2001:db8:abcd::/48",
            "739185026412",
            "TOP_SECRET",
        ] {
            assert!(!text.contains(secret), "{format} leaked {secret}");
        }
        assert!(text.contains("aws_route_table"));
        assert!(text.contains("resource_"));
    }
}

#[test]
fn selection_and_legend_settings_do_not_reintroduce_original_names() {
    let result = run_with_args(
        INPUT,
        &[
            "--anonymize",
            "--format",
            "html",
            "--focus",
            "aws_route_table.private_table_sentinel",
            "--focus-depth",
            "1",
            "--changed-only",
            "--omit-relationship-legend",
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(!text.contains("sentinel"));
    assert!(text.contains("\"anonymize\":true"));
    assert!(text.contains("\"selected_view\":true"));
}

#[test]
fn anonymize_rejects_optional_reports_before_writing_output() {
    for flags in [
        vec!["--diagnostics"],
        vec!["--timings"],
        vec!["--layout-diagnostics"],
        vec!["--coverage-report", "-"],
        vec!["--coverage-report-json", "-"],
        vec!["--layout-diagnostics-json", "-"],
        vec!["--format", "html", "--html-diagnostics"],
        vec!["--format", "html", "--html-semantic-diagnostics"],
        vec!["--list-modules"],
    ] {
        let args: Vec<_> = std::iter::once("--anonymize").chain(flags).collect();
        let result = run_with_args(INPUT, &args);
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty());
    }
    let ordinary = run_with_args(INPUT, &[]);
    let disabled = run_with_args(INPUT, &["--anonymize", "--no-anonymize"]);
    assert_eq!(ordinary.stdout, disabled.stdout);
}

#[test]
fn anonymize_config_is_opt_in_and_cli_override_restores_ordinary_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    std::fs::write(&path, "[generation]\nanonymize = true\n").unwrap();
    let result = run_with_args(INPUT, &["--config", path.to_str().unwrap()]);
    assert!(result.status.success());
    assert!(!String::from_utf8_lossy(&result.stdout).contains("sentinel"));
    let disabled = run_with_args(
        INPUT,
        &["--config", path.to_str().unwrap(), "--no-anonymize"],
    );
    assert_eq!(disabled.stdout, run_with_args(INPUT, &[]).stdout);
}

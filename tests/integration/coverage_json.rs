use super::cli;

#[test]
fn json_report_preserves_diagram_bytes_and_stderr_for_single_and_named_inputs() {
    let dir = tempfile::tempdir().unwrap();
    for format in ["svg", "html"] {
        for named in [false, true] {
            let command = || {
                let mut c = cli();
                if named {
                    c.args([
                        "--state",
                        "one=tests/fixtures/containment-plan.json",
                        "--state",
                        "two=tests/fixtures/containment-plan.json",
                    ]);
                } else {
                    c.arg("tests/fixtures/containment-plan.json");
                }
                c.args([
                    "--format",
                    format,
                    "-o",
                    "-",
                    "--changed-only",
                    "--diagnostics",
                ]);
                c
            };
            let plain = command().output().unwrap();
            let json = dir.path().join("coverage.json");
            let markdown = dir.path().join("coverage.md");
            let reported = command()
                .arg("--coverage-report-json")
                .arg(&json)
                .args(["--coverage-json-details", "2"])
                .arg("--coverage-report")
                .arg(&markdown)
                .output()
                .unwrap();
            assert!(
                reported.status.success(),
                "{}",
                String::from_utf8_lossy(&reported.stderr)
            );
            assert_eq!(plain.stdout, reported.stdout);
            assert_eq!(plain.stderr, reported.stderr);
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&json).unwrap()).unwrap();
            assert_eq!(value["schema_version"], 1);
            assert_eq!(value["cross_state"], "not_collected");
            assert_eq!(
                value["states"].as_array().unwrap().len(),
                if named { 2 } else { 1 }
            );
            let text = std::fs::read_to_string(markdown).unwrap();
            assert!(text.contains(&format!(
                "| Resource instances | {} | {} |",
                value["summary"]["input"]["resources"], value["summary"]["selected"]["resources"]
            )));
        }
    }
}

#[test]
fn json_report_rejects_all_output_collisions_and_failed_writes() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.json");
    let diagram = dir.path().join("diagram.svg");
    let config = dir.path().join("settings.toml");
    std::fs::write(&input, r#"{"format_version":"1.2","resource_changes":[]}"#).unwrap();
    std::fs::write(&config, "").unwrap();
    for destination in [
        input.clone(),
        diagram.clone(),
        dir.path().join("./diagram.svg"),
        config.clone(),
        dir.path().join("missing/report.json"),
        dir.path().into(),
    ] {
        let before = std::fs::read(&input).unwrap();
        let output = cli()
            .arg(&input)
            .arg("-o")
            .arg(&diagram)
            .arg("--config")
            .arg(&config)
            .arg("--coverage-report-json")
            .arg(&destination)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{destination:?}");
        assert_eq!(std::fs::read(&input).unwrap(), before);
        assert_eq!(std::fs::read(&config).unwrap(), b"");
        assert!(!diagram.exists());
    }
    for other in [
        "--coverage-report",
        "--layout-diagnostics-json",
        "--provider-plan",
    ] {
        let report = dir.path().join("report");
        std::fs::write(&report, "sentinel").unwrap();
        let output = cli()
            .arg(&input)
            .arg("-o")
            .arg(&diagram)
            .arg("--coverage-report-json")
            .arg(&report)
            .arg(other)
            .arg(&report)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(std::fs::read(&report).unwrap(), b"sentinel");
        assert!(!diagram.exists());
    }
    for args in [
        vec!["--coverage-report-json"],
        vec!["--coverage-report-json", "-"],
        vec!["--coverage-report-json", "a", "--coverage-report-json", "b"],
        vec!["--coverage-json-details", "0"],
        vec![
            "--coverage-report-json",
            "a",
            "--coverage-json-details",
            "1001",
        ],
        vec![
            "--coverage-report-json",
            "a",
            "--coverage-json-details",
            "-1",
        ],
    ] {
        let output = cli().arg(&input).args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}

#[cfg(unix)]
#[test]
fn json_report_protects_hard_and_symbolic_aliases_before_writing_reports() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.json");
    std::fs::write(&input, r#"{"format_version":"1.2","resource_changes":[]}"#).unwrap();
    for hard in [false, true] {
        let alias = dir.path().join(format!("alias-{hard}"));
        if hard {
            std::fs::hard_link(&input, &alias).unwrap();
        } else {
            std::os::unix::fs::symlink(&input, &alias).unwrap();
        }
        for other in [
            None,
            Some("--coverage-report"),
            Some("--layout-diagnostics-json"),
        ] {
            let mut command = cli();
            command
                .arg(&input)
                .args(["-o", "-", "--coverage-report-json"])
                .arg(&alias);
            if let Some(option) = other {
                command.arg(option).arg(&input);
            }
            let output = command.output().unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
        }
    }
}

use super::cli;

#[test]
fn coverage_report_is_opt_in_and_preserves_svg_html_and_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("coverage.md");
    for format in ["svg", "html"] {
        for named in [false, true] {
            let command = || {
                let mut command = cli();
                if named {
                    command.args([
                        "--state",
                        "one=tests/fixtures/containment-plan.json",
                        "--state",
                        "two=tests/fixtures/containment-plan.json",
                    ]);
                } else {
                    command.arg("tests/fixtures/containment-plan.json");
                }
                command.args([
                    "--format",
                    format,
                    "-o",
                    "-",
                    "--diagnostics",
                    "--changed-only",
                ]);
                if format == "html" {
                    command.arg("--html-semantic-diagnostics");
                }
                command
            };
            let plain = command().output().unwrap();
            assert!(!report.exists());
            let reported = command()
                .arg("--coverage-report")
                .arg(&report)
                .output()
                .unwrap();
            assert!(
                reported.status.success(),
                "{}",
                String::from_utf8_lossy(&reported.stderr)
            );
            assert_eq!(plain.stdout, reported.stdout);
            assert_eq!(plain.stderr, reported.stderr);
            let diagram = String::from_utf8(reported.stdout).unwrap();
            assert!(!diagram.contains("Diagram Coverage"));
            assert!(!diagram.contains("data-view-coverage"));
            if format == "html" {
                assert!(diagram.contains("viewer-semantic-diagnostics"));
            }
            let markdown = std::fs::read_to_string(&report).unwrap();
            assert!(markdown.starts_with("# Terraform coverage report\n"));
            assert!(markdown.contains("## Whole diagram"));
            assert!(markdown.contains("changed-only"));
            if named {
                assert!(markdown.contains("## State: two"));
            }
            std::fs::remove_file(&report).unwrap();
        }
    }
}

#[test]
fn coverage_report_rejects_collisions_and_write_failures_before_diagram_write() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.json");
    std::fs::write(&input, r#"{"format_version":"1.2","resource_changes":[]}"#).unwrap();
    let original = std::fs::read(&input).unwrap();
    let diagram = dir.path().join("diagram.svg");
    for destination in [
        input.clone(),
        diagram.clone(),
        dir.path().join("./diagram.svg"),
        dir.path().join("missing/report.md"),
        dir.path().to_owned(),
    ] {
        let output = cli()
            .arg(&input)
            .arg("-o")
            .arg(&diagram)
            .arg("--coverage-report")
            .arg(destination)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(std::fs::read(&input).unwrap(), original);
        assert!(!diagram.exists());
    }
    let other = dir.path().join("other.json");
    let output = cli()
        .arg(&input)
        .arg("-o")
        .arg(&diagram)
        .arg("--coverage-report")
        .arg(&other)
        .arg("--layout-diagnostics-json")
        .arg(&other)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!other.exists());
    assert!(!diagram.exists());
    for args in [
        vec!["--coverage-report"],
        vec!["--coverage-report", "-"],
        vec!["--coverage-report", "a", "--coverage-report", "b"],
    ] {
        assert!(
            !cli()
                .arg(&input)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    let evidence = dir.path().join("evidence.tfplan");
    std::fs::write(&evidence, b"evidence must survive").unwrap();
    let output = cli()
        .arg(&input)
        .arg("--provider-plan")
        .arg(&evidence)
        .arg("--coverage-report")
        .arg(&evidence)
        .arg("-o")
        .arg(&diagram)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::read(&evidence).unwrap(), b"evidence must survive");
}

#[cfg(unix)]
#[test]
fn coverage_report_rejects_filesystem_aliases() {
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
        let output = cli()
            .arg(&input)
            .args(["-o", "-", "--coverage-report"])
            .arg(&alias)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    for report_link in [true, false] {
        let diagram = dir.path().join(format!("diagram-{report_link}"));
        let report = dir.path().join(format!("report-{report_link}"));
        let (link, target) = if report_link {
            (&report, &diagram)
        } else {
            (&diagram, &report)
        };
        std::os::unix::fs::symlink(target, link).unwrap();
        assert!(
            !cli()
                .arg(&input)
                .arg("-o")
                .arg(&diagram)
                .arg("--coverage-report")
                .arg(&report)
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(!target.exists());
    }
}

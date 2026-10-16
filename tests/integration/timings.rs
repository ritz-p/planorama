use super::cli;

#[test]
fn timings_preserve_bytes_and_report_pipeline_without_input_names() {
    for format in ["svg", "html"] {
        for named in [false, true] {
            let inputs = if named {
                vec![
                    "--state",
                    "private-a=tests/fixtures/multi/producer.json",
                    "--state",
                    "private-b=tests/fixtures/multi/consumer.json",
                    "--remote-state",
                    "private-b:data.terraform_remote_state.network=private-a",
                ]
            } else {
                vec!["examples/plan.json"]
            };
            let run = |enabled| {
                let mut command = cli();
                command.args(&inputs).args([
                    "--format",
                    format,
                    "-o",
                    "-",
                    "--diagnostics",
                    "--layout-diagnostics",
                ]);
                if enabled {
                    command.arg("--timings");
                }
                let output = command.output().unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                output
            };
            let plain = run(false);
            let timed = run(true);
            assert_eq!(plain.stdout, timed.stdout);
            let stderr = String::from_utf8(timed.stderr).unwrap();
            assert!(stderr.contains("layout diagnostics"));
            let report = stderr.split_once("timings: total_ms=").unwrap().1;
            assert!(!report.contains("private-"));
            assert!(!report.contains("examples/"));
            assert!(!report.contains("tests/fixtures/"));
            if named {
                assert!(report.contains("stage=cross_state_resolution "));
                assert!(report.contains("stage=cross_state_routing "));
            }
            for stage in [
                "input_io",
                "provider_evidence",
                "terraform_parse_resolve",
                "semantic_transform",
                "selection_projection",
                "layout_routing",
                "composition_serialization",
                "output_io",
            ] {
                assert!(
                    report.contains(&format!("stage={stage} ")),
                    "missing {stage}: {report}"
                );
            }
            let rows: Vec<_> = report
                .lines()
                .filter(|line| line.starts_with("timing: "))
                .collect();
            for (index, row) in rows.iter().enumerate() {
                let fields: std::collections::BTreeMap<_, _> = row
                    .split_whitespace()
                    .skip(1)
                    .map(|field| field.split_once('=').unwrap())
                    .collect();
                assert_eq!(fields["id"].parse::<usize>().unwrap(), index + 1);
                assert!(fields["parent"].parse::<usize>().unwrap() <= index);
                if fields["stage"] == "layout_routing" {
                    let parent = fields["parent"].parse::<usize>().unwrap();
                    if named {
                        assert!(parent > 0);
                        assert!(rows[parent - 1].contains("stage=composition_serialization "));
                    } else {
                        assert_eq!(parent, 0);
                    }
                }
                for field in ["inclusive_ms", "exclusive_ms"] {
                    assert!(fields[field].parse::<f64>().unwrap().is_finite());
                }
            }
            assert_eq!(
                rows.iter()
                    .filter(|row| row.contains("stage=terraform_parse_resolve "))
                    .count(),
                if named { 2 } else { 1 }
            );
            assert!(report.contains("input_bytes="));
            assert!(report.contains("resources="));
            assert!(report.contains("nodes="));
            assert!(report.contains(&format!("output_bytes={}", timed.stdout.len())));
            assert!(
                !String::from_utf8(plain.stderr)
                    .unwrap()
                    .contains("timings:")
            );
        }
    }
}

#[test]
fn timing_option_preserves_error_exit_and_empty_stdout() {
    for enabled in [false, true] {
        let mut command = cli();
        command.args([
            "examples/plan.json",
            "--focus",
            "missing.resource",
            "-o",
            "-",
        ]);
        if enabled {
            command.arg("--timings");
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("planorama:"));
    }
}

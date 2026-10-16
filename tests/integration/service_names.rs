use super::{cli, support};

#[test]
fn icon_mode_validation_and_default_output() {
    for args in [vec!["--icon-mode"], vec!["--icon-mode", "none"]] {
        let output = cli().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("icon or service-name"));
    }
    let input = include_bytes!("../fixtures/data-icons-plan.json");
    let default = support::run(input);
    let explicit = support::run_with_args(input, &["--icon-mode", "icon"]);
    assert!(default.status.success() && explicit.status.success());
    assert_eq!(default.stdout, explicit.stdout);
    assert!(String::from_utf8_lossy(&default.stdout).contains("<symbol "));
    let help = cli().arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&help.stdout).contains("--icon-mode MODE"));
}

#[test]
fn service_names_are_used_in_svg_html_and_named_states() {
    for format in ["svg", "html"] {
        for named in [false, true] {
            let mut command = cli();
            command.args(["--icon-mode", "service-name", "--format", format, "-o", "-"]);
            if named {
                command.args([
                    "--state",
                    "services=tests/fixtures/data-icons-plan.json",
                    "--state",
                    "builtin=tests/fixtures/terraform-plan.json",
                ]);
            } else {
                command.arg("tests/fixtures/data-icons-plan.json");
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let output = String::from_utf8(output.stdout).unwrap();
            assert!(output.contains("data-service-name=\"true\""));
            assert!(output.contains("Amazon EC2"));
            assert!(!output.contains("planorama-icon"));
            assert!(!output.contains("<symbol"));
            assert!(!output.contains("<image"));
            if named {
                assert!(output.contains("Terraform built-in"));
            }
            if format == "html" {
                assert!(output.contains("<!doctype html>"));
            } else {
                roxmltree::Document::parse(&output).unwrap();
                if named {
                    assert_eq!(
                        output,
                        std::fs::read_to_string("examples/service-names.svg")
                            .unwrap()
                            .replace("\r\n", "\n")
                    );
                }
            }
        }
    }
}

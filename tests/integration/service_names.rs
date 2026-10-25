use super::{cli, support};

#[test]
fn gateway_endpoints_use_resource_names_without_inferring_private_link() {
    let input = serde_json::json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc_endpoint.gateway","type":"aws_vpc_endpoint",
         "provider_name":"registry.terraform.io/hashicorp/aws",
         "change":{"actions":["create"],"after":{"vpc_endpoint_type":"Gateway"}}},
        {"address":"aws_vpc_endpoint_route_table_association.gateway","type":"aws_vpc_endpoint_route_table_association",
         "provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"]}}
    ]}).to_string();
    for format in ["svg", "html"] {
        let output = support::run_with_args(
            input.as_bytes(),
            &["--icon-mode", "service-name", "--format", format],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains("AWS PrivateLink"));
        assert!(!text.contains("planorama-icon"));
        assert!(text.contains("data-display-name=\"VPC Endpoint\""));
        assert!(text.contains("data-display-name=\"Endpoint Route Table Association\""));
    }
    let output = support::run(input.as_bytes());
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Amazon VPC"));
}

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
    let removed = support::run_with_args(input, &["--icon-mode", "icon"]);
    assert!(!removed.status.success());
    assert!(String::from_utf8_lossy(&removed.stderr).contains("use service-name"));
    let explicit = support::run_with_args(input, &["--icon-mode", "service-name"]);
    assert!(default.status.success() && explicit.status.success());
    assert_eq!(default.stdout, explicit.stdout);
    assert!(!String::from_utf8_lossy(&default.stdout).contains("<symbol "));
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

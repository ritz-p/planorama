use super::support;

#[test]
fn athena_svg_and_offline_html_preserve_configuration_references_and_filtered_links() {
    let fixture = include_bytes!("../fixtures/athena-plan.json");
    for format in ["svg", "html"] {
        let output = support::run_with_args(fixture, &["--format", format]);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("Amazon Athena"));
        assert!(text.contains("9 resources"));
        for label in [
            "References: aws_s3_bucket.results",
            "References: aws_kms_key.results",
        ] {
            assert!(text.contains(label));
        }
        assert!(!text.contains("arn:aws:"));
        assert!(!text.contains("fixture-only"));
        let svg = &text[text.find("<svg ").unwrap()..text.rfind("</svg>").unwrap() + 6];
        let doc = roxmltree::Document::parse(svg).unwrap();
        for target in doc
            .descendants()
            .filter_map(|n| n.attribute("href"))
            .filter_map(|h| h.strip_prefix('#'))
        {
            assert!(
                doc.descendants().any(|n| n.attribute("id") == Some(target)),
                "{target}"
            );
        }
        let output = support::run_with_args(
            fixture,
            &[
                "--format",
                format,
                "--focus",
                "aws_athena_workgroup.analytics",
                "--focus-depth",
                "0",
            ],
        );
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("1 resource"));
        assert!(text.contains("Unavailable"));
        assert!(!text.contains("data-edge-kind=\"dependency\""));
    }
    let output =
        support::run_with_args(fixture, &["--icon-mode", "service-name", "--changed-only"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Amazon Athena")
    );
}

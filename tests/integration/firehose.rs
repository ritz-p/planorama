use super::support;

#[test]
fn firehose_svg_and_offline_html_keep_distinct_references_and_filtered_links() {
    let fixture = include_bytes!("../fixtures/firehose-plan.json");
    for format in ["svg", "html"] {
        let output = support::run_with_args(fixture, &["--format", format]);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("planorama-icon-aws-firehose"));
        assert!(text.contains("5 resources"));
        for label in [
            "Kinesis source",
            "S3 delivery destination",
            "S3 backup destination",
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
                "aws_kinesis_firehose_delivery_stream.archive",
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
            .contains("Amazon Data Firehose")
    );
}

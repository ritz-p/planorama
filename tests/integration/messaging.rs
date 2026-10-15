use super::{cli, support};
use roxmltree::Document;

#[test]
fn subscription_svg_html_filters_and_named_states_keep_real_navigation() {
    verify(
        "tests/fixtures/sns-subscriptions-plan.json",
        "subscription",
        "aws_sns_topic_subscription.jobs",
        8,
    );
}

#[test]
fn event_source_svg_html_filters_and_named_states_keep_real_navigation() {
    verify(
        "tests/fixtures/event-sources-plan.json",
        "eventsource",
        "aws_lambda_event_source_mapping.jobs",
        9,
    );
}

fn verify(path: &str, kind: &str, address: &str, resources: usize) {
    let bytes = std::fs::read(path).unwrap();
    let fixture = bytes.as_slice();
    for format in ["svg", "html"] {
        let output = support::run_with_args(fixture, &["--format", format]);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let start = text.find("<svg ").unwrap();
        let end = text.rfind("</svg>").unwrap() + 6;
        let doc = Document::parse(&text[start..end]).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("data-edge-kind") == Some(kind))
                .count(),
            3
        );
        assert_eq!(
            doc.descendants()
                .filter(|n| n.has_attribute("data-relationship-entry"))
                .count(),
            3
        );
        assert_eq!(
            doc.descendants()
                .filter(|n| n.has_attribute("data-relationship-marker"))
                .count(),
            6
        );
        assert!(text.contains(&format!("{resources} resources")));
        for link in doc
            .descendants()
            .filter_map(|n| n.attribute("href"))
            .filter_map(|h| h.strip_prefix('#'))
        {
            assert!(
                doc.descendants().any(|n| n.attribute("id") == Some(link)),
                "{link}"
            );
        }
        let filtered = support::run_with_args(
            fixture,
            &["--format", format, "--focus", address, "--focus-depth", "0"],
        );
        assert!(filtered.status.success());
        let text = String::from_utf8(filtered.stdout).unwrap();
        assert!(!text.contains(&format!("data-edge-kind=\"{kind}\"")));
        assert!(text.contains("Unavailable"));
        let changed = support::run_with_args(fixture, &["--format", format, "--changed-only"]);
        assert!(changed.status.success());
        let text = String::from_utf8(changed.stdout).unwrap();
        assert!(text.contains(&format!("data-edge-kind=\"{kind}\"")));
        assert!(text.contains("aws_lambda_function.worker"));
    }
    let first = format!("one={path}");
    let second = format!("two={path}");
    let render = |states: [&str; 2]| {
        let output = cli()
            .args(["--state", states[0], "--state", states[1], "-o", "-"])
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    let text = render([&first, &second]);
    assert_eq!(text, render([&second, &first]));
    let doc = Document::parse(&text).unwrap();
    for state in ["one", "two"] {
        let panel = doc
            .descendants()
            .find(|n| n.attribute("data-state-id") == Some(state))
            .unwrap();
        assert_eq!(
            panel
                .descendants()
                .filter(|n| n.attribute("data-edge-kind") == Some(kind))
                .count(),
            3
        );
        for edge in panel
            .descendants()
            .filter(|n| n.attribute("data-edge-kind") == Some(kind))
        {
            for attribute in ["data-source", "data-target"] {
                let id = edge.attribute(attribute).unwrap();
                assert!(panel.descendants().any(|n| n.attribute("id") == Some(id)));
            }
        }
    }
    let mut split: serde_json::Value = serde_json::from_slice(fixture).unwrap();
    split["resource_changes"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["type"] != "aws_sqs_queue" && r["type"] != "aws_lambda_function");
    let dir = tempfile::tempdir().unwrap();
    let partial = dir.path().join("partial.json");
    std::fs::write(&partial, split.to_string()).unwrap();
    let output = cli()
        .args([
            "--state",
            &format!("partial={}", partial.display()),
            "--state",
            &second,
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let doc = Document::parse(&text).unwrap();
    let panel = doc
        .descendants()
        .find(|n| n.attribute("data-state-id") == Some("partial"))
        .unwrap();
    assert!(
        !panel
            .descendants()
            .any(|n| n.attribute("data-edge-kind") == Some(kind))
    );
}

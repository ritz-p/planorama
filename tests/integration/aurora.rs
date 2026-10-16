use super::{render, support};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[test]
fn aurora_members_keep_unique_identities_links_actions_and_deterministic_output() {
    let input = include_str!("../fixtures/aurora-plan.json");
    let svg = render(input.as_bytes());
    assert!(svg.contains("2 member cards") && svg.contains("8 resources"));
    assert!(svg.contains("create (1)") && svg.contains("update (1)"));
    assert!(!svg.contains("TOP_SECRET"));
    assert_eq!(
        svg,
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/aurora.svg"))
            .unwrap()
            .replace("\r\n", "\n")
    );
    let doc = roxmltree::Document::parse(&svg).unwrap();
    assert!(!svg.contains("data-route-table="));
    assert_eq!(
        doc.descendants()
            .filter(|node| node.attribute("data-member-cluster").is_some())
            .count(),
        1
    );
    let cards: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-terraform-address").is_some())
        .collect();
    assert_eq!(cards.len(), 8);
    assert_eq!(
        cards
            .iter()
            .map(|n| n.attribute("id").unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        8
    );
    let ids: BTreeSet<_> = doc
        .descendants()
        .filter_map(|n| n.attribute("id"))
        .collect();
    for link in doc
        .descendants()
        .filter_map(|n| n.attribute("href"))
        .filter_map(|s| s.strip_prefix('#'))
    {
        assert!(ids.contains(link), "missing {link}");
    }
    for card in cards
        .iter()
        .filter(|n| n.attribute("data-embedded-member").is_some())
    {
        assert_eq!(
            card.attribute("data-member-owner"),
            Some("aws_rds_cluster.main")
        );
        assert!(card.children().any(|n| n.has_tag_name("use")));
        assert!(
            card.descendants()
                .any(|n| n.text().is_some_and(|t| t.contains("ap-northeast-1")))
        );
    }
    let mut value: Value = serde_json::from_str(input).unwrap();
    value["resource_changes"].as_array_mut().unwrap().reverse();
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(svg, render(value.to_string().as_bytes()));
}

#[test]
fn aurora_long_members_lifecycle_html_and_named_states_remain_accessible() {
    let input = include_str!("../fixtures/aurora-plan.json");
    let long = format!("aws_rds_cluster_instance.{}", "long_member_".repeat(25));
    let mut value: Value =
        serde_json::from_str(&input.replace("aws_rds_cluster_instance.member", &long)).unwrap();
    value["resource_changes"][5]["previous_address"] =
        json!("aws_rds_cluster_instance.old[\"<&>\"]");
    value["resource_changes"][5]["change"]["importing"] = json!({"id":"TOP_SECRET_IMPORT"});
    for (format, address_format) in [
        ("svg", "qualified"),
        ("svg", "terraform"),
        ("html", "qualified"),
        ("html", "terraform"),
    ] {
        let output = support::run_with_args(
            value.to_string().as_bytes(),
            &["--format", format, "--address-format", address_format],
        );
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains("TOP_SECRET"));
        assert!(text.contains("data-import=\"true\"") && text.contains("data-previous-address="));
        let svg = &text[text.find("<svg").unwrap()..text.rfind("</svg>").unwrap() + 6];
        let doc = roxmltree::Document::parse(svg).unwrap();
        for card in doc
            .descendants()
            .filter(|n| n.attribute("data-embedded-member").is_some())
        {
            let visible = card
                .children()
                .filter(|node| node.has_tag_name("text"))
                .filter_map(|node| node.text())
                .collect::<String>();
            let native = card.attribute("data-terraform-address").unwrap();
            let expected = if address_format == "qualified" {
                format!("module.root.{native}")
            } else {
                native.to_string()
            };
            assert!(visible.starts_with(&expected), "{visible}");
            let rect = card.children().find(|n| n.has_tag_name("rect")).unwrap();
            let top: usize = rect.attribute("y").unwrap().parse().unwrap();
            let height: usize = rect.attribute("height").unwrap().parse().unwrap();
            for text in card.descendants().filter(|n| n.has_tag_name("text")) {
                let y: usize = text.attribute("y").unwrap().parse().unwrap();
                assert!(y > top && y < top + height);
            }
        }
    }
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/aurora-plan.json"
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"))
        .args([
            "--state",
            &format!("first={path}"),
            "--state",
            &format!("second={path}"),
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let members: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-embedded-member").is_some())
        .collect();
    assert_eq!(members.len(), 4);
    let clusters: Vec<_> = doc
        .descendants()
        .filter_map(|node| node.attribute("data-member-cluster"))
        .collect();
    assert_eq!(clusters.len(), 2);
    assert_ne!(clusters[0], clusters[1]);
    for cluster in clusters {
        assert!(
            doc.descendants()
                .any(|node| node.attribute("id") == Some(cluster)
                    && node.attribute("data-terraform-address") == Some("aws_rds_cluster.main"))
        );
    }
    assert_eq!(
        members
            .iter()
            .map(|n| n.attribute("id").unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        4
    );
    let focused = support::run_with_args(
        input.as_bytes(),
        &[
            "--focus",
            "aws_rds_cluster_instance.member[0]",
            "--focus-depth",
            "0",
        ],
    );
    assert!(focused.status.success());
    let svg = String::from_utf8(focused.stdout).unwrap();
    assert!(!svg.contains("data-embedded-member"));
    assert!(svg.contains("data-terraform-address=\"aws_rds_cluster_instance.member[0]\""));
}

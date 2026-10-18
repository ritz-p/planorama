use super::{render, support};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[test]
fn efs_children_preserve_long_addresses_lifecycle_modes_and_reference_links() {
    let mut value: Value = serde_json::from_str(include_str!("../fixtures/efs-plan.json")).unwrap();
    let address = format!("aws_efs_access_point.app[\"{}<&>\"]", "indexed_".repeat(35));
    value["resource_changes"][2]["address"] = json!(address);
    value["resource_changes"][2]["previous_address"] = json!("aws_efs_access_point.old");
    value["resource_changes"][2]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut deposed = value["resource_changes"][2].clone();
    deposed["deposed"] = json!("old-object");
    deposed["change"] =
        json!({"actions":["delete"],"before":{"file_system_id":"fs-synthetic"},"after":null});
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(deposed);
    for format in ["svg", "html"] {
        for icons in ["icon", "service-name"] {
            let output = support::run_with_args(
                value.to_string().as_bytes(),
                &["--format", format, "--icon-mode", icons],
            );
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(!text.contains("PRIVATE_"));
            let svg = &text[text.find("<svg").unwrap()..text.rfind("</svg>").unwrap() + 6];
            let doc = roxmltree::Document::parse(svg).unwrap();
            let cards: Vec<_> = doc
                .descendants()
                .filter(|n| n.attribute("data-terraform-address").is_some())
                .collect();
            assert_eq!(cards.len(), 7);
            assert_eq!(
                cards
                    .iter()
                    .filter(|n| n.attribute("data-embedded-member").is_some())
                    .count(),
                1
            );
            let child = cards
                .iter()
                .find(|n| n.attribute("data-embedded-member").is_some())
                .unwrap();
            assert_eq!(
                child.attribute("data-terraform-address"),
                Some(address.as_str())
            );
            assert_eq!(child.attribute("data-import"), Some("true"));
            assert_eq!(
                child.attribute("data-previous-address"),
                Some("aws_efs_access_point.old")
            );
            let rect = child.children().find(|n| n.has_tag_name("rect")).unwrap();
            let top = rect.attribute("y").unwrap().parse::<usize>().unwrap();
            let bottom = top + rect.attribute("height").unwrap().parse::<usize>().unwrap();
            for line in child.descendants().filter(|n| n.has_tag_name("text")) {
                let y = line.attribute("y").unwrap().parse::<usize>().unwrap();
                assert!(y > top && y < bottom);
            }
            let ids: BTreeSet<_> = doc
                .descendants()
                .filter_map(|n| n.attribute("id"))
                .collect();
            for target in doc
                .descendants()
                .filter_map(|n| n.attribute("href"))
                .filter_map(|s| s.strip_prefix('#'))
            {
                assert!(ids.contains(target), "{target}");
            }
            assert!(
                cards
                    .iter()
                    .any(|n| n.attribute("data-deposed-key") == Some("old-object")
                        && n.attribute("data-embedded-member").is_none())
            );
        }
    }
}

#[test]
fn efs_example_is_deterministic_and_named_states_keep_distinct_children() {
    let input = include_str!("../fixtures/efs-plan.json");
    let svg = render(input.as_bytes());
    assert_eq!(
        svg,
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/efs.svg"))
            .unwrap()
            .replace("\r\n", "\n")
    );
    let mut value: Value = serde_json::from_str(input).unwrap();
    value["resource_changes"].as_array_mut().unwrap().reverse();
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(svg, render(value.to_string().as_bytes()));
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/efs-plan.json");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"))
        .args([
            "--state",
            &format!("one={path}"),
            "--state",
            &format!("two={path}"),
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let doc = roxmltree::Document::parse(&text).unwrap();
    let children: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-embedded-member").is_some())
        .collect();
    assert_eq!(children.len(), 2);
    assert_ne!(children[0].attribute("id"), children[1].attribute("id"));
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-terraform-address").is_some())
            .count(),
        12
    );
}

#[test]
fn cross_state_file_system_reference_stays_navigation_without_local_ownership() {
    let temp = tempfile::tempdir().unwrap();
    let mut producer: Value =
        serde_json::from_str(include_str!("../fixtures/efs-plan.json")).unwrap();
    producer["configuration"]["root_module"]["outputs"]["file_system"] =
        json!({"expression":{"references":["aws_efs_file_system.shared.id"]}});
    let mut consumer = producer.clone();
    consumer["configuration"]["root_module"]["resources"][2]["expressions"]["file_system_id"] =
        json!({"references":["data.terraform_remote_state.storage.outputs.file_system"]});
    consumer["resource_changes"].as_array_mut().unwrap().push(json!({"address":"data.terraform_remote_state.storage","mode":"data","type":"terraform_remote_state"}));
    let producer_path = temp.path().join("producer.json");
    let consumer_path = temp.path().join("consumer.json");
    std::fs::write(&producer_path, producer.to_string()).unwrap();
    std::fs::write(&consumer_path, consumer.to_string()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"))
        .args([
            "--state",
            &format!("producer={}", producer_path.display()),
            "--state",
            &format!("consumer={}", consumer_path.display()),
            "--remote-state",
            "consumer:data.terraform_remote_state.storage=producer",
            "--format",
            "html",
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let svg = &text[text.find("<svg").unwrap()..text.rfind("</svg>").unwrap() + 6];
    let doc = roxmltree::Document::parse(svg).unwrap();
    let consumer = doc
        .descendants()
        .find(|n| n.attribute("data-state-id") == Some("consumer"))
        .unwrap();
    let child = consumer
        .descendants()
        .find(|n| n.attribute("data-terraform-address") == Some("aws_efs_access_point.app"))
        .unwrap();
    assert!(child.attribute("data-embedded-member").is_none());
    assert!(
        doc.descendants()
            .any(|n| n.attribute("data-cross-state").is_some())
    );
}

use super::{render, support};
use serde_json::{Value, json};

#[test]
fn elasticache_named_states_keep_unique_resource_ids() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/elasticache-plan.json"
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
    let document = roxmltree::Document::parse(&svg).unwrap();
    let cards: Vec<_> = document
        .descendants()
        .filter(|n| n.attribute("data-terraform-address").is_some())
        .collect();
    assert_eq!(cards.len(), 10);
    let ids: std::collections::BTreeSet<_> =
        cards.iter().map(|n| n.attribute("id").unwrap()).collect();
    assert_eq!(ids.len(), 10);
}

#[test]
fn elasticache_cards_preserve_scope_actions_identity_and_safe_filtering() {
    let input = include_str!("../fixtures/elasticache-plan.json");
    let svg = render(input.as_bytes());
    assert!(!svg.contains("data-account="));
    assert!(svg.contains("data-region=\"ap-northeast-1\""));
    assert_eq!(svg.matches("data-placement=\"inferred\"").count(), 2);
    assert!(svg.contains("create (1)") && svg.contains("update (1)"));
    assert!(!svg.contains("TOP_SECRET"));
    assert_eq!(
        svg,
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/elasticache.svg"
        ))
        .unwrap()
        .replace("\r\n", "\n")
    );
    let mut plan: Value = serde_json::from_str(input).unwrap();
    plan["resource_changes"].as_array_mut().unwrap().reverse();
    plan["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(svg, render(plan.to_string().as_bytes()));
    let long = format!(
        "aws_elasticache_replication_group.{}",
        "long_name_".repeat(30)
    );
    let svg = render(
        input
            .replace("aws_elasticache_replication_group.main", &long)
            .as_bytes(),
    );
    let document = roxmltree::Document::parse(&svg).unwrap();
    let card = document
        .descendants()
        .find(|n| n.attribute("data-terraform-address") == Some(&long))
        .unwrap();
    assert!(
        card.children().any(|n| n.has_tag_name("use"))
            || card
                .descendants()
                .any(|n| n.attribute("href").is_some_and(|s| s.contains("cache")))
    );
    for args in [
        vec![
            "--focus",
            "aws_elasticache_replication_group.main",
            "--focus-depth",
            "0",
        ],
        vec!["--changed-only"],
    ] {
        let mut plan: Value = serde_json::from_str(input).unwrap();
        for change in plan["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .take(4)
        {
            change["change"]["actions"] = json!(["no-op"]);
        }
        let output = support::run_with_args(plan.to_string().as_bytes(), &args);
        assert!(output.status.success());
        let svg = String::from_utf8(output.stdout).unwrap();
        assert!(svg.contains("data-terraform-address=\"aws_elasticache_replication_group.main\""));
        assert!(!svg.contains("data-terraform-address=\"aws_elasticache_subnet_group.main\""));
        assert!(!svg.contains("data-placement=\"inferred\""));
        assert!(!svg.contains("data-terraform-address=\"aws_vpc.main\""));
        assert_eq!(svg.matches("data-terraform-address=").count(), 1);
    }
    let mut plan: Value = serde_json::from_str(input).unwrap();
    for change in plan["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .skip(3)
    {
        change["mode"] = json!("data");
        change["address"] = json!(format!("data.{}", change["address"].as_str().unwrap()));
    }
    let svg = render(plan.to_string().as_bytes());
    for kind in ["subnet_group", "replication_group"] {
        assert!(svg.contains(&format!(
            "data-terraform-address=\"data.aws_elasticache_{kind}.main\""
        )));
    }
}

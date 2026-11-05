use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/efs-configuration-plan.json")).unwrap()
}

fn complete(v: &Value, target: usize, path: &str) -> bool {
    let raw = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&raw);
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == v["resource_changes"][target]["address"].as_str().unwrap()
            && r.attribute == path
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn configured_relationships_require_complete_matching_current_evidence() {
    for (target, path, source, output) in [
        (0, "kms_key_id", 6, "arn"),
        (1, "file_system_id", 0, "id"),
        (1, "subnet_id", 3, "id"),
        (1, "security_groups", 5, "id"),
        (7, "file_system_id", 0, "id"),
        (8, "file_system_id", 0, "id"),
        (11, "source_file_system_id", 0, "id"),
        (11, "destination.file_system_id", 9, "id"),
        (11, "destination.kms_key_id", 10, "name"),
    ] {
        assert!(complete(&fixture(), target, path), "{target} {path}");
        for case in [
            "unknown",
            "source-unknown",
            "mismatch",
            "literal",
            "partial",
            "deleted",
            "deposed",
            "data",
            "region",
        ] {
            let mut v = fixture();
            let first = path.split('.').next().unwrap();
            match case {
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({first:true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("wrong")
                }
                "literal" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"]
                        .as_object_mut()
                        .unwrap()
                        .remove(first);
                }
                "partial" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][first] =
                        json!({"references":["var.unknown"]})
                }
                "deleted" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "region" => {
                    v["configuration"]["root_module"]["resources"][source]["expressions"]["region"] =
                        json!({"constant_value":"eu-west-1"})
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, target, path), "{target} {path} {case}");
        }
    }
}

#[test]
fn encryption_network_and_helper_ownership_are_conservative() {
    let mut v = fixture();
    v["resource_changes"][0]["change"]["after"]["encrypted"] = json!(false);
    assert!(!complete(&v, 0, "kms_key_id"));
    v = fixture();
    v["resource_changes"][5]["change"]["after"]["vpc_id"] = json!("other");
    assert!(!complete(&v, 1, "security_groups"));
    v = fixture();
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 12);
    for kind in ["aws_efs_backup_policy", "aws_efs_file_system_policy"] {
        assert!(
            graph
                .nodes
                .iter()
                .find(|n| n.resource_type == kind)
                .unwrap()
                .member_owner
                .is_some()
        );
        let node = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == kind)
            .unwrap();
        assert_eq!(node.placement, crate::model::Placement::Spatial);
        assert!(crate::view::routes::owner(&graph, node).is_some());
    }
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_efs_replication_configuration")
            .unwrap()
            .member_owner
            .is_none()
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &v.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_POLICY"));
        assert!(!output.contains("PRIVATE_PATH"));
    }
}

#[test]
fn efs_policy_filtering_and_replication_account_evidence_preserve_fallback() {
    let v = fixture();
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    for address in [
        "aws_efs_backup_policy.app",
        "aws_efs_file_system_policy.app",
    ] {
        let selected = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some(address.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.nodes.len(), 1);
        assert!(crate::view::routes::owner(&selected, &selected.nodes[0]).is_none());
    }
    for case in ["unknown", "different", "conflict"] {
        let mut parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        for node in &mut parsed.nodes {
            if node.address == "aws_efs_file_system.replica" {
                let config = node.provider_configuration.as_mut().unwrap();
                match case {
                    "unknown" => config.account = None,
                    "different" => {
                        config.account = crate::model::AwsAccountScope::new("210987654321")
                    }
                    _ => config.account_conflict = true,
                }
            }
        }
        let raw = semantic::Input::new(&parsed);
        let r = raw
            .attributes
            .iter()
            .find(|r| r.attribute == "destination.file_system_id")
            .unwrap();
        assert!(!super::super::service_references::verified(&raw, r));
    }
}

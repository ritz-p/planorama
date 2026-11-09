use crate::{
    model::{EdgeKind, ReferenceKind},
    plan, semantic,
};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/fsx-children-plan.json")).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, index: usize, path: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][index]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == path
                && r.complete
                && Some(r.label)
                    == super::label(v["resource_changes"][index]["type"].as_str().unwrap(), path)
        })
}
const CASES: &[(usize, &str, usize, &str)] = &[
    (1, "file_system_id", 0, "id"),
    (2, "storage_virtual_machine_id", 1, "id"),
    (4, "parent_volume_id", 3, "root_volume_id"),
    (5, "parent_volume_id", 4, "id"),
    (5, "origin_snapshot.snapshot_arn", 6, "arn"),
    (6, "volume_id", 3, "root_volume_id"),
    (7, "volume_id", 5, "id"),
    (10, "file_system_id", 8, "id"),
    (11, "volume_id", 2, "id"),
    (12, "file_system_id", 8, "id"),
    (14, "openzfs_configuration.volume_id", 3, "root_volume_id"),
    (14, "s3_access_point.vpc_configuration.vpc_id", 13, "id"),
    (15, "openzfs_configuration.volume_id", 5, "id"),
    (16, "file_system_id", 9, "id"),
];

#[test]
fn verified_children_keep_membership_lifecycle_and_access_restriction_separate() {
    let v = fixture();
    for &(target, path, _, _) in CASES {
        assert!(complete(&v, target, path), "{target} {path}");
    }
    for (i, path) in [
        (17, "parent_volume_id"),
        (18, "parent_volume_id"),
        (19, "file_system_id"),
    ] {
        assert!(!complete(&v, i, path));
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 20);
    assert_eq!(g.nodes.len(), 20);
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    for i in [1, 2, 4, 5] {
        let node = g
            .nodes
            .iter()
            .find(|n| n.address == v["resource_changes"][i]["address"].as_str().unwrap())
            .unwrap();
        assert!(
            node.references
                .iter()
                .any(|r| r.complete && r.kind == ReferenceKind::Membership)
        );
        assert!(node.member_owner.is_none());
    }
}

#[test]
fn children_require_explicit_unique_compatible_current_outputs() {
    for &(target, path, source, output) in CASES {
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "mismatch",
            "wrong-type",
            "wrong-output",
            "delete",
            "deposed",
            "consumer-delete",
            "consumer-deposed",
            "data",
            "provider",
            "scope",
            "missing",
        ] {
            let mut v = fixture();
            let first = path.split('.').next().unwrap();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"]
                        .as_object_mut()
                        .unwrap()
                        .remove(first);
                }
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({first:true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("wrong")
                }
                "wrong-type" => v["resource_changes"][source]["type"] = json!("aws_s3_bucket"),
                "wrong-output" => {
                    let config =
                        &mut v["configuration"]["root_module"]["resources"][target]["expressions"];
                    let text = config
                        .to_string()
                        .replace(&format!(".{output}\""), ".wrong\"");
                    *config = serde_json::from_str(&text).unwrap();
                }
                "delete" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "consumer-delete" => {
                    v["resource_changes"][target]["change"]["actions"] = json!(["delete"])
                }
                "consumer-deposed" => v["resource_changes"][target]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][source]["provider_name"] = json!("foreign.example/aws")
                }
                "scope" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"},"allowed_account_ids":{"constant_value":["999999999999"]}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("other");
                }
                "missing" => {
                    v["resource_changes"][source]["address"] = json!("aws_s3_bucket.absent")
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, target, path), "{target} {path} {case}");
        }
    }
}

#[test]
fn cycles_discriminators_and_exclusive_backup_sources_are_rejected() {
    let mut ambiguous = fixture();
    ambiguous["configuration"]["root_module"]["resources"][5]["expressions"]["parent_volume_id"] = json!({"references":["aws_fsx_openzfs_volume.parent.id", "aws_fsx_openzfs_file_system.storage.root_volume_id"]});
    assert!(!complete(&ambiguous, 5, "parent_volume_id"));
    let mut dangling = fixture();
    dangling["resource_changes"][4]["change"]["after"]["parent_volume_id"] = json!("vol-cycle");
    dangling["configuration"]["root_module"]["resources"][4]["expressions"]["parent_volume_id"] =
        json!({"references":["aws_fsx_openzfs_volume.cycle.id"]});
    assert!(!complete(&dangling, 4, "parent_volume_id"));
    assert!(!complete(&dangling, 5, "parent_volume_id"));
    let mut v = fixture();
    v["resource_changes"][4]["change"]["after"]["parent_volume_id"] = json!("vol-child");
    v["configuration"]["root_module"]["resources"][4]["expressions"]["parent_volume_id"] =
        json!({"references":["aws_fsx_openzfs_volume.child.id"]});
    assert!(!complete(&v, 4, "parent_volume_id"));
    assert!(!complete(&v, 5, "parent_volume_id"));
    assert!(complete(&v, 5, "origin_snapshot.snapshot_arn"));
    for (i, field, value, path) in [
        (2, "volume_type", "OTHER", "storage_virtual_machine_id"),
        (14, "type", "S3", "openzfs_configuration.volume_id"),
        (10, "volume_id", "vol-ontap", "file_system_id"),
        (11, "file_system_id", "fs-lustre", "volume_id"),
    ] {
        let mut v = fixture();
        v["resource_changes"][i]["change"]["after"][field] = json!(value);
        assert!(!complete(&v, i, path));
    }
    let mut v = fixture();
    v["resource_changes"][5]["change"]["after"]["origin_snapshot"][0]["copy_strategy"] =
        json!("UNKNOWN");
    assert!(!complete(&v, 5, "origin_snapshot.snapshot_arn"));
    assert!(complete(&v, 5, "parent_volume_id"));
    let mut v = fixture();
    v["resource_changes"][8]["type"] = json!("aws_fsx_openzfs_file_system");
    assert!(!complete(&v, 10, "file_system_id"));
    assert!(!complete(&v, 12, "file_system_id"));
}

#[test]
fn filtering_and_named_states_do_not_invent_owners_or_expose_values() {
    let v = fixture();
    let g = graph(&v);
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_fsx_openzfs_volume.child".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        selected
            .nodes
            .iter()
            .filter(|n| n.address == "aws_fsx_openzfs_volume.child")
            .count(),
        1
    );
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.from < selected.nodes.len() && e.to < selected.nodes.len())
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: v.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("arn:aws:"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    }
}

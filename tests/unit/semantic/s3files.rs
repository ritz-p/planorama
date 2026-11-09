use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/s3files-plan.json");
fn pair(s: usize, t: usize) -> Value {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut p = v.clone();
    p["resource_changes"] = json!([v["resource_changes"][s], v["resource_changes"][t]]);
    p["configuration"]["root_module"]["resources"] = json!([
        v["configuration"]["root_module"]["resources"][s],
        v["configuration"]["root_module"]["resources"][t]
    ]);
    p
}
#[test]
fn filesystem_configuration_uses_exact_current_outputs_without_efs_aliasing() {
    for (s, t, path, label) in [
        (0, 3, "bucket", "Backing bucket"),
        (1, 3, "role_arn", "Configured access role"),
        (2, 3, "kms_key_id", "File system encryption key"),
        (3, 4, "file_system_id", "S3 Files file system"),
        (3, 5, "file_system_id", "S3 Files file system"),
        (3, 6, "file_system_id", "S3 Files file system"),
        (3, 10, "file_system_id", "S3 Files file system"),
    ] {
        super::super::service_references::tests::check_service_fixture(
            &pair(s, t).to_string(),
            label,
            path,
        );
    }
    let mut v = pair(3, 4);
    v["resource_changes"][0]["type"] = json!("aws_efs_file_system");
    assert!(
        !semantic::transform(&plan::parse(&v.to_string()).unwrap())
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "S3 Files file system" && r.complete)
    );
    let mut v = pair(2, 3);
    v["resource_changes"][1]["change"]["after"]["kms_key_id"] = json!("key");
    v["configuration"]["root_module"]["resources"][1]["expressions"]["kms_key_id"] =
        json!({"references":["aws_kms_key.main.id"]});
    assert!(
        !semantic::transform(&plan::parse(&v.to_string()).unwrap())
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "File system encryption key" && r.complete)
    );
}
#[test]
fn mount_target_requires_complete_subnet_security_group_and_common_vpc_proof() {
    for case in 0..10 {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            1 => {
                v["resource_changes"][10]["change"]["after"]["security_groups"] =
                    json!(["sg-files", "literal"])
            }
            2 => v["resource_changes"][9]["change"]["after"]["vpc_id"] = json!("different"),
            3 => {
                v["resource_changes"][10]["change"]["after_unknown"] =
                    json!({"security_groups":true})
            }
            4 => v["resource_changes"][8]["change"]["after_sensitive"] = json!({"id":true}),
            5 => v["resource_changes"][7]["change"]["actions"] = json!(["delete"]),
            6 => {
                v["configuration"]["root_module"]["resources"][9]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-files"})
            }
            7 => v["resource_changes"][9]["deposed"] = json!("old"),
            8 => v["resource_changes"][10]["provider_name"] = json!("example.com/foreign/aws"),
            9 => {
                v["resource_changes"][10]["change"]["after"]["subnet_id"] = json!(["subnet-files"])
            }
            _ => {}
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.coverage.selected.resources, 11);
        assert_eq!(
            g.edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment
                    && g.nodes[e.to].resource_type == "aws_s3files_mount_target"
                    && g.nodes[e.from].resource_type == "aws_subnet"),
            case == 0,
            "case {case}"
        );
        assert!(
            !g.edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment
                    && matches!(
                        g.nodes[e.to].resource_type.as_str(),
                        "aws_s3files_file_system" | "aws_s3_bucket"
                    ))
        );
    }
}
#[test]
fn independent_storage_helpers_and_payloads_survive_rendering() {
    let p = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&p);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        3
    );
    assert!(
        g.nodes
            .iter()
            .filter(|n| matches!(
                n.resource_type.as_str(),
                "aws_s3files_file_system" | "aws_s3files_mount_target"
            ))
            .all(|n| n.placement == crate::model::Placement::Spatial && n.member_owner.is_none())
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_FILES_PAYLOAD"));
        assert!(!out.contains("eni-computed"));
    }
    let mut changed = p;
    changed.nodes[9]
        .provider_configuration
        .as_mut()
        .unwrap()
        .account_conflict = true;
    let g = semantic::transform(&changed);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && g.nodes[e.to].resource_type == "aws_s3files_mount_target")
    );
}

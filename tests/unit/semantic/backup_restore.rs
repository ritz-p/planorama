use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/backup-restore-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap())
}
fn proven(v: &Value, path: &str) -> bool {
    graph(v).nodes.iter().any(|n| {
        n.references.iter().any(|r| {
            r.attribute == path
                && Some(r.label) == super::label(&n.resource_type, path)
                && r.complete
                && !r.dependency
                && !r.targets.is_empty()
        })
    })
}
#[test]
fn restore_testing_references_keep_plan_ownership_and_shared_endpoints_separate() {
    let v = fixture();
    for path in [
        super::VAULTS,
        "restore_testing_plan_name",
        "iam_role_arn",
        super::PROTECTED,
    ] {
        assert!(proven(&v, path), "{path}");
    }
    let raw = crate::plan::parse(FIXTURE).unwrap();
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 5);
    for n in &raw.nodes {
        let found: Vec<_> = g
            .nodes
            .iter()
            .filter(|r| r.address == n.address && r.deposed_key == n.deposed_key)
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].action, n.action);
    }
    assert!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == super::SELECTION)
            .unwrap()
            .member_owner
            .is_some()
    );
    for kind in ["aws_backup_vault", "aws_iam_role", "aws_efs_file_system"] {
        assert!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == kind)
                .unwrap()
                .member_owner
                .is_none()
        );
    }
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
}
#[test]
fn restore_references_require_complete_current_typed_value_proof() {
    for (source, target, path, field) in [
        (0, 3, super::VAULTS, "arn"),
        (1, 4, "iam_role_arn", "arn"),
        (2, 4, super::PROTECTED, "arn"),
        (3, 4, "restore_testing_plan_name", "name"),
    ] {
        for case in 0..13 {
            let mut v = fixture();
            match case {
                0 => v["resource_changes"][source]["change"]["after"][field] = json!("other"),
                1 => v["resource_changes"][source]["change"]["after_unknown"] = json!({field:true}),
                2 => {
                    v["resource_changes"][source]["change"]["after_sensitive"] = json!({field:true})
                }
                3 => v["resource_changes"][source]["mode"] = json!("data"),
                4 => v["resource_changes"][source]["deposed"] = json!("old"),
                5 => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                6 => v["resource_changes"][source]["type"] = json!("aws_sqs_queue"),
                7 => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                8 => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"] =
                        json!({})
                }
                9 => v["resource_changes"][target]["change"]["after_unknown"] = json!(true),
                10 => v["resource_changes"][target]["change"]["after_sensitive"] = json!(true),
                11 => {
                    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("aws.other");
                }
                _ => v["resource_changes"][target]["change"]["actions"] = json!(["delete"]),
            }
            assert!(!proven(&v, path), "{path} case {case}");
        }
    }
}
#[test]
fn vault_navigation_is_concrete_complete_and_exclusion_aware() {
    let base = fixture();
    let arn = base["resource_changes"][0]["change"]["after"]["arn"].clone();
    for case in 0..9 {
        let mut v = base.clone();
        match case {
            0 => {
                v["resource_changes"][3]["change"]["after"]["recovery_point_selection"][0]["include_vaults"] =
                    json!(["*"])
            }
            1 => {
                v["resource_changes"][3]["change"]["after"]["recovery_point_selection"][0]["exclude_vaults"] =
                    json!(["*"])
            }
            2 => {
                v["resource_changes"][3]["change"]["after"]["recovery_point_selection"][0]["exclude_vaults"] =
                    json!([arn])
            }
            3 => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"recovery_point_selection":[{"exclude_vaults":true}]})
            }
            4 => {
                v["resource_changes"][3]["change"]["after_sensitive"] =
                    json!({"recovery_point_selection":[{"include_vaults":true}]})
            }
            5 => {
                v["resource_changes"][3]["change"]["after"]["recovery_point_selection"][0]["include_vaults"] =
                    json!([
                        arn,
                        "arn:aws:backup:us-east-1:111111111111:backup-vault:unmanaged"
                    ])
            }
            6 => {
                v["resource_changes"][3]["change"]["after"]["recovery_point_selection"][0]["include_vaults"] =
                    json!([])
            }
            7 => v["resource_changes"][3]["change"]["after"]["recovery_point_selection"]
                .as_array_mut()
                .unwrap()
                .push(json!({"include_vaults":[arn]})),
            _ => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"recovery_point_selection":[{"include_vaults":[true]}]})
            }
        }
        assert!(!proven(&v, super::VAULTS), "case {case}");
    }
    let mut disjoint = base;
    disjoint["resource_changes"][3]["change"]["after"]["recovery_point_selection"][0]["exclude_vaults"] =
        json!(["arn:aws:backup:us-east-1:111111111111:backup-vault:other"]);
    assert!(proven(&disjoint, super::VAULTS));
}
#[test]
fn protected_resource_type_and_conditions_never_guess_endpoints_or_subnets() {
    for case in 0..7 {
        let mut v = fixture();
        match case {
            0 => {
                v["resource_changes"][4]["change"]["after"]["protected_resource_type"] =
                    json!("EC2")
            }
            1 => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"protected_resource_type":true})
            }
            2 => {
                v["resource_changes"][4]["change"]["after"]["protected_resource_arns"] =
                    json!(["*"])
            }
            3 => {
                v["resource_changes"][4]["change"]["after"]["protected_resource_conditions"] =
                    json!([{"string_equals":[{"key":"PRIVATE_KEY","value":"PRIVATE_VALUE"}]}])
            }
            4 => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"protected_resource_conditions":true})
            }
            5 => v["resource_changes"][4]["change"]["after"]["protected_resource_arns"] = json!([]),
            _ => v["resource_changes"][4]["change"]["after"]["protected_resource_arns"]
                .as_array_mut()
                .unwrap()
                .push(json!(
                    "arn:aws:elasticfilesystem:us-east-1:111111111111:file-system/missing"
                )),
        }
        assert!(!proven(&v, super::PROTECTED), "case {case}");
        assert!(proven(&v, "restore_testing_plan_name"));
    }
    let g = graph(&fixture());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_backup_restore_testing_selection.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert_eq!(selected.nodes.len(), 1);
}

#[test]
fn protected_types_are_qualified_and_named_states_do_not_supply_missing_endpoints() {
    for (resource_type, kind, arn) in [
        (
            "EC2",
            "aws_instance",
            "arn:aws:ec2:us-east-1:111111111111:instance/i-protected",
        ),
        (
            "EBS",
            "aws_ebs_volume",
            "arn:aws:ec2:us-east-1:111111111111:volume/vol-protected",
        ),
        (
            "RDS",
            "aws_db_instance",
            "arn:aws:rds:us-east-1:111111111111:db:protected",
        ),
        (
            "Aurora",
            "aws_rds_cluster",
            "arn:aws:rds:us-east-1:111111111111:cluster:protected",
        ),
        (
            "DocumentDB",
            "aws_docdb_cluster",
            "arn:aws:rds:us-east-1:111111111111:cluster:protected",
        ),
        (
            "Neptune",
            "aws_neptune_cluster",
            "arn:aws:rds:us-east-1:111111111111:cluster:protected",
        ),
        (
            "DynamoDB",
            "aws_dynamodb_table",
            "arn:aws:dynamodb:us-east-1:111111111111:table/protected",
        ),
        ("S3", "aws_s3_bucket", "arn:aws:s3:::protected"),
        (
            "FSx",
            "aws_fsx_lustre_file_system",
            "arn:aws:fsx:us-east-1:111111111111:file-system/fs-protected",
        ),
        (
            "FSx",
            "aws_fsx_windows_file_system",
            "arn:aws:fsx:us-east-1:111111111111:file-system/fs-protected",
        ),
        (
            "FSx",
            "aws_fsx_ontap_file_system",
            "arn:aws:fsx:us-east-1:111111111111:file-system/fs-protected",
        ),
        (
            "FSx",
            "aws_fsx_openzfs_file_system",
            "arn:aws:fsx:us-east-1:111111111111:file-system/fs-protected",
        ),
    ] {
        let mut v = fixture();
        let address = format!("{kind}.protected");
        v["resource_changes"][2]["address"] = json!(address);
        v["resource_changes"][2]["type"] = json!(kind);
        v["resource_changes"][2]["change"]["after"]["arn"] = json!(arn);
        v["configuration"]["root_module"]["resources"][2]["address"] = json!(address);
        v["configuration"]["root_module"]["resources"][4]["expressions"]["protected_resource_arns"] =
            json!({"references":[format!("{address}.arn")]});
        v["resource_changes"][4]["change"]["after"]["protected_resource_type"] =
            json!(resource_type);
        v["resource_changes"][4]["change"]["after"]["protected_resource_arns"] = json!([arn]);
        if resource_type == "Aurora" {
            v["resource_changes"][2]["change"]["after"]["engine"] = json!("aurora-postgresql");
        }
        assert!(proven(&v, super::PROTECTED), "{resource_type} {kind}");
        if resource_type == "Aurora" {
            v["resource_changes"][2]["change"]["after_unknown"] = json!({"engine":true});
            assert!(!proven(&v, super::PROTECTED));
        }
    }
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(2);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "first".into(),
                    json: FIXTURE.into(),
                },
                crate::NamedInput {
                    state: "second".into(),
                    json: missing.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("<metadata id=\"cross-state-relationships\">"));
    }
}

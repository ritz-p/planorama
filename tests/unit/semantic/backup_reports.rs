use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/backup-report-plan.json");
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
fn complete_report_references_keep_independent_actions_scope_and_coverage() {
    let v = fixture();
    assert!(proven(&v, super::BUCKET));
    assert!(proven(&v, super::FRAMEWORKS));
    let raw = crate::plan::parse(FIXTURE).unwrap();
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 4);
    for n in &raw.nodes {
        let nodes: Vec<_> = g
            .nodes
            .iter()
            .filter(|a| a.address == n.address && a.deposed_key == n.deposed_key)
            .collect();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, n.action);
        assert!(nodes[0].member_owner.is_none());
    }
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    assert!(
        !g.nodes.iter().flat_map(|n| &n.references).any(|r| matches!(
            r.attribute.as_str(),
            "report_setting.accounts"
                | "report_setting.organization_units"
                | "report_setting.regions"
        ) && !r.targets.is_empty())
    );
}
#[test]
fn report_endpoints_require_complete_explicit_current_typed_evidence() {
    for (source, path, block, field) in [
        (
            0,
            super::BUCKET,
            "report_delivery_channel",
            "s3_bucket_name",
        ),
        (1, super::FRAMEWORKS, "report_setting", "framework_arns"),
    ] {
        for case in 0..13 {
            let mut v = fixture();
            let output = if source == 0 { "bucket" } else { "arn" };
            match case {
                0 => v["resource_changes"][source]["change"]["after"][output] = json!("other"),
                1 => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                2 => {
                    v["resource_changes"][source]["change"]["after_sensitive"] =
                        json!({output:true})
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
                    v["configuration"]["root_module"]["resources"][3]["expressions"][block] =
                        json!([])
                }
                9 => v["resource_changes"][3]["change"]["after_unknown"] = json!({block:true}),
                10 => {
                    v["resource_changes"][3]["change"]["after_sensitive"] =
                        json!({block:[{field:true}]})
                }
                11 => {
                    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("aws.other");
                }
                _ => v["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            }
            assert!(!proven(&v, path), "{path} case {case}");
        }
    }
}
#[test]
fn framework_collection_rejects_partial_ambiguous_and_wildcard_evidence() {
    for case in 0..6 {
        let mut v = fixture();
        match case {
            0 => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["report_setting"]
                    [0]["framework_arns"] = json!({"references":["aws_backup_framework.first.arn"]})
            }
            1 => {
                v["resource_changes"][3]["change"]["after"]["report_setting"][0]["framework_arns"] =
                    json!(["*"])
            }
            2 => {
                v["resource_changes"][2]["change"]["after"]["arn"] =
                    v["resource_changes"][1]["change"]["after"]["arn"].clone()
            }
            3 => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"report_setting":[{"framework_arns":[false,true]}]})
            }
            4 => v["resource_changes"][3]["change"]["after"]["report_setting"]
                .as_array_mut()
                .unwrap()
                .push(json!({"framework_arns":[]})),
            _ => {
                v["resource_changes"][3]["change"]["after"]["report_setting"][0]["framework_arns"] =
                    json!([])
            }
        }
        assert!(!proven(&v, super::FRAMEWORKS), "case {case}");
    }
}

#[test]
fn report_references_preserve_account_and_filtered_state_boundaries() {
    let mut raw = crate::plan::parse(FIXTURE).unwrap();
    for node in &mut raw.nodes {
        node.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("111111111111");
    }
    raw.nodes
        .iter_mut()
        .find(|n| n.resource_type == super::KIND)
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("222222222222");
    let g = crate::semantic::transform(&raw);
    assert!(
        !g.nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Backup report delivery bucket" && r.complete)
    );
    let g = graph(&fixture());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_backup_report_plan.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert_eq!(selected.nodes.len(), 1);
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
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

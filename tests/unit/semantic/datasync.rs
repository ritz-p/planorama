use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/datasync-plan.json")).unwrap()
}

#[test]
fn datasync_task_accepts_only_typed_locations_and_singleton_reports() {
    let original = fixture();
    let address = "aws_datasync_task.sync";
    let index = original["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["address"] == address)
        .unwrap();
    for path in [
        "source_location_arn",
        "destination_location_arn",
        "cloudwatch_log_group_arn",
        "task_report_config.s3_destination.s3_bucket_arn",
        "task_report_config.s3_destination.bucket_access_role_arn",
    ] {
        assert!(complete(&original, address, path), "{path}");
    }
    for location in original["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| {
            n["type"]
                .as_str()
                .unwrap()
                .starts_with("aws_datasync_location_")
        })
    {
        let mut v = original.clone();
        v["resource_changes"][index]["change"]["after"]["source_location_arn"] =
            location["change"]["after"]["arn"].clone();
        v["configuration"]["root_module"]["resources"][index]["expressions"]["source_location_arn"] =
            json!({"references":[format!("{}.arn",location["address"].as_str().unwrap())]});
        assert!(
            complete(&v, address, "source_location_arn"),
            "{}",
            location["type"]
        );
    }
    for case in [
        "unknown-parent",
        "multiple-parents",
        "multiple-destinations",
        "object-parent",
        "unknown-role",
    ] {
        let mut v = original.clone();
        let report =
            v["resource_changes"][index]["change"]["after"]["task_report_config"][0].clone();
        match case {
            "unknown-parent" => v["resource_changes"][index]["change"]["after_unknown"] = json!({"task_report_config":true}),
            "multiple-parents" => v["resource_changes"][index]["change"]["after"]["task_report_config"].as_array_mut().unwrap().push(report),
            "multiple-destinations" => v["resource_changes"][index]["change"]["after"]["task_report_config"][0]["s3_destination"].as_array_mut().unwrap().push(report["s3_destination"][0].clone()),
            "object-parent" => v["resource_changes"][index]["change"]["after"]["task_report_config"] = report,
            "unknown-role" => v["resource_changes"][index]["change"]["after_unknown"] = json!({"task_report_config":[{"s3_destination":[{"bucket_access_role_arn":true}]}]}),
            _ => unreachable!(),
        }
        assert!(
            !complete(
                &v,
                address,
                "task_report_config.s3_destination.bucket_access_role_arn"
            ),
            "{case}"
        );
    }
}
fn complete(v: &Value, address: &str, path: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == path
                && r.complete
                && super::label(address.split('.').next().unwrap(), path) == Some(r.label)
        })
}

#[test]
fn datasync_typed_references_require_complete_current_explicit_evidence() {
    let original = fixture();
    let parsed = plan::parse(&original.to_string()).unwrap();
    let mut count = 0;
    for r in &parsed.attributes {
        let address = &parsed.nodes[r.target].address;
        if super::label(&parsed.nodes[r.target].resource_type, &r.attribute).is_none() {
            continue;
        }
        count += 1;
        assert!(
            complete(&original, address, &r.attribute),
            "{address} {}",
            r.attribute
        );
        let index = original["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n["address"] == *address)
            .unwrap();
        let source = original["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n["address"] == parsed.nodes[r.sources[0]].address)
            .unwrap();
        for case in [
            "unknown",
            "mismatch",
            "delete",
            "deposed",
            "data",
            "foreign",
            "literal",
            "scope",
            "wrong-kind",
        ] {
            let mut v = original.clone();
            match case {
                "unknown" => v["resource_changes"][index]["change"]["after_unknown"] = json!(true),
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][if r.attribute
                        == "vpc_endpoint_id"
                    {
                        "id"
                    } else {
                        "arn"
                    }] = json!("different")
                }
                "delete" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "foreign" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "literal" => {
                    v["configuration"]["root_module"]["resources"][index]["expressions"] = json!({})
                }
                "wrong-kind" => v["resource_changes"][source]["type"] = json!("aws_iam_policy"),
                "scope" => {
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("missing")
                }
                _ => unreachable!(),
            }
            assert!(
                !complete(&v, address, &r.attribute),
                "{address} {} {case}",
                r.attribute
            );
        }
    }
    assert!(count >= 25, "{count}");
}

#[test]
fn datasync_preserves_independent_identity_and_private_values() {
    let v = fixture();
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources as usize, parsed.nodes.len());
    for node in g
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_datasync_"))
    {
        assert!(node.member_owner.is_none());
        if node.resource_type == "aws_datasync_agent" {
            assert_eq!(node.placement, crate::model::Placement::Spatial);
        }
    }
    for edge in g.edges.iter().filter(|e| {
        e.kind == crate::model::EdgeKind::Containment
            && g.nodes[e.to].resource_type.starts_with("aws_datasync_")
    }) {
        assert_eq!(g.nodes[edge.from].address, "aws_vpc.main");
        assert!(matches!(
            g.nodes[edge.to].resource_type.as_str(),
            "aws_datasync_agent" | "aws_datasync_location_efs"
        ));
    }
    assert!(
        g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && g.nodes[e.to].address == "aws_datasync_agent.one")
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            &v.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!out.content.contains("PRIVATE_"));
    }
}

#[test]
fn datasync_preserves_filtering_named_states_and_lifecycle() {
    let mut v = fixture();
    let mut old = v["resource_changes"][8].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for source in &parsed.nodes {
        let matches = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].action, source.action);
    }
    let filtered = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_datasync_location_efs.shared".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        filtered
            .edges
            .iter()
            .all(|e| e.from < filtered.nodes.len() && e.to < filtered.nodes.len())
    );
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
        &crate::Options::default(),
    )
    .unwrap();
    assert!(out.content.contains("data-state-id=\"one\""));
    assert!(out.content.contains("data-state-id=\"two\""));
    assert!(
        !out.content
            .contains("<metadata id=\"cross-state-relationships\">")
    );
}

#[test]
fn datasync_collections_types_and_storage_consistency_fail_closed() {
    for case in [
        "partial",
        "duplicate",
        "unknown-element",
        "wrong-type",
        "wrong-fs",
        "wrong-encryption",
        "ontap-scope",
        "endpoint-mode",
        "ambiguous",
    ] {
        let mut v = fixture();
        let (address, path) = match case {
            "wrong-fs" | "wrong-encryption" => {
                ("aws_datasync_location_efs.shared", "access_point_arn")
            }
            "ontap-scope" => (
                "aws_datasync_location_fsx_ontap_file_system.shared",
                "storage_virtual_machine_arn",
            ),
            "endpoint-mode" => ("aws_datasync_agent.one", "vpc_endpoint_id"),
            _ => ("aws_datasync_agent.one", "security_group_arns"),
        };
        let index = v["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n["address"] == address)
            .unwrap();
        match case {
            "partial" => v["resource_changes"][index]["change"]["after"]["security_group_arns"]
                .as_array_mut()
                .unwrap()
                .push(json!("literal")),
            "duplicate" => {
                v["resource_changes"][index]["change"]["after"]["security_group_arns"][1] =
                    v["resource_changes"][index]["change"]["after"]["security_group_arns"][0]
                        .clone()
            }
            "unknown-element" => {
                v["resource_changes"][index]["change"]["after_unknown"] =
                    json!({"security_group_arns":[false,true]})
            }
            "wrong-type" => v["resource_changes"][2]["type"] = json!("aws_subnet"),
            "wrong-fs" => {
                v["resource_changes"][7]["change"]["after"]["file_system_id"] = json!("fs-other")
            }
            "wrong-encryption" => {
                v["resource_changes"][index]["change"]["after"]["in_transit_encryption"] =
                    json!("NONE")
            }
            "ontap-scope" => {
                v["resource_changes"][index]["change"]["after"]["fsx_filesystem_arn"] =
                    json!("arn:aws:fsx:us-west-2:999999999999:file-system/fs-ontap")
            }
            "endpoint-mode" => {
                v["resource_changes"][4]["change"]["after"]["vpc_endpoint_type"] = json!("Gateway")
            }
            "ambiguous" => {
                let duplicate = v["resource_changes"][2].clone();
                v["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            _ => unreachable!(),
        }
        if case == "ambiguous" {
            assert!(plan::parse(&v.to_string()).is_err());
            continue;
        }
        assert!(!complete(&v, address, path), "{case}");
    }
}

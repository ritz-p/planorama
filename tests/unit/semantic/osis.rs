use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/osis-plan.json");
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, target: usize, path: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][target]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == path
                && r.complete
                && Some(r.label)
                    == super::label(
                        v["resource_changes"][target]["type"].as_str().unwrap(),
                        path,
                    )
        })
}
fn parent(v: &Value, target: usize) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment
                && g.nodes[e.to].address == v["resource_changes"][target]["address"]
        })
        .map(|e| g.nodes[e.from].address.clone())
}
fn set(root: &mut Value, path: &str, value: Value) {
    let mut parts = path.split('.').peekable();
    let mut at = root;
    while let Some(p) = parts.next() {
        if parts.peek().is_none() {
            at[p] = value;
            return;
        }
        at = &mut at[p][0];
    }
}
const REFERENCES: &[(usize, usize, &str, &str)] = &[
    (7, 1, "vpc_options.subnet_ids", "id"),
    (7, 3, "vpc_options.security_group_ids", "id"),
    (7, 4, "pipeline_role_arn", "arn"),
    (7, 5, "encryption_at_rest_options.kms_key_arn", "arn"),
    (
        7,
        6,
        "log_publishing_options.cloudwatch_log_destination.log_group",
        "name",
    ),
    (8, 7, "pipeline_arn", "pipeline_arn"),
    (8, 1, "vpc_options.subnet_ids", "id"),
    (8, 3, "vpc_options.security_group_ids", "id"),
    (9, 7, "resource_arn", "pipeline_arn"),
];
#[test]
fn osis_links_exact_outputs_and_proves_independent_vpc_context() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for &(target, _, path, _) in REFERENCES {
        assert!(complete(&v, target, path), "{target} {path}");
    }
    for target in [7, 8] {
        assert_eq!(parent(&v, target).as_deref(), Some("aws_vpc.main"));
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 10);
    for n in &g.nodes {
        assert!(n.member_owner.is_none());
        if n.resource_type == "aws_osis_pipeline" {
            assert_eq!(n.placement, Placement::Spatial);
        }
        if n.resource_type == "aws_osis_resource_policy" {
            assert_eq!(n.placement, Placement::Logical);
        }
    }
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
        assert!(!out.contains("PRIVATE_"));
        assert!(out.contains("Pipeline log group"));
    }
}
#[test]
fn osis_references_require_exact_complete_compatible_current_evidence() {
    for &(target, source, path, output) in REFERENCES {
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "wrong-output",
            "wrong-type",
            "mismatch",
            "deleted",
            "consumer-deleted",
            "deposed",
            "consumer-deposed",
            "data",
            "consumer-data",
            "provider",
            "region",
            "account",
            "missing",
            "duplicate",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => set(
                    &mut v["configuration"]["root_module"]["resources"][target]["expressions"],
                    path,
                    json!({"constant_value":"literal"}),
                ),
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] =
                        json!({path.split('.').next().unwrap():true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "wrong-output" => {
                    let a = v["resource_changes"][source]["address"]
                        .as_str()
                        .unwrap()
                        .to_string();
                    set(
                        &mut v["configuration"]["root_module"]["resources"][target]["expressions"],
                        path,
                        json!({"references":[format!("{a}.wrong")]}),
                    );
                }
                "wrong-type" => {
                    v["resource_changes"][source]["type"] = json!("aws_opensearch_domain")
                }
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("different")
                }
                "deleted" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "consumer-deleted" => {
                    v["resource_changes"][target]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "consumer-deposed" => v["resource_changes"][target]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "consumer-data" => v["resource_changes"][target]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "missing" => {
                    v["resource_changes"][source]["address"] =
                        json!("module.other.aws_osis_pipeline.other")
                }
                "region" | "account" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":if case=="region"{"us-west-2"}else{"us-east-1"}},"allowed_account_ids":{"constant_value":["999999999999"]}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("other");
                }
                "duplicate" => {
                    let n = v["resource_changes"][source].clone();
                    v["resource_changes"].as_array_mut().unwrap().push(n);
                    assert!(plan::parse(&v.to_string()).is_err());
                    continue;
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, target, path), "{target} {path} {case}");
        }
    }
}
#[test]
fn osis_network_distinguishes_management_and_requires_complete_sets() {
    for target in [7, 8] {
        for case in [
            "partial",
            "duplicate",
            "empty",
            "missing-sg",
            "unknown-member",
            "object",
            "multiple",
            "split-vpc",
            "literal-vpc",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "partial" => {
                    v["resource_changes"][target]["change"]["after"]["vpc_options"][0]["subnet_ids"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("literal"))
                }
                "duplicate" => {
                    v["resource_changes"][target]["change"]["after"]["vpc_options"][0]["subnet_ids"] =
                        json!(["subnet-a", "subnet-a"])
                }
                "empty" => {
                    v["resource_changes"][target]["change"]["after"]["vpc_options"][0]["subnet_ids"] =
                        json!([])
                }
                "missing-sg" => {
                    v["resource_changes"][target]["change"]["after"]["vpc_options"][0]["security_group_ids"] =
                        Value::Null
                }
                "unknown-member" => {
                    v["resource_changes"][target]["change"]["after_unknown"] =
                        json!({"vpc_options":[{"subnet_ids":[false,true]}]})
                }
                "object" => {
                    v["resource_changes"][target]["change"]["after"]["vpc_options"] =
                        v["resource_changes"][target]["change"]["after"]["vpc_options"][0].clone()
                }
                "multiple" => {
                    let n =
                        v["resource_changes"][target]["change"]["after"]["vpc_options"][0].clone();
                    v["resource_changes"][target]["change"]["after"]["vpc_options"]
                        .as_array_mut()
                        .unwrap()
                        .push(n);
                }
                "split-vpc" => {
                    v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("other-vpc")
                }
                "literal-vpc" => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"]["vpc_id"] =
                        json!({"constant_value":"vpc-main"})
                }
                _ => unreachable!(),
            }
            assert!(parent(&v, target).is_none(), "{target} {case}");
        }
    }
    for mode in [json!("CUSTOMER"), Value::Null, json!("OTHER")] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][7]["change"]["after"]["vpc_options"][0]["vpc_endpoint_management"] =
            mode;
        assert!(complete(&v, 7, super::SUBNETS));
        assert!(parent(&v, 7).is_none());
        assert!(parent(&v, 8).is_some());
    }
    for unknown in [false, true] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        if unknown {
            v["resource_changes"][8]["change"]["after_unknown"] = json!({"vpc_id":true});
        } else {
            v["resource_changes"][8]["change"]["after"]["vpc_id"] = json!("other");
        }
        assert!(complete(&v, 8, super::SUBNETS));
        assert!(parent(&v, 8).is_none());
    }
}
#[test]
fn osis_enabled_logs_and_typed_arns_do_not_read_payloads_or_deprecated_ids() {
    for enabled in [Value::Null, json!(false)] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][7]["change"]["after"]["log_publishing_options"][0]["is_logging_enabled"] =
            enabled;
        assert!(!complete(&v, 7, REFERENCES[4].2));
    }
    for value in [
        "ingestion",
        "arn:aws:es:us-east-1:123456789012:domain/example",
        "arn:aws:osis:us-east-1:123456789012:pipeline/*",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][7]["change"]["after"]["pipeline_arn"] = json!(value);
        v["resource_changes"][8]["change"]["after"]["pipeline_arn"] = json!(value);
        v["resource_changes"][9]["change"]["after"]["resource_arn"] = json!(value);
        assert!(!complete(&v, 8, "pipeline_arn"));
        assert!(!complete(&v, 9, "resource_arn"));
    }
}
#[test]
fn osis_preserves_lifecycle_selection_and_named_state_identity() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][7]["previous_address"] = json!("aws_osis_pipeline.old");
    v["resource_changes"][7]["change"]["actions"] = json!(["delete", "create"]);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in &parsed.nodes {
        let matches: Vec<_> = g
            .nodes
            .iter()
            .filter(|other| other.address == n.address && other.deposed_key == n.deposed_key)
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].action, n.action);
        assert_eq!(matches[0].previous_address, n.previous_address);
    }
    for address in [
        "aws_osis_pipeline.ingestion",
        "aws_osis_pipeline_endpoint.endpoint",
    ] {
        let selected = view::single(
            &g,
            &view::Options {
                focus: Some(address.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            !selected
                .edges
                .iter()
                .any(|e| e.kind == EdgeKind::Containment)
        );
    }
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(7);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
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
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    }
}

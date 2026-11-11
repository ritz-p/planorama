use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/transfer-webapp-plan.json");
const REFERENCES: &[(usize, usize, &str, &str)] = &[
    (7, 6, super::INSTANCE, "arns"),
    (7, 5, super::ROLE, "arn"),
    (7, 0, super::VPC, "id"),
    (7, 2, super::SUBNETS, "id"),
    (7, 4, super::GROUPS, "id"),
    (8, 7, "web_app_id", "web_app_id"),
    (9, 6, super::INSTANCE, "arns"),
    (9, 5, super::ROLE, "arn"),
];
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, t: usize, p: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == p
                && r.complete
                && Some(r.label)
                    == super::label(v["resource_changes"][t]["type"].as_str().unwrap(), p)
        })
}
fn set(root: &mut Value, path: &str, value: Value) {
    let mut at = root;
    let mut parts = path.split('.').peekable();
    while let Some(p) = parts.next() {
        if parts.peek().is_none() {
            at[p] = value;
            return;
        }
        at = &mut at[p][0];
    }
}
#[test]
fn transfer_webapp_requires_exact_current_configured_outputs() {
    for &(t, s, p, output) in REFERENCES {
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "wrong-output",
            "wrong-type",
            "mismatch",
            "delete",
            "deposed",
            "data",
            "consumer-delete",
            "consumer-deposed",
            "consumer-data",
            "provider",
            "region",
            "binding",
            "ambiguous",
            "shape",
        ] {
            let mut v = fixture();
            match case {
                "literal" | "ambiguous" => set(
                    &mut v["configuration"]["root_module"]["resources"][t]["expressions"],
                    p,
                    if case == "literal" {
                        json!({"constant_value":"literal"})
                    } else {
                        json!({"references":["var.dynamic"]})
                    },
                ),
                "unknown" => {
                    v["resource_changes"][t]["change"]["after_unknown"] =
                        json!({p.split('.').next().unwrap():true})
                }
                "source-unknown" => {
                    v["resource_changes"][s]["change"]["after_unknown"] = json!({output:true})
                }
                "wrong-output" => {
                    let a = v["resource_changes"][s]["address"]
                        .as_str()
                        .unwrap()
                        .to_string();
                    set(
                        &mut v["configuration"]["root_module"]["resources"][t]["expressions"],
                        p,
                        json!({"references":[format!("{a}.wrong")]}),
                    );
                }
                "wrong-type" => v["resource_changes"][s]["type"] = json!("aws_sqs_queue"),
                "mismatch" => {
                    v["resource_changes"][s]["change"]["after"][output] = json!("different")
                }
                "delete" | "consumer-delete" => {
                    v["resource_changes"][if case == "delete" { s } else { t }]["change"]["actions"] =
                        json!(["delete"])
                }
                "deposed" | "consumer-deposed" => {
                    v["resource_changes"][if case == "deposed" { s } else { t }]["deposed"] =
                        json!("old")
                }
                "data" | "consumer-data" => {
                    v["resource_changes"][if case == "data" { s } else { t }]["mode"] =
                        json!(if case == "data" && s == 6 {
                            "managed"
                        } else {
                            "data"
                        })
                }
                "provider" => {
                    v["resource_changes"][s]["provider_name"] = json!("example.org/foreign/aws")
                }
                "region" | "binding" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":if case=="region"{"us-west-2"}else{"us-east-1"}},"allowed_account_ids":{"constant_value":["999999999999"]}}});
                    v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                        json!("other");
                }
                "shape" => {
                    let value = v["resource_changes"][s]["change"]["after"][output].clone();
                    set(
                        &mut v["resource_changes"][t]["change"]["after"],
                        p,
                        if matches!(p, super::SUBNETS | super::GROUPS) {
                            value
                        } else {
                            json!([value])
                        },
                    );
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, t, p), "{t} {p} {case}");
        }
    }
}
fn parent(v: &Value, t: usize) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment
                && g.nodes[e.to].address == v["resource_changes"][t]["address"]
        })
        .map(|e| g.nodes[e.from].address.clone())
}
#[test]
fn transfer_webapp_has_only_proven_vpc_placement_and_independent_service_outputs() {
    let v = fixture();
    for &(t, _, p, _) in REFERENCES {
        assert!(complete(&v, t, p), "{t} {p}");
    }
    assert_eq!(parent(&v, 7).as_deref(), Some("aws_vpc.main"));
    assert_eq!(parent(&v, 9), None);
    let parsed = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 10);
    for n in &parsed.nodes {
        let found: Vec<_> = g.nodes.iter().filter(|s| s.address == n.address).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].action, n.action);
    }
    for n in &g.nodes {
        if n.resource_type == super::WEB {
            assert_eq!(n.placement, Placement::Spatial);
            assert!(n.member_owner.is_none());
        }
        if n.resource_type == "aws_transfer_web_app_customization" {
            assert!(n.member_owner.is_some());
        }
    }
    assert!(!g.nodes.iter().any(|n| matches!(
        n.resource_type.as_str(),
        "aws_vpc_endpoint" | "aws_ssoadmin_application"
    )));
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
        assert!(out.contains("Identity Center instance"));
    }
}
#[test]
fn transfer_webapp_network_requires_complete_explicit_sets_and_common_vpc() {
    for p in [super::SUBNETS, super::GROUPS] {
        for case in [
            "literal",
            "extra",
            "empty",
            "duplicate",
            "parent-mismatch",
            "parent-literal",
            "parent-unknown",
            "vpc-literal",
            "vpc-mismatch",
        ] {
            let mut v = fixture();
            let s = if p == super::SUBNETS { 2 } else { 4 };
            match case {
                "literal" => set(
                    &mut v["configuration"]["root_module"]["resources"][7]["expressions"],
                    p,
                    json!({"constant_value":[]}),
                ),
                "extra" => {
                    v["resource_changes"][7]["change"]["after"]["endpoint_details"][0]["vpc"][0]
                        [p.rsplit('.').next().unwrap()]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("external"))
                }
                "empty" => set(
                    &mut v["resource_changes"][7]["change"]["after"],
                    p,
                    json!([]),
                ),
                "duplicate" => {
                    let a = &mut v["resource_changes"][7]["change"]["after"]["endpoint_details"][0]
                        ["vpc"][0][p.rsplit('.').next().unwrap()];
                    let first = a[0].clone();
                    a.as_array_mut().unwrap().push(first);
                }
                "parent-mismatch" => {
                    v["resource_changes"][s]["change"]["after"]["vpc_id"] = json!("vpc-other");
                    v["configuration"]["root_module"]["resources"][s]["expressions"]["vpc_id"] =
                        json!({"references":["aws_vpc.other.id"]});
                }
                "parent-literal" => {
                    v["configuration"]["root_module"]["resources"][s]["expressions"]["vpc_id"] =
                        json!({"constant_value":"vpc-main"})
                }
                "parent-unknown" => {
                    v["resource_changes"][s]["change"]["after_unknown"] = json!({"vpc_id":true})
                }
                "vpc-literal" => set(
                    &mut v["configuration"]["root_module"]["resources"][7]["expressions"],
                    super::VPC,
                    json!({"constant_value":"vpc-main"}),
                ),
                "vpc-mismatch" => set(
                    &mut v["resource_changes"][7]["change"]["after"],
                    super::VPC,
                    json!("vpc-other"),
                ),
                _ => unreachable!(),
            }
            assert_eq!(parent(&v, 7), None, "{p} {case}");
            assert!(complete(&v, 7, super::ROLE));
        }
    }
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][7]["expressions"]["endpoint_details"][0]["vpc"]
        [0]
    .as_object_mut()
    .unwrap()
    .remove("security_group_ids");
    assert_eq!(parent(&v, 7), None);
}
#[test]
fn transfer_webapp_identity_center_uses_only_the_singleton_data_arn_contract() {
    for case in [
        "multiple",
        "empty",
        "scalar",
        "duplicate",
        "unknown-member",
        "wrong-field",
        "wrong-index",
    ] {
        let mut v = fixture();
        let arn = v["resource_changes"][6]["change"]["after"]["arns"][0].clone();
        match case {
            "multiple" => {
                v["resource_changes"][6]["change"]["after"]["arns"] =
                    json!([arn, "arn:aws:sso:::instance/other"])
            }
            "empty" => v["resource_changes"][6]["change"]["after"]["arns"] = json!([]),
            "scalar" => v["resource_changes"][6]["change"]["after"]["arns"] = arn,
            "duplicate" => {
                v["resource_changes"][6]["change"]["after"]["arns"] = json!([arn.clone(), arn])
            }
            "unknown-member" => {
                v["resource_changes"][6]["change"]["after_unknown"] = json!({"arns":[true]})
            }
            "wrong-field" | "wrong-index" => set(
                &mut v["configuration"]["root_module"]["resources"][7]["expressions"],
                super::INSTANCE,
                json!({"references":[if case=="wrong-field"{"data.aws_ssoadmin_instances.identity.identity_store_ids"}else{"data.aws_ssoadmin_instances.identity.arns[1]"}]}),
            ),
            _ => unreachable!(),
        }
        assert!(!complete(&v, 7, super::INSTANCE), "{case}");
    }
    let mut v = fixture();
    set(
        &mut v["configuration"]["root_module"]["resources"][7]["expressions"],
        super::INSTANCE,
        json!({"references":["data.aws_ssoadmin_instances.identity.arns[0]"]}),
    );
    assert!(complete(&v, 7, super::INSTANCE));
}
#[test]
fn transfer_webapp_requires_singleton_framework_blocks() {
    for (outer, inner, p) in [
        ("endpoint_details", "vpc", super::VPC),
        (
            "identity_provider_details",
            "identity_center_config",
            super::INSTANCE,
        ),
        (
            "identity_provider_details",
            "identity_center_config",
            super::ROLE,
        ),
    ] {
        for is_outer in [true, false] {
            for multiple in [true, false] {
                let mut v = fixture();
                let a = if is_outer {
                    &mut v["resource_changes"][7]["change"]["after"][outer]
                } else {
                    &mut v["resource_changes"][7]["change"]["after"][outer][0][inner]
                };
                let row = a[0].clone();
                *a = if multiple {
                    json!([row.clone(), row])
                } else {
                    row
                };
                assert!(!complete(&v, 7, p));
            }
        }
    }
}
#[test]
fn transfer_webapp_preserves_filtered_network_fallback_and_named_state_identity() {
    let g = graph(&fixture());
    for address in [
        "aws_transfer_web_app.private",
        "aws_transfer_web_app_customization.branding",
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
        assert_eq!(selected.coverage.selected.resources, 1);
        assert!(
            !selected
                .edges
                .iter()
                .any(|e| e.kind == EdgeKind::Containment)
        );
    }
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: FIXTURE.into(),
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
    let mut v = fixture();
    v["resource_changes"][8]["previous_address"] = json!("aws_transfer_web_app_customization.old");
    v["resource_changes"][8]["change"]["actions"] = json!(["delete", "create"]);
    assert!(complete(&v, 8, "web_app_id"));
    assert_eq!(graph(&v).coverage.selected.resources, 10);
}

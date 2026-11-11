use crate::{
    model::{EdgeKind, Placement, RegionScope},
    plan, semantic, view,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/rolesanywhere-plan.json");
const REFERENCES: &[(usize, usize, &str, &str)] = &[
    (5, 0, "role_arns", "arn"),
    (5, 2, "managed_policy_arns", "arn"),
    (6, 4, super::CA, "arn"),
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
fn rolesanywhere_references_require_exact_current_configured_outputs() {
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
                        json!("data")
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
                        if p == super::CA {
                            json!([value])
                        } else {
                            value
                        },
                    );
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn rolesanywhere_keeps_regional_configuration_and_global_iam_independent() {
    let v = fixture();
    for &(t, _, p, _) in REFERENCES {
        assert!(complete(&v, t, p), "{p}");
    }
    let parsed = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 7);
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    for n in &g.nodes {
        let source = parsed
            .nodes
            .iter()
            .find(|s| s.address == n.address)
            .unwrap();
        assert_eq!(n.action, source.action);
        assert_eq!(g.nodes.iter().filter(|s| s.address == n.address).count(), 1);
        if n.resource_type.starts_with("aws_rolesanywhere_") {
            assert_eq!(n.placement, Placement::Logical);
            assert_eq!(
                n.entity.scope.as_ref().unwrap().region,
                Some(RegionScope("us-east-1".into()))
            );
        }
        if n.resource_type.starts_with("aws_iam_") {
            assert!(
                n.entity
                    .scope
                    .as_ref()
                    .is_some_and(|s| s.global && s.region.is_none())
            );
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
        assert!(out.contains("Trust anchor CA"));
    }
}
#[test]
fn rolesanywhere_requires_complete_sets_and_exclusive_known_ca_source() {
    for p in ["role_arns", "managed_policy_arns"] {
        for case in [
            "missing",
            "extra",
            "duplicate",
            "unknown-member",
            "partial-reference",
        ] {
            let mut v = fixture();
            let values = &mut v["resource_changes"][5]["change"]["after"][p];
            match case {
                "missing" => {
                    values.as_array_mut().unwrap().pop();
                }
                "extra" => values.as_array_mut().unwrap().push(json!("literal-extra")),
                "duplicate" => {
                    values[1] = values[0].clone();
                }
                "unknown-member" => {
                    v["resource_changes"][5]["change"]["after_unknown"] = json!({p:[false,true]})
                }
                "partial-reference" => {
                    v["configuration"]["root_module"]["resources"][5]["expressions"][p]["references"].as_array_mut().unwrap().pop();
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, 5, p), "{p} {case}");
        }
    }
    for case in [
        "bundle",
        "unknown-type",
        "pem",
        "unknown-pem",
        "outer-object",
        "outer-multi",
        "inner-object",
        "inner-multi",
    ] {
        let mut v = fixture();
        let a = &mut v["resource_changes"][6]["change"]["after"];
        match case {
            "bundle" => a["source"][0]["source_type"] = json!("CERTIFICATE_BUNDLE"),
            "unknown-type" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"source":[{"source_type":true}]})
            }
            "pem" => {
                a["source"][0]["source_data"][0]["x509_certificate_data"] = json!("PRIVATE_PEM")
            }
            "unknown-pem" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"source":[{"source_data":[{"x509_certificate_data":true}]}]})
            }
            "outer-object" => a["source"] = a["source"][0].clone(),
            "outer-multi" => {
                let row = a["source"][0].clone();
                a["source"].as_array_mut().unwrap().push(row);
            }
            "inner-object" => {
                a["source"][0]["source_data"] = a["source"][0]["source_data"][0].clone()
            }
            "inner-multi" => {
                let row = a["source"][0]["source_data"][0].clone();
                a["source"][0]["source_data"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 6, super::CA), "{case}");
    }
}
#[test]
fn rolesanywhere_retains_selection_lifecycle_and_named_state_boundaries() {
    let g = graph(&fixture());
    let selected = view::single(
        &g,
        &view::Options {
            focus: Some("aws_rolesanywhere_profile.sessions".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(4);
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
    v["resource_changes"][5]["previous_address"] = json!("aws_rolesanywhere_profile.old");
    v["resource_changes"][5]["change"]["actions"] = json!(["delete", "create"]);
    assert!(complete(&v, 5, "role_arns"));
    assert_eq!(graph(&v).coverage.selected.resources, 7);
}
#[test]
fn rolesanywhere_uses_verified_account_evidence_for_global_iam_bindings() {
    for foreign in [false, true] {
        let mut v = fixture();
        v["terraform_version"] = json!("1.9.8");
        v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
        let mut other = v["configuration"]["provider_config"]["aws"].clone();
        other["alias"] = json!("global");
        other["expressions"]["region"] = json!({"constant_value":"us-west-2"});
        if foreign {
            other["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                json!("arn:aws:iam::999999999999:role/Test");
        }
        v["configuration"]["provider_config"]["aws.global"] = other;
        for i in 0..4 {
            v["configuration"]["root_module"]["resources"][i]["provider_config_key"] =
                json!("aws.global");
        }
        let parsed = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        let g = semantic::transform(&parsed);
        let profile = g
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_rolesanywhere_profile")
            .unwrap();
        for p in ["role_arns", "managed_policy_arns"] {
            assert_eq!(
                profile.references.iter().any(|r| r.attribute == p
                    && r.complete
                    && Some(r.label) == super::label(&profile.resource_type, p)),
                !foreign,
                "{p} foreign={foreign}"
            );
        }
        assert!(profile.entity.scope.as_ref().unwrap().account.is_some());
    }
}

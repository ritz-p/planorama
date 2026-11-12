use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/private-ca-references-plan.json");
const REFERENCES: &[(usize, usize, &str, &str)] = &[
    (1, 0, super::CRL, "id"),
    (2, 1, "certificate_authority_arn", "arn"),
    (3, 1, "resource_arn", "arn"),
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
fn private_ca_helpers_have_distinct_actions_and_do_not_own_the_crl_bucket() {
    let v = fixture();
    for &(t, _, p, _) in REFERENCES {
        assert!(complete(&v, t, p));
    }
    let parsed = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 5);
    for node in &parsed.nodes {
        let found: Vec<_> = g
            .nodes
            .iter()
            .filter(|n| n.address == node.address && n.deposed_key == node.deposed_key)
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].action, node.action);
    }
    for t in [2, 3] {
        assert!(
            g.nodes
                .iter()
                .find(|n| n.address == v["resource_changes"][t]["address"])
                .unwrap()
                .member_owner
                .is_some()
        );
    }
    let bucket = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_s3_bucket")
        .unwrap();
    assert!(bucket.member_owner.is_none());
    let ca = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_acmpca_certificate_authority")
        .unwrap();
    assert_eq!(ca.placement, Placement::Spatial);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment && g.nodes[e.from].address == ca.address)
    );
    assert!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_acm_certificate")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Signing CA reference" && r.complete)
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
        assert!(!out.contains("PRIVATE_"));
        assert!(out.contains("CRL bucket"));
    }
}
#[test]
fn private_ca_references_require_exact_current_configured_outputs() {
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
            "array",
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
                "array" => {
                    let value = v["resource_changes"][s]["change"]["after"][output].clone();
                    set(
                        &mut v["resource_changes"][t]["change"]["after"],
                        p,
                        json!([value]),
                    );
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn private_ca_crl_requires_enabled_singleton_blocks_and_bucket_name_identity() {
    for enabled in [Value::Null, json!(false), json!("true")] {
        let mut v = fixture();
        v["resource_changes"][1]["change"]["after"]["revocation_configuration"][0]["crl_configuration"]
            [0]["enabled"] = enabled;
        assert!(!complete(&v, 1, super::CRL));
        assert!(complete(&v, 2, "certificate_authority_arn"));
    }
    let mut v = fixture();
    v["resource_changes"][1]["change"]["after_unknown"] =
        json!({"revocation_configuration":[{"crl_configuration":[{"enabled":true}]}]});
    assert!(!complete(&v, 1, super::CRL));
    for outer in [false, true] {
        for multiple in [false, true] {
            let mut v = fixture();
            let value = if outer {
                &mut v["resource_changes"][1]["change"]["after"]["revocation_configuration"]
            } else {
                &mut v["resource_changes"][1]["change"]["after"]["revocation_configuration"][0]["crl_configuration"]
            };
            let row = value[0].clone();
            *value = if multiple {
                json!([row.clone(), row])
            } else {
                row
            };
            assert!(!complete(&v, 1, super::CRL));
        }
    }
    let mut v = fixture();
    set(
        &mut v["configuration"]["root_module"]["resources"][1]["expressions"],
        super::CRL,
        json!({"references":["aws_s3_bucket.crl.bucket"]}),
    );
    assert!(complete(&v, 1, super::CRL));
    let mut v = fixture();
    v["resource_changes"][0]["change"]["after"]["arn"] = json!("arn:aws:s3:::crl-storage");
    set(
        &mut v["configuration"]["root_module"]["resources"][1]["expressions"],
        super::CRL,
        json!({"references":["aws_s3_bucket.crl.arn"]}),
    );
    set(
        &mut v["resource_changes"][1]["change"]["after"],
        super::CRL,
        json!("arn:aws:s3:::crl-storage"),
    );
    assert!(!complete(&v, 1, super::CRL));
}
#[test]
fn private_ca_preserves_filter_fallback_and_named_state_boundaries() {
    let g = graph(&fixture());
    for address in ["aws_acmpca_permission.renewal", "aws_acmpca_policy.sharing"] {
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
        assert_eq!(selected.nodes.len(), 1);
    }
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
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
    v["resource_changes"][2]["previous_address"] = json!("aws_acmpca_permission.old");
    v["resource_changes"][2]["change"]["actions"] = json!(["delete", "create"]);
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 5);
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_acmpca_permission")
            .unwrap()
            .previous_address
            .as_deref(),
        Some("aws_acmpca_permission.old")
    );
}

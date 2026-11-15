use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/transfer-as2-plan.json");
const REFERENCES: &[(usize, usize, &str, &str)] = &[
    (4, 2, "certificate_ids", "certificate_id"),
    (5, 2, "certificate_ids", "certificate_id"),
    (6, 0, "server_id", "id"),
    (6, 1, "access_role", "arn"),
    (6, 4, "local_profile_id", "profile_id"),
    (6, 5, "partner_profile_id", "profile_id"),
    (7, 1, "access_role", "arn"),
    (7, 1, "logging_role", "arn"),
    (7, 4, "as2_config.local_profile_id", "profile_id"),
    (7, 5, "as2_config.partner_profile_id", "profile_id"),
    (10, 1, "access_role", "arn"),
    (10, 8, "sftp_config.user_secret_id", "arn"),
    (10, 9, super::LATTICE, "arn"),
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
fn transfer_as2_requires_exact_current_outputs() {
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
                        if p == "certificate_ids" {
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
#[test]
fn transfer_as2_preserves_shared_profiles_certificates_and_independent_connectors() {
    let v = fixture();
    for &(t, _, p, _) in REFERENCES {
        assert!(complete(&v, t, p), "{t} {p}");
    }
    let parsed = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 11);
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    for n in &g.nodes {
        if n.resource_type == "aws_transfer_agreement" {
            assert!(n.member_owner.is_some());
        } else {
            assert!(n.member_owner.is_none(), "{}", n.address);
        }
        if n.resource_type == "aws_transfer_connector" {
            assert_eq!(n.placement, Placement::Spatial);
        }
    }
    for n in &parsed.nodes {
        let found: Vec<_> = g.nodes.iter().filter(|s| s.address == n.address).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].action, n.action);
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
        assert!(out.contains("SFTP user secret"));
    }
}
#[test]
fn transfer_as2_profile_types_and_certificate_sets_are_verified_without_key_inspection() {
    for case in [
        "unknown-type",
        "wrong-type",
        "unknown-usage",
        "wrong-usage",
        "incomplete",
        "duplicate",
        "literal-member",
    ] {
        let mut v = fixture();
        match case {
            "unknown-type" => {
                v["resource_changes"][4]["change"]["after_unknown"] = json!({"profile_type":true})
            }
            "wrong-type" => {
                v["resource_changes"][4]["change"]["after"]["profile_type"] = json!("OTHER")
            }
            "unknown-usage" => {
                v["resource_changes"][2]["change"]["after_unknown"] = json!({"usage":true})
            }
            "wrong-usage" => v["resource_changes"][2]["change"]["after"]["usage"] = json!("OTHER"),
            "incomplete" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["certificate_ids"]
                    ["references"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            "duplicate" => {
                v["resource_changes"][4]["change"]["after"]["certificate_ids"] =
                    json!(["cert-SIGNING", "cert-SIGNING"])
            }
            "literal-member" => v["resource_changes"][4]["change"]["after"]["certificate_ids"]
                .as_array_mut()
                .unwrap()
                .push(json!("external-cert")),
            _ => unreachable!(),
        }
        assert!(!complete(&v, 4, "certificate_ids"), "{case}");
    }
    for (s, p, t) in [
        (4, "local_profile_id", 6),
        (5, "partner_profile_id", 6),
        (4, "as2_config.local_profile_id", 7),
        (5, "as2_config.partner_profile_id", 7),
    ] {
        let mut v = fixture();
        v["resource_changes"][s]["change"]["after"]["profile_type"] =
            json!(if s == 4 { "PARTNER" } else { "LOCAL" });
        assert!(!complete(&v, t, p));
    }
    let mut v = fixture();
    for s in [2, 3] {
        v["resource_changes"][s]["change"]["after_unknown"] =
            json!({"private_key":true,"certificate":true,"certificate_chain":true});
    }
    assert!(complete(&v, 4, "certificate_ids"));
    assert!(complete(&v, 5, "certificate_ids"));
}
#[test]
fn transfer_connector_modes_and_lattice_egress_are_not_interchangeable() {
    for (t, selected, other, p) in [
        (
            7,
            "as2_config",
            "sftp_config",
            "as2_config.local_profile_id",
        ),
        (
            10,
            "sftp_config",
            "as2_config",
            "sftp_config.user_secret_id",
        ),
        (10, "sftp_config", "as2_config", super::LATTICE),
    ] {
        for case in [
            "both",
            "unknown-selected",
            "unknown-other",
            "multi",
            "object",
            "non-object",
        ] {
            let mut v = fixture();
            let a = &mut v["resource_changes"][t]["change"]["after"];
            match case {
                "both" => a[other] = json!([{}]),
                "unknown-selected" => {
                    v["resource_changes"][t]["change"]["after_unknown"] = json!({selected:true})
                }
                "unknown-other" => {
                    v["resource_changes"][t]["change"]["after_unknown"] = json!({other:true})
                }
                "multi" => {
                    let row = a[selected][0].clone();
                    a[selected].as_array_mut().unwrap().push(row);
                }
                "object" => a[selected] = a[selected][0].clone(),
                "non-object" => a[selected] = json!(["wrong"]),
                _ => unreachable!(),
            }
            assert!(!complete(&v, t, p), "{p} {case}");
            assert!(complete(&v, t, "access_role"));
        }
    }
    for case in [
        "url",
        "unknown-url",
        "outer-object",
        "inner-object",
        "multiple",
    ] {
        let mut v = fixture();
        let a = &mut v["resource_changes"][10]["change"]["after"];
        match case {
            "url" => a["url"] = json!("PRIVATE_URL"),
            "unknown-url" => {
                v["resource_changes"][10]["change"]["after_unknown"] = json!({"url":true})
            }
            "outer-object" => a["egress_config"] = a["egress_config"][0].clone(),
            "inner-object" => {
                a["egress_config"][0]["vpc_lattice"] =
                    a["egress_config"][0]["vpc_lattice"][0].clone()
            }
            "multiple" => {
                let row = a["egress_config"][0]["vpc_lattice"][0].clone();
                a["egress_config"][0]["vpc_lattice"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 10, super::LATTICE), "{case}");
    }
    let mut v = fixture();
    v["resource_changes"][10]["change"]["after_unknown"] =
        json!({"sftp_config":[{"trusted_host_keys":true}]});
    assert!(complete(&v, 10, super::LATTICE));
    assert!(complete(&v, 10, "sftp_config.user_secret_id"));
}
#[test]
fn transfer_sftp_secret_names_are_exact_and_state_selection_preserves_identity() {
    let mut v = fixture();
    set(
        &mut v["resource_changes"][10]["change"]["after"],
        "sftp_config.user_secret_id",
        json!("sftp-login"),
    );
    set(
        &mut v["configuration"]["root_module"]["resources"][10]["expressions"],
        "sftp_config.user_secret_id",
        json!({"references":["aws_secretsmanager_secret.login.name"]}),
    );
    assert!(complete(&v, 10, "sftp_config.user_secret_id"));
    v["resource_changes"][8]["change"]["after"]["name"] = json!("different");
    assert!(!complete(&v, 10, "sftp_config.user_secret_id"));
    let g = graph(&fixture());
    let selected = view::single(
        &g,
        &view::Options {
            focus: Some("aws_transfer_agreement.incoming".into()),
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
    v["resource_changes"][6]["previous_address"] = json!("aws_transfer_agreement.old");
    v["resource_changes"][6]["change"]["actions"] = json!(["delete", "create"]);
    assert!(complete(&v, 6, "server_id"));
    assert_eq!(graph(&v).coverage.selected.resources, 11);
}

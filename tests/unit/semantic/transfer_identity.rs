use crate::{model::EdgeKind, plan, semantic, view};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/transfer-identity-plan.json");
const REFERENCES: &[(usize, usize, &str, &str)] = &[
    (3, 0, "server_id", "id"),
    (3, 2, "role", "arn"),
    (4, 0, "server_id", "id"),
    (4, 2, "role", "arn"),
    (5, 1, "server_id", "id"),
    (5, 2, "role", "arn"),
    (6, 0, "server_id", "id"),
    (7, 0, "server_id", "id"),
    (7, 4, "user_name", "user_name"),
    (8, 0, "resource_arn", "arn"),
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
fn transfer_identity_requires_exact_current_outputs() {
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
fn transfer_identity_owns_helpers_without_owning_storage_or_roles() {
    let v = fixture();
    for &(t, _, p, _) in REFERENCES {
        assert!(complete(&v, t, p), "{t} {p}");
    }
    let parsed = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 10);
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    for t in 3..9 {
        let node = g
            .nodes
            .iter()
            .find(|n| n.address == v["resource_changes"][t]["address"])
            .unwrap();
        assert!(node.member_owner.is_some(), "{t}");
    }
    for node in &parsed.nodes {
        let all: Vec<_> = g
            .nodes
            .iter()
            .filter(|n| n.address == node.address)
            .collect();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].action, node.action);
    }
    for kind in ["aws_iam_role", "aws_s3_bucket"] {
        assert!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == kind)
                .unwrap()
                .member_owner
                .is_none()
        );
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
        assert!(out.contains("Transfer user"));
    }
}
#[test]
fn transfer_ssh_key_requires_the_users_verified_server_identity() {
    for case in [
        "other-user",
        "literal-user-server",
        "unknown-user-server",
        "other-key-server",
        "literal-key-server",
        "unknown-server",
        "deleted-server",
        "foreign-server",
    ] {
        let mut v = fixture();
        match case {
            "other-user" => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["user_name"] =
                    json!({"references":["aws_transfer_user.two.user_name"]})
            }
            "literal-user-server" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["server_id"] =
                    json!({"constant_value":"s-one"})
            }
            "unknown-user-server" => {
                v["resource_changes"][4]["change"]["after_unknown"] = json!({"server_id":true})
            }
            "other-key-server" => {
                v["resource_changes"][7]["change"]["after"]["server_id"] = json!("s-two");
                v["configuration"]["root_module"]["resources"][7]["expressions"]["server_id"] =
                    json!({"references":["aws_transfer_server.two.id"]});
            }
            "literal-key-server" => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["server_id"] =
                    json!({"constant_value":"s-one"})
            }
            "unknown-server" => {
                v["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true})
            }
            "deleted-server" => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            "foreign-server" => {
                v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                v["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                    json!("other");
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 7, "user_name"), "{case}");
        assert!(
            graph(&v)
                .nodes
                .iter()
                .find(|n| n.resource_type == "aws_transfer_ssh_key")
                .unwrap()
                .member_owner
                .is_none()
        );
    }
    let mut v = fixture();
    v["resource_changes"][7]["change"]["after"]["server_id"] = json!("s-two");
    v["configuration"]["root_module"]["resources"][7]["expressions"]["server_id"] =
        json!({"references":["aws_transfer_server.two.id"]});
    v["configuration"]["root_module"]["resources"][7]["expressions"]["user_name"] =
        json!({"references":["aws_transfer_user.two.user_name"]});
    assert!(complete(&v, 7, "user_name"));
}
#[test]
fn transfer_tags_use_only_managed_taggable_transfer_arn_outputs() {
    for (source, output, accepted) in [
        (4, "arn", true),
        (6, "arn", true),
        (9, "arn", false),
        (0, "id", false),
        (7, "ssh_key_id", false),
    ] {
        let mut v = fixture();
        let addr = v["resource_changes"][source]["address"]
            .as_str()
            .unwrap()
            .to_string();
        v["resource_changes"][8]["change"]["after"]["resource_arn"] =
            v["resource_changes"][source]["change"]["after"][output].clone();
        v["configuration"]["root_module"]["resources"][8]["expressions"]["resource_arn"] =
            json!({"references":[format!("{addr}.{output}")]});
        assert_eq!(
            complete(&v, 8, "resource_arn"),
            accepted,
            "{source} {output}"
        );
    }
}
#[test]
fn transfer_identity_preserves_filtered_historical_and_named_state_resources() {
    let g = graph(&fixture());
    for address in [
        "aws_transfer_ssh_key.login",
        "aws_transfer_access.directory",
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
    }
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
    v["resource_changes"][7]["previous_address"] = json!("aws_transfer_ssh_key.old");
    v["resource_changes"][7]["change"]["actions"] = json!(["delete", "create"]);
    assert!(complete(&v, 7, "user_name"));
    assert_eq!(graph(&v).coverage.selected.resources, 10);
}
#[test]
fn transfer_tag_owners_retain_independently_verified_provider_accounts() {
    for kind in [
        "aws_transfer_server",
        "aws_transfer_user",
        "aws_transfer_host_key",
        "aws_transfer_certificate",
        "aws_transfer_profile",
        "aws_transfer_agreement",
        "aws_transfer_connector",
        "aws_transfer_web_app",
        "aws_transfer_workflow",
    ] {
        let mut v = fixture();
        v["terraform_version"] = json!("1.9.8");
        v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
        let address = format!("{kind}.owner");
        let mut source = v["resource_changes"][0].clone();
        source["type"] = json!(kind);
        source["address"] = json!(address);
        v["resource_changes"] = json!([source, v["resource_changes"][8].clone()]);
        let mut tag = v["configuration"]["root_module"]["resources"][8].clone();
        tag["expressions"]["resource_arn"] = json!({"references":[format!("{address}.arn")]});
        v["configuration"]["root_module"]["resources"] = json!([{"address":address,"type":kind,"provider_config_key":"aws","expressions":{}},tag]);
        let parsed = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        let g = semantic::transform(&parsed);
        assert!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == "aws_transfer_tag")
                .unwrap()
                .member_owner
                .is_some(),
            "{kind}"
        );
        assert!(
            g.nodes
                .iter()
                .all(|n| n.entity.scope.as_ref().unwrap().account.is_some())
        );
    }
}

use crate::{
    model::{EdgeKind, Graph, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/directory-service-plan.json");
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap())
}
fn verified(v: &Value, t: usize, p: &str) -> bool {
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
fn set(v: &mut Value, p: &str, x: Value) {
    *v.pointer_mut(&format!("/{}", p.replace('.', "/"))).unwrap() = x;
}
fn placed(g: &Graph, address: &str, parent: &str) -> bool {
    g.edges.iter().any(|e| {
        e.kind == EdgeKind::Containment
            && g.nodes[e.to].address == address
            && g.nodes[e.from].address == parent
    })
}
#[test]
fn directory_service_typed_network_owners_and_recipient_identity_are_independent() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (6, super::VPC),
        (6, super::SUBNETS),
        (7, super::CONNECT_VPC),
        (7, super::CONNECT_SUBNETS),
        (16, "log_group_name"),
        (17, super::VPC),
        (17, super::SUBNETS),
        (18, "directory_id"),
        (18, "target.id"),
        (19, "shared_directory_id"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    for t in (10..=17).chain(20..=21) {
        assert!(verified(&v, t, "directory_id"), "{t}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 23);
    assert_eq!(g.nodes.len(), 23);
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        assert_eq!(n.placement, Placement::Logical);
        assert!(
            n.references
                .iter()
                .all(|r| !super::indexed(&n.resource_type, &r.attribute))
        );
        if super::HELPERS.contains(&n.resource_type.as_str()) {
            assert_eq!(
                n.member_owner.as_ref().unwrap().target.address,
                "aws_directory_service_directory.primary"
            );
        } else {
            assert!(n.member_owner.is_none());
        }
    }
    assert!(placed(
        &g,
        "aws_directory_service_directory.primary",
        "aws_vpc.primary"
    ));
    assert!(placed(
        &g,
        "aws_directory_service_directory.connector",
        "aws_vpc.primary"
    ));
    assert!(placed(
        &g,
        "aws_directory_service_region.west",
        "aws_vpc.replica"
    ));
    assert!(!placed(
        &g,
        "aws_directory_service_region.west",
        "aws_vpc.primary"
    ));
    let region = g
        .nodes
        .iter()
        .find(|n| n.resource_type == super::REGION)
        .unwrap();
    assert_eq!(
        region
            .entity
            .scope
            .as_ref()
            .unwrap()
            .region
            .as_ref()
            .unwrap()
            .0,
        "us-east-1"
    );
    assert!(
        !g.edges
            .iter()
            .any(|e| g.nodes[e.from].resource_type == "aws_security_group"
                && super::resource(&g.nodes[e.to].resource_type))
    );
}
#[test]
fn directory_service_requires_exact_current_managed_output_and_complete_provenance() {
    for (t, path, p, s, out, wrong) in [
        (6, "vpc_settings.0.vpc_id", super::VPC, 0, "id", "arn"),
        (
            7,
            "connect_settings.0.subnet_ids",
            super::CONNECT_SUBNETS,
            1,
            "id",
            "arn",
        ),
        (10, "directory_id", "directory_id", 6, "id", "name"),
        (16, "log_group_name", "log_group_name", 8, "name", "arn"),
        (18, "target.0.id", "target.id", 9, "id", "arn"),
        (
            19,
            "shared_directory_id",
            "shared_directory_id",
            18,
            "shared_directory_id",
            "id",
        ),
    ] {
        for case in [
            "literal",
            "unknown",
            "sensitive",
            "mismatch",
            "output",
            "data",
            "delete",
            "deposed",
            "provider",
            "source-sensitive",
            "consumer-delete",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => set(
                    &mut v,
                    &format!("configuration.root_module.resources.{t}.expressions.{path}"),
                    json!({"constant_value":"literal"}),
                ),
                "unknown" | "sensitive" => {
                    v["resource_changes"][t]["change"][if case == "unknown" {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({path.split('.').next().unwrap():true})
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][out] = json!("other"),
                "output" => {
                    let a = v["resource_changes"][s]["address"]
                        .as_str()
                        .unwrap()
                        .to_string();
                    set(
                        &mut v,
                        &format!("configuration.root_module.resources.{t}.expressions.{path}"),
                        json!({"references":[format!("{a}.{wrong}")]}),
                    );
                }
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => {
                    v["resource_changes"][s]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                "source-sensitive" => {
                    v["resource_changes"][s]["change"]["after_sensitive"] = json!({out:true})
                }
                "consumer-delete" => {
                    v["resource_changes"][t]["change"]["actions"] = json!(["delete"])
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn directory_service_network_type_shapes_and_common_vpc_are_required() {
    for case in [
        "simple",
        "wrong-type",
        "unknown-type",
        "other-block",
        "unknown-other",
        "partial-subnets",
        "object",
        "two-blocks",
        "foreign-parent",
        "unknown-parent",
        "filtered",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "simple" => v["resource_changes"][6]["change"]["after"]["type"] = json!("SimpleAD"),
            "wrong-type" => {
                v["resource_changes"][6]["change"]["after"]["type"] = json!("ADConnector")
            }
            "unknown-type" => {
                v["resource_changes"][6]["change"]["after_unknown"] = json!({"type":true})
            }
            "other-block" => {
                v["resource_changes"][6]["change"]["after"]["connect_settings"] = json!([{}])
            }
            "unknown-other" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"connect_settings":true})
            }
            "partial-subnets" => {
                v["resource_changes"][6]["change"]["after"]["vpc_settings"][0]["subnet_ids"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("external"))
            }
            "object" => {
                let a = v["resource_changes"][6]["change"]["after"]["vpc_settings"][0].clone();
                v["resource_changes"][6]["change"]["after"]["vpc_settings"] = a;
            }
            "two-blocks" => {
                let a = v["resource_changes"][6]["change"]["after"]["vpc_settings"][0].clone();
                v["resource_changes"][6]["change"]["after"]["vpc_settings"] = json!([a, a]);
            }
            "foreign-parent" => {
                v["resource_changes"][1]["change"]["after"]["vpc_id"] = json!("other")
            }
            "unknown-parent" => {
                v["resource_changes"][1]["change"]["after_unknown"] = json!({"vpc_id":true})
            }
            "filtered" => v["resource_changes"][1]["change"]["actions"] = json!(["no-op"]),
            _ => unreachable!(),
        }
        let selected = crate::view::single(
            &graph(&v),
            &crate::view::Options {
                changed_only: case == "filtered",
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            placed(
                &selected,
                "aws_directory_service_directory.primary",
                "aws_vpc.primary"
            ),
            case == "simple",
            "{case}"
        );
        assert!(verified(&v, 10, "directory_id"));
    }
}
#[test]
fn directory_service_region_control_and_destination_bindings_stay_separate() {
    for case in [
        "unknown-region",
        "wrong-region",
        "foreign-account",
        "unknown-account",
        "parent-type",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown-region" => {
                v["resource_changes"][17]["change"]["after_unknown"] = json!({"region_name":true})
            }
            "wrong-region" => {
                v["resource_changes"][17]["change"]["after"]["region_name"] = json!("eu-west-1")
            }
            "foreign-account" => {
                v["configuration"]["provider_config"]["aws.west"]["expressions"]["assume_role"][0]
                    ["role_arn"]["constant_value"] = json!("arn:aws:iam::888888888888:role/Test")
            }
            "unknown-account" => {
                v["configuration"]["provider_config"]["aws.west"]["expressions"]["assume_role"] =
                    json!([])
            }
            "parent-type" => {
                v["resource_changes"][6]["change"]["after"]["type"] = json!("SimpleAD")
            }
            _ => unreachable!(),
        }
        assert_eq!(
            verified(&v, 17, super::VPC),
            case == "parent-type",
            "{case}"
        );
        assert_eq!(
            verified(&v, 17, "directory_id"),
            case != "parent-type",
            "{case}"
        );
    }
    let raw = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert!(!placed(
        &raw,
        "aws_directory_service_region.west",
        "aws_vpc.replica"
    ));
}
#[test]
fn directory_service_share_requires_independent_distinct_recipient_account_proof() {
    for case in [
        "target-type",
        "unknown-target",
        "malformed-target",
        "wrong-recipient",
        "same-account",
        "unknown-owner",
        "unknown-recipient",
        "foreign-region",
        "wrong-output",
        "computed-only",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "target-type" => {
                v["resource_changes"][18]["change"]["after"]["target"][0]["type"] =
                    json!("ORGANIZATION")
            }
            "unknown-target" => {
                v["resource_changes"][18]["change"]["after_unknown"] = json!({"target":true})
            }
            "malformed-target" => {
                v["resource_changes"][18]["change"]["after"]["target"][0]["id"] = json!("invalid")
            }
            "wrong-recipient" => {
                v["resource_changes"][18]["change"]["after"]["target"][0]["id"] =
                    json!("888888888888")
            }
            "same-account" => {
                v["configuration"]["provider_config"]["aws.recipient"]["expressions"]["assume_role"]
                    [0]["role_arn"]["constant_value"] =
                    json!("arn:aws:iam::123456789012:role/Test");
                v["resource_changes"][18]["change"]["after"]["target"][0]["id"] =
                    json!("123456789012");
            }
            "unknown-owner" | "unknown-recipient" => {
                v["configuration"]["provider_config"][if case == "unknown-owner" {
                    "aws"
                } else {
                    "aws.recipient"
                }]["expressions"]["assume_role"] = json!([])
            }
            "foreign-region" => {
                v["configuration"]["provider_config"]["aws.recipient"]["expressions"]["region"]["constant_value"] =
                    json!("us-west-2")
            }
            "wrong-output" => {
                v["resource_changes"][18]["change"]["after"]["id"] = json!("d-recipient");
                v["configuration"]["root_module"]["resources"][19]["expressions"]["shared_directory_id"] =
                    json!({"references":["aws_directory_service_shared_directory.main.id"]});
            }
            "computed-only" => {
                v["configuration"]["root_module"]["resources"][19]["expressions"] = json!({})
            }
            _ => unreachable!(),
        }
        assert!(!verified(&v, 19, "shared_directory_id"), "{case}");
        assert!(
            graph(&v)
                .nodes
                .iter()
                .find(|n| n.resource_type == super::ACCEPT)
                .unwrap()
                .member_owner
                .is_none()
        );
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][19]["change"]["after_unknown"] =
        json!({"owner_directory_id":true,"owner_account_id":true});
    assert!(verified(&v, 19, "shared_directory_id"));
}
#[test]
fn directory_service_lifecycle_filter_named_states_and_privacy_preserve_exact_coverage() {
    for t in (10..=17).chain(20..=21) {
        for action in [json!(["update"]), json!(["delete", "create"])] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            v["resource_changes"][t]["change"]["actions"] = action;
            v["resource_changes"][t]["change"]["importing"] = json!({"id":"directory,member"});
            v["resource_changes"][t]["previous_address"] =
                json!("aws_directory_service_directory_setting.old");
            assert!(verified(&v, t, "directory_id"));
            assert_eq!(graph(&v).coverage.selected.resources, 23);
        }
    }
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let selected = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            focus: Some("aws_directory_service_region.west".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(selected.edges.is_empty());
    let plans = plan::parse_inputs(
        ["a", "b"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!output.content.contains("PRIVATE_"));
        assert!(!output.content.contains("192.0.2.10"));
    }
}

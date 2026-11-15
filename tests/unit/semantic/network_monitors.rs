use crate::{
    model::{EdgeKind, Graph, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/network-monitors-plan.json");
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
fn placed(g: &Graph) -> bool {
    g.edges.iter().any(|e| {
        e.kind == EdgeKind::Containment
            && g.nodes[e.to].resource_type == super::PROBE
            && g.nodes[e.from].resource_type == "aws_vpc"
    })
}
#[test]
fn network_monitors_typed_complete_references_keep_monitor_and_probe_identities() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (4, "scope_arn"),
        (4, super::LOCAL),
        (4, super::REMOTE),
        (6, "monitor_name"),
        (6, "source_arn"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 9);
    assert_eq!(g.nodes.len(), 9);
    assert!(placed(&g));
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        assert_eq!(n.placement, Placement::Logical);
        assert!(
            n.references
                .iter()
                .all(|r| !super::indexed(&n.resource_type, &r.attribute))
        );
        if n.resource_type == super::PROBE {
            assert_eq!(
                n.member_owner.as_ref().unwrap().target.address,
                "aws_networkmonitor_monitor.main"
            );
        } else {
            assert!(n.member_owner.is_none());
        }
    }
    let scope = g
        .nodes
        .iter()
        .position(|n| n.resource_type == super::SCOPE)
        .unwrap();
    let account = g
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_organizations_account")
        .unwrap();
    assert!(
        !g.edges
            .iter()
            .any(|e| (e.from == scope && e.to == account) || (e.from == account && e.to == scope))
    );
    let instance = g
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_instance")
        .unwrap();
    assert!(
        !g.edges
            .iter()
            .any(|e| e.from == instance || e.to == instance)
    );
}
#[test]
fn network_monitors_require_explicit_known_current_outputs_and_compatible_boundaries() {
    for (t, path, p, s, out) in [
        (4, "scope_arn", "scope_arn", 3, "scope_arn"),
        (4, "local_resource.0.identifier", super::LOCAL, 0, "arn"),
        (4, "remote_resource.1.identifier", super::REMOTE, 1, "arn"),
        (6, "monitor_name", "monitor_name", 5, "monitor_name"),
        (6, "source_arn", "source_arn", 1, "arn"),
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
            "region",
            "account",
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
                        json!({"references":[format!("{a}.id")]}),
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
                "region" | "account" => {
                    let mut alias = v["configuration"]["provider_config"]["aws"].clone();
                    alias["alias"] = json!("other");
                    if case == "region" {
                        alias["expressions"]["region"]["constant_value"] = json!("us-west-2");
                    } else {
                        alias["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                            json!("arn:aws:iam::999999999999:role/Test");
                    }
                    v["configuration"]["provider_config"]["aws.other"] = alias;
                    v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                        json!("aws.other");
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn network_monitors_discriminate_local_and_remote_kinds_and_complete_collection_shapes() {
    for case in [
        "local-type",
        "remote-eks",
        "az",
        "region",
        "service",
        "mixed-selector",
        "partial",
        "unknown-type",
        "sensitive-type",
        "object",
        "duplicate",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let p = match case {
            "local-type" => {
                v["resource_changes"][4]["change"]["after"]["local_resource"][2]["type"] =
                    json!("AWS::EC2::Subnet");
                super::LOCAL
            }
            "remote-eks" => {
                let item = v["resource_changes"][4]["change"]["after"]["local_resource"][2].clone();
                let expr=v["configuration"]["root_module"]["resources"][4]["expressions"]["local_resource"][2].clone();
                v["resource_changes"][4]["change"]["after"]["remote_resource"] = json!([item]);
                v["configuration"]["root_module"]["resources"][4]["expressions"]["remote_resource"] =
                    json!([expr]);
                super::REMOTE
            }
            "az" | "region" | "service" => {
                v["resource_changes"][4]["change"]["after"]["remote_resource"][0]["type"] =
                    json!(match case {
                        "az" => "AWS::AvailabilityZone",
                        "region" => "AWS::Region",
                        _ => "AWS::AWSService",
                    });
                super::REMOTE
            }
            "mixed-selector" => {
                v["resource_changes"][4]["change"]["after"]["local_resource"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"type":"AWS::Region","identifier":"us-east-1"}));
                v["configuration"]["root_module"]["resources"][4]["expressions"]["local_resource"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"identifier":{"constant_value":"us-east-1"}}));
                super::LOCAL
            }
            "partial" => {
                v["resource_changes"][4]["change"]["after"]["local_resource"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"type":"AWS::EC2::Subnet","identifier":"external"}));
                super::LOCAL
            }
            "unknown-type" | "sensitive-type" => {
                v["resource_changes"][4]["change"][if case == "unknown-type" {
                    "after_unknown"
                } else {
                    "after_sensitive"
                }] = json!({"local_resource":[{},{},{"type":true}]});
                super::LOCAL
            }
            "object" => {
                let item =
                    v["resource_changes"][4]["change"]["after"]["remote_resource"][0].clone();
                v["resource_changes"][4]["change"]["after"]["remote_resource"] = item;
                super::REMOTE
            }
            "duplicate" => {
                let item = v["resource_changes"][4]["change"]["after"]["local_resource"][0].clone();
                v["resource_changes"][4]["change"]["after"]["local_resource"][1] = item;
                super::LOCAL
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, 4, p), "{case}");
        assert!(verified(&v, 4, "scope_arn"));
    }
}
#[test]
fn network_monitors_probe_context_does_not_place_monitor_and_survives_only_complete_network_proof()
{
    for case in ["known", "mismatch", "unknown", "filtered"] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "mismatch" => v["resource_changes"][1]["change"]["after"]["vpc_id"] = json!("other"),
            "unknown" => {
                v["resource_changes"][1]["change"]["after_unknown"] = json!({"vpc_id":true})
            }
            "filtered" => v["resource_changes"][1]["change"]["actions"] = json!(["no-op"]),
            _ => (),
        }
        assert!(verified(&v, 6, "monitor_name"));
        let g = graph(&v);
        assert!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == super::PROBE)
                .unwrap()
                .member_owner
                .is_some()
        );
        assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment
            && matches!(
                g.nodes[e.to].resource_type.as_str(),
                super::FLOW | super::MONITOR
            )));
        let selected = crate::view::single(
            &g,
            &crate::view::Options {
                changed_only: case == "filtered",
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(placed(&selected), case == "known", "{case}");
    }
}
#[test]
fn network_monitors_preserve_explicit_alias_lifecycle_and_named_state_identity() {
    for case in [
        "same-account-alias",
        "update",
        "replacement",
        "import",
        "move",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "same-account-alias" => {
                let mut alias = v["configuration"]["provider_config"]["aws"].clone();
                alias["alias"] = json!("other");
                v["configuration"]["provider_config"]["aws.other"] = alias;
                v["configuration"]["root_module"]["resources"][5]["provider_config_key"] =
                    json!("aws.other");
            }
            "update" => v["resource_changes"][6]["change"]["actions"] = json!(["update"]),
            "replacement" => {
                v["resource_changes"][6]["change"]["actions"] = json!(["delete", "create"])
            }
            "import" => {
                v["resource_changes"][6]["change"]["importing"] = json!({"id":"network,probe"})
            }
            "move" => {
                v["resource_changes"][6]["previous_address"] = json!("aws_networkmonitor_probe.old")
            }
            _ => (),
        }
        assert!(verified(&v, 6, "monitor_name"), "{case}");
        assert_eq!(graph(&v).coverage.selected.resources, 9);
    }
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
}
#[test]
fn network_monitors_filtered_probe_and_private_values_do_not_manufacture_resources() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let selected = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            focus: Some("aws_networkmonitor_probe.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(!placed(&selected));
    assert_eq!(selected.nodes[0].resource_type, super::PROBE);
    assert!(selected.edges.is_empty());
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
        assert!(!output.content.contains("192.0.2.1"));
    }
}

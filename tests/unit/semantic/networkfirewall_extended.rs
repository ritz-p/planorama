use crate::{
    model::{EdgeKind, Graph, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/networkfirewall-extended-plan.json");
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
            && g.nodes[e.to].resource_type == super::ASSOCIATION
            && g.nodes[e.from].address == "aws_vpc.endpoint"
    })
}
#[test]
fn networkfirewall_extended_exact_contracts_preserve_independent_coverage() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (12, super::CA),
        (12, super::CERT),
        (13, super::IPSET),
        (14, super::TLS_PATH),
        (16, "transit_gateway_id"),
        (17, "firewall_arn"),
        (17, "vpc_id"),
        (17, super::SUBNET),
        (18, "transit_gateway_attachment_id"),
        (19, super::CLUSTER),
        (20, super::CLUSTER),
        (21, "resource_arn"),
        (22, "resource_arn"),
        (23, super::IPSET),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 24);
    assert_eq!(g.nodes.len(), 24);
    assert!(placed(&g));
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        if n.resource_type != super::FIREWALL {
            assert_eq!(n.placement, Placement::Logical);
        }
        assert!(n.member_owner.is_none());
        assert!(
            n.references
                .iter()
                .all(|r| !super::indexed(&n.resource_type, &r.attribute))
        );
    }
    let fw = g
        .nodes
        .iter()
        .position(|n| n.address == "aws_networkfirewall_firewall.vpc")
        .unwrap();
    assert!(g.edges.iter().any(|e| e.to == fw
        && e.kind == EdgeKind::Containment
        && g.nodes[e.from].address == "aws_vpc.firewall"));
}
#[test]
fn networkfirewall_extended_proofs_require_known_current_explicit_exact_endpoints() {
    for (t, path, canonical, s, out) in [
        (
            12,
            "tls_inspection_configuration.0.server_certificate_configuration.0.certificate_authority_arn",
            super::CA,
            5,
            "arn",
        ),
        (
            12,
            "tls_inspection_configuration.0.server_certificate_configuration.0.server_certificate.0.resource_arn",
            super::CERT,
            6,
            "arn",
        ),
        (
            13,
            "rule_group.0.reference_sets.0.ip_set_references.0.ip_set_reference.0.reference_arn",
            super::IPSET,
            10,
            "arn",
        ),
        (
            14,
            "firewall_policy.0.tls_inspection_configuration_arn",
            super::TLS_PATH,
            12,
            "arn",
        ),
        (16, "transit_gateway_id", "transit_gateway_id", 4, "id"),
        (17, "firewall_arn", "firewall_arn", 15, "arn"),
        (17, "vpc_id", "vpc_id", 0, "id"),
        (17, "subnet_mapping.0.subnet_id", super::SUBNET, 1, "id"),
        (
            19,
            "container_monitoring_configuration.0.cluster_arn",
            super::CLUSTER,
            7,
            "arn",
        ),
        (21, "resource_arn", "resource_arn", 14, "arn"),
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
                        json!({"references":[format!("{a}.wrong")]}),
                    );
                }
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "region" | "account" => {
                    let mut provider = v["configuration"]["provider_config"]["aws"].clone();
                    provider["alias"] = json!("other");
                    if case == "region" {
                        provider["expressions"]["region"]["constant_value"] = json!("us-west-2");
                    } else {
                        provider["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                            json!("arn:aws:iam::999999999999:role/Test");
                    }
                    v["configuration"]["provider_config"]["aws.other"] = provider;
                    v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                        json!("aws.other");
                }
                "consumer-delete" => {
                    v["resource_changes"][t]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => {
                    v["resource_changes"][s]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                "source-sensitive" => {
                    v["resource_changes"][s]["change"]["after_sensitive"] = json!({out:true})
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, canonical), "{t} {canonical} {case}");
        }
    }
}
#[test]
fn networkfirewall_extended_collections_and_discriminators_do_not_accept_partial_or_mixed_sets() {
    for case in [
        "subnet-shape",
        "certificate-shape",
        "certificate-partial",
        "cluster-type",
        "cluster-partial",
        "rule-type",
        "ipset-partial",
        "mixed-ipsets",
        "pca",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let (t, p) = match case {
            "subnet-shape" => {
                v["resource_changes"][17]["change"]["after"]["subnet_mapping"] =
                    json!({"subnet_id":"subnet-endpoint"});
                (17, super::SUBNET)
            }
            "certificate-shape" => {
                let x =
                    v["resource_changes"][12]["change"]["after"]["tls_inspection_configuration"][0]
                        .clone();
                v["resource_changes"][12]["change"]["after"]["tls_inspection_configuration"] = x;
                (12, super::CA)
            }
            "certificate-partial" => {
                v["resource_changes"][12]["change"]["after"]["tls_inspection_configuration"][0]["server_certificate_configuration"][0]["server_certificate"].as_array_mut().unwrap().push(json!({"resource_arn":"external"}));
                (12, super::CERT)
            }
            "cluster-type" => {
                v["resource_changes"][19]["change"]["after"]["type"] = json!("EKS");
                (19, super::CLUSTER)
            }
            "cluster-partial" => {
                v["resource_changes"][19]["change"]["after"]["container_monitoring_configuration"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"cluster_arn":"external"}));
                (19, super::CLUSTER)
            }
            "rule-type" => {
                v["resource_changes"][13]["change"]["after"]["type"] = json!("STATELESS");
                (13, super::IPSET)
            }
            "ipset-partial" => {
                v["resource_changes"][13]["change"]["after"]["rule_group"][0]["reference_sets"][0]
                    ["ip_set_references"]
                    .as_array_mut()
                    .unwrap()
                    .push(
                        json!({"key":"missing","ip_set_reference":[{"reference_arn":"external"}]}),
                    );
                (13, super::IPSET)
            }
            "mixed-ipsets" => {
                let a = v["resource_changes"][19]["change"]["after"]["container_association_arn"]
                    .clone();
                set(
                    &mut v,
                    "resource_changes.13.change.after.rule_group.0.reference_sets.0.ip_set_references.1.ip_set_reference.0.reference_arn",
                    a,
                );
                set(
                    &mut v,
                    "configuration.root_module.resources.13.expressions.rule_group.0.reference_sets.0.ip_set_references.1.ip_set_reference.0.reference_arn",
                    json!({"references":["aws_networkfirewall_container_association.ecs.container_association_arn"]}),
                );
                (13, super::IPSET)
            }
            "pca" => {
                v["resource_changes"][5]["type"] = json!("aws_acmpca_certificate_authority");
                (12, super::CA)
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, t, p), "{case}");
    }
}
#[test]
fn networkfirewall_extended_attachment_requires_exact_firewall_computed_output_and_tgw_mode() {
    for case in [
        "unknown",
        "sensitive",
        "literal",
        "singular-path",
        "wrong-source",
        "wrong-value",
        "multiple-status",
        "vpc-mode",
        "unknown-az",
        "duplicate-az",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown" | "sensitive" => {
                v["resource_changes"][16]["change"][if case == "unknown" {
                    "after_unknown"
                } else {
                    "after_sensitive"
                }] = json!({"firewall_status":true})
            }
            "literal" => {
                v["configuration"]["root_module"]["resources"][18]["expressions"]["transit_gateway_attachment_id"] =
                    json!({"constant_value":"tgw-attach-main"})
            }
            "singular-path" => {
                v["configuration"]["root_module"]["resources"][18]["expressions"]["transit_gateway_attachment_id"] = json!({"references":["aws_networkfirewall_firewall.tgw.firewall_status[0].transit_gateway_attachment_sync_state[0].attachment_id"]})
            }
            "wrong-source" => {
                v["resource_changes"][16]["type"] = json!("aws_ec2_transit_gateway_vpc_attachment");
            }
            "wrong-value" => {
                v["resource_changes"][18]["change"]["after"]["transit_gateway_attachment_id"] =
                    json!("other")
            }
            "multiple-status" => {
                let x = v["resource_changes"][16]["change"]["after"]["firewall_status"][0].clone();
                v["resource_changes"][16]["change"]["after"]["firewall_status"]
                    .as_array_mut()
                    .unwrap()
                    .push(x);
            }
            "vpc-mode" => {
                v["resource_changes"][16]["change"]["after"]["vpc_id"] = json!("vpc-firewall")
            }
            "unknown-az" => {
                v["resource_changes"][16]["change"]["after_unknown"] =
                    json!({"availability_zone_mapping":true})
            }
            "duplicate-az" => {
                v["resource_changes"][16]["change"]["after"]["availability_zone_mapping"][1]["availability_zone_id"] =
                    json!("use1-az1")
            }
            _ => unreachable!(),
        }
        assert!(!verified(&v, 18, "transit_gateway_attachment_id"), "{case}");
    }
}
#[test]
fn networkfirewall_extended_network_context_is_independent_and_requires_visible_common_vpc() {
    for case in ["known", "mismatch", "unknown", "partial", "filtered"] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "mismatch" => {
                v["resource_changes"][1]["change"]["after"]["vpc_id"] = json!("vpc-firewall")
            }
            "unknown" => {
                v["resource_changes"][17]["change"]["after_unknown"] =
                    json!({"subnet_mapping":true})
            }
            "partial" => v["resource_changes"][17]["change"]["after"]["subnet_mapping"]
                .as_array_mut()
                .unwrap()
                .push(json!({"subnet_id":"external"})),
            "filtered" => v["resource_changes"][1]["change"]["actions"] = json!(["no-op"]),
            _ => (),
        }
        assert!(verified(&v, 17, "firewall_arn"));
        let g = graph(&v);
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
fn networkfirewall_extended_cross_account_tgw_qualifies_only_verified_gateway_owner() {
    for qualified in [false, true] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let mut alias = v["configuration"]["provider_config"]["aws"].clone();
        alias["alias"] = json!("owner");
        alias["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
            json!("arn:aws:iam::999999999999:role/Test");
        v["configuration"]["provider_config"]["aws.owner"] = alias;
        for t in [4, 18] {
            v["configuration"]["root_module"]["resources"][t]["provider_config_key"] =
                json!("aws.owner");
        }
        if qualified {
            v["resource_changes"][16]["change"]["after"]["transit_gateway_owner_account_id"] =
                json!("999999999999");
        }
        assert_eq!(verified(&v, 16, "transit_gateway_id"), qualified);
        assert_eq!(verified(&v, 18, "transit_gateway_attachment_id"), qualified);
        assert_eq!(
            graph(&v)
                .nodes
                .iter()
                .find(|n| n.address == "aws_networkfirewall_firewall.tgw")
                .unwrap()
                .entity
                .scope
                .as_ref()
                .unwrap()
                .account,
            crate::model::AwsAccountScope::new("123456789012")
        );
        let raw = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert!(
            !raw.nodes
                .iter()
                .find(|n| n.resource_type == super::ACCEPTER)
                .unwrap()
                .references
                .iter()
                .any(|r| r.label == "Firewall attachment source" && r.complete)
        );
    }
}
#[test]
fn networkfirewall_extended_named_states_filter_and_outputs_keep_safe_identity() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let selected = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            focus: Some("aws_networkfirewall_vpc_endpoint_association.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(!placed(&selected));
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
    }
}

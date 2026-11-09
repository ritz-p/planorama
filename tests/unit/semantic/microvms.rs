use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/microvms-plan.json");

fn verified(v: &Value, target: usize, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    let address = v["resource_changes"][target]["address"].as_str().unwrap();
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == address
            && r.attribute == path
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn exact_image_versions_and_roles_preserve_current_identity() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (source, target, path, label) in [
        (1, 5, "base_image_arn", "Base image"),
        (5, 6, "image_arn", "MicroVM image"),
        (0, 5, "build_role_arn", "Image build role"),
        (0, 6, "execution_role_arn", "MicroVM execution role"),
    ] {
        let mut pair = v.clone();
        pair["resource_changes"] =
            json!([v["resource_changes"][source], v["resource_changes"][target]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][source],
            v["configuration"]["root_module"]["resources"][target]
        ]);
        if source == 0 {
            super::super::service_references::tests::check_service_fixture(
                &pair.to_string(),
                label,
                path,
            );
            pair["resource_changes"][0]["change"]["after_sensitive"] = json!({"arn":true});
            assert!(!verified(&pair, 1, path));
        } else {
            super::super::service_references::tests::check_current_scalar_fixture(
                &pair.to_string(),
                label,
                path,
                "arn",
            );
        }
    }
    for (source, target, path, version) in [
        (1, 5, "base_image_arn", "base_image_version"),
        (5, 6, "image_arn", "image_version"),
    ] {
        for case in [
            "mismatch",
            "missing",
            "unknown",
            "sensitive",
            "source-unknown",
            "source-sensitive",
        ] {
            let mut v = v.clone();
            match case {
                "mismatch" => {
                    v["resource_changes"][target]["change"]["after"][version] = json!("other")
                }
                "missing" => {
                    v["resource_changes"][target]["change"]["after"][version] = Value::Null
                }
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({version:true})
                }
                "sensitive" => {
                    v["resource_changes"][target]["change"]["after_sensitive"] =
                        json!({version:true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] =
                        json!({"image_version":true})
                }
                _ => {
                    v["resource_changes"][source]["change"]["after_sensitive"] =
                        json!({"image_version":true})
                }
            }
            assert!(!verified(&v, target, path), "{case} {path}");
        }
    }
}

#[test]
fn connector_lists_require_complete_managed_arn_provenance() {
    for (target, path) in [
        (5, "egress_network_connectors"),
        (6, "ingress_network_connectors"),
    ] {
        for case in [
            "valid",
            "marker",
            "unknown",
            "sensitive",
            "deleted",
            "data",
            "deposed",
            "wrong-output",
            "literal",
            "region",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "marker" => v["resource_changes"][target]["change"]["after"][path]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("INTERNET_EGRESS")),
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({path:[true]})
                }
                "sensitive" => {
                    v["resource_changes"][target]["change"]["after_sensitive"] =
                        json!({path:[true]})
                }
                "deleted" => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
                "data" => v["resource_changes"][2]["mode"] = json!("data"),
                "deposed" => v["resource_changes"][2]["deposed"] = json!("old"),
                "wrong-output" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][path] =
                        json!({"references":["aws_lambdacore_network_connector.shared.name"]})
                }
                "literal" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][path] =
                        json!({})
                }
                "region" => {
                    v["configuration"]["root_module"]["resources"][2]["expressions"]["region"] =
                        json!({"constant_value":"us-west-2"})
                }
                _ => {}
            }
            assert_eq!(verified(&v, target, path), case == "valid", "{case} {path}");
        }
    }
}

#[test]
fn logging_requires_singleton_blocks_and_explicit_matching_parent() {
    for case in [
        "valid",
        "wrong-parent",
        "literal-parent",
        "literal-group",
        "unknown-group",
        "sensitive-stream",
        "disabled",
        "unknown-disabled",
        "shape",
        "deleted-parent",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "wrong-parent" => {
                v["resource_changes"][4]["change"]["after"]["log_group_name"] = json!("other")
            }
            "literal-parent" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"] = json!({})
            }
            "literal-group" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["logging"][0]["cloudwatch"]
                    [0]["log_group"] = json!({})
            }
            "unknown-group" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"logging":[{"cloudwatch":[{"log_group":true}]}]})
            }
            "sensitive-stream" => {
                v["resource_changes"][4]["change"]["after_sensitive"] = json!({"name":true})
            }
            "disabled" => {
                v["resource_changes"][6]["change"]["after"]["logging"][0]["disabled"] = json!([{}])
            }
            "unknown-disabled" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"logging":[{"disabled":true}]})
            }
            "shape" => {
                v["resource_changes"][6]["change"]["after"]["logging"] =
                    v["resource_changes"][6]["change"]["after"]["logging"][0].clone()
            }
            "deleted-parent" => v["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            _ => {}
        }
        assert_eq!(verified(&v, 6, super::STREAM), case == "valid", "{case}");
    }
    let mut p = plan::parse(FIXTURE).unwrap();
    for node in &mut p.nodes {
        node.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_cloudwatch_log_group")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    let raw = semantic::Input::new(&p);
    assert!(!raw.attributes.iter().any(
        |r| r.attribute == super::STREAM && super::super::service_references::verified(&raw, r)
    ));
}

#[test]
fn microvm_is_an_independent_workload_without_inferred_traffic() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 7);
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_lambdamicrovms_microvm")
            .unwrap()
            .placement,
        crate::model::Placement::Spatial
    );
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    assert!(g.edges.iter().all(|e| !matches!(
        e.kind,
        crate::model::EdgeKind::Containment | crate::model::EdgeKind::Connection
    )));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            crate::generate(
                FIXTURE,
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("MicroVM image")
        );
    }
}

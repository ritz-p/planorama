use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/appsync-domains-plan.json");
fn input() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn verified(v: &Value, address: &str, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == address
            && r.attribute == path
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn certificates_and_associations_preserve_shared_domain_and_api_identity() {
    let v = input();
    for name in ["acm", "iam"] {
        assert!(verified(
            &v,
            &format!("aws_appsync_domain_name.{name}"),
            "certificate_arn"
        ));
        for field in ["api_id", "domain_name"] {
            assert!(verified(
                &v,
                &format!("aws_appsync_domain_name_api_association.{name}"),
                field
            ));
        }
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 8);
    assert_eq!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type == "aws_appsync_graphql_api")
            .count(),
        1
    );
    assert_eq!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type == super::DOMAIN)
            .count(),
        2
    );
    assert!(
        !g.nodes
            .iter()
            .any(|n| n.resource_type == super::ASSOCIATION)
    );
    assert_eq!(
        g.edges
            .iter()
            .filter(|e| e.kind == crate::model::EdgeKind::Association)
            .count(),
        2
    );
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
}

#[test]
fn certificate_proof_requires_east_region_arn_and_current_managed_evidence() {
    for case in 0..14 {
        let mut v = input();
        match case {
            0 => {
                v["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"})
            }
            1 => v["resource_changes"][0]["change"]["after_unknown"] = json!({"arn":true}),
            2 => {
                v["resource_changes"][4]["change"]["after_sensitive"] =
                    json!({"certificate_arn":true})
            }
            3 => v["resource_changes"][4]["change"]["after"]["certificate_arn"] = "wrong".into(),
            4 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["certificate_arn"] =
                    json!({"references":["aws_acm_certificate.main.id"]})
            }
            5 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["certificate_arn"] = json!({"constant_value":"arn:aws:acm:us-east-1:111111111111:certificate/example"})
            }
            6 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["certificate_arn"] =
                    json!({"references":["aws_acm_certificate.main.arn","var.unresolved"]})
            }
            7 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["certificate_arn"] = json!({"references":["aws_acm_certificate.main.arn","aws_iam_server_certificate.main.arn"]})
            }
            8 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            9 => v["resource_changes"][0]["deposed"] = "old".into(),
            10 => v["resource_changes"][0]["mode"] = "data".into(),
            11 => v["resource_changes"][0]["provider_name"] = "example.com/custom/aws".into(),
            12 => v["resource_changes"][4]["change"]["actions"] = json!(["delete"]),
            _ => {
                v["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]})
            }
        }
        assert!(
            !verified(&v, "aws_appsync_domain_name.acm", "certificate_arn"),
            "case {case}"
        );
    }
}

#[test]
fn association_lowering_requires_both_exact_service_outputs() {
    for case in 0..8 {
        let mut v = input();
        match case {
            0 => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["domain_name"] =
                    json!({"constant_value":"acm.example.com"})
            }
            1 => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["domain_name"] =
                    json!({"references":["aws_appsync_domain_name.acm.id"]})
            }
            2 => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["api_id"] =
                    json!({"references":["aws_appsync_api.events.api_id"]});
                v["resource_changes"][3]["change"]["after"]["api_id"] = "graphql-api".into();
            }
            3 => v["resource_changes"][6]["change"]["after"]["api_id"] = "wrong".into(),
            4 => v["resource_changes"][4]["change"]["actions"] = json!(["delete"]),
            5 => v["resource_changes"][6]["change"]["actions"] = json!(["delete"]),
            6 => {
                v["configuration"]["provider_config"]["aws.west"] =
                    v["configuration"]["provider_config"]["aws"].clone();
                v["configuration"]["provider_config"]["aws.west"]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"});
                v["configuration"]["root_module"]["resources"][6]["provider_config_key"] =
                    "aws.west".into();
            }
            _ => v["resource_changes"][6]["change"]["after_unknown"] = json!({"domain_name":true}),
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.coverage.selected.resources, 8);
        assert!(
            g.nodes
                .iter()
                .any(|n| n.address == "aws_appsync_domain_name_api_association.acm"),
            "case {case}"
        );
    }
}

#[test]
fn cross_region_certificates_keep_account_boundaries_and_filtered_coverage() {
    let mut v = input();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Fixture"}}]);
    v["configuration"]["provider_config"]["aws.west"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["provider_config"]["aws.west"]["expressions"]["region"] =
        json!({"constant_value":"us-west-2"});
    v["configuration"]["root_module"]["resources"][4]["provider_config_key"] = "aws.west".into();
    let mut parsed = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&parsed);
    assert!(raw.attributes.iter().any(|r| raw.nodes[r.target].address
        == "aws_appsync_domain_name.acm"
        && r.attribute == "certificate_arn"
        && super::super::service_references::verified(&raw, r)));
    assert!(!verified(
        &v,
        "aws_appsync_domain_name.acm",
        "certificate_arn"
    ));
    parsed
        .nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_acm_certificate")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("222222222222");
    let raw = semantic::Input::new(&parsed);
    assert!(!raw.attributes.iter().any(|r| raw.nodes[r.target].address
        == "aws_appsync_domain_name.acm"
        && r.attribute == "certificate_arn"
        && super::super::service_references::verified(&raw, r)));
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_appsync_domain_name.acm".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|name| crate::model::PlanInput {
                state_id: crate::model::StateId::new(name).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            !crate::generate(
                FIXTURE,
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("PRIVATE_")
        );
    }
}

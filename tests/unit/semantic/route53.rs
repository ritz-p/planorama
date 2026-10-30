use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    let mut v: Value =
        serde_json::from_str(include_str!("../../fixtures/route53-plan.json")).unwrap();
    v["configuration"]["provider_config"] = json!({"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}});
    for r in v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
    {
        r["provider_config_key"] = json!("aws");
    }
    for (kind, name, values) in [
        (
            "aws_route53_delegation_set",
            "main",
            json!({"id":"delegation-one"}),
        ),
        ("aws_vpc", "other", json!({"id":"vpc-other"})),
    ] {
        add(&mut v, kind, name, values, json!({}));
    }
    v["resource_changes"][3]["change"]["after"] = json!({"zone_id":"private-zone", "delegation_set_id":"delegation-one", "vpc":[{"vpc_id":"vpc-other", "vpc_region":"us-east-1"},{"vpc_id":"vpc-synthetic","vpc_region":"us-east-1"}]});
    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({"delegation_set_id":{"references":["aws_route53_delegation_set.main.id"]},"vpc":[{"vpc_id":{"references":["aws_vpc.network.id"]}},{"vpc_id":{"references":["aws_vpc.other.id"]}}]});
    for kind in [
        "aws_route53_zone_association",
        "aws_route53_vpc_association_authorization",
    ] {
        add(
            &mut v,
            kind,
            "main",
            json!({"zone_id":"private-zone", "vpc_id":"vpc-other","vpc_region":"us-east-1"}),
            json!({"zone_id":{"references":["aws_route53_zone.private.zone_id"]},"vpc_id":{"references":["aws_vpc.other.id"]}}),
        );
    }
    for address in ["aws_route53_zone.public", "aws_lb.web", "aws_vpc.network"] {
        v["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(json!({"address":address,"provider_config_key":"aws","expressions":{}}));
    }
    v
}

fn add(v: &mut Value, kind: &str, name: &str, values: Value, expressions: Value) {
    let address = format!("{kind}.{name}");
    v["resource_changes"].as_array_mut().unwrap().push(
        json!({"address":address,"type":kind,"change":{"actions":["create"],"after":values}}),
    );
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":address,"provider_config_key":"aws","expressions":expressions}));
}

fn verified(v: &Value, kind: &str, field: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    p.attributes.iter().any(|r| {
        p.nodes[r.target].resource_type == kind
            && r.attribute == field
            && r.service_reference_verified
    })
}

#[test]
fn route53_complete_vpc_sets_and_distinct_authorization_references() {
    let v = fixture();
    for (kind, field) in [
        ("aws_route53_zone", "vpc.vpc_id"),
        ("aws_route53_zone", "delegation_set_id"),
        ("aws_route53_zone_association", "zone_id"),
        ("aws_route53_zone_association", "vpc_id"),
        ("aws_route53_vpc_association_authorization", "zone_id"),
        ("aws_route53_vpc_association_authorization", "vpc_id"),
    ] {
        assert!(verified(&v, kind, field), "{kind} {field}");
    }
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let zone = g
        .nodes
        .iter()
        .find(|n| n.address == "aws_route53_zone.private")
        .unwrap();
    assert!(zone.entity.scope.as_ref().unwrap().global);
    assert!(
        zone.references
            .iter()
            .any(|r| r.label == "Associated VPC" && r.complete && r.targets.len() == 2)
    );
    assert!(g.nodes.iter().any(|n| {
        n.references
            .iter()
            .any(|r| r.label == "Authorized VPC" && r.complete)
    }));
    assert_eq!(g.coverage.selected.resources, 9);
    let mut indexed: Value =
        serde_json::from_str(&v.to_string().replace("aws_vpc.other", "aws_vpc.other[0]")).unwrap();
    indexed["configuration"]["root_module"]["resources"][3]["address"] = json!("aws_vpc.other");
    assert!(verified(&indexed, "aws_route53_zone", "vpc.vpc_id"));
}

#[test]
fn route53_collection_requires_complete_current_correlated_evidence() {
    for case in 0..12 {
        let mut v = fixture();
        match case {
            0 => {
                v["resource_changes"][3]["change"]["after"]["vpc"][0]["vpc_region"] =
                    json!("us-west-2")
            }
            1 => v["resource_changes"][3]["change"]["after"]["vpc"][0]["vpc_id"] = json!("wrong"),
            2 => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"vpc":[{"vpc_id":true},{}]})
            }
            3 => {
                v["resource_changes"][3]["change"]["after_sensitive"] =
                    json!({"vpc":[{}, {"vpc_region":true}]})
            }
            4 => v["resource_changes"][6]["change"]["actions"] = json!(["delete"]),
            5 => v["resource_changes"][6]["provider_name"] = json!("example.com/foreign/aws"),
            6 => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["vpc"][1] =
                    json!({"vpc_id":{"constant_value":"vpc-other"}})
            }
            7 => {
                v["resource_changes"][3]["change"]["after"]["vpc"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            8 => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["vpc"][1]["vpc_id"] =
                    json!({"references":["aws_vpc.other.id","var.dynamic"]})
            }
            9 => v["resource_changes"][6]["change"]["after_sensitive"] = json!({"id":true}),
            10 => v["resource_changes"][6]["deposed"] = json!("old"),
            _ => {
                v["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]})
            }
        }
        assert!(
            !verified(&v, "aws_route53_zone", "vpc.vpc_id"),
            "case {case}"
        );
        let diagnostics = crate::provider::diagnostics(&plan::parse(&v.to_string()).unwrap());
        assert!(
            diagnostics.iter().any(|d| d.attribute == "vpc.vpc_id"),
            "case {case}"
        );
    }
}

#[test]
fn route53_helper_fields_and_delegation_require_typed_identity() {
    for case in 0..6 {
        let mut v = fixture();
        match case {
            0 => v["resource_changes"][7]["change"]["after"]["vpc_region"] = json!("us-west-2"),
            1 => v["resource_changes"][7]["change"]["after_unknown"] = json!({"vpc_region":true}),
            2 => v["resource_changes"][7]["change"]["after_sensitive"] = json!({"vpc_id":true}),
            3 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.other.arn"]})
            }
            4 => {
                v["configuration"]["root_module"]["resources"][4]["provider_config_key"] =
                    json!("aws.unresolved")
            }
            _ => v["resource_changes"][7]["change"]["actions"] = json!(["delete"]),
        }
        assert!(
            !verified(&v, "aws_route53_zone_association", "vpc_id"),
            "case {case}"
        );
    }
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][1]["expressions"]["delegation_set_id"] =
        json!({"references":["aws_route53_delegation_set.main.arn"]});
    assert!(!verified(&v, "aws_route53_zone", "delegation_set_id"));
}

#[test]
fn route53_provider_accounts_are_independent_of_vpc_regions_and_owning_account() {
    let mut v = fixture();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
    v["configuration"]["provider_config"]["aws.west"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["provider_config"]["aws.west"]["expressions"]["region"] =
        json!({"constant_value":"us-west-2"});
    v["configuration"]["root_module"]["resources"][3]["provider_config_key"] = json!("aws.west");
    v["resource_changes"][3]["change"]["after"]["vpc"][0]["vpc_region"] = json!("us-west-2");
    let proven = |v: &Value| {
        let p = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        semantic::transform(&p).nodes.iter().any(|n| {
            n.address == "aws_route53_zone.private"
                && n.references.iter().any(|r| {
                    r.attribute == "vpc.vpc_id" && r.label == "Associated VPC" && r.complete
                })
        })
    };
    assert!(proven(&v));
    v["configuration"]["provider_config"]["aws.west"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::222222222222:role/Test"}}]);
    v["resource_changes"][7]["change"]["after"]["owning_account"] = json!("222222222222");
    assert!(!proven(&v));
}

#[test]
fn route53_named_states_do_not_supply_missing_endpoints() {
    let v = fixture();
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(6);
    assert!(!verified(&filtered, "aws_route53_zone", "vpc.vpc_id"));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: filtered.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    }
}

use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/ipam-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn input(v: &Value, accounts: bool) -> semantic::Input {
    let mut p = plan::parse(&v.to_string()).unwrap();
    if accounts {
        for n in &mut p.nodes {
            if let Some(c) = &mut n.provider_configuration {
                c.account = crate::model::AwsAccountScope::new("123456789012");
            }
        }
    }
    semantic::Input::new(&p)
}
fn verified(raw: &semantic::Input, v: &Value, target: usize, path: &str) -> bool {
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == v["resource_changes"][target]["address"].as_str().unwrap()
            && r.attribute == path
            && super::super::service_references::verified(raw, r)
    })
}
fn complete(v: &Value, target: usize, path: &str, accounts: bool) -> bool {
    verified(&input(v, accounts), v, target, path)
}

#[test]
fn scalar_ipam_owners_require_current_complete_explicit_typed_outputs() {
    let v = fixture();
    for (target, source, path, label) in [
        (1, 0, "ipam_id", "Scope IPAM"),
        (2, 1, "ipam_scope_id", "Pool allocation scope"),
        (11, 2, "source_ipam_pool_id", "Parent IPAM pool"),
        (7, 5, "subnet_id", "Reservation subnet"),
        (8, 2, "ipam_pool_id", "IPAM pool owner"),
        (9, 2, "ipam_pool_id", "IPAM pool owner"),
        (10, 2, "ipam_pool_id", "IPAM pool owner"),
        (4, 2, "ipv4_ipam_pool_id", "IPv4 IPAM pool"),
        (4, 3, "ipv6_ipam_pool_id", "IPv6 IPAM pool"),
        (5, 2, "ipv4_ipam_pool_id", "IPv4 IPAM pool"),
        (5, 3, "ipv6_ipam_pool_id", "IPv6 IPAM pool"),
        (14, 4, "vpc_id", "CIDR VPC owner"),
        (14, 2, "ipv4_ipam_pool_id", "IPv4 IPAM pool"),
        (15, 4, "vpc_id", "CIDR VPC owner"),
        (15, 3, "ipv6_ipam_pool_id", "IPv6 IPAM pool"),
    ] {
        let mut pair = v.clone();
        pair["resource_changes"] =
            json!([v["resource_changes"][source], v["resource_changes"][target]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][source],
            v["configuration"]["root_module"]["resources"][target]
        ]);
        super::super::service_references::tests::check_current_scalar_fixture(
            &pair.to_string(),
            label,
            path,
            "id",
        );
    }
}

#[test]
fn allocation_family_locale_and_ipv6_modes_do_not_invent_networks() {
    for case in [
        "family",
        "locale",
        "generated",
        "byoip",
        "unknown-mode",
        "reservation-type",
        "parent-family",
        "parent-scope",
    ] {
        let mut v = fixture();
        let (target, path) = match case {
            "family" => {
                v["resource_changes"][2]["change"]["after"]["address_family"] = json!("ipv6");
                (4, "ipv4_ipam_pool_id")
            }
            "locale" => {
                v["resource_changes"][2]["change"]["after"]["locale"] = json!("us-west-2");
                (4, "ipv4_ipam_pool_id")
            }
            "generated" => {
                v["resource_changes"][15]["change"]["after"]["assign_generated_ipv6_cidr_block"] =
                    json!(true);
                (15, "ipv6_ipam_pool_id")
            }
            "byoip" => {
                v["resource_changes"][15]["change"]["after"]["ipv6_pool"] =
                    json!("ipv6pool-literal");
                (15, "ipv6_ipam_pool_id")
            }
            "unknown-mode" => {
                v["resource_changes"][15]["change"]["after_unknown"] =
                    json!({"assign_generated_ipv6_cidr_block":true});
                (15, "ipv6_ipam_pool_id")
            }
            "reservation-type" => {
                v["resource_changes"][7]["change"]["after"]["reservation_type"] = json!("other");
                (7, "subnet_id")
            }
            "parent-family" => {
                v["resource_changes"][11]["change"]["after"]["address_family"] = json!("ipv6");
                (11, "source_ipam_pool_id")
            }
            _ => {
                v["resource_changes"][11]["change"]["after"]["ipam_scope_id"] =
                    json!("another-scope");
                (11, "source_ipam_pool_id")
            }
        };
        assert!(!complete(&v, target, path, true), "{case}");
    }
    let mut defaults = fixture();
    defaults["resource_changes"][0]["change"]["after"]["private_default_scope_id"] =
        json!("scope-main");
    defaults["configuration"]["root_module"]["resources"][2]["expressions"]["ipam_scope_id"] =
        json!({"references":["aws_vpc_ipam.main.private_default_scope_id"]});
    assert!(!complete(&defaults, 2, "ipam_scope_id", true));
    let mut allocation = fixture();
    allocation["resource_changes"][9]["change"]["after"]["resource_id"] = json!("vpc-main");
    allocation["configuration"]["root_module"]["resources"][9]["expressions"]["resource_id"] =
        json!({"references":["aws_vpc.main.id"]});
    assert!(!complete(&allocation, 9, "resource_id", true));
    let mut v = fixture();
    v["resource_changes"][7]["change"]["after_unknown"] = json!({"cidr_block":true});
    assert!(complete(&v, 7, "subnet_id", false));
    v["configuration"]["root_module"]["resources"][7]["expressions"] = json!({});
    assert!(!complete(&v, 7, "subnet_id", false));
}

#[test]
fn planning_vpc_requires_correlated_type_owner_region_and_parent_locale() {
    let base = fixture();
    assert!(complete(&base, 11, "source_resource.resource_id", true));
    assert!(!complete(&base, 11, "source_resource.resource_id", false));
    let mut raw = input(&base, true);
    raw.graph
        .nodes
        .iter_mut()
        .find(|n| n.address == "aws_vpc_ipam_pool.parent")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    assert!(!verified(&raw, &base, 11, "source_resource.resource_id"));
    for case in [
        "literal",
        "partial",
        "type",
        "owner",
        "region",
        "parent-locale",
        "target-locale",
        "parent-literal",
        "unknown",
        "sensitive",
        "malformed-mask",
        "planned-sensitive",
        "shape",
        "deleted",
        "deposed",
        "foreign",
        "data",
    ] {
        let mut v = base.clone();
        match case {
            "literal" => {
                v["configuration"]["root_module"]["resources"][11]["expressions"]["source_resource"] =
                    json!([])
            }
            "partial" => {
                v["configuration"]["root_module"]["resources"][11]["expressions"]["source_resource"]
                    [0]["resource_id"]["references"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("var.other"))
            }
            "type" => {
                v["resource_changes"][11]["change"]["after"]["source_resource"][0]["resource_type"] =
                    json!("subnet")
            }
            "owner" => {
                v["resource_changes"][11]["change"]["after"]["source_resource"][0]["resource_owner"] =
                    json!("210987654321")
            }
            "region" => {
                v["resource_changes"][11]["change"]["after"]["source_resource"][0]["resource_region"] =
                    json!("us-west-2")
            }
            "parent-locale" => {
                v["resource_changes"][2]["change"]["after"]["locale"] = json!("us-west-2")
            }
            "target-locale" => {
                v["resource_changes"][11]["change"]["after"]["locale"] = json!("us-west-2")
            }
            "parent-literal" => {
                v["configuration"]["root_module"]["resources"][11]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("source_ipam_pool_id");
            }
            "unknown" => {
                v["resource_changes"][11]["change"]["after_unknown"] =
                    json!({"source_resource":[{"resource_owner":true}]})
            }
            "sensitive" => {
                v["resource_changes"][11]["change"]["after_sensitive"] =
                    json!({"source_resource":[{"resource_id":true}]})
            }
            "malformed-mask" => {
                v["resource_changes"][11]["change"]["after_unknown"] =
                    json!({"source_resource":{"resource_owner":true}})
            }
            "planned-sensitive" => {
                v["planned_values"] = json!({"root_module":{"resources":[{"address":"aws_vpc_ipam_pool.planning","type":"aws_vpc_ipam_pool","mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","values":v["resource_changes"][11]["change"]["after"],"sensitive_values":{"source_resource":[{"resource_owner":true}]}}]}})
            }
            "deposed" => v["resource_changes"][4]["deposed"] = json!("old"),
            "shape" => {
                let item =
                    v["resource_changes"][11]["change"]["after"]["source_resource"][0].clone();
                v["resource_changes"][11]["change"]["after"]["source_resource"]
                    .as_array_mut()
                    .unwrap()
                    .push(item);
            }
            "deleted" => v["resource_changes"][4]["change"]["actions"] = json!(["delete"]),
            "foreign" => {
                v["resource_changes"][4]["provider_name"] = json!("example.com/foreign/aws")
            }
            _ => v["resource_changes"][4]["mode"] = json!("data"),
        }
        assert!(
            !complete(&v, 11, "source_resource.resource_id", true),
            "{case}"
        );
    }
    let mut v = base.clone();
    v["configuration"]["provider_config"]["aws.control"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
    for i in [0, 1, 2, 3, 11] {
        v["configuration"]["root_module"]["resources"][i]["provider_config_key"] =
            json!("aws.control");
    }
    assert!(complete(&v, 11, "source_resource.resource_id", true));
    assert!(complete(&v, 4, "ipv4_ipam_pool_id", true));
    assert!(!complete(&v, 4, "ipv4_ipam_pool_id", false));
    let control = input(&v, true);
    assert_eq!(
        control
            .nodes
            .iter()
            .find(|n| n.address == "aws_vpc_ipam_pool.parent")
            .unwrap()
            .entity
            .scope
            .as_ref()
            .unwrap()
            .region
            .as_ref()
            .unwrap()
            .0
            .as_str(),
        "us-west-2"
    );
}

#[test]
fn outpost_is_a_singleton_data_contract_with_owner_and_zone_proof() {
    let base = fixture();
    assert!(complete(&base, 5, "outpost_arn", true));
    assert!(!complete(&base, 5, "outpost_arn", false));
    for case in [
        "literal",
        "arn",
        "owner",
        "owner-unknown",
        "zone",
        "zone-id",
        "managed",
        "plural",
        "unknown",
        "sensitive",
        "foreign",
        "deleted",
    ] {
        let mut v = base.clone();
        match case {
            "literal" => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("outpost_arn");
            }
            "arn" => v["resource_changes"][6]["change"]["after"]["arn"] = json!("other"),
            "owner" => {
                v["resource_changes"][6]["change"]["after"]["owner_id"] = json!("210987654321")
            }
            "owner-unknown" => {
                v["resource_changes"][6]["change"]["after_unknown"] = json!({"owner_id":true})
            }
            "zone" => {
                v["resource_changes"][5]["change"]["after"]["availability_zone"] =
                    json!("us-east-1b")
            }
            "zone-id" => {
                v["resource_changes"][5]["change"]["after"]["availability_zone_id"] =
                    json!("use1-az2")
            }
            "managed" => v["resource_changes"][6]["mode"] = json!("managed"),
            "plural" => v["resource_changes"][6]["type"] = json!("aws_outposts_outposts"),
            "unknown" => v["resource_changes"][6]["change"]["after_unknown"] = json!(true),
            "sensitive" => {
                v["resource_changes"][6]["change"]["after_sensitive"] = json!({"arn":true})
            }
            "foreign" => {
                v["resource_changes"][6]["provider_name"] = json!("example.com/foreign/aws")
            }
            _ => v["resource_changes"][6]["change"]["actions"] = json!(["delete"]),
        }
        assert!(!complete(&v, 5, "outpost_arn", true), "{case}");
    }
}

#[test]
fn discovery_pair_and_delegation_keep_home_owner_and_control_account_separate() {
    let base = fixture();
    for path in ["ipam_id", "ipam_resource_discovery_id"] {
        assert!(complete(&base, 13, path, true));
        assert!(!complete(&base, 13, path, false));
    }
    for case in ["owner", "region", "literal", "sensitive"] {
        let mut v = base.clone();
        match case {
            "owner" => {
                v["resource_changes"][13]["change"]["after"]["owner_id"] = json!("210987654321")
            }
            "region" => {
                v["resource_changes"][13]["change"]["after"]["ipam_region"] = json!("us-west-2")
            }
            "literal" => {
                v["configuration"]["root_module"]["resources"][13]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("ipam_resource_discovery_id");
            }
            _ => v["resource_changes"][13]["change"]["after_sensitive"] = json!({"owner_id":true}),
        }
        for path in ["ipam_id", "ipam_resource_discovery_id"] {
            assert!(!complete(&v, 13, path, true), "{case} {path}");
        }
    }
    let mut v = base.clone();
    v["resource_changes"][13]["change"]["after"]["owner_id"] = json!("210987654321");
    v["configuration"]["provider_config"]["aws.discovery"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","alias":"discovery","expressions":{"region":{"constant_value":"us-east-1"}}});
    v["configuration"]["root_module"]["resources"][12]["provider_config_key"] =
        json!("aws.discovery");
    let mut raw = input(&v, true);
    let d = raw
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_vpc_ipam_resource_discovery")
        .unwrap();
    d.provider_configuration.as_mut().unwrap().account =
        crate::model::AwsAccountScope::new("210987654321");
    assert!(verified(&raw, &v, 13, "ipam_id"));
    assert!(verified(&raw, &v, 13, "ipam_resource_discovery_id"));
    let mut home = base.clone();
    home["configuration"]["provider_config"]["aws.home"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","alias":"home","expressions":{"region":{"constant_value":"us-west-2"}}});
    for i in [0, 1, 2, 3, 8, 9, 10, 11] {
        home["configuration"]["root_module"]["resources"][i]["provider_config_key"] =
            json!("aws.home");
    }
    home["resource_changes"][13]["change"]["after"]["ipam_region"] = json!("us-west-2");
    for path in ["ipam_id", "ipam_resource_discovery_id"] {
        assert!(complete(&home, 13, path, true));
    }
    assert!(complete(&base, 17, "delegated_admin_account_id", true));
    let raw = input(&base, true);
    let n = raw
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_vpc_ipam_organization_admin_account")
        .unwrap();
    assert!(n.entity.scope.as_ref().unwrap().global);
    assert!(n.entity.scope.as_ref().unwrap().account.is_none());
    let mut v = base.clone();
    v["resource_changes"][17]["change"]["after"]["delegated_admin_account_id"] =
        json!("123456789012");
    assert!(!complete(&v, 17, "delegated_admin_account_id", true));
}

#[test]
fn helpers_preserve_coverage_filtering_lifecycle_and_safe_named_state_outputs() {
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    let g = semantic::transform(&p);
    assert_eq!(g.coverage.selected.resources, 18);
    for kind in [
        "aws_ec2_subnet_cidr_reservation",
        "aws_vpc_ipv4_cidr_block_association",
        "aws_vpc_ipv6_cidr_block_association",
    ] {
        let n = g.nodes.iter().find(|n| n.resource_type == kind).unwrap();
        assert!(crate::view::routes::owner(&g, n).is_some());
        let selected = crate::view::single(
            &g,
            &crate::view::Options {
                focus: Some(n.address.clone()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.nodes.len(), 1);
        assert!(crate::view::routes::owner(&selected, &selected.nodes[0]).is_none());
    }
    assert!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type.starts_with("aws_vpc_ipam_pool"))
            .all(|n| n.placement == crate::model::Placement::Logical)
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
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
        let r = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!r.content.contains("PRIVATE_"));
        assert!(r.content.contains("IPAM pool"));
    }
}

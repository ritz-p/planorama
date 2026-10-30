use crate::{plan, semantic};
use serde_json::{Value, json};

const TYPES: [&str; 3] = [
    "aws_route53_resolver_config",
    "aws_route53_resolver_dnssec_config",
    "aws_route53_resolver_firewall_config",
];

fn fixture(kind: &str) -> Value {
    let address = format!("{kind}.main");
    json!({"format_version":"1.2",
    "resource_changes":[
        {"address":"aws_vpc.main[0]", "type":"aws_vpc", "change":{"actions":["create"],"after":{"id":"vpc-main"}}},
        {"address":address,"type":kind,"change":{"actions":["create"],"after":{"resource_id":"vpc-main","autodefined_reverse_flag":"ENABLE","validation_status":"ENABLED","firewall_fail_open":"ENABLED"}}}
    ],
    "configuration":{"provider_config":{"aws":{"full_name":"registry.terraform.io/hashicorp/aws", "expressions":{"region":{"constant_value":"us-east-1"}}}},
    "root_module":{"resources":[
        {"address":"aws_vpc.main","provider_config_key":"aws","expressions":{}},
        {"address":address,"provider_config_key":"aws","expressions":{"resource_id":{"references":["aws_vpc.main[0].id"]}}}
    ]}}})
}

fn owned(v: &Value, kind: &str) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let n = g.nodes.iter().find(|n| n.resource_type == kind).unwrap();
    assert_eq!(g.coverage.selected.resources, 2);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && g.nodes[e.to].resource_type == kind)
    );
    n.member_owner
        .as_ref()
        .is_some_and(|o| o.target.address == "aws_vpc.main[0]" && o.attribute == "resource_id")
}

#[test]
fn resolver_helpers_have_unique_vpc_owner_without_network_containment() {
    for kind in TYPES {
        let v = fixture(kind);
        assert!(owned(&v, kind), "{kind}");
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == kind)
                .unwrap()
                .placement,
            crate::model::Placement::Logical
        );
        let mut unknown_behavior = v.clone();
        unknown_behavior["resource_changes"][1]["change"]["after_unknown"] = json!({"autodefined_reverse_flag":true,"validation_status":true,"firewall_fail_open":true});
        assert!(owned(&unknown_behavior, kind));
    }
}

#[test]
fn resolver_helper_owner_requires_complete_current_typed_reference() {
    for kind in TYPES {
        for case in 0..12 {
            let mut v = fixture(kind);
            match case {
                0 => v["resource_changes"][1]["change"]["after"]["resource_id"] = json!("other"),
                1 => {
                    v["resource_changes"][1]["change"]["after_unknown"] =
                        json!({"resource_id":true})
                }
                2 => {
                    v["resource_changes"][1]["change"]["after_sensitive"] =
                        json!({"resource_id":true})
                }
                3 => v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({}),
                4 => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"]["resource_id"] =
                        json!({"references":["aws_vpc.main[0].arn"]})
                }
                5 => v["resource_changes"][0]["mode"] = json!("data"),
                6 => v["resource_changes"][1]["mode"] = json!("data"),
                7 => v["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                8 => v["resource_changes"][0]["deposed"] = json!("old"),
                9 => v["resource_changes"][1]["provider_name"] = json!("example.com/custom/aws"),
                10 => v["resource_changes"][0]["type"] = json!("aws_subnet"),
                _ => {
                    v["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                        json!("aws.missing")
                }
            }
            assert!(!owned(&v, kind), "{kind} case {case}");
        }
    }
}

#[test]
fn resolver_helper_accounts_regions_and_named_states_remain_distinct() {
    let kind = TYPES[0];
    let mut v = fixture(kind);
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
    v["configuration"]["provider_config"]["aws.other"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["root_module"]["resources"][1]["provider_config_key"] = json!("aws.other");
    let owned = |v: &Value| {
        let p = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        semantic::transform(&p)
            .nodes
            .iter()
            .any(|n| n.resource_type == kind && n.member_owner.is_some())
    };
    assert!(owned(&v));
    v["configuration"]["provider_config"]["aws.other"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::222222222222:role/Test"}}]);
    assert!(!owned(&v));
    v["configuration"]["provider_config"]["aws.other"]["expressions"]["region"] =
        json!({"constant_value":"us-west-2"});
    assert!(!owned(&v));
    let complete = fixture(kind);
    let mut filtered = complete.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let result = crate::generate_named(
        &[
            crate::NamedInput {
                state: "one".into(),
                json: complete.to_string(),
            },
            crate::NamedInput {
                state: "two".into(),
                json: filtered.to_string(),
            },
        ],
        &crate::Options {
            format: crate::OutputFormat::Html,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !result
            .content
            .contains("<metadata id=\"cross-state-relationships\">")
    );
}

use serde_json::{Value, json};
const HELPERS: &str = include_str!("../../fixtures/redshift-helpers-plan.json");

fn fixture(input: &str) -> Value {
    serde_json::from_str(input).unwrap()
}
fn index(v: &Value, address: &str) -> usize {
    v["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["address"] == address)
        .unwrap()
}
fn proven(input: &Value, address: &str, path: &str) -> bool {
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    let graph = crate::semantic::transform(&raw);
    graph
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            Some(r.label)
                == super::label(
                    &raw.nodes
                        .iter()
                        .find(|n| n.address == address)
                        .unwrap()
                        .resource_type,
                    path,
                )
                && r.attribute == path
                && r.complete
                && !r.dependency
        })
}
fn verify_fixture(input: &str, minimum: usize) {
    let base = fixture(input);
    let raw = crate::plan::parse(input).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert_eq!(graph.coverage.selected.resources, raw.nodes.len());
    let mut checked = 0;
    for r in &raw.attributes {
        let node = &raw.nodes[r.target];
        if super::label(&node.resource_type, &r.attribute).is_none()
            || node.resource_type == super::PARTNER
        {
            continue;
        }
        let addr = &node.address;
        assert!(
            proven(&base, addr, &r.attribute),
            "{addr} {} {r:?}",
            r.attribute
        );
        let endpoint = &raw.nodes[r.sources[0]];
        let source = index(&base, &endpoint.address);
        let output = r.source_attributes_by_instance[&r.sources[0]]
            .first()
            .unwrap();
        let target = index(&base, addr);
        for case in 0..8 {
            let mut bad = base.clone();
            match case {
                0 => {
                    bad["resource_changes"][source]["change"]["after"][output] = json!("different")
                }
                1 => {
                    bad["resource_changes"][source]["change"]["after_sensitive"] =
                        json!({output:true})
                }
                2 => bad["resource_changes"][source]["deposed"] = json!("previous"),
                3 => bad["resource_changes"][source]["mode"] = json!("data"),
                4 => bad["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                5 => {
                    bad["configuration"]["root_module"]["resources"][target]["expressions"] =
                        json!({})
                }
                6 => bad["resource_changes"][target]["change"]["after_unknown"] = json!(true),
                _ => {
                    bad["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"eu-west-1"}}});
                    bad["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("aws.other");
                }
            }
            assert!(
                !proven(&bad, addr, &r.attribute),
                "{addr} {} case {case}",
                r.attribute
            );
        }
        checked += 1;
    }
    assert!(checked >= minimum, "{checked}");
    for node in &graph.nodes {
        if let Some((path, _, _)) = super::owner(&node.resource_type) {
            assert_eq!(
                node.member_owner.is_some(),
                proven(&base, &node.address, path),
                "{}",
                node.address
            );
        }
    }
}

#[test]
fn workload_helpers_require_live_explicit_endpoints() {
    verify_fixture(HELPERS, 20)
}
#[test]
fn modes_qualify_each_workload_reference_independently() {
    for (address, path, field, value) in [
        (
            "aws_redshift_event_subscription.cluster",
            "source_ids",
            "source_type",
            json!("cluster-snapshot"),
        ),
        (
            "aws_redshift_event_subscription.cluster",
            "source_ids",
            "source_ids",
            json!(["warehouse"]),
        ),
        (
            "aws_redshift_logging.audit",
            "bucket_name",
            "log_destination_type",
            json!("cloudwatch"),
        ),
        (
            "aws_redshiftdata_statement.cluster",
            "cluster_identifier",
            "workgroup_name",
            json!("analytics"),
        ),
        (
            "aws_redshiftdata_statement.serverless",
            "workgroup_name",
            "cluster_identifier",
            json!("warehouse"),
        ),
    ] {
        let mut v = fixture(HELPERS);
        let i = index(&v, address);
        v["resource_changes"][i]["change"]["after"][field] = value;
        assert!(!proven(&v, address, path));
    }
    let mut v = fixture(HELPERS);
    let a = "aws_redshift_scheduled_action.resize_cluster";
    let i = index(&v, a);
    v["resource_changes"][i]["change"]["after"]["target_action"][0]["pause_cluster"] =
        json!([{"cluster_identifier":"warehouse"}]);
    assert!(!proven(
        &v,
        a,
        "target_action.resize_cluster.cluster_identifier"
    ));
    assert!(proven(&v, a, "iam_role"));
}

#[test]
fn partner_requires_independently_proven_cluster_account() {
    for account in [None, Some("111111111111"), Some("222222222222")] {
        let mut raw = crate::plan::parse(HELPERS).unwrap();
        for n in &mut raw.nodes {
            n.provider_configuration.as_mut().unwrap().account =
                account.and_then(crate::model::AwsAccountScope::new)
        }
        let g = crate::semantic::transform(&raw);
        let n = g
            .nodes
            .iter()
            .find(|n| n.resource_type == super::PARTNER)
            .unwrap();
        assert_eq!(
            n.references
                .iter()
                .any(|r| r.label == "Partner cluster" && r.complete),
            account == Some("111111111111")
        );
    }
}

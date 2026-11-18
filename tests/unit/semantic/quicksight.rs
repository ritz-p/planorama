use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/quicksight-connection-plan.json");

fn verified(plan: &Value, address: &str, path: &str) -> bool {
    let graph = semantic::transform(&plan::parse(&plan.to_string()).unwrap());
    graph
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == path
                && r.complete
                && !r.dependency
                && !r.targets.is_empty()
                && super::label(address.split('.').next().unwrap(), path) == Some(r.label)
        })
}

#[test]
fn connection_requires_common_vpc_and_data_source_remains_independent() {
    let value: Value = serde_json::from_str(FIXTURE).unwrap();
    for path in ["role_arn", "subnet_ids", "security_group_ids"] {
        assert!(
            verified(&value, "aws_quicksight_vpc_connection.one", path),
            "{path}"
        );
    }
    for path in [
        "parameters.athena.work_group",
        "parameters.athena.role_arn",
        "vpc_connection_properties.vpc_connection_arn",
    ] {
        assert!(
            verified(&value, "aws_quicksight_data_source.analytics", path),
            "{path}"
        );
    }
    let contained = |v: &Value| {
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.coverage.selected.resources, 7);
        g.edges
            .iter()
            .filter(|e| {
                e.kind == EdgeKind::Containment
                    && g.nodes[e.to].resource_type.starts_with("aws_quicksight_")
            })
            .map(|e| {
                (
                    g.nodes[e.from].address.clone(),
                    g.nodes[e.to].address.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        contained(&value),
        [(
            "aws_vpc.one".into(),
            "aws_quicksight_vpc_connection.one".into()
        )]
    );
    let mut partial = value;
    partial["resource_changes"][4]["change"]["after"]["subnet_ids"] =
        json!(["subnet-one", "unresolved"]);
    assert!(contained(&partial).is_empty());
}

#[test]
fn data_source_requires_current_typed_unmasked_evidence() {
    for case in 0..9 {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            0 => value["resource_changes"][6]["change"]["after"]["type"] = json!("S3"),
            1 => {
                value["resource_changes"][6]["change"]["after_unknown"] = json!({"parameters":true})
            }
            2 => value["resource_changes"][5]["change"]["after_sensitive"] = json!({"name":true}),
            3 => value["resource_changes"][5]["deposed"] = json!("old"),
            4 => value["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            5 => {
                value["resource_changes"][6]["change"]["after"]["aws_account_id"] =
                    json!("999999999999")
            }
            6 => value["resource_changes"][5]["mode"] = json!("data"),
            7 => {
                value["configuration"]["root_module"]["resources"][6]["expressions"]["parameters"]
                    [0]["athena"][0]["work_group"] = json!({"constant_value":"analytics"})
            }
            _ => value["resource_changes"][5]["change"]["after"]["name"] = json!("other"),
        }
        assert!(
            !verified(
                &value,
                "aws_quicksight_data_source.analytics",
                "parameters.athena.work_group"
            ),
            "{case}"
        );
    }
}

use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/routes-plan.json")).unwrap()
}

#[test]
fn supported_route_families_lower_and_keep_individual_change_metadata() {
    for (field, kind) in [
        ("gateway_id", "aws_internet_gateway"),
        ("nat_gateway_id", "aws_nat_gateway"),
        ("transit_gateway_id", "aws_ec2_transit_gateway"),
        ("vpc_peering_connection_id", "aws_vpc_peering_connection"),
    ] {
        let mut input = input();
        let address = format!("{kind}.main");
        input["resource_changes"][1]["address"] = address.clone().into();
        input["resource_changes"][1]["type"] = kind.into();
        for index in 0..2 {
            input["resource_changes"][index + 2]["change"]["after"] =
                json!({"route_table_id":"TOP_SECRET_TABLE",field:"TOP_SECRET_TARGET"});
            let expressions =
                input["configuration"]["root_module"]["resources"][index]["expressions"]
                    .as_object_mut()
                    .unwrap();
            expressions.remove("gateway_id");
            expressions.insert(
                field.into(),
                json!({"references":[format!("{address}.id")]}),
            );
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        let routes: Vec<_> = graph.edges.iter().filter(|e| e.change.is_some()).collect();
        assert_eq!(routes.len(), 2);
        for edge in &routes {
            assert_eq!(edge.kind, EdgeKind::Association);
            assert_eq!(graph.nodes[edge.from].address, "aws_route_table.public");
            assert_eq!(graph.nodes[edge.to].address, address);
            let change = edge.change.as_ref().unwrap();
            let original = raw
                .nodes
                .iter()
                .find(|n| n.address == change.address)
                .unwrap();
            assert_eq!(change.metadata, original.metadata);
            assert_eq!(change.action, original.action);
        }
        assert!(graph.nodes.iter().any(|n| n.address == "aws_route.literal"));
    }
}

#[test]
fn traversal_provenance_does_not_prove_endpoint_value_identity() {
    for case in 0..6 {
        let mut input = input();
        match case {
            0 => {
                input["resource_changes"][2]["change"]["after"]["gateway_id"] =
                    "TOP_SECRET_OTHER".into()
            }
            1 => {
                input["resource_changes"][2]["change"]["after"]["route_table_id"] =
                    "TOP_SECRET_OTHER".into()
            }
            2 => {
                input["resource_changes"][2]["change"]["after"] = Value::Null;
                input["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"gateway_id":true,"route_table_id":true});
            }
            3 => input["resource_changes"][1]["change"]["after_unknown"] = json!({"id":true}),
            4 => {
                input["resource_changes"][2]["change"]
                    .as_object_mut()
                    .unwrap()
                    .remove("after");
            }
            _ => input["resource_changes"][2]["change"]["after_unknown"] = json!(true),
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        assert!(
            graph.nodes.iter().any(|n| n.address == "aws_route.default"),
            "case {case}"
        );
        assert!(
            semantic::diagnostics::collect(&raw)
                .iter()
                .any(|d| d.address == "aws_route.default"
                    && d.reason == crate::model::DiagnosticReason::UnprovenEndpointValue)
        );
        assert!(!format!("{raw:?}").contains("TOP_SECRET"));
    }
}

#[test]
fn ambiguous_unsupported_and_consumed_routes_keep_cards() {
    for case in 0..8 {
        let mut input = input();
        let expressions = &mut input["configuration"]["root_module"]["resources"][0]["expressions"];
        match case {
            0 => expressions["nat_gateway_id"] = json!({"constant_value":"TOP_SECRET"}),
            1 => {
                expressions["gateway_id"] =
                    json!({"references":["aws_internet_gateway.missing.id"]})
            }
            2 => expressions["gateway_id"] = json!({"references":["aws_route_table.public.id"]}),
            3 => expressions["route_table_id"] = json!({"constant_value":"TOP_SECRET"}),
            4 => {
                expressions["gateway_id"] = json!({"references":["aws_internet_gateway.main.tags"]})
            }
            5 => input["resource_changes"][2]["provider_name"] = "example.com/custom/aws".into(),
            6 => input["resource_changes"][2]["deposed"] = "old".into(),
            _ => {
                input["configuration"]["root_module"]["resources"][1]["expressions"]["extra"] =
                    json!({"references":["aws_route.default.id"]})
            }
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        assert!(
            graph.nodes.iter().any(|n| n.address == "aws_route.default"),
            "case {case}"
        );
    }
}

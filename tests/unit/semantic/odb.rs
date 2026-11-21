use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/odb-plan.json")).unwrap()
}

fn verified(input: &Value, address: &str, attribute: &str, endpoint: &str) -> bool {
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    graph
        .nodes
        .iter()
        .find(|node| node.address == address)
        .unwrap()
        .references
        .iter()
        .any(|reference| {
            reference.attribute == attribute
                && reference.complete
                && !reference.dependency
                && super::label(address.split('.').next().unwrap(), attribute)
                    == Some(reference.label)
                && reference
                    .targets
                    .iter()
                    .any(|target| target.id.address == endpoint)
        })
}

#[test]
fn odb_verifies_scoped_infrastructure_and_service_links() {
    let input = fixture();
    for (address, attribute, endpoint) in [
        (
            "aws_odb_cloud_vm_cluster.cluster",
            "odb_network_id",
            "aws_odb_network.primary",
        ),
        (
            "aws_odb_cloud_vm_cluster.cluster",
            "cloud_exadata_infrastructure_id",
            "aws_odb_cloud_exadata_infrastructure.exadata",
        ),
        (
            "aws_odb_cloud_autonomous_vm_cluster.autonomous",
            "odb_network_arn",
            "aws_odb_network.primary",
        ),
        (
            "aws_odb_autonomous_database.database",
            "odb_network_id",
            "aws_odb_network.primary",
        ),
        (
            "aws_odb_network_peering_connection.application",
            "odb_network_arn",
            "aws_odb_network.primary",
        ),
        (
            "aws_odb_network_peering_connection.application",
            "peer_network_id",
            "aws_vpc.application",
        ),
        (
            "aws_odb_iam_role_association.encryption",
            "resource_arn",
            "aws_odb_cloud_vm_cluster.cluster",
        ),
        (
            "aws_odb_iam_role_association.encryption",
            "iam_role_arn",
            "aws_iam_role.encryption",
        ),
    ] {
        assert!(
            verified(&input, address, attribute, endpoint),
            "{address}.{attribute}"
        );
    }
}

#[test]
fn odb_rejects_ambiguous_selectors_and_unproven_endpoints() {
    let mut input = fixture();
    let address = "aws_odb_cloud_vm_cluster.cluster";
    let endpoint = "aws_odb_network.primary";
    input["configuration"]["root_module"]["resources"][2]["expressions"]["odb_network_arn"] =
        json!({"references":["aws_odb_network.primary.arn"]});
    assert!(!verified(&input, address, "odb_network_id", endpoint));

    let mut input = fixture();
    input["resource_changes"][1]["change"]["after_unknown"] = json!({"id":true});
    assert!(!verified(&input, address, "odb_network_id", endpoint));
    input["resource_changes"][1]["change"]["after_unknown"] = Value::Null;
    input["resource_changes"][1]["change"]["actions"] = json!(["delete"]);
    assert!(!verified(&input, address, "odb_network_id", endpoint));

    let mut input = fixture();
    input["resource_changes"][8]["change"]["after"]["aws_integration"] = json!("Unknown");
    assert!(!verified(
        &input,
        "aws_odb_iam_role_association.encryption",
        "resource_arn",
        address
    ));

    let mut input = fixture();
    input["resource_changes"][6]["change"]["after"]["peer_network_id"] =
        input["resource_changes"][1]["change"]["after"]["id"].clone();
    input["configuration"]["root_module"]["resources"][6]["expressions"]["peer_network_id"] =
        json!({"references":["aws_odb_network.primary.id"]});
    assert!(!verified(
        &input,
        "aws_odb_network_peering_connection.application",
        "peer_network_id",
        endpoint
    ));
}

#[test]
fn odb_peering_accepts_distinct_odb_network_ids_in_scope() {
    let mut input = fixture();
    let provider = "registry.terraform.io/hashicorp/aws";
    input["resource_changes"][5] = json!({
        "address":"aws_odb_network.peer","type":"aws_odb_network","name":"peer",
        "mode":"managed","provider_name":provider,
        "change":{"actions":["create"],"after":{"id":"odbnet-peer"}}
    });
    input["configuration"]["root_module"]["resources"][5] =
        json!({"address":"aws_odb_network.peer","provider_config_key":"aws","expressions":{}});
    input["resource_changes"][6]["change"]["after"]["peer_network_id"] = json!("odbnet-peer");
    input["configuration"]["root_module"]["resources"][6]["expressions"]["peer_network_id"] =
        json!({"references":["aws_odb_network.peer.id"]});

    let address = "aws_odb_network_peering_connection.application";
    let endpoint = "aws_odb_network.peer";
    assert!(verified(&input, address, "peer_network_id", endpoint));

    input["configuration"]["provider_config"]["aws.other"] = json!({
        "name":"aws","full_name":provider,"alias":"other",
        "expressions":{"region":{"constant_value":"us-west-2"}}
    });
    input["configuration"]["root_module"]["resources"][5]["provider_config_key"] =
        json!("aws.other");
    assert!(!verified(&input, address, "peer_network_id", endpoint));
}

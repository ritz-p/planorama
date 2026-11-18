use crate::{
    model::{EdgeKind, Graph},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/emr-cluster-plan.json");
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
fn parent(g: &Graph, address: &str) -> Option<String> {
    let t = g.nodes.iter().position(|n| n.address == address).unwrap();
    g.edges
        .iter()
        .find(|e| e.to == t && e.kind == EdgeKind::Containment)
        .map(|e| g.nodes[e.from].address.clone())
}
#[test]
fn cluster_placement_uses_only_configured_ec2_network_fields() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    let t = 11;
    v["resource_changes"][t]["change"]["after_unknown"] =
        json!({"ec2_attributes":[{"subnet_id":true}]});
    v["resource_changes"][t]["change"]["after"]["ec2_attributes"][0]["service_access_security_group"] =
        json!("sg-computed");
    assert_eq!(
        parent(&graph(&v), "aws_emr_cluster.main").as_deref(),
        Some("aws_vpc.main")
    );
    v["configuration"]["root_module"]["resources"][t]["expressions"]["ec2_attributes"][0]["service_access_security_group"] =
        json!({"constant_value":"sg-computed"});
    assert_eq!(parent(&graph(&v), "aws_emr_cluster.main"), None);
}

#[test]
fn cluster_configuration_and_owned_helpers_require_complete_inputs() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    for p in [
        "ec2_attributes.subnet_ids",
        "ec2_attributes.additional_slave_security_groups",
        "ec2_attributes.instance_profile",
        "ec2_attributes.emr_managed_master_security_group",
        "log_uri",
        "log_encryption_kms_key_id",
        "custom_ami_id",
        "service_role",
        "security_configuration",
        "autoscaling_role",
    ] {
        assert!(verified(&v, 11, p), "{p}");
    }
    let g = graph(&v);
    assert_eq!(
        parent(&g, "aws_emr_cluster.main").as_deref(),
        Some("aws_vpc.main")
    );
    for n in g.nodes.iter().filter(|n| {
        matches!(
            n.resource_type.as_str(),
            "aws_emr_instance_fleet" | "aws_emr_instance_group" | "aws_emr_managed_scaling_policy"
        )
    }) {
        assert!(n.member_owner.is_some(), "{}", n.address);
        assert!(crate::view::presentation::owner(&g, n).is_some());
    }
    v["resource_changes"][11]["change"]["after"]["ec2_attributes"][0]["additional_slave_security_groups"] =
        json!("sg-example,sg-unconfigured");
    assert!(!verified(
        &v,
        11,
        "ec2_attributes.additional_slave_security_groups"
    ));
    assert_eq!(parent(&graph(&v), "aws_emr_cluster.main"), None);
    v["resource_changes"][11]["change"]["after"]["log_uri"] =
        json!("s3://example-emr-logs-other/logs");
    assert!(!verified(&v, 11, "log_uri"));
}

#[test]
fn emr_studio_requires_verified_configuration() {
    let mut v: Value =
        serde_json::from_str(include_str!("../../fixtures/emr-studio-plan.json")).unwrap();
    for p in [
        "vpc_id",
        "subnet_ids",
        "engine_security_group_id",
        "workspace_security_group_id",
        "service_role",
        "user_role",
        "encryption_key_arn",
        "default_s3_location",
    ] {
        assert!(verified(&v, 8, p), "{p}");
    }
    for p in ["studio_id", "session_policy_arn"] {
        assert!(verified(&v, 9, p), "{p}");
        let g = graph(&v);
        let mapping = g
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_emr_studio_session_mapping")
            .unwrap();
        assert!(crate::view::presentation::owner(&g, mapping).is_some());
    }
    assert_eq!(
        parent(&graph(&v), "aws_emr_studio.main").as_deref(),
        Some("aws_vpc.main")
    );
    v["resource_changes"][8]["change"]["after"]["auth_mode"] = json!("IAM");
    assert!(!verified(&v, 8, "user_role"));
    assert!(verified(&v, 8, "service_role"));
    v["resource_changes"][8]["change"]["after_unknown"] =
        json!({"workspace_security_group_id":true});
    assert_eq!(parent(&graph(&v), "aws_emr_studio.main"), None);
}

#[test]
fn emr_containers_requires_verified_configuration() {
    let mut v: Value =
        serde_json::from_str(include_str!("../../fixtures/emr-containers-plan.json")).unwrap();
    assert!(verified(&v, 5, "container_provider.id"));
    for p in [
        "kms_key_arn",
        "job_template_data.execution_role_arn",
        super::TEMPLATE_LOG,
        super::TEMPLATE_BUCKET,
    ] {
        assert!(verified(&v, 6, p), "{p}");
    }
    v["resource_changes"][5]["change"]["after"]["container_provider"][0]["type"] = json!("OTHER");
    assert!(!verified(&v, 5, "container_provider.id"));
    v["resource_changes"][6]["change"]["after"]["job_template_data"][0]["execution_role_arn"] =
        json!("${template.role}");
    assert!(!verified(&v, 6, "job_template_data.execution_role_arn"));
    assert_eq!(
        parent(&graph(&v), "aws_emrcontainers_virtual_cluster.main"),
        None
    );
}

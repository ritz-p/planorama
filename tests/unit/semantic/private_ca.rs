use crate::{
    model::{Placement, ResourceRole},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/private-ca-plan.json");

#[test]
fn private_ca_preserves_owner_accounts_and_keeps_shared_ca_accounts_unknown() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut data = value["resource_changes"][0].clone();
    data["address"] = json!("data.aws_acmpca_certificate_authority.lookup");
    data["mode"] = json!("data");
    data["change"]["actions"] = json!(["read"]);
    value["resource_changes"].as_array_mut().unwrap().push(data);
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "address":"data.aws_acmpca_certificate_authority.lookup",
            "mode":"data",
            "provider_config_key":"aws"
        }));
    let mut parsed = plan::parse(&value.to_string()).unwrap();
    for account in [None, crate::model::AwsAccountScope::new("123456789012")] {
        for node in &mut parsed.nodes {
            node.provider_configuration.as_mut().unwrap().account = account.clone();
        }
        let graph = semantic::transform(&parsed);
        for node in &graph.nodes {
            let expected = match (node.resource_type.as_str(), node.mode) {
                ("aws_acmpca_certificate", _)
                | ("aws_acmpca_certificate_authority", crate::model::EntityMode::Data) => None,
                _ => account.as_ref(),
            };
            assert_eq!(
                node.entity.scope.as_ref().unwrap().account.as_ref(),
                expected,
                "{}",
                node.address
            );
        }
    }
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 7);
    let data = graph
        .nodes
        .iter()
        .find(|n| n.mode == crate::model::EntityMode::Data)
        .unwrap();
    assert_eq!(data.placement, Placement::Logical);
    assert_eq!(data.role, ResourceRole::Node);
    for resource in [
        "aws_acmpca_certificate_authority",
        "aws_acmpca_certificate",
        "aws_acmpca_certificate_authority_certificate",
    ] {
        assert_eq!(
            crate::icons::icon_for(crate::icons::Provider::Aws, resource),
            Some(crate::icons::ResourceIcon::PrivateCa)
        );
        assert_eq!(
            crate::icons::icon_for(crate::icons::Provider::Other, resource),
            None
        );
    }
    assert_eq!(super::classify("aws_acmpca_policy"), ResourceRole::Unknown);
    assert_eq!(super::icon_for("aws_acmpca_certificate_fake"), None);
}

#[test]
fn private_ca_verifies_signing_installation_and_acm_references_without_inventing_hierarchy() {
    for (source, consumer, label, attribute) in [
        (0, 1, "Signing CA reference", "certificate_authority_arn"),
        (2, 3, "Target CA reference", "certificate_authority_arn"),
        (1, 3, "Installed certificate reference", "certificate"),
        (2, 4, "Signing CA reference", "certificate_authority_arn"),
    ] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        for path in ["resource_changes", "configuration"] {
            let resources = if path == "configuration" {
                &mut value[path]["root_module"]["resources"]
            } else {
                &mut value[path]
            };
            let original = resources.as_array().unwrap().clone();
            *resources = json!([original[source], original[consumer]]);
        }
        super::service_references::tests::check_service_fixture(
            &value.to_string(),
            label,
            attribute,
        );
    }
    let parsed = plan::parse(FIXTURE).unwrap();
    assert!(!format!("{parsed:?}").contains("PRIVATE_"));
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 6);
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.kind == crate::model::EdgeKind::Dependency)
    );
    for node in graph
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_acmpca_"))
    {
        assert_eq!(node.placement, Placement::Logical);
        assert_eq!(
            node.role,
            if node.address.ends_with(".install") {
                ResourceRole::Controller
            } else {
                ResourceRole::Node
            }
        );
        assert!(node.entity.scope.as_ref().is_some_and(|s| !s.global && s.region.as_ref().is_some_and(|r| r.0 == "us-east-1")));
    }
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.address.ends_with(".external"))
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Signing CA reference" && !r.complete && r.targets.is_empty())
    );
    assert!(
        semantic::diagnostics::collect(&parsed)
            .iter()
            .any(|d| d.address.ends_with(".external"))
    );
    assert_eq!(
        parsed
            .attributes
            .iter()
            .filter(|r| r.service_reference_verified)
            .count(),
        4
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(output.contains("planorama-icon-aws-private-ca"));
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("arn:aws:"));
    }
}

#[test]
fn private_ca_retains_lifecycle_and_rejects_unproven_consumers_and_scopes() {
    for case in 0..8 {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            0 => value["resource_changes"][1]["provider_name"] = json!("example.com/foreign/aws"),
            1 => value["resource_changes"][1]["change"]["after_unknown"] = json!(true),
            2 => value["resource_changes"][1]["deposed"] = json!("old"),
            3 => value["resource_changes"][1]["mode"] = json!("data"),
            4 => value["resource_changes"][1]["type"] = json!("aws_acmpca_permission"),
            5 => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            6 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["certificate_authority_arn"] =
                    json!({"references":["aws_acmpca_certificate_authority.missing.arn"]})
            }
            _ => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["certificate_authority_arn"] =
                    json!({"references":["aws_acmpca_certificate_authority.root.id"]})
            }
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        assert!(
            !parsed
                .attributes
                .iter()
                .any(
                    |r| parsed.nodes[r.target].address == "aws_acmpca_certificate.subordinate"
                        && r.service_reference_verified
                ),
            "case {case}"
        );
        assert_eq!(semantic::transform(&parsed).coverage.selected.resources, 6);
    }
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["resource_changes"][1]["previous_address"] = json!("aws_acmpca_certificate.old");
    value["resource_changes"][1]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][1].clone();
    old["deposed"] = json!("old");
    old["change"]["before"] = old["change"]["after"].take();
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 7);
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.references.is_empty())
    );
    let current = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_acmpca_certificate.subordinate" && n.deposed_key.is_none())
        .unwrap();
    assert_eq!(
        current.previous_address.as_deref(),
        Some("aws_acmpca_certificate.old")
    );
    assert!(current.metadata.import.is_some());
    value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
        json!({"references":["var.region"]});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .all(|n| n.entity.scope.as_ref().is_none_or(|s| s.region.is_none()))
    );
}

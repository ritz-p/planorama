use crate::{
    model::{Placement, ResourceRole},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/private-ca-plan.json");

#[test]
fn certificate_ownership_requires_signing_proof_scope_and_no_consumers() {
    for case in [
        "valid",
        "consumer",
        "unknown",
        "ambiguous",
        "data",
        "deposed",
        "scope",
        "account",
    ] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "consumer" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["certificate"] =
                    json!({"references":["aws_acm_certificate.application.arn"]});
            }
            "unknown" => {
                value["resource_changes"][4]["change"]["after_unknown"]["certificate_authority_arn"] =
                    json!(true)
            }
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][4]["expressions"]["certificate_authority_arn"] = json!({"references":["aws_acmpca_certificate_authority.root.arn", "aws_acmpca_certificate_authority.subordinate.arn"]})
            }
            "data" => value["resource_changes"][4]["mode"] = json!("data"),
            "deposed" => value["resource_changes"][4]["deposed"] = json!("old"),
            _ => {}
        }
        let mut raw = semantic::Input::new(&plan::parse(&value.to_string()).unwrap());
        let index = raw
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_acm_certificate")
            .unwrap();
        match case {
            "scope" => raw.graph.nodes[index].entity.scope = None,
            "account" => {
                raw.graph.nodes[index]
                    .entity
                    .scope
                    .as_mut()
                    .unwrap()
                    .account = crate::model::AwsAccountScope::new("999999999999")
            }
            _ => {}
        }
        let graph = crate::provider::transform(raw);
        let child = &graph.nodes[index];
        assert_eq!(child.member_owner.is_some(), case == "valid", "{case}");
        assert_eq!(child.placement, Placement::Spatial);
        assert!(
            graph
                .nodes
                .iter()
                .filter(|n| n.resource_type == "aws_acmpca_certificate")
                .all(|n| n.member_owner.is_none())
        );
        assert_eq!(graph.nodes.len(), 6);
        assert!(
            graph
                .edges
                .iter()
                .all(|e| e.kind != crate::model::EdgeKind::Containment)
        );
        let layout = crate::layout::Layout::new(&graph);
        assert!(layout.bounds[index].width > 0);
        if case == "valid" {
            let owner = crate::view::routes::owner(&graph, child).unwrap();
            let bounds = layout.bounds[index];
            let frame = layout.bounds[owner];
            assert!(bounds.origin.x > frame.origin.x && bounds.right() < frame.right());
            assert!(
                bounds.origin.y > frame.origin.y
                    && bounds.origin.y + bounds.height < frame.origin.y + frame.height
            );
        }
    }
}

#[test]
fn certificate_selection_releases_filtered_issuers_and_keeps_shared_cards() {
    let shared = semantic::transform(
        &plan::parse(include_str!("../../fixtures/certificate-cards-plan.json")).unwrap(),
    );
    assert_eq!(shared.coverage.selected.resources, 5);
    for node in &shared.nodes {
        assert_eq!(
            node.member_owner.is_some(),
            node.address == "aws_acm_certificate.unused"
        );
    }
    assert_eq!(
        shared
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .filter(|r| r.complete && r.label == "Certificate reference")
            .count(),
        2
    );
    let input = FIXTURE.replace(
        "aws_acm_certificate.application",
        "module.application.aws_acm_certificate.application",
    );
    let graph = semantic::transform(&plan::parse(&input).unwrap());
    for options in [
        crate::view::Options {
            focus: Some("module.application.aws_acm_certificate.application".into()),
            depth: Some(0),
            ..Default::default()
        },
        crate::view::Options {
            modules: vec!["module.application".into()],
            ..Default::default()
        },
    ] {
        let selected = crate::view::single(&graph, &options).unwrap();
        assert_eq!(selected.coverage.selected.resources, 1);
        assert_eq!(selected.nodes.len(), 1);
        assert!(crate::view::routes::owner(&selected, &selected.nodes[0]).is_none());
        let output = crate::svg::render(&selected, &crate::layout::Layout::new(&selected));
        assert!(output.contains("Unavailable"));
    }
    let graph =
        semantic::transform(&plan::parse(include_str!("../../fixtures/acm-plan.json")).unwrap());
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.resource_type == "aws_acm_certificate")
            .count(),
        1
    );
    assert!(graph.nodes.iter().all(|n| n.member_owner.is_none()));
    assert_eq!(graph.coverage.selected.resources, 4);
}

#[test]
fn embedded_certificate_preserves_long_identity_lifecycle_and_named_state_exports() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    let address = format!(
        "aws_acm_certificate.application[\"{}<&\"]",
        "long".repeat(70)
    );
    value["resource_changes"][4]["address"] = json!(address);
    value["resource_changes"][4]["previous_address"] = json!("aws_acm_certificate.old");
    value["resource_changes"][4]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][4].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 7);
    let child = graph
        .nodes
        .iter()
        .find(|n| n.address == address && n.deposed_key.is_none())
        .unwrap();
    assert!(child.member_owner.is_some());
    assert_eq!(
        child.previous_address.as_deref(),
        Some("aws_acm_certificate.old")
    );
    assert!(child.metadata.import.is_some());
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.member_owner.is_none())
    );
    let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    let doc = roxmltree::Document::parse(&svg).unwrap();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-terraform-address") == Some(address.as_str()))
            .count(),
        2
    );
    let architecture = crate::model::architecture::Architecture {
        states: [
            (crate::model::StateId::new("one").unwrap(), graph.clone()),
            (crate::model::StateId::new("two").unwrap(), graph),
        ]
        .into(),
        relationships: Vec::new(),
    };
    let selected = crate::view::apply(&architecture, &Default::default()).unwrap();
    let html = crate::html::render_states(&selected, crate::svg::Options::default());
    assert!(!html.contains("PRIVATE_"));
    let text = &html[html.find("id=\"viewer-data\"").unwrap()..];
    let resources: Vec<Value> =
        serde_json::from_str(&text[text.find('>').unwrap() + 1..text.find("</script>").unwrap()])
            .unwrap();
    assert_eq!(resources.len(), 14);
    for state in ["one", "two"] {
        let child = resources
            .iter()
            .find(|r| {
                r["state"] == state
                    && r["address"] == address
                    && r["deposed"].as_str().is_none_or(str::is_empty)
            })
            .unwrap();
        let parent = resources
            .iter()
            .find(|r| {
                r["state"] == state
                    && r["address"] == "aws_acmpca_certificate_authority.subordinate"
            })
            .unwrap();
        assert_eq!(child["context"], json!([parent["id"]]));
        assert_ne!(child["id"], parent["id"]);
    }
}

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
    assert_eq!(data.placement, Placement::Spatial);
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
    assert_eq!(super::classify("aws_acmpca_policy"), ResourceRole::Policy);
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
        assert_eq!(
            node.placement,
            if node.address.ends_with(".install") {
                Placement::Logical
            } else {
                Placement::Spatial
            }
        );
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

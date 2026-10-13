use super::*;

#[test]
fn spatial_account_evidence_does_not_infer_region_or_global_classification() {
    for resource_type in [
        "aws_vpc",
        "aws_s3_bucket",
        "aws_vpc_peering_connection",
        "aws_unclassified_example",
        "aws_cloudfront_distribution",
    ] {
        let address = format!("{resource_type}.example");
        let input = serde_json::json!({
            "format_version": "1.2",
            "resource_changes": [{"address": address, "type": resource_type, "change": {"actions": ["create"]}}],
            "configuration": {
                "provider_config": {"aws": {"full_name": "hashicorp/aws", "expressions": {
                    "assume_role": [{"role_arn": {"constant_value": "arn:aws:iam::123456789012:role/deploy"}}]
                }}},
                "root_module": {"resources": [{"address": address, "provider_config_key": "aws"}]}
            }
        });
        let graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap());
        let output = render(&graph, &Layout::new(&graph));
        let document = roxmltree::Document::parse(&output).unwrap();
        let panel = document
            .descendants()
            .find(|n| n.attribute("data-account") == Some("123456789012"))
            .unwrap();
        assert!(
            !document
                .descendants()
                .any(|n| n.attribute("data-region").is_some())
        );
        assert_eq!(
            document
                .descendants()
                .any(|n| n.attribute("data-global") == Some("true")),
            resource_type == "aws_cloudfront_distribution"
        );
        assert!(panel.children().any(|n| n.has_tag_name("text")
            && n.text().is_some_and(|text| text.contains("123456789012"))));
    }
}

#[test]
fn spatial_scopes_expose_distinct_account_identity() {
    let mut input = serde_json::json!({
        "format_version": "1.2",
        "resource_changes": [],
        "configuration": {"provider_config": {}, "root_module": {"resources": []}}
    });
    for (alias, account) in [
        ("first", Some("123456789012")),
        ("second", Some("210987654321")),
        ("unknown", None),
    ] {
        let provider = format!("aws.{alias}");
        let mut expressions = serde_json::json!({"region": {"constant_value": "us-east-1"}});
        if let Some(account) = account {
            expressions["assume_role"] = serde_json::json!([{"role_arn": {"constant_value": format!("arn:aws:iam::{account}:role/deploy")}}]);
        }
        input["configuration"]["provider_config"][&provider] = serde_json::json!({"full_name": "hashicorp/aws", "alias": alias, "expressions": expressions});
        for resource_type in ["aws_s3_bucket", "aws_cloudfront_distribution"] {
            let address = format!("{resource_type}.{alias}");
            input["resource_changes"].as_array_mut().unwrap().push(serde_json::json!({"address": address, "type": resource_type, "change": {"actions": ["create"]}}));
            input["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({"address": address, "provider_config_key": provider}));
        }
    }
    let graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap());
    let output = render(&graph, &Layout::new(&graph));
    let document = roxmltree::Document::parse(&output).unwrap();
    for account in [Some("123456789012"), Some("210987654321"), None] {
        for (attribute, value) in [("data-region", "us-east-1"), ("data-global", "true")] {
            let panel = document
                .descendants()
                .find(|n| {
                    n.attribute(attribute) == Some(value) && n.attribute("data-account") == account
                })
                .unwrap();
            if let Some(account) = account {
                assert!(
                    document
                        .descendants()
                        .any(|n| n.attribute("data-account") == Some(account)
                            && n.children().any(|c| c.has_tag_name("text")
                                && c.text().is_some_and(|label| label.contains(account))))
                );
            } else {
                assert!(
                    !panel
                        .children()
                        .filter_map(|n| n.text())
                        .any(|text| text.contains("Account:"))
                );
            }
        }
    }
}

#[test]
fn policy_and_controller_fallbacks_keep_resources_actions_and_dependencies() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[
        {"address":"aws_security_group.app","type":"aws_security_group","change":{"actions":["update"]}},
        {"address":"aws_ecs_service.app","type":"aws_ecs_service","change":{"actions":["create"]}}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_security_group.app"},
        {"address":"aws_ecs_service.app","expressions":{"security_groups":{"references":["aws_security_group.app.id"]}}}
    ]}}}"#).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert_eq!(graph.0, crate::semantic::base_graph(&raw));
    let output = render(&graph, &Layout::new(&graph));
    for value in [
        "data-role=\"policy\"",
        "data-role=\"controller\"",
        "aws_security_group.app",
        "aws_ecs_service.app",
        "update (1)",
        "create (1)",
    ] {
        assert!(output.contains(value), "missing {value}");
    }
    assert_eq!(
        output
            .lines()
            .filter(|line| line.contains("data-edge-kind=") && line.contains("marker-end="))
            .count(),
        1
    );
    assert_eq!(output, render(&graph, &Layout::new(&graph)));
}

#[test]
fn mixed_relationships_identify_each_kind_without_dependency_only_captions() {
    let raw = crate::plan::parse(include_str!("../fixtures/association-plan.json")).unwrap();
    let mut graph = crate::semantic::transform(&raw);
    graph.0.edges.push(crate::model::Edge::from((0, 1)));
    let output = render(&graph, &Layout::new(&graph));
    assert!(output.contains(" (dependency)</title>"));
    assert!(output.contains(" (association; create:"));
    assert!(output.contains("3 resources (2 cards), 2 relationships"));
    assert!(!output.contains("reference edges"));
    assert!(!output.contains("Arrows point from dependencies"));
}

#[test]
fn escapes_markup_and_filters_invalid_control_characters() {
    assert_eq!(
        escape("<script>&\"'\u{0}\u{1}\n\r\t"),
        "&lt;script&gt;&amp;&quot;&apos;\n\r\t"
    );
}

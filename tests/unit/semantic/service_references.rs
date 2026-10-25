use crate::{layout::Layout, plan, semantic, svg};
use serde_json::{Value, json};

#[test]
fn ecr_validates_configuration_without_inventing_image_pull_relationships() {
    let fixture = include_str!("../../fixtures/ecr-plan.json");
    check_service_fixture(fixture, "Configured repository", "repository");
    let mut value: Value = serde_json::from_str(fixture).unwrap();
    value["resource_changes"].as_array_mut().unwrap().swap(1, 2);
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .swap(1, 2);
    check_service_fixture(&value.to_string(), "Configured repository", "repository");
    for case in ["unknown_region", "other_region", "foreign_consumer"] {
        let mut changed: Value = serde_json::from_str(fixture).unwrap();
        match case {
            "unknown_region" => {
                changed["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]})
            }
            "other_region" => {
                changed["configuration"]["root_module"]["resources"][0]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"})
            }
            _ => changed["resource_changes"][1]["provider_name"] = json!("example.com/foreign/aws"),
        }
        let graph = semantic::transform(&plan::parse(&changed.to_string()).unwrap());
        assert!(
            !graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_ecr_repository_policy.app")
                .unwrap()
                .references
                .iter()
                .any(|r| r.label == "Configured repository" && r.complete),
            "{case}"
        );
    }
    let graph = semantic::transform(&plan::parse(fixture).unwrap());
    assert_eq!(graph.coverage.selected.resources, 4);
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.resource_type.starts_with("aws_ecr_"))
            .all(|n| n
                .entity
                .scope
                .as_ref()
                .is_some_and(|s| !s.global && s.region.is_some()))
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            fixture,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(output.contains("Configured repository"));
        assert!(!output.contains("planorama-icon-aws-ecr"));
    }
    value["configuration"]["root_module"]["resources"][3]["expressions"] = json!({});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_ecs_task_definition")
            .unwrap()
            .references
            .is_empty()
    );
}

#[test]
fn efs_preserves_regional_storage_and_validates_mount_target_relationships() {
    let fixture = include_str!("../../fixtures/efs-plan.json");
    check_service_fixture(fixture, "File system", "file_system_id");
    let mut access: Value = serde_json::from_str(fixture).unwrap();
    access["resource_changes"]
        .as_array_mut()
        .unwrap()
        .swap(1, 2);
    access["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .swap(1, 2);
    check_service_fixture(&access.to_string(), "File system", "file_system_id");
    let graph = semantic::transform(&plan::parse(fixture).unwrap());
    let index = |address: &str| {
        graph
            .nodes
            .iter()
            .position(|n| n.address == address)
            .unwrap()
    };
    let fs = index("aws_efs_file_system.shared");
    let mount = index("aws_efs_mount_target.app");
    let subnet = index("aws_subnet.app");
    let sg = index("aws_security_group.mount");
    assert!(graph.edges.iter().any(|e| e.from == subnet
        && e.to == mount
        && e.kind == crate::model::EdgeKind::Containment));
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.from == sg && e.to == mount && e.kind == crate::model::EdgeKind::Connection)
    );
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.to == fs && e.kind == crate::model::EdgeKind::Containment)
    );
    assert!(
        graph.nodes[fs]
            .entity
            .scope
            .as_ref()
            .is_some_and(|s| !s.global && s.region.is_some())
    );
    for case in 0..5 {
        let mut value: Value = serde_json::from_str(fixture).unwrap();
        match case {
            0 => value["resource_changes"][3]["provider_name"] = json!("example.com/foreign/aws"),
            1 => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            2 => value["resource_changes"][1]["deposed"] = json!("old"),
            3 => value["resource_changes"][3]["change"]["after_unknown"] = json!(true),
            _ => value["resource_changes"][3]["type"] = json!("aws_instance"),
        }
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| graph.nodes[e.to].address == "aws_efs_mount_target.app"
                    && e.kind == crate::model::EdgeKind::Containment)
        );
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            fixture,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_PATH"));
        assert!(!output.contains("12345"));
        assert!(output.contains("File system"));
    }
    for case in 0..6 {
        let mut value: Value = serde_json::from_str(fixture).unwrap();
        match case {
            0 => value["resource_changes"][5]["provider_name"] = json!("example.com/foreign/aws"),
            1 => value["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            2 => value["resource_changes"][5]["mode"] = json!("data"),
            3 => {
                value["resource_changes"][1]["change"]["after"]["security_groups"] =
                    json!(["sg-synthetic", "literal"])
            }
            4 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["security_groups"] =
                    json!({"references":["aws_security_group.mount", "var.selector"]})
            }
            _ => value["resource_changes"][5]["type"] = json!("aws_instance"),
        }
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| graph.nodes[e.to].address == "aws_efs_mount_target.app"
                    && e.kind == crate::model::EdgeKind::Connection)
        );
        assert_eq!(graph.coverage.selected.resources, 6);
    }
    let mut value: Value = serde_json::from_str(fixture).unwrap();
    value["resource_changes"][1]["previous_address"] = json!("aws_efs_mount_target.old");
    value["resource_changes"][1]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][1].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 7);
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.address == "aws_efs_mount_target.app")
            .count(),
        2
    );
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| graph.nodes[e.to].deposed_key.is_some()
                && matches!(
                    e.kind,
                    crate::model::EdgeKind::Containment | crate::model::EdgeKind::Connection
                ))
    );
}

#[test]
fn ssm_preserves_parameter_identity_without_values_or_path_inference() {
    let fixture = include_str!("../../fixtures/ssm-plan.json");
    for parameter_type in ["String", "StringList", "SecureString", "PRIVATE_TYPE"] {
        let mut value: Value = serde_json::from_str(fixture).unwrap();
        for key in ["value", "insecure_value", "description", "name"] {
            value["resource_changes"][0]["change"]["after"][key] =
                json!("PRIVATE_/environment/<script>&payload");
        }
        value["resource_changes"][0]["change"]["after"]["type"] = json!(parameter_type);
        let parsed = plan::parse(&value.to_string()).unwrap();
        assert!(!format!("{parsed:?}").contains("PRIVATE_"));
        let graph = semantic::transform(&parsed);
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.coverage.selected.resources, 3);
        assert!(
            graph
                .nodes
                .iter()
                .filter(|n| n.resource_type == "aws_ssm_parameter")
                .all(|n| n.placement == crate::model::Placement::Logical
                    && n.entity
                        .scope
                        .as_ref()
                        .is_some_and(|s| !s.global && s.region.is_some()))
        );
        assert!(
            parsed
                .attributes
                .iter()
                .all(|r| !r.service_reference_verified)
        );
        for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
            let output = crate::generate(
                &value.to_string(),
                &crate::Options {
                    format,
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(!output.content.contains("PRIVATE_"));
            assert!(
                !output
                    .content
                    .contains("SYNTHETIC_PARAMETER_NOT_FOR_OUTPUT")
            );
            assert!(
                output
                    .content
                    .contains("References: module.root.aws_ssm_parameter.app")
            );
        }
    }
    let graph = semantic::transform(&plan::parse(fixture).unwrap());
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_ecs_task_definition.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(svg::render(&filtered, &Layout::new(&filtered)).contains("Unavailable"));
    for case in 0..5 {
        let mut value: Value = serde_json::from_str(fixture).unwrap();
        match case {
            0 => value["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            1 => value["resource_changes"][0]["change"]["after_unknown"] = json!(true),
            2 => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            3 => value["resource_changes"][0]["deposed"] = json!("old"),
            _ => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["container_definitions"] =
                    json!({"references":["aws_ssm_parameter.app", "var.selector"]})
            }
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert_eq!(graph.coverage.selected.resources, 3);
        assert!(
            parsed
                .attributes
                .iter()
                .all(|r| !r.service_reference_verified)
        );
        if case == 0 {
            assert!(
                !graph
                    .nodes
                    .iter()
                    .find(|n| n.address == "aws_ssm_parameter.app")
                    .unwrap()
                    .provider
                    .is_aws()
            );
        }
    }
    let indexed = fixture.replace("aws_ssm_parameter.app", "aws_ssm_parameter.app[0]");
    assert_eq!(
        semantic::transform(&plan::parse(&indexed).unwrap())
            .coverage
            .selected
            .resources,
        3
    );
    let mut ambiguous: Value = serde_json::from_str(fixture).unwrap();
    let mut second = ambiguous["resource_changes"][0].clone();
    ambiguous["resource_changes"][0]["address"] = json!("aws_ssm_parameter.app[0]");
    second["address"] = json!("aws_ssm_parameter.app[1]");
    ambiguous["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let parsed = plan::parse(&ambiguous.to_string()).unwrap();
    assert_eq!(semantic::transform(&parsed).coverage.selected.resources, 4);
    assert!(
        semantic::diagnostics::collect(&parsed)
            .iter()
            .any(|d| d.reason == crate::model::DiagnosticReason::MultipleMatchingInstances)
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: fixture.into(),
            })
            .collect(),
    )
    .unwrap();
    assert_eq!(plans.states.len(), 2);
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
}

#[test]
fn maintained_service_examples_keep_valid_links_service_names_and_module_selection() {
    for (fixture, name) in [
        (
            include_str!("../../fixtures/api-gateway-rest-plan.json"),
            "Amazon API Gateway",
        ),
        (
            include_str!("../../fixtures/api-gateway-v2-plan.json"),
            "Amazon API Gateway",
        ),
        (
            include_str!("../../fixtures/private-ca-plan.json"),
            "AWS Private CA",
        ),
        (include_str!("../../fixtures/efs-plan.json"), "Amazon EFS"),
        (include_str!("../../fixtures/ebs-plan.json"), "Amazon EBS"),
        (
            include_str!("../../fixtures/route53-plan.json"),
            "Amazon Route 53",
        ),
        (
            include_str!("../../fixtures/acm-plan.json"),
            "AWS Certificate Manager",
        ),
        (
            include_str!("../../fixtures/kms-plan.json"),
            "AWS Key Management Service",
        ),
        (
            include_str!("../../fixtures/secrets-manager-plan.json"),
            "AWS Secrets Manager",
        ),
        (
            include_str!("../../fixtures/ssm-plan.json"),
            "AWS Systems Manager",
        ),
    ] {
        for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
            let output = crate::generate(
                fixture,
                &crate::Options {
                    format,
                    icon_mode: crate::IconMode::ServiceName,
                    ..Default::default()
                },
            )
            .unwrap()
            .content;
            assert!(output.contains(name));
            let start = output.find("<svg").unwrap();
            let end = output.rfind("</svg>").unwrap() + 6;
            let doc = roxmltree::Document::parse(&output[start..end]).unwrap();
            let ids: std::collections::BTreeSet<_> = doc
                .descendants()
                .filter_map(|n| n.attribute("id"))
                .collect();
            for href in doc
                .descendants()
                .filter_map(|n| n.attribute("href"))
                .filter_map(|h| h.strip_prefix('#'))
            {
                assert!(ids.contains(href), "dangling link in {name}: {href}");
            }
        }
        let mut value: Value = serde_json::from_str(fixture).unwrap();
        for resource in value["resource_changes"].as_array_mut().unwrap() {
            resource["address"] = json!(format!(
                "module.services.{}",
                resource["address"].as_str().unwrap()
            ));
        }
        let module = value["configuration"]["root_module"].take();
        value["configuration"]["root_module"] =
            json!({"module_calls":{"services":{"module":module}}});
        let output = crate::generate(
            &value.to_string(),
            &crate::Options {
                modules: vec!["module.services".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert!(output.content.contains("module.services"));
    }
}

#[test]
fn secrets_keep_version_identity_and_never_infer_json_usage_or_retain_payloads() {
    let fixture = include_str!("../../fixtures/secrets-manager-plan.json");
    check_service_fixture(fixture, "Version secret", "secret_id");
    for data in [false, true] {
        for field in ["id", "arn", "name"] {
            let mut value: Value = serde_json::from_str(fixture).unwrap();
            if data {
                let address = "data.aws_secretsmanager_secret_version.app";
                value["resource_changes"][1]["address"] = json!(address);
                value["resource_changes"][1]["mode"] = json!("data");
                value["resource_changes"][1]["change"]["actions"] = json!(["read"]);
                value["configuration"]["root_module"]["resources"][1]["address"] = json!(address);
                value["configuration"]["root_module"]["resources"][1]["mode"] = json!("data");
            }
            value["resource_changes"][1]["change"]["after"]["secret_id"] =
                value["resource_changes"][0]["change"]["after"][field].clone();
            value["configuration"]["root_module"]["resources"][1]["expressions"]["secret_id"] =
                json!({"references": [format!("aws_secretsmanager_secret.app.{field}")]});
            let parsed = plan::parse(&value.to_string()).unwrap();
            let reference = parsed
                .attributes
                .iter()
                .find(|r| r.attribute == "secret_id")
                .unwrap();
            assert_eq!(
                reference.service_reference_verified,
                field != "name",
                "{data} {field}"
            );
        }
    }
    let mut value: Value = serde_json::from_str(fixture).unwrap();
    for key in [
        "secret_string",
        "secret_binary",
        "secret_string_wo",
        "version_payload",
        "rotation_lambda_arn",
    ] {
        value["resource_changes"][1]["change"]["after"][key] = json!("PRIVATE_<script>&payload");
    }
    value["resource_changes"][2]["change"]["after"]["container_definitions"] =
        json!("[{\"environment\":[{\"name\":\"TOKEN\",\"value\":\"PRIVATE_ENV\"}]}]");
    let parsed = plan::parse(&value.to_string()).unwrap();
    assert!(!format!("{parsed:?}").contains("PRIVATE_"));
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.nodes.len(), 4);
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.resource_type == "aws_secretsmanager_secret")
            .count(),
        2
    );
    let task = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_ecs_task_definition")
        .unwrap();
    assert!(
        task.references
            .iter()
            .any(|r| r.attribute == "container_definitions"
                && r.label == "References"
                && r.complete)
    );
    assert!(
        !parsed
            .attributes
            .iter()
            .any(|r| r.attribute == "container_definitions" && r.service_reference_verified)
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &value.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!output.content.contains("PRIVATE_"));
        assert!(!output.content.contains("SYNTHETIC_PAYLOAD_NOT_FOR_OUTPUT"));
        assert!(output.content.contains("Version secret"));
    }
    value["configuration"]["root_module"]["resources"][2]["expressions"]["container_definitions"] =
        json!({"constant_value":"literal secret ARN including :key:stage:version suffix"});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let task = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_ecs_task_definition")
        .unwrap();
    assert!(task.references.is_empty());
}

#[test]
fn kms_uses_only_allowlisted_configuration_fields_and_never_exposes_key_material() {
    let fixture = include_str!("../../fixtures/kms-plan.json");
    check_service_fixture(fixture, "Alias key", "target_key_id");
    let mut value: Value = serde_json::from_str(fixture).unwrap();
    value["resource_changes"][0]["change"]["after"]["policy"] = json!("PRIVATE_POLICY_<script>&");
    value["resource_changes"][0]["change"]["after"]["key_material"] = json!("PRIVATE_KEY");
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .filter(|r| r.label == "Encryption key reference" && r.complete)
            .count(),
        4
    );
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.resource_type.starts_with("aws_kms_"))
            .all(|n| n.placement == crate::model::Placement::Spatial
                && n.entity
                    .scope
                    .as_ref()
                    .is_some_and(|s| !s.global && s.region.is_some()))
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &value.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!output.content.contains("PRIVATE_"));
    }
    value["resource_changes"][3]["type"] = json!("aws_instance");
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert!(
        !graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_db_instance.data")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Encryption key reference")
    );
    value["resource_changes"][0]["type"] = json!("aws_kms_replica_key");
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert!(
        !graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Alias key" && r.complete)
    );
}

pub(in crate::provider::aws) fn check_service_fixture(fixture: &str, label: &str, attribute: &str) {
    let original: Value = serde_json::from_str(fixture).unwrap();
    let source = original["resource_changes"][0]["address"].as_str().unwrap();
    let consumer = original["resource_changes"][1]["address"].as_str().unwrap();
    for case in 0..11 {
        let mut value = original.clone();
        match case {
            1 => value["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            2 => value["resource_changes"][0]["change"]["after_unknown"] = json!(true),
            3 => value["resource_changes"][1]["change"]["after"][attribute] = json!("different"),
            4 => value["resource_changes"][0]["type"] = json!("aws_instance"),
            5 => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            6 => value["resource_changes"][0]["deposed"] = json!("old"),
            7 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"][attribute] =
                    json!({"constant_value":"literal"})
            }
            8 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"][attribute] =
                    json!({"references":[format!("{source}.arn"), "aws_instance.missing.id"]})
            }
            9 => value["resource_changes"][0]["mode"] = json!("data"),
            10 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"][attribute] =
                    json!({"references":[source, "var.selector"]})
            }
            _ => {}
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert_eq!(graph.coverage.selected.resources, parsed.nodes.len());
        assert_eq!(
            graph
                .nodes
                .iter()
                .find(|n| n.address == consumer)
                .unwrap()
                .references
                .iter()
                .any(|r| r.label == label && r.complete),
            case == 0,
            "case {case}: {consumer}"
        );
    }
    let indexed = fixture.replace(source, &format!("{source}[0]"));
    let mut indexed: Value = serde_json::from_str(&indexed).unwrap();
    indexed["configuration"]["root_module"]["resources"][0]["address"] = json!(source);
    let graph = semantic::transform(&plan::parse(&indexed.to_string()).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.address == consumer)
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == label && r.complete)
    );
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some(consumer.into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(svg::render(&filtered, &Layout::new(&filtered)).contains("Unavailable"));
    let mut ambiguous = original.clone();
    let mut second = ambiguous["resource_changes"][0].clone();
    ambiguous["resource_changes"][0]["address"] = json!(format!("{source}[0]"));
    second["address"] = json!(format!("{source}[1]"));
    ambiguous["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let graph = semantic::transform(&plan::parse(&ambiguous.to_string()).unwrap());
    assert!(
        !graph
            .nodes
            .iter()
            .find(|n| n.address == consumer)
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == label && r.complete)
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: fixture.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for plan in plans.states.values() {
        let graph = semantic::transform(plan);
        assert_eq!(
            graph.coverage.selected.resources,
            original["resource_changes"].as_array().unwrap().len()
        );
    }
}

#[test]
fn acm_preserves_certificate_workflow_scope_and_safe_references() {
    let fixture = include_str!("../../fixtures/acm-plan.json");
    check_service_fixture(fixture, "Certificate reference", "certificate_arn");
    let mut value: Value = serde_json::from_str(fixture).unwrap();
    for key in [
        "private_key",
        "certificate_body",
        "certificate_chain",
        "domain_name",
        "domain_validation_options",
    ] {
        value["resource_changes"][0]["change"]["after"][key] = json!("PRIVATE_<script>&secret");
    }
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.resource_type.starts_with("aws_acm_"))
            .all(|n| n.placement
                == if n.resource_type == "aws_acm_certificate" {
                    crate::model::Placement::Spatial
                } else {
                    crate::model::Placement::Logical
                }
                && n.entity
                    .scope
                    .as_ref()
                    .is_some_and(|s| !s.global && s.region.is_some()))
    );
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_cloudfront_distribution")
            .unwrap()
            .entity
            .scope
            .as_ref()
            .unwrap()
            .global
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .filter(|r| r.label == "Certificate reference" && r.complete)
            .count(),
        3
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &value.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!output.content.contains("PRIVATE_"));
        assert!(output.content.contains("Certificate reference"));
    }
}

#[test]
fn route53_rejects_ambiguous_and_dynamic_instances_and_accepts_nlb() {
    for dynamic in [false, true] {
        let mut value: Value =
            serde_json::from_str(include_str!("../../fixtures/route53-plan.json")).unwrap();
        let mut second = value["resource_changes"][2].clone();
        value["resource_changes"][2]["address"] = json!("aws_lb.web[0]");
        second["address"] = json!("aws_lb.web[1]");
        value["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(second);
        if dynamic {
            value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["name"] =
                json!({"references":["aws_lb.web","var.index"]});
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert!(
            !graph
                .nodes
                .iter()
                .flat_map(|n| &n.references)
                .any(|r| r.label == "Alias target" && r.complete)
        );
        assert!(!semantic::diagnostics::collect(&parsed).is_empty());
    }
    let input = include_str!("../../fixtures/route53-plan.json").replace("application", "network");
    let graph = semantic::transform(&plan::parse(&input).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Alias target" && r.complete)
    );
}

#[test]
fn service_links_keep_state_lifecycle_and_coverage_identity() {
    let fixture = include_str!("../../fixtures/route53-plan.json");
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: fixture.into(),
            })
            .collect(),
    )
    .unwrap();
    for plan in plans.states.values() {
        let graph = semantic::transform(plan);
        assert_eq!(graph.coverage.input.resources, 5);
        assert_eq!(graph.coverage.selected.resources, 5);
        assert!(
            graph
                .nodes
                .iter()
                .flat_map(|n| &n.references)
                .flat_map(|r| &r.targets)
                .all(|t| t.state.is_none())
        );
    }
    let mut value: Value = serde_json::from_str(fixture).unwrap();
    value["resource_changes"][1]["previous_address"] = json!("aws_route53_record.old");
    value["resource_changes"][1]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][1].clone();
    old["deposed"] = json!("previous");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 6);
    let current = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_route53_record.alias" && n.deposed_key.is_none())
        .unwrap();
    assert_eq!(
        current.previous_address.as_deref(),
        Some("aws_route53_record.old")
    );
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| !n
                .references
                .iter()
                .any(|r| r.label == "Alias target" && r.complete))
    );
    assert!(!svg::render(&graph, &Layout::new(&graph)).contains("PRIVATE_IMPORT"));
}

#[test]
fn route53_indexed_identity_filtering_and_cloudfront() {
    let input =
        include_str!("../../fixtures/route53-plan.json").replace("aws_lb.web", "aws_lb.web[0]");
    let parsed = plan::parse(&input).unwrap();
    let graph = semantic::transform(&parsed);
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Alias target" && r.complete)
    );
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_route53_record.alias".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let output = svg::render(&filtered, &Layout::new(&filtered));
    assert!(output.contains("Unavailable"));
    assert!(!output.contains("href=\"#aws_lb.web"));
    let mut value: Value =
        serde_json::from_str(include_str!("../../fixtures/route53-plan.json")).unwrap();
    value["resource_changes"][2]["type"] = json!("aws_cloudfront_distribution");
    let text = value
        .to_string()
        .replace("aws_lb.web", "aws_cloudfront_distribution.web")
        .replace("dns_name", "domain_name");
    let mut value: Value = serde_json::from_str(&text).unwrap();
    value["resource_changes"][2]["change"]["after"]["hosted_zone_id"] = json!("Z-LB");
    value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["zone_id"] =
        json!({"references":["aws_cloudfront_distribution.web.hosted_zone_id"]});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Alias target" && r.complete)
    );
}

#[test]
fn route53_requires_complete_current_typed_attribute_and_value_evidence() {
    for case in 0..13 {
        let mut value: Value =
            serde_json::from_str(include_str!("../../fixtures/route53-plan.json")).unwrap();
        match case {
            1 => value["resource_changes"][2]["provider_name"] = json!("example.com/foreign/aws"),
            2 => value["resource_changes"][2]["change"]["after_unknown"] = json!(true),
            3 => {
                value["resource_changes"][2]["change"]["after"]["load_balancer_type"] =
                    json!("gateway")
            }
            4 => {
                value["resource_changes"][1]["change"]["after"]["alias"][0]["zone_id"] =
                    json!("different")
            }
            5 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["name"] =
                    json!({"references":["aws_lb.web.tags"]})
            }
            6 => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            7 => value["resource_changes"][2]["deposed"] = json!("old"),
            8 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["name"] =
                    json!({"references":["aws_lb.web.dns_name","aws_lb.missing.dns_name"]})
            }
            9 => value["resource_changes"][2]["mode"] = json!("data"),
            10 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["name"] =
                    json!({"constant_value":"synthetic.invalid"})
            }
            11 => value["resource_changes"][1]["change"]["after_unknown"] = json!({"alias":true}),
            12 => value["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            _ => {}
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert_eq!(graph.nodes.len(), parsed.nodes.len());
        let alias = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_route53_record.alias")
            .unwrap();
        assert_eq!(
            alias
                .references
                .iter()
                .any(|r| r.label == "Alias target" && r.complete),
            case == 0,
            "case {case}"
        );
        assert_eq!(graph.edges.len(), parsed.edges.len());
        let output = svg::render(&graph, &Layout::new(&graph));
        assert!(!output.contains("synthetic.invalid"));
        assert!(!output.contains("Z-SYNTHETIC"));
    }
}

use crate::{plan, semantic, view};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/dms-endpoints-plan.json");
const PATHS: &[(usize, &str)] = &[
    (6, "certificate_arn"),
    (6, "kms_key_arn"),
    (6, "secrets_manager_arn"),
    (6, "secrets_manager_access_role_arn"),
    (7, "kinesis_settings.stream_arn"),
    (7, "kinesis_settings.service_access_role_arn"),
    (8, "redshift_settings.bucket_name"),
    (8, "redshift_settings.service_access_role_arn"),
    (8, "redshift_settings.server_side_encryption_kms_key_id"),
    (9, "bucket_name"),
    (9, "service_access_role_arn"),
    (9, "certificate_arn"),
    (9, "kms_key_arn"),
    (9, "server_side_encryption_kms_key_id"),
    (10, "service_access_role"),
];

fn complete(v: &Value, target: usize, path: &str) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let kind = v["resource_changes"][target]["type"].as_str().unwrap();
    let expected = super::label(kind, path).or_else(|| super::super::databases::label(kind, path));
    g.nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][target]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| r.attribute == path && r.complete && Some(r.label) == expected)
}

#[test]
fn exact_typed_endpoints_and_s3_task_targets_are_verified_without_payloads() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for &(t, p) in PATHS {
        assert!(complete(&v, t, p), "{p}");
    }
    assert!(complete(&v, 11, "target_endpoint_arn"));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let rendered = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!rendered.contains("PRIVATE_"));
    }
}

#[test]
fn consumers_require_current_typed_explicit_complete_and_nonsensitive_evidence() {
    for &(t, p) in PATHS {
        for case in [
            "literal",
            "unknown",
            "sensitive",
            "deleted",
            "deposed",
            "data",
            "foreign",
            "wrong-direction",
            "wrong-engine",
        ] {
            if case == "wrong-engine" && t == 9 {
                continue;
            }
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            let root = p.split('.').next().unwrap();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][t]["expressions"][root] =
                        json!({"constant_value":"literal"});
                }
                "unknown" => {
                    v["resource_changes"][t]["change"]["after_unknown"] = json!({root:true})
                }
                "sensitive" => {
                    v["resource_changes"][t]["change"]["after_sensitive"] = json!({root:true})
                }
                "deleted" => v["resource_changes"][t]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][t]["deposed"] = json!("old"),
                "data" => v["resource_changes"][t]["mode"] = json!("data"),
                "foreign" => {
                    v["resource_changes"][t]["provider_name"] = json!("example.org/foreign/aws")
                }
                "wrong-direction" => {
                    v["resource_changes"][t]["change"]["after"]["endpoint_type"] = json!("unknown")
                }
                "wrong-engine" => {
                    v["resource_changes"][t]["change"]["after"]["engine_name"] = json!("unknown")
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, t, p), "{t}/{p}/{case}");
        }
    }
}

#[test]
fn endpoint_types_outputs_values_scope_and_lifecycle_are_independent() {
    for case in [
        "type",
        "output",
        "value",
        "unknown",
        "sensitive",
        "foreign",
        "data",
        "delete",
        "deposed",
        "region",
        "ambiguous",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "type" => v["resource_changes"][3]["type"] = json!("aws_acm_certificate"),
            "output" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["certificate_arn"] =
                    json!({"references":["aws_dms_certificate.certificate.id"]})
            }
            "value" => {
                v["resource_changes"][3]["change"]["after"]["certificate_arn"] = json!("different")
            }
            "unknown" => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"certificate_arn":true})
            }
            "sensitive" => {
                v["resource_changes"][3]["change"]["after_sensitive"] =
                    json!({"certificate_arn":true})
            }
            "foreign" => {
                v["resource_changes"][3]["provider_name"] = json!("example.org/foreign/aws")
            }
            "data" => v["resource_changes"][3]["mode"] = json!("data"),
            "delete" => v["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            "deposed" => v["resource_changes"][3]["deposed"] = json!("old"),
            "region" => {
                v["resource_changes"][3]["change"]["after"]["region"] = json!("us-west-2");
                v["configuration"]["root_module"]["resources"][3]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"});
            }
            "ambiguous" => {
                let mut other = v["resource_changes"][3].clone();
                other["address"] = json!("aws_dms_certificate.certificate[1]");
                v["resource_changes"][3]["address"] = json!("aws_dms_certificate.certificate[0]");
                v["resource_changes"].as_array_mut().unwrap().push(other);
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 6, "certificate_arn"), "{case}");
    }
}

#[test]
fn nested_modes_owner_constraints_and_task_direction_fail_closed() {
    for (t, path, field, value) in [
        (
            7,
            "kinesis_settings.stream_arn",
            "engine_name",
            json!("mysql"),
        ),
        (
            8,
            "redshift_settings.bucket_name",
            "engine_name",
            json!("kinesis"),
        ),
        (
            9,
            "server_side_encryption_kms_key_id",
            "encryption_mode",
            json!("SSE_S3"),
        ),
        (
            9,
            "server_side_encryption_kms_key_id",
            "endpoint_type",
            json!("source"),
        ),
        (
            9,
            "bucket_name",
            "expected_bucket_owner",
            json!("999999999999"),
        ),
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][t]["change"]["after"][field] = value;
        assert!(!complete(&v, t, path), "{path}");
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][7]["change"]["after"]["kinesis_settings"] = json!([
        {"stream_arn":"arn:aws:kinesis:us-east-1:111111111111:stream/changes"}, {}
    ]);
    assert!(!complete(&v, 7, "kinesis_settings.stream_arn"));
    v = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][9]["change"]["after"]["endpoint_type"] = json!("source");
    assert!(!complete(&v, 11, "target_endpoint_arn"));
}

#[test]
fn filtering_and_named_states_keep_exactly_once_source_identity() {
    let raw = plan::parse(FIXTURE).unwrap();
    let graph = semantic::transform(&raw);
    assert_eq!(graph.coverage.input.resources, raw.nodes.len());
    assert_eq!(graph.coverage.selected.resources, raw.nodes.len());
    let filtered = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_dms_endpoint.source".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(filtered.coverage.selected.resources, 1);
    let svg = crate::generate(
        FIXTURE,
        &crate::Options {
            focus: Some("aws_dms_endpoint.source".into()),
            focus_depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    let document = roxmltree::Document::parse(&svg).unwrap();
    for href in document
        .descendants()
        .filter_map(|n| n.attribute("href"))
        .filter_map(|s| s.strip_prefix('#'))
    {
        assert!(
            document
                .descendants()
                .any(|n| n.attribute("id") == Some(href))
        );
    }
    let inputs = ["one", "two"]
        .into_iter()
        .map(|state| crate::NamedInput {
            state: state.into(),
            json: FIXTURE.into(),
        })
        .collect::<Vec<_>>();
    let result = crate::generate_named(&inputs, &crate::Options::default()).unwrap();
    assert!(!result.content.contains("PRIVATE_"));
}

#[test]
fn bucket_owner_requires_independent_matching_account_evidence() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][9]["change"]["after"]["expected_bucket_owner"] = json!("111111111111");
    assert!(!complete(&v, 9, "bucket_name"));
    let mut raw = plan::parse(&v.to_string()).unwrap();
    for node in &mut raw.nodes {
        node.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("111111111111");
    }
    let verified = |raw: &crate::model::TerraformPlan| {
        semantic::transform(raw)
            .nodes
            .iter()
            .find(|n| n.address == "aws_dms_s3_endpoint.storage")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "DMS storage bucket" && r.complete)
    };
    assert!(verified(&raw));
    raw.nodes
        .iter_mut()
        .find(|n| n.address == "aws_s3_bucket.storage")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("999999999999");
    assert!(!verified(&raw));
}

#[test]
fn scalar_fields_cannot_be_substituted_with_single_element_arrays() {
    for &(target, path) in PATHS {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let values = &mut v["resource_changes"][target]["change"]["after"];
        let field = if let Some((block, field)) = path.split_once('.') {
            &mut values[block][0][field]
        } else {
            &mut values[path]
        };
        *field = json!([field.clone()]);
        assert!(!complete(&v, target, path), "{path}");
    }
}

use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/cloudfront-security-plan.json");
fn input() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn verified(v: &Value, index: usize, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == v["resource_changes"][index]["address"].as_str().unwrap()
            && r.attribute == path
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn complete_key_and_profile_collections_preserve_shared_global_identities() {
    let v = input();
    for (index, path) in [
        (2, "items"),
        (3, super::ENCRYPTION),
        (4, super::ENCRYPTION),
        (5, super::CONTENT),
        (5, super::QUERY),
    ] {
        assert!(verified(&v, index, path), "{index}/{path}");
    }
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 9);
    for n in graph
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_cloudfront_"))
    {
        assert_eq!(n.placement, crate::model::Placement::Logical);
        assert!(n.member_owner.is_none());
        assert!(
            !n.references
                .iter()
                .any(|r| super::indexed(&n.resource_type, &r.attribute))
        );
        assert!(n.entity.scope.as_ref().unwrap().global);
        assert!(n.entity.scope.as_ref().unwrap().region.is_none());
    }
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    let mut reordered = input();
    reordered["resource_changes"][2]["change"]["after"]["items"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert!(verified(&reordered, 2, "items"));
    let items = &mut reordered["resource_changes"][5]["change"]["after"]["query_arg_profile_config"]
        [0]["query_arg_profiles"][0]["items"];
    items.as_array_mut().unwrap().reverse();
    assert!(!verified(&reordered, 5, super::QUERY));
    reordered["configuration"]["root_module"]["resources"][5]["expressions"]["query_arg_profile_config"][0]["query_arg_profiles"][0]["items"].as_array_mut().unwrap().reverse();
    assert!(verified(&reordered, 5, super::QUERY));
}

#[test]
fn sets_and_indexed_rows_require_complete_typed_known_current_provenance() {
    for case in 0..15 {
        let mut v = input();
        match case {
            0 => v["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true}),
            1 => v["resource_changes"][2]["change"]["after_sensitive"] = json!({"items":true}),
            2 => v["resource_changes"][2]["change"]["after"]["items"][0] = "literal-other".into(),
            3 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["items"] =
                    json!({"references":["aws_cloudfront_public_key.first.id","var.missing"]})
            }
            4 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["items"] =
                    json!({"constant_value":["public-first","public-second"]})
            }
            5 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["items"] = json!({"references":["aws_cloudfront_public_key.first.arn","aws_cloudfront_public_key.second.arn"]})
            }
            6 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            7 => v["resource_changes"][0]["deposed"] = "old".into(),
            8 => v["resource_changes"][0]["mode"] = "data".into(),
            9 => v["resource_changes"][0]["provider_name"] = "example.com/custom/aws".into(),
            10 => v["resource_changes"][2]["change"]["after_unknown"] = json!({"items":[false]}),
            11 => v["resource_changes"][2]["change"]["after"]["items"] = json!([]),
            12 => v["resource_changes"][1]["change"]["after"]["id"] = "public-first".into(),
            13 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["items"] =
                    json!({"references":["aws_cloudfront_field_level_encryption_profile.first.id"]})
            }
            _ => {
                v["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                    "aws.missing".into()
            }
        }
        assert!(!verified(&v, 2, "items"), "case {case}");
    }
    for case in 0..8 {
        let mut v = input();
        match case {
            0 => {
                v["resource_changes"][3]["change"]["after_sensitive"] =
                    json!({"encryption_entities":[{"items":[{}, {"public_key_id":true}]}]})
            }
            1 => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"encryption_entities":[{"items":[false]}]})
            }
            2 => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["encryption_entities"]
                    [0]["items"][1]["public_key_id"] = json!({"constant_value":"public-second"})
            }
            3 => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["encryption_entities"]
                    [0]["items"][1]["public_key_id"] =
                    json!({"references":["aws_cloudfront_public_key.first.id"]})
            }
            4 => {
                v["resource_changes"][3]["change"]["after"]["encryption_entities"][0]["items"][1]
                    ["public_key_id"] = Value::Null
            }
            5 => {
                let row =
                    v["resource_changes"][3]["change"]["after"]["encryption_entities"][0].clone();
                v["resource_changes"][3]["change"]["after"]["encryption_entities"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            6 => {
                v["resource_changes"][3]["change"]["after"]["encryption_entities"][0]["items"] =
                    json!([])
            }
            _ => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["encryption_entities"]
                    [0]["items"][0]["public_key_id"] =
                    json!({"references":["aws_cloudfront_public_key.first.id","var.dynamic"]});
            }
        }
        assert!(!verified(&v, 3, super::ENCRYPTION), "indexed case {case}");
    }
    let mut v = input();
    v["resource_changes"][5]["change"]["after_unknown"] = json!({"content_type_profile_config":[{"content_type_profiles":[{"items":[{}, {"profile_id":true},{}]}]}]});
    assert!(!verified(&v, 5, super::CONTENT));
}

#[test]
fn trust_store_bundle_requires_independent_region_bucket_key_and_optional_version() {
    let path = |field| format!("{}.{}", super::BUNDLE, field);
    let v = input();
    for field in ["bucket", "key", "version"] {
        assert!(verified(&v, 8, &path(field)), "{field}");
    }
    for case in 0..14 {
        let mut v = input();
        match case {
            0 => {
                v["resource_changes"][8]["change"]["after"]["ca_certificates_bundle_source"][0]["ca_certificates_bundle_s3_location"]
                    [0]["region"] = "us-east-1".into()
            }
            1 => {
                v["resource_changes"][8]["change"]["after_unknown"] = json!({"ca_certificates_bundle_source":[{"ca_certificates_bundle_s3_location":[{"region":true}]}]})
            }
            2 => {
                v["resource_changes"][8]["change"]["after_sensitive"] =
                    json!({"ca_certificates_bundle_source":true})
            }
            3 => v["resource_changes"][7]["change"]["after"]["bucket"] = "other-bucket".into(),
            4 => v["resource_changes"][7]["change"]["after"]["key"] = "other.pem".into(),
            5 => v["resource_changes"][7]["change"]["after"]["version_id"] = "other-version".into(),
            6 => {
                v["configuration"]["root_module"]["resources"][8]["expressions"]["ca_certificates_bundle_source"]
                    [0]["ca_certificates_bundle_s3_location"][0]["key"] =
                    json!({"references":["aws_s3_object.bundle.id"]})
            }
            7 => {
                v["configuration"]["root_module"]["resources"][8]["expressions"]["ca_certificates_bundle_source"]
                    [0]["ca_certificates_bundle_s3_location"][0]["version"] =
                    json!({"constant_value":"version-1"})
            }
            8 => {
                v["resource_changes"][8]["change"]["after_unknown"] = json!({"ca_certificates_bundle_source":[{"ca_certificates_bundle_s3_location":[{"version":true}]}]})
            }
            9 => v["resource_changes"][7]["change"]["actions"] = json!(["delete"]),
            10 => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["bucket"] =
                    json!({"constant_value":"bundle-bucket"})
            }
            11 => v["resource_changes"][6]["change"]["after_unknown"] = json!({"id":true}),
            12 => {
                v["resource_changes"][8]["change"]["after"]["ca_certificates_bundle_source"] =
                    json!([])
            }
            _ => {
                v["resource_changes"][8]["change"]["after"]["ca_certificates_bundle_source"][0]["ca_certificates_bundle_s3_location"]
                    [0]["region"] = Value::Null
            }
        }
        assert!(!verified(&v, 8, &path("key")), "bundle case {case}");
    }
    let mut optional = input();
    optional["resource_changes"][8]["change"]["after"]["ca_certificates_bundle_source"][0]["ca_certificates_bundle_s3_location"][0].as_object_mut().unwrap().remove("version");
    optional["configuration"]["root_module"]["resources"][8]["expressions"]["ca_certificates_bundle_source"][0]["ca_certificates_bundle_s3_location"][0].as_object_mut().unwrap().remove("version");
    assert!(verified(&optional, 8, &path("key")));
    assert!(!verified(&optional, 8, &path("version")));
}

#[test]
fn late_accounts_selection_states_and_payloads_preserve_resource_boundaries() {
    let mut v = input();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Fixture"}}]);
    let mut other = v["configuration"]["provider_config"]["aws"].clone();
    other["alias"] = "global".into();
    other["expressions"]["region"] = json!({"constant_value":"us-east-1"});
    v["configuration"]["provider_config"]["aws.global"] = other;
    v["configuration"]["root_module"]["resources"][8]["provider_config_key"] = "aws.global".into();
    let p = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    assert!(
        raw.attributes
            .iter()
            .any(|r| r.attribute == format!("{}.key", super::BUNDLE)
                && super::super::service_references::verified(&raw, r))
    );
    let graph = semantic::transform(&p);
    for n in &graph.nodes {
        if n.resource_type.starts_with("aws_cloudfront_") {
            assert!(n.entity.scope.as_ref().unwrap().account.is_some());
        }
    }
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_cloudfront_trust_store.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    for kind in [
        "aws_cloudfront_public_key",
        "aws_s3_bucket",
        "aws_s3_object",
    ] {
        let mut p = p.clone();
        p.nodes
            .iter_mut()
            .find(|n| n.resource_type == kind)
            .unwrap()
            .provider_configuration
            .as_mut()
            .unwrap()
            .account = crate::model::AwsAccountScope::new("222222222222");
        let raw = semantic::Input::new(&p);
        let path = if kind == "aws_cloudfront_public_key" {
            "items".to_string()
        } else {
            format!("{}.key", super::BUNDLE)
        };
        assert!(!raw.attributes.iter().any(|r|r.attribute==path&&super::super::service_references::verified(&raw,r)),"{kind}");
    }
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|name| crate::model::PlanInput {
                state_id: crate::model::StateId::new(name).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            !crate::generate(
                FIXTURE,
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("PRIVATE_")
        );
    }
}

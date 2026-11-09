use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/s3tables-plan.json");
fn pair(s: usize, t: usize) -> Value {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut p = v.clone();
    p["resource_changes"] = json!([v["resource_changes"][s], v["resource_changes"][t]]);
    p["configuration"]["root_module"]["resources"] = json!([
        v["configuration"]["root_module"]["resources"][s],
        v["configuration"]["root_module"]["resources"][t]
    ]);
    p
}
fn complete(v: &Value, label: &str, accounts: bool) -> bool {
    let mut p = plan::parse(&v.to_string()).unwrap();
    if accounts {
        for n in &mut p.nodes {
            if let Some(c) = n.provider_configuration.as_mut() {
                c.account = crate::model::AwsAccountScope::new("123456789012");
            }
        }
    }
    semantic::transform(&p)
        .nodes
        .iter()
        .flat_map(|n| &n.references)
        .any(|r| r.label == label && r.complete)
}
#[test]
fn table_owners_require_exact_current_and_bucket_qualified_identity() {
    for (s, t, path, label) in [
        (1, 2, "table_bucket_arn", "Table bucket"),
        (2, 3, "namespace", "Qualified namespace"),
        (2, 3, "table_bucket_arn", "Table bucket"),
        (1, 4, "table_bucket_arn", "Table bucket"),
        (3, 5, "name", "Qualified table owner"),
        (1, 8, "table_bucket_arn", "Table bucket"),
        (3, 9, "table_arn", "Replication source table"),
        (6, 8, "role", "Configured replication role"),
        (6, 9, "role", "Configured replication role"),
    ] {
        super::super::service_references::tests::check_service_fixture(
            &pair(s, t).to_string(),
            label,
            path,
        );
    }
    for (s, t, label) in [
        (2, 3, "Qualified namespace"),
        (3, 5, "Qualified table owner"),
    ] {
        for case in 0..4 {
            let mut v = pair(s, t);
            match case {
                1 => {
                    v["resource_changes"][0]["change"]["after"]["table_bucket_arn"] =
                        json!("other-bucket")
                }
                2 => {
                    v["resource_changes"][1]["change"]["after_unknown"] =
                        json!({"table_bucket_arn":true})
                }
                3 => {
                    v["resource_changes"][0]["change"]["after_sensitive"] =
                        json!({"namespace":true})
                }
                _ => {}
            }
            assert_eq!(complete(&v, label, false), case == 0, "{label}: {case}");
        }
    }
}
#[test]
fn table_encryption_requires_object_shape_known_kms_mode_and_key_arn() {
    for t in [1, 3] {
        for case in 0..9 {
            let mut v = pair(0, t);
            match case {
                1 => {
                    v["resource_changes"][1]["change"]["after"]["encryption_configuration"]["sse_algorithm"] =
                        json!("AES256")
                }
                2 => {
                    v["resource_changes"][1]["change"]["after_unknown"] =
                        json!({"encryption_configuration":{"sse_algorithm":true}})
                }
                3 => {
                    v["resource_changes"][1]["change"]["after_sensitive"] =
                        json!({"encryption_configuration":true})
                }
                4 => {
                    let b = v["resource_changes"][1]["change"]["after"]["encryption_configuration"]
                        .clone();
                    v["resource_changes"][1]["change"]["after"]["encryption_configuration"] =
                        json!([b]);
                }
                5 => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"]["encryption_configuration"]
                        ["kms_key_arn"] = json!({"constant_value":"literal"})
                }
                6 => v["resource_changes"][0]["type"] = json!("aws_kms_alias"),
                7 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                8 => v["resource_changes"][0]["change"]["after_sensitive"] = json!({"arn":true}),
                _ => {}
            }
            assert_eq!(
                complete(&v, "Table encryption key", false),
                case == 0,
                "{t}: {case}"
            );
        }
    }
}
#[test]
fn replication_destinations_require_complete_correlated_sets_and_compatible_scope() {
    for t in [8, 9] {
        for case in 0..9 {
            let mut v = pair(7, t);
            match case {
                1 => v["resource_changes"][1]["change"]["after"]["rule"][0]["destination"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"destination_table_bucket_arn":"literal"})),
                2 => v["resource_changes"][1]["change"]["after_unknown"] = json!({"rule":true}),
                3 => v["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
                4 => v["resource_changes"][0]["mode"] = json!("data"),
                5 => v["resource_changes"][0]["deposed"] = json!("old"),
                6 => {
                    let rule = v["resource_changes"][1]["change"]["after"]["rule"][0].clone();
                    v["resource_changes"][1]["change"]["after"]["rule"]
                        .as_array_mut()
                        .unwrap()
                        .push(rule);
                }
                7 => {
                    v["configuration"]["provider_config"]["aws.west"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                _ => {}
            }
            assert_eq!(
                complete(&v, "Configured replication destinations", case != 8),
                case == 0,
                "{t}: {case}"
            );
        }
    }
}
#[test]
fn tables_remain_independent_and_payloads_are_never_rendered() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 10);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        5
    );
    assert!(
        g.nodes
            .iter()
            .filter(|n| matches!(
                n.resource_type.as_str(),
                "aws_s3tables_table" | "aws_s3tables_table_bucket"
            ))
            .all(|n| n.member_owner.is_none() && n.placement == crate::model::Placement::Spatial)
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_TABLE_PAYLOAD"));
    }
}

#[test]
fn replication_set_rows_do_not_swap_and_direct_bucket_outputs_work() {
    let mut v = pair(1, 3);
    v["configuration"]["root_module"]["resources"][1]["expressions"]["table_bucket_arn"] =
        json!({"references":["aws_s3tables_table_bucket.main.arn"]});
    assert!(complete(&v, "Table bucket", false));
    let mut v = pair(7, 8);
    let mut bucket = v["resource_changes"][0].clone();
    bucket["address"] = json!("aws_s3tables_table_bucket.second");
    bucket["change"]["after"]["arn"] =
        json!("arn:aws:s3tables:us-west-2:123456789012:bucket/second");
    v["resource_changes"].as_array_mut().unwrap().push(bucket);
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_s3tables_table_bucket.second","provider_config_key":"aws.west","expressions":{}}));
    v["resource_changes"][1]["change"]["after"]["rule"][0]["destination"].as_array_mut().unwrap().push(json!({"destination_table_bucket_arn":"arn:aws:s3tables:us-west-2:123456789012:bucket/second"}));
    v["configuration"]["root_module"]["resources"][1]["expressions"]["rule"][0]["destination"].as_array_mut().unwrap().push(json!({"destination_table_bucket_arn":{"references":["aws_s3tables_table_bucket.second.arn"]}}));
    assert!(complete(&v, "Configured replication destinations", true));
    v["configuration"]["root_module"]["resources"][1]["expressions"]["rule"][0]["destination"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(!complete(&v, "Configured replication destinations", true));
}

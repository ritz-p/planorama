use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/s3vectors-plan.json");
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
fn complete(v: &Value, label: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .flat_map(|n| &n.references)
        .any(|r| r.label == label && r.complete)
}
#[test]
fn vector_owners_require_specialized_outputs_and_current_bucket_identity() {
    for (t, path) in [(2, "vector_bucket_name"), (3, "vector_bucket_arn")] {
        let p = pair(1, t);
        super::super::service_references::tests::check_service_fixture(
            &p.to_string(),
            "Vector bucket owner",
            path,
        );
        for wrong in ["arn", "id", "name", "index_arn"] {
            let mut v = p.clone();
            let actual = v["resource_changes"][0]["change"]["after"][path].clone();
            v["resource_changes"][0]["change"]["after"][wrong] = actual;
            v["configuration"]["root_module"]["resources"][1]["expressions"][path] =
                json!({"references":[format!("aws_s3vectors_vector_bucket.main.{wrong}")]});
            assert!(!complete(&v, "Vector bucket owner"), "{path}: {wrong}");
        }
    }
}
#[test]
fn vector_encryption_requires_exact_singleton_list_known_kms_mode_and_arn() {
    for t in [1, 2] {
        for case in 0..12 {
            let mut v = pair(0, t);
            match case {
                1 => {
                    v["resource_changes"][1]["change"]["after"]["encryption_configuration"][0]["sse_type"] =
                        json!("AES256")
                }
                2 => {
                    v["resource_changes"][1]["change"]["after_unknown"] =
                        json!({"encryption_configuration":[{"sse_type":true}]})
                }
                3 => {
                    v["resource_changes"][1]["change"]["after_sensitive"] =
                        json!({"encryption_configuration":true})
                }
                4 => {
                    v["resource_changes"][1]["change"]["after"]["encryption_configuration"] =
                        v["resource_changes"][1]["change"]["after"]["encryption_configuration"][0]
                            .clone()
                }
                5 => {
                    let b = v["resource_changes"][1]["change"]["after"]["encryption_configuration"]
                        [0]
                    .clone();
                    v["resource_changes"][1]["change"]["after"]["encryption_configuration"]
                        .as_array_mut()
                        .unwrap()
                        .push(b);
                }
                6 => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"]["encryption_configuration"]
                        [0]["kms_key_arn"] = json!({"constant_value":"literal"})
                }
                7 => v["resource_changes"][0]["type"] = json!("aws_kms_alias"),
                8 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                9 => v["resource_changes"][0]["change"]["after_sensitive"] = json!({"arn":true}),
                10 => v["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
                11 => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
                        json!({"constant_value":"us-west-2"})
                }
                _ => {}
            }
            assert_eq!(
                complete(&v, "Vector encryption key"),
                case == 0,
                "{t}: {case}"
            );
        }
    }
}
#[test]
fn vector_bucket_scope_coverage_and_payload_privacy_are_preserved() {
    let p = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&p);
    assert_eq!(g.coverage.selected.resources, 4);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        2
    );
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_s3vectors_vector_bucket")
            .unwrap()
            .placement,
        crate::model::Placement::Spatial
    );
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
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
        assert!(!out.contains("PRIVATE_VECTOR_PAYLOAD"));
    }
    let mut changed = p;
    changed
        .nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_s3vectors_vector_bucket")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account_conflict = true;
    assert!(
        !semantic::transform(&changed)
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Vector bucket owner" && r.complete)
    );
}

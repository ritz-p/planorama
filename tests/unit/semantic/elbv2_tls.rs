use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    let definitions = [
        (
            "aws_lb_listener.app",
            json!({"arn":"PRIVATE_LISTENER", "certificate_arn":"PRIVATE_CERT", "mutual_authentication":[{"mode":"verify","trust_store_arn":"PRIVATE_TRUST"}]}),
            json!({"certificate_arn":{"references":["aws_acm_certificate.app.arn"]},"mutual_authentication":[{"mode":{"constant_value":"verify"},"trust_store_arn":{"references":["aws_lb_trust_store.app.arn"]}}]}),
        ),
        (
            "aws_acm_certificate.app",
            json!({"arn":"PRIVATE_CERT"}),
            json!({}),
        ),
        (
            "aws_iam_server_certificate.app",
            json!({"arn":"PRIVATE_IAM"}),
            json!({}),
        ),
        (
            "aws_lb_listener_certificate.app",
            json!({"listener_arn":"PRIVATE_LISTENER","certificate_arn":"PRIVATE_IAM"}),
            json!({"listener_arn":{"references":["aws_lb_listener.app.arn"]},"certificate_arn":{"references":["aws_iam_server_certificate.app.arn"]}}),
        ),
        (
            "aws_lb_trust_store.app",
            json!({"arn":"PRIVATE_TRUST", "id":"PRIVATE_TRUST", "ca_certificates_bundle_s3_bucket":"PRIVATE_BUCKET", "ca_certificates_bundle_s3_key":"PRIVATE_KEY", "ca_certificates_bundle_s3_object_version":"PRIVATE_VERSION"}),
            json!({"ca_certificates_bundle_s3_bucket":{"references":["aws_s3_bucket.app.id"]},"ca_certificates_bundle_s3_key":{"references":["aws_s3_object.app.key"]},"ca_certificates_bundle_s3_object_version":{"references":["aws_s3_object.app.version_id"]}}),
        ),
        (
            "aws_lb_trust_store_revocation.app",
            json!({"trust_store_arn":"PRIVATE_TRUST", "revocations_s3_bucket":"PRIVATE_BUCKET", "revocations_s3_key":"PRIVATE_KEY", "revocations_s3_object_version":"PRIVATE_VERSION", "revocation_id":1}),
            json!({"trust_store_arn":{"references":["aws_lb_trust_store.app.arn"]},"revocations_s3_bucket":{"references":["aws_s3_bucket.app.id"]},"revocations_s3_key":{"references":["aws_s3_object.app.key"]},"revocations_s3_object_version":{"references":["aws_s3_object.app.version_id"]}}),
        ),
        (
            "aws_s3_bucket.app",
            json!({"id":"PRIVATE_BUCKET", "bucket":"PRIVATE_BUCKET"}),
            json!({}),
        ),
        (
            "aws_s3_object.app",
            json!({"bucket":"PRIVATE_BUCKET", "key":"PRIVATE_KEY", "version_id":"PRIVATE_VERSION"}),
            json!({}),
        ),
    ];
    json!({"format_version":"1.2", "resource_changes": definitions.iter().map(|(address,value,_)| json!({"address":address,"type":address.split('.').next().unwrap(),"change":{"actions":["create"],"after":value}})).collect::<Vec<_>>(),
        "configuration":{"provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":definitions.iter().map(|(address,_,expressions)|json!({"address":address,"provider_config_key":"aws","expressions":expressions})).collect::<Vec<_>>()}}})
}

fn complete(input: &Value, address: &str, label: &str) -> bool {
    semantic::transform(&plan::parse(&input.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == address)
        .is_some_and(|n| n.references.iter().any(|r| r.label == label && r.complete))
}

#[test]
fn listener_certificates_and_trust_sources_preserve_independent_resources() {
    for alias in [false, true] {
        let input = fixture().to_string();
        let input = if alias {
            input.replace("aws_lb_listener", "aws_alb_listener")
        } else {
            input
        };
        let raw = plan::parse(&input).unwrap();
        let graph = semantic::transform(&raw);
        assert!(graph.edges.iter().any(|e| {
            e.kind == EdgeKind::Association
                && e.change
                    .as_ref()
                    .is_some_and(|c| c.address.ends_with("listener_certificate.app"))
        }));
        assert_eq!(graph.coverage.selected.resources, 8);
        for label in [
            "Certificate reference",
            "Verify-mode trust store",
            "Revocation trust store",
            "CA bundle bucket",
            "CA bundle object",
            "CA bundle object version",
            "Revocation source object version",
        ] {
            assert!(
                graph
                    .nodes
                    .iter()
                    .flat_map(|n| &n.references)
                    .any(|r| r.label == label && r.complete),
                "{label}"
            );
        }
        assert!(!graph.edges.iter().any(|e| e.kind == EdgeKind::Containment));
        let iam = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_iam_server_certificate")
            .unwrap();
        assert!(iam.entity.scope.as_ref().unwrap().global);
        assert!(!format!("{raw:?}{graph:?}").contains("PRIVATE_"));
    }
    let mut input = fixture();
    input["resource_changes"][0]["change"]["after"]["certificate_arn"] = json!("PRIVATE_IAM");
    input["configuration"]["root_module"]["resources"][0]["expressions"]["certificate_arn"] =
        json!({"references":["aws_iam_server_certificate.app.arn"]});
    assert!(complete(
        &input,
        "aws_lb_listener.app",
        "Certificate reference"
    ));
    input["resource_changes"][1]["change"]["after"]["id"] = json!("PRIVATE_CERT");
    input["resource_changes"][0]["change"]["after"]["certificate_arn"] = json!("PRIVATE_CERT");
    input["configuration"]["root_module"]["resources"][0]["expressions"]["certificate_arn"] =
        json!({"references":["aws_acm_certificate.app.id"]});
    assert!(complete(
        &input,
        "aws_lb_listener.app",
        "Certificate reference"
    ));
}

#[test]
fn mutual_authentication_requires_one_complete_verify_block() {
    for case in 0..9 {
        let mut input = fixture();
        match case {
            0 => {
                input["resource_changes"][0]["change"]["after"]["mutual_authentication"][0]["mode"] =
                    json!("passthrough")
            }
            1 => {
                input["resource_changes"][0]["change"]["after_unknown"] =
                    json!({"mutual_authentication":[{"mode":true}]})
            }
            2 => {
                input["resource_changes"][0]["change"]["after_sensitive"] =
                    json!({"mutual_authentication":[{"trust_store_arn":true}]})
            }
            3 => input["resource_changes"][0]["change"]["after"]["mutual_authentication"]
                .as_array_mut()
                .unwrap()
                .push(json!({"mode":"verify","trust_store_arn":"PRIVATE_TRUST"})),
            4 => input["resource_changes"][4]["type"] = json!("aws_cloudfront_trust_store"),
            5 => {
                input["resource_changes"][0]["change"]["after"]["mutual_authentication"][0]["trust_store_arn"] =
                    json!("OTHER")
            }
            6 => {
                input["configuration"]["root_module"]["resources"][0]["expressions"]["mutual_authentication"] =
                    json!([{"trust_store_arn":{"constant_value":"PRIVATE_TRUST"}}])
            }
            7 => {
                input["resource_changes"][0]["change"]["after_sensitive"] =
                    json!({"mutual_authentication":true})
            }
            _ => {
                input["resource_changes"][0]["change"]["after_unknown"] =
                    json!({"mutual_authentication":[{},{}]})
            }
        }
        assert!(
            !complete(&input, "aws_lb_listener.app", "Verify-mode trust store"),
            "{case}"
        );
    }
}

#[test]
fn object_provenance_is_bucket_key_and_version_correlated() {
    for case in 0..9 {
        let mut input = fixture();
        match case {
            0 => input["resource_changes"][7]["change"]["after"]["bucket"] = json!("OTHER"),
            1 => input["resource_changes"][7]["change"]["after"]["key"] = json!("OTHER"),
            2 => input["resource_changes"][7]["change"]["after"]["version_id"] = json!("OTHER"),
            3 => {
                input["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"ca_certificates_bundle_s3_object_version":true})
            }
            4 => {
                input["configuration"]["root_module"]["resources"][4]["expressions"]["ca_certificates_bundle_s3_object_version"] =
                    json!({"constant_value":"PRIVATE_VERSION"})
            }
            5 => input["resource_changes"][7]["change"]["after_sensitive"] = json!({"key":true}),
            6 => {
                input["configuration"]["root_module"]["resources"][4]["expressions"]["ca_certificates_bundle_s3_bucket"] =
                    json!({"constant_value":"PRIVATE_BUCKET"})
            }
            7 => {
                let mut other = input["resource_changes"][7].clone();
                other["address"] = json!("aws_s3_object.other");
                input["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(other);
                input["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"address":"aws_s3_object.other","provider_config_key":"aws"}));
                input["configuration"]["root_module"]["resources"][4]["expressions"]["ca_certificates_bundle_s3_object_version"] =
                    json!({"references":["aws_s3_object.other.version_id"]});
            }
            _ => input["resource_changes"][7]["mode"] = json!("data"),
        }
        assert!(
            !complete(&input, "aws_lb_trust_store.app", "CA bundle object"),
            "{case}"
        );
    }
    let mut input = fixture();
    input["resource_changes"][4]["change"]["after"]
        .as_object_mut()
        .unwrap()
        .remove("ca_certificates_bundle_s3_object_version");
    input["configuration"]["root_module"]["resources"][4]["expressions"]
        .as_object_mut()
        .unwrap()
        .remove("ca_certificates_bundle_s3_object_version");
    assert!(complete(
        &input,
        "aws_lb_trust_store.app",
        "CA bundle object"
    ));
    assert!(!complete(
        &input,
        "aws_lb_trust_store.app",
        "CA bundle object version"
    ));
}

#[test]
fn certificate_associations_reject_wrong_identity_values_and_lifecycle() {
    for case in 0..10 {
        let mut input = fixture();
        match case {
            0 => input["resource_changes"][2]["change"]["after"]["arn"] = json!("OTHER"),
            1 => input["resource_changes"][2]["provider_name"] = json!("example.com/foreign/aws"),
            2 => input["resource_changes"][2]["mode"] = json!("data"),
            3 => input["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            4 => input["resource_changes"][0]["deposed"] = json!("old"),
            5 => input["resource_changes"][2]["change"]["after_sensitive"] = json!({"arn":true}),
            6 => {
                input["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
                    json!("aws.foreign")
            }
            7 => {
                input["configuration"]["root_module"]["resources"][3]["expressions"]["certificate_arn"] =
                    json!({"references":["aws_iam_server_certificate.app.id"]})
            }
            8 => {
                input["configuration"]["root_module"]["resources"][3]["expressions"]["listener_arn"] =
                    json!({"references":["aws_lb_listener.app.arn","var.unknown"]})
            }
            _ => {
                input["configuration"]["root_module"]["resources"][3]["depends_on"] =
                    json!(["aws_s3_bucket.app"])
            }
        }
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        assert!(
            !graph.edges.iter().any(|e| e.kind == EdgeKind::Association),
            "{case}"
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|n| n.address == "aws_lb_listener_certificate.app")
        );
    }
}

#[test]
fn indexed_certificates_and_named_states_do_not_mix_identities() {
    let mut input = fixture();
    input["resource_changes"][3]["change"]["after"]["certificate_arn"] = json!("PRIVATE_CERT");
    input["resource_changes"][1]["address"] = json!("aws_acm_certificate.app[0]");
    input["resource_changes"][1]["index"] = json!(0);
    let mut other = input["resource_changes"][1].clone();
    other["address"] = json!("aws_acm_certificate.app[1]");
    other["index"] = json!(1);
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(other);
    for (reference, valid) in [
        ("aws_acm_certificate.app[0].arn", true),
        ("aws_acm_certificate.app.arn", false),
    ] {
        input["configuration"]["root_module"]["resources"][3]["expressions"]["certificate_arn"] =
            json!({"references":[reference]});
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        assert_eq!(
            graph.edges.iter().any(|e| e.kind == EdgeKind::Association),
            valid
        );
    }
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: fixture().to_string(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for plan in plans.states.values() {
        let graph = semantic::transform(plan);
        assert_eq!(graph.coverage.selected.resources, 8);
        let selected = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some("aws_lb_listener.app".into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            !selected
                .edges
                .iter()
                .any(|e| e.kind == EdgeKind::Association)
        );
    }
}

#[test]
fn tls_reference_bindings_reject_foreign_accounts_and_regional_targets() {
    use crate::model::AwsAccountScope;
    let raw = plan::parse(&fixture().to_string()).unwrap();
    for source in [
        "aws_acm_certificate.app",
        "aws_iam_server_certificate.app",
        "aws_lb_trust_store.app",
        "aws_s3_object.app",
    ] {
        let mut raw = raw.clone();
        for node in &mut raw.nodes {
            node.provider_configuration.as_mut().unwrap().account = Some(
                AwsAccountScope::new(if node.address == source {
                    "222222222222"
                } else {
                    "111111111111"
                })
                .unwrap(),
            );
        }
        let graph = semantic::transform(&raw);
        assert!(
            !graph
                .nodes
                .iter()
                .flat_map(|n| n
                    .references
                    .iter()
                    .filter(|r| super::label(&n.resource_type, &r.attribute) == Some(r.label)))
                .filter(|r| r.complete && !r.dependency)
                .flat_map(|r| &r.targets)
                .any(|t| t.id.address == source)
        );
    }
    let mut input = fixture();
    input["configuration"]["provider_config"]["aws.other"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
    input["configuration"]["root_module"]["resources"][4]["provider_config_key"] =
        json!("aws.other");
    assert!(!complete(
        &input,
        "aws_lb_listener.app",
        "Verify-mode trust store"
    ));
}

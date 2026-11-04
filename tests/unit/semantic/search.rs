use crate::{plan, semantic};
use serde_json::{Value, json};

const DOMAINS: &str = include_str!("../../fixtures/search-domains-plan.json");
const NETWORK: &str = include_str!("../../fixtures/search-network-plan.json");
const APPLICATIONS: &str = include_str!("../../fixtures/search-applications-plan.json");

#[test]
fn named_states_and_late_account_evidence_keep_lookup_and_association_identity() {
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: APPLICATIONS.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    let mut parsed = plan::parse(APPLICATIONS).unwrap();
    parsed
        .nodes
        .iter_mut()
        .find(|n| n.address == "data.aws_ssoadmin_instances.main")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    let raw = semantic::Input::new(&parsed);
    let r = raw
        .attributes
        .iter()
        .find(|r| r.attribute == super::INSTANCE)
        .unwrap();
    assert!(!super::super::service_references::verified(&raw, r));
    let mut v = fixture(APPLICATIONS);
    let t = index(&v, "aws_opensearch_package_association.main");
    let graph = semantic::transform(&plan::parse(APPLICATIONS).unwrap());
    assert_eq!(graph.coverage.selected.resources, 10);
    assert!(
        !graph
            .nodes
            .iter()
            .any(|n| n.resource_type == "aws_opensearch_package_association")
    );
    v["configuration"]["root_module"]["resources"][t]["expressions"]["domain_name"] =
        json!({"constant_value":"main"});
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 10);
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.resource_type == "aws_opensearch_package_association")
    );
}

fn fixture(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}
fn index(v: &Value, address: &str) -> usize {
    v["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["address"] == address)
        .unwrap()
}
fn verified(v: &Value, address: &str, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == address
            && r.attribute == path
            && super::super::service_references::verified(&raw, r)
    })
}
fn field<'a>(mut v: &'a mut Value, path: &str) -> &'a mut Value {
    for key in path.split('.') {
        if v.is_array() {
            v = &mut v[0];
        }
        v = &mut v[key];
    }
    v
}

#[test]
fn configured_search_references_preserve_exact_service_types_and_output_contracts() {
    for input in [DOMAINS, NETWORK, APPLICATIONS] {
        let p = plan::parse(input).unwrap();
        let raw = semantic::Input::new(&p);
        let refs: Vec<_> = raw
            .attributes
            .iter()
            .filter(|r| super::label(&raw.nodes[r.target].resource_type, &r.attribute).is_some())
            .collect();
        assert!(refs.len() >= 8);
        for r in refs {
            assert!(
                super::super::service_references::verified(&raw, r),
                "{} {}",
                raw.nodes[r.target].address,
                r.attribute
            );
        }
    }
}

#[test]
fn unknown_incomplete_ambiguous_lifecycle_and_scope_evidence_cannot_verify() {
    for (input, target, path, source, output) in [
        (
            DOMAINS,
            "aws_opensearch_domain.main",
            "cognito_options.role_arn",
            "aws_iam_role.search",
            "arn",
        ),
        (
            DOMAINS,
            "aws_elasticsearch_domain.main",
            "encrypt_at_rest.kms_key_id",
            "aws_kms_key.search",
            "key_id",
        ),
        (
            DOMAINS,
            "aws_cloudsearch_domain_service_access_policy.main",
            "domain_name",
            "aws_cloudsearch_domain.main",
            "name",
        ),
        (
            DOMAINS,
            "aws_opensearch_domain.main",
            "log_publishing_options.cloudwatch_log_group_arn",
            "aws_cloudwatch_log_group.search0",
            "arn",
        ),
        (
            NETWORK,
            "aws_opensearch_vpc_endpoint.main",
            "domain_arn",
            "aws_opensearch_domain.main",
            "arn",
        ),
        (
            NETWORK,
            "aws_elasticsearch_domain.main",
            super::SUBNETS,
            "aws_subnet.client",
            "id",
        ),
        (
            APPLICATIONS,
            "aws_opensearch_application.main",
            "data_source.data_source_arn",
            "aws_opensearch_domain.main",
            "arn",
        ),
        (
            APPLICATIONS,
            "aws_opensearch_package.main",
            "package_source.s3_bucket_name",
            "aws_s3_bucket.packages",
            "bucket",
        ),
    ] {
        for case in [
            "literal",
            "partial",
            "ambiguous",
            "unknown",
            "source-unknown",
            "mismatch",
            "delete",
            "deposed",
            "data",
            "scope",
        ] {
            let mut v = fixture(input);
            let t = index(&v, target);
            let s = index(&v, source);
            match case {
                "literal" => {
                    let first = path.split('.').next().unwrap();
                    v["configuration"]["root_module"]["resources"][t]["expressions"]
                        .as_object_mut()
                        .unwrap()
                        .remove(first);
                }
                "partial" | "ambiguous" => field(
                    &mut v["configuration"]["root_module"]["resources"][t]["expressions"],
                    path,
                )["references"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!(if case == "partial" {
                        "var.other"
                    } else {
                        "aws_opensearch_domain.main.domain_name"
                    })),
                "unknown" => {
                    v["resource_changes"][t]["change"]["after_unknown"] =
                        json!({path.split('.').next().unwrap():true})
                }
                "source-unknown" => {
                    v["resource_changes"][s]["change"]["after_unknown"] = json!({output:true})
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][output] = json!("wrong"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                _ => {
                    v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                        json!("unknown")
                }
            }
            assert!(!verified(&v, target, path), "{target} {path} {case}");
        }
    }
}

#[test]
fn enabled_blocks_complete_masks_and_kms_normalization_are_required() {
    for (path, gate) in [
        ("cognito_options.user_pool_id", "cognito_options.enabled"),
        (
            "domain_endpoint_options.custom_endpoint_certificate_arn",
            "domain_endpoint_options.custom_endpoint_enabled",
        ),
        ("encrypt_at_rest.kms_key_id", "encrypt_at_rest.enabled"),
        (
            "log_publishing_options.cloudwatch_log_group_arn",
            "log_publishing_options.enabled",
        ),
    ] {
        let mut v = fixture(DOMAINS);
        let t = index(&v, "aws_opensearch_domain.main");
        *field(&mut v["resource_changes"][t]["change"]["after"], gate) = json!(false);
        assert!(!verified(&v, "aws_opensearch_domain.main", path));
    }
    let mut v = fixture(DOMAINS);
    let t = index(&v, "aws_opensearch_domain.main");
    v["resource_changes"][t]["change"]["after_unknown"] = json!({"log_publishing_options":[{}]});
    assert!(!verified(
        &v,
        "aws_opensearch_domain.main",
        "log_publishing_options.cloudwatch_log_group_arn"
    ));
    let mut v = fixture(DOMAINS);
    let t = index(&v, "aws_elasticsearch_domain.main");
    let k = index(&v, "aws_kms_key.search");
    *field(
        &mut v["resource_changes"][t]["change"]["after"],
        "encrypt_at_rest.kms_key_id",
    ) = v["resource_changes"][k]["change"]["after"]["key_id"].clone();
    assert!(verified(
        &v,
        "aws_elasticsearch_domain.main",
        "encrypt_at_rest.kms_key_id"
    ));
    let mut v = fixture(NETWORK);
    let t = index(&v, "aws_elasticsearch_vpc_endpoint.main");
    v["configuration"]["root_module"]["resources"][t]["expressions"]["domain_arn"] =
        json!({"references":["aws_opensearch_domain.main.arn"]});
    assert!(!verified(
        &v,
        "aws_elasticsearch_vpc_endpoint.main",
        "domain_arn"
    ));
}

#[test]
fn network_uses_complete_local_subnets_and_groups_without_domain_inheritance() {
    let v = fixture(NETWORK);
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let parent = graph
        .nodes
        .iter()
        .position(|n| n.address == "aws_vpc.client")
        .unwrap();
    for address in [
        "aws_elasticsearch_domain.main",
        "aws_opensearch_vpc_endpoint.main",
        "aws_elasticsearch_vpc_endpoint.main",
    ] {
        let target = graph
            .nodes
            .iter()
            .position(|n| n.address == address)
            .unwrap();
        assert!(graph.edges.iter().any(|e| e.from == parent
            && e.to == target
            && e.kind == crate::model::EdgeKind::Containment));
    }
    let domain = graph
        .nodes
        .iter()
        .position(|n| n.address == "aws_opensearch_domain.main")
        .unwrap();
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.to == domain && e.kind == crate::model::EdgeKind::Containment)
    );
    let mut v = v;
    let group = index(&v, "aws_security_group.client");
    v["resource_changes"][group]["change"]["after"]["vpc_id"] = json!("vpc-other");
    for path in [super::SUBNETS, super::GROUPS] {
        assert!(!verified(&v, "aws_opensearch_vpc_endpoint.main", path));
    }
}

#[test]
fn lookup_identity_is_singleton_data_and_helpers_preserve_filtered_coverage() {
    let mut v = fixture(APPLICATIONS);
    let s = index(&v, "data.aws_ssoadmin_instances.main");
    assert!(verified(
        &v,
        "aws_opensearch_application.main",
        super::INSTANCE
    ));
    v["resource_changes"][s]["change"]["after"]["arns"]
        .as_array_mut()
        .unwrap()
        .push(json!("arn:aws:sso:::instance/ssoins-2222222222222222"));
    assert!(!verified(
        &v,
        "aws_opensearch_application.main",
        super::INSTANCE
    ));
    for input in [DOMAINS, APPLICATIONS] {
        let graph = semantic::transform(&plan::parse(input).unwrap());
        for n in graph
            .nodes
            .iter()
            .filter(|n| super::helper(&n.resource_type))
        {
            assert!(crate::view::routes::owner(&graph, n).is_some());
            let selected = crate::view::single(
                &graph,
                &crate::view::Options {
                    focus: Some(n.address.clone()),
                    depth: Some(0),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(selected.coverage.selected.resources, 1);
            assert!(crate::view::routes::owner(&selected, &selected.nodes[0]).is_none());
        }
        for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
            let out = crate::generate(
                input,
                &crate::Options {
                    format,
                    ..Default::default()
                },
            )
            .unwrap()
            .content;
            assert!(!out.contains("PRIVATE_"));
        }
    }
}

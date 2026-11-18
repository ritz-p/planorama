use crate::plan;
use proptest::{prelude::*, test_runner::RngSeed};
use serde_json::{Value, json};

fn config() -> ProptestConfig {
    let mut config = ProptestConfig::default();
    if std::env::var_os("PROPTEST_CASES").is_none() {
        config.cases = 24;
    }
    if config.rng_seed == RngSeed::Random {
        config.rng_seed = RngSeed::Fixed(1372);
    }
    config.failure_persistence = None;
    config
}

fn rotate(input: &mut Value, offset: usize) {
    for path in ["/resource_changes", "/configuration/root_module/resources"] {
        let resources = input.pointer_mut(path).unwrap().as_array_mut().unwrap();
        let shift = offset % resources.len();
        resources.rotate_left(shift);
    }
}

fn check_keyed_reference_selects_only_its_instance(
    key: String,
    offset: usize,
) -> Result<(), TestCaseError> {
    let quoted = serde_json::to_string(&key).unwrap();
    let selected = format!("terraform_data.producer[{quoted}]");
    let other_key = format!("{key}-other");
    let keys = [key.clone(), other_key.clone()];
    let mut changes = Vec::new();
    for (name, index) in [
        ("producer", Some(&key)),
        ("producer", Some(&other_key)),
        ("consumer", None),
        ("unrelated", None),
    ] {
        let address = match index {
            Some(k) => format!(
                "terraform_data.{name}[{}]",
                serde_json::to_string(k).unwrap()
            ),
            None => format!("terraform_data.{name}"),
        };
        let mut change = json!({"address":address,"mode":"managed","type":"terraform_data","name":name,
                "provider_name":"terraform.io/builtin/terraform","change":{"actions":["create"],"before":null,
                "after":{"input":key},"after_unknown":{"id":true,"output":true},"after_sensitive":{}}});
        if let Some(k) = index {
            change["index"] = json!(k);
        }
        changes.push(change);
    }
    let mut input = json!({"format_version":"1.2","resource_changes":changes,"configuration":{"root_module":{"resources":[
        {"address":"terraform_data.producer","mode":"managed","type":"terraform_data","name":"producer","provider_config_key":"terraform",
         "for_each_expression":{"constant_value":keys},"expressions":{"input":{"constant_value":key}}},
        {"address":"terraform_data.consumer","mode":"managed","type":"terraform_data","name":"consumer","provider_config_key":"terraform",
         "expressions":{"input":{"references":[format!("{selected}.input"),"terraform_data.producer"]}}},
        {"address":"terraform_data.unrelated","mode":"managed","type":"terraform_data","name":"unrelated","provider_config_key":"terraform",
         "expressions":{"input":{"constant_value":key}}}
    ]}}});
    rotate(&mut input, offset);
    let raw = plan::parse(&input.to_string()).unwrap();
    let consumer = raw
        .nodes
        .iter()
        .position(|n| n.address == "terraform_data.consumer")
        .unwrap();
    let field = raw
        .attributes
        .iter()
        .find(|r| r.target == consumer && r.attribute == "input")
        .unwrap();
    prop_assert!(field.complete && field.instances_explicit);
    let sources: Vec<_> = field
        .sources
        .iter()
        .map(|&i| raw.nodes[i].address.as_str())
        .collect();
    prop_assert_eq!(sources, [selected.as_str()]);
    let edges: Vec<_> = raw
        .edges
        .iter()
        .filter(|e| e.to == consumer)
        .map(|e| raw.nodes[e.from].address.as_str())
        .collect();
    prop_assert_eq!(edges, [selected.as_str()]);
    Ok(())
}

fn check_aws_certificate_proof_requires_compatible_scope(
    offset: usize,
    region: &str,
) -> Result<(), TestCaseError> {
    let mut input: Value = serde_json::from_str(include_str!("../fixtures/acm-plan.json")).unwrap();
    rotate(&mut input, offset);
    for same_scope in [true, false] {
        if !same_scope {
            let mut alias = input["configuration"]["provider_config"]["aws"].clone();
            alias["alias"] = json!("remote");
            alias["expressions"]["region"]["constant_value"] = json!(region);
            input["configuration"]["provider_config"]["aws.remote"] = alias;
            let listener = input["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == "aws_lb_listener.https")
                .unwrap();
            listener["provider_config_key"] = json!("aws.remote");
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        let reference = raw
            .attributes
            .iter()
            .find(|r| {
                r.attribute == "certificate_arn"
                    && raw.nodes[r.target].address == "aws_lb_listener.https"
            })
            .unwrap();
        prop_assert_eq!(reference.sources.len(), 1);
        prop_assert_eq!(
            &raw.nodes[reference.sources[0]].address,
            "aws_acm_certificate_validation.site"
        );
        prop_assert_eq!(reference.service_reference_verified, same_scope);
    }
    Ok(())
}

fn check_imported_worker_identity_requires_unique_visible_name(
    name: String,
    offset: usize,
) -> Result<(), TestCaseError> {
    let mut base: Value =
        serde_json::from_str(include_str!("../fixtures/cloudflare-imported-plan.json")).unwrap();
    rotate(&mut base, offset);
    for (duplicate, masked, unknown) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
    ] {
        let mut input = base.clone();
        let changes = input["resource_changes"].as_array_mut().unwrap();
        let worker = changes
            .iter_mut()
            .find(|n| n["type"] == "cloudflare_worker")
            .unwrap();
        worker["change"]["after"]["name"] = json!(name);
        let mut other = worker.clone();
        other["address"] = json!("cloudflare_worker.other");
        other["name"] = json!("other");
        other["change"]["after"]["id"] = json!("20000000-0000-4000-8000-000000000002");
        if !duplicate {
            other["change"]["after"]["name"] = json!(format!("{name}-other"));
        }
        if masked {
            worker["change"]["after_sensitive"]["name"] = json!(true);
        }
        if unknown {
            worker["change"]["after"]
                .as_object_mut()
                .unwrap()
                .remove("name");
            worker["change"]["after_unknown"]["name"] = json!(true);
        }
        let route = changes
            .iter_mut()
            .find(|n| n["type"] == "cloudflare_workers_route")
            .unwrap();
        route["change"]["after"]["script"] = json!(name);
        changes.push(other);
        input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(json!({"address":"cloudflare_worker.other","provider_config_key":"cloudflare"}));
        let raw = plan::parse_with_value_matches(&input.to_string(), true).unwrap();
        let recovered: Vec<_> = raw
            .configured_value_matches
            .iter()
            .filter(|m| m.attribute == "script")
            .map(|m| {
                (
                    raw.nodes[m.consumer].address.as_str(),
                    raw.nodes[m.source].address.as_str(),
                )
            })
            .collect();
        if duplicate || masked || unknown {
            prop_assert!(recovered.is_empty());
        } else {
            prop_assert_eq!(
                recovered,
                [("cloudflare_workers_route.api", "cloudflare_worker.api")]
            );
        }
        prop_assert!(!raw.edges.iter().any(|e| raw.nodes[e.to].address
            == "cloudflare_workers_route.api"
            && raw.nodes[e.from].resource_type == "cloudflare_worker"));
    }
    Ok(())
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn keyed_reference_selects_only_its_instance(key in "[a-z0-9.]{1,8}", offset in 0usize..6) {
        check_keyed_reference_selects_only_its_instance(key, offset)?;
    }

    #[test]
    fn aws_certificate_proof_requires_compatible_scope(offset in 0usize..8, region in prop::sample::select(vec!["us-west-1","eu-west-1","ap-south-1"])) {
        check_aws_certificate_proof_requires_compatible_scope(offset, region)?;
    }

    #[test]
    fn imported_worker_identity_requires_unique_visible_name(name in "w[a-z]{1,6}", offset in 0usize..8) {
        check_imported_worker_identity_requires_unique_visible_name(name, offset)?;
    }
}

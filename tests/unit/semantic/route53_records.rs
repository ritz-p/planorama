use crate::{plan, semantic};
use serde_json::{Value, json};

fn add(v: &mut Value, kind: &str, name: &str, values: Value, expressions: Value) {
    let address = format!("{kind}.{name}");
    v["resource_changes"].as_array_mut().unwrap().push(
        json!({"address":address,"type":kind,"change":{"actions":["create"],"after":values}}),
    );
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":address,"provider_config_key":"aws","expressions":expressions}));
}

fn fixture() -> Value {
    let mut v = json!({"format_version":"1.2","resource_changes":[],"configuration":{"provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[]}}});
    add(
        &mut v,
        "aws_route53_zone",
        "zone",
        json!({"zone_id":"zone"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_route53_health_check",
        "health",
        json!({"id":"health"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_route53_cidr_collection",
        "collection",
        json!({"id":"collection"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_route53_cidr_location",
        "location",
        json!({"cidr_collection_id":"collection","name":"location"}),
        json!({"cidr_collection_id":{"references":["aws_route53_cidr_collection.collection.id"]}}),
    );
    add(
        &mut v,
        "aws_route53_traffic_policy",
        "policy",
        json!({"id":"policy","version":2}),
        json!({}),
    );
    add(
        &mut v,
        "aws_route53_traffic_policy_instance",
        "instance",
        json!({"id":"instance","hosted_zone_id":"zone","traffic_policy_id":"policy","traffic_policy_version":2}),
        json!({"hosted_zone_id":{"references":["aws_route53_zone.zone.zone_id"]},"traffic_policy_id":{"references":["aws_route53_traffic_policy.policy.id"]}}),
    );
    add(
        &mut v,
        "aws_elb",
        "classic",
        json!({"dns_name":"classic.invalid","zone_id":"classic-zone"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_globalaccelerator_accelerator",
        "accelerator",
        json!({"dns_name":"accelerator.invalid","hosted_zone_id":"accelerator-zone"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_route53_record",
        "record",
        json!({"zone_id":"zone","health_check_id":"health","cidr_routing_policy":[{"collection_id":"collection","location_name":"location"}],"alias":[{"name":"classic.invalid","zone_id":"classic-zone"}]}),
        json!({"zone_id":{"references":["aws_route53_zone.zone.zone_id"]},"health_check_id":{"references":["aws_route53_health_check.health.id"]},"cidr_routing_policy":[{"collection_id":{"references":["aws_route53_cidr_collection.collection.id"]},"location_name":{"references":["aws_route53_cidr_location.location.name"]}}],"alias":[{"name":{"references":["aws_elb.classic.dns_name"]},"zone_id":{"references":["aws_elb.classic.zone_id"]}}]}),
    );
    add(
        &mut v,
        "aws_route53_records_exclusive",
        "exclusive",
        json!({"zone_id":"zone","resource_record_set":[{"name":"b.invalid","type":"A","traffic_policy_instance_id":"instance","alias_target":[{"dns_name":"accelerator.invalid","hosted_zone_id":"accelerator-zone"}]},{"name":"a.invalid","type":"A","health_check_id":"health","cidr_routing_config":[{"collection_id":"collection","location_name":"location"}],"alias_target":[{"dns_name":"classic.invalid","hosted_zone_id":"classic-zone"}]}]}),
        json!({"zone_id":{"references":["aws_route53_zone.zone.zone_id"]},"resource_record_set":[{"health_check_id":{"references":["aws_route53_health_check.health.id"]},"cidr_routing_config":[{"collection_id":{"references":["aws_route53_cidr_collection.collection.id"]},"location_name":{"references":["aws_route53_cidr_location.location.name"]}}],"alias_target":[{"dns_name":{"references":["aws_elb.classic.dns_name"]},"hosted_zone_id":{"references":["aws_elb.classic.zone_id"]}}]},{"traffic_policy_instance_id":{"references":["aws_route53_traffic_policy_instance.instance.id"]},"alias_target":[{"dns_name":{"references":["aws_globalaccelerator_accelerator.accelerator.dns_name"]},"hosted_zone_id":{"references":["aws_globalaccelerator_accelerator.accelerator.hosted_zone_id"]}}]}]}),
    );
    v
}

fn proven(v: &Value, kind: &str, field: &str, label: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .any(|n| {
            n.resource_type == kind
                && n.references
                    .iter()
                    .any(|r| r.attribute == field && r.label == label && r.complete)
        })
}

#[test]
fn route53_record_references_and_exclusive_sets_preserve_distinct_identities() {
    let v = fixture();
    for (kind, path, label) in [
        ("aws_route53_record", "alias.name", "Alias target"),
        (
            "aws_route53_record",
            "health_check_id",
            "Health check reference",
        ),
        (
            "aws_route53_record",
            "cidr_routing_policy.collection_id",
            "CIDR collection",
        ),
        (
            "aws_route53_record",
            "cidr_routing_policy.location_name",
            "CIDR location",
        ),
        (
            "aws_route53_cidr_location",
            "cidr_collection_id",
            "CIDR collection membership",
        ),
        (
            "aws_route53_traffic_policy_instance",
            "traffic_policy_id",
            "Traffic policy version",
        ),
        (
            "aws_route53_traffic_policy_instance",
            "hosted_zone_id",
            "Hosted zone",
        ),
        (
            "aws_route53_records_exclusive",
            "resource_record_set.alias_target.dns_name",
            "Alias target",
        ),
        (
            "aws_route53_records_exclusive",
            "resource_record_set.health_check_id",
            "Health check reference",
        ),
        (
            "aws_route53_records_exclusive",
            "resource_record_set.cidr_routing_config.location_name",
            "CIDR location",
        ),
        (
            "aws_route53_records_exclusive",
            "resource_record_set.traffic_policy_instance_id",
            "Traffic policy instance",
        ),
    ] {
        assert!(proven(&v, kind, path, label), "{kind} {path}");
    }
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(g.coverage.selected.resources, 10);
    assert!(crate::svg::render(&g, &crate::layout::Layout::new(&g)).contains("Record management"));
    assert!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_route53_records_exclusive")
            .unwrap()
            .member_owner
            .is_some()
    );
    assert_eq!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type == "aws_route53_record")
            .count(),
        1
    );
    let mut ga = v.clone();
    ga["resource_changes"][8]["change"]["after"]["alias"] =
        json!([{"name":"accelerator.invalid","zone_id":"accelerator-zone"}]);
    ga["configuration"]["root_module"]["resources"][8]["expressions"]["alias"] = json!([{"name":{"references":["aws_globalaccelerator_accelerator.accelerator.dns_name"]},"zone_id":{"references":["aws_globalaccelerator_accelerator.accelerator.hosted_zone_id"]}}]);
    assert!(proven(
        &ga,
        "aws_route53_record",
        "alias.name",
        "Alias target"
    ));
}

#[test]
fn route53_exclusive_nested_proofs_never_cross_set_elements() {
    for case in 0..13 {
        let mut v = fixture();
        match case {
            0 => {
                v["resource_changes"][9]["change"]["after"]["resource_record_set"][0]["alias_target"]
                    [0]["hosted_zone_id"] = json!("classic-zone")
            }
            1 => {
                v["resource_changes"][9]["change"]["after_unknown"] =
                    json!({"resource_record_set":[{}, {"health_check_id":true}]})
            }
            2 => {
                v["resource_changes"][9]["change"]["after_sensitive"] =
                    json!({"resource_record_set":[{"alias_target":[{"dns_name":true}]},{}]})
            }
            3 => v["resource_changes"][7]["change"]["after_sensitive"] = json!({"dns_name":true}),
            4 => v["resource_changes"][7]["change"]["actions"] = json!(["delete"]),
            5 => v["resource_changes"][7]["provider_name"] = json!("example.com/foreign/aws"),
            6 => {
                v["configuration"]["root_module"]["resources"][9]["expressions"]["resource_record_set"]
                    [1]["alias_target"][0]["dns_name"] = json!({"references":["aws_globalaccelerator_accelerator.accelerator.dns_name","var.dynamic"]})
            }
            7 => {
                v["resource_changes"][9]["change"]["after"]["resource_record_set"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            8 => {
                v["configuration"]["root_module"]["resources"][9]["expressions"]["resource_record_set"]
                    [0]["health_check_id"] = json!({"constant_value":"health"})
            }
            9 => {
                v["resource_changes"][3]["change"]["after"]["cidr_collection_id"] =
                    json!("other-collection")
            }
            10 => v["resource_changes"][7]["deposed"] = json!("old"),
            11 => {
                v["configuration"]["root_module"]["resources"][7]["provider_config_key"] =
                    json!("aws.other")
            }
            _ => {
                v["resource_changes"][5]["change"]["after"]["hosted_zone_id"] = json!("other-zone")
            }
        }
        assert!(
            !proven(
                &v,
                "aws_route53_records_exclusive",
                "resource_record_set.alias_target.dns_name",
                "Alias target"
            ),
            "case {case}"
        );
    }
}

#[test]
fn route53_routing_helpers_require_qualified_ids_and_known_versions() {
    for case in 0..7 {
        let mut v = fixture();
        match case {
            0 => v["resource_changes"][5]["change"]["after"]["traffic_policy_version"] = json!(1),
            1 => v["resource_changes"][4]["change"]["after_sensitive"] = json!({"version":true}),
            2 => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"traffic_policy_version":true})
            }
            3 => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["traffic_policy_id"] =
                    json!({"references":["aws_route53_traffic_policy.policy.arn"]})
            }
            4 => v["resource_changes"][5]["change"]["after"]["traffic_policy_version"] = json!("2"),
            5 => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["traffic_policy_version"] =
                    json!({"references":["aws_route53_traffic_policy.policy.name"]})
            }
            _ => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["traffic_policy_version"] =
                    json!({"references":["var.version"]})
            }
        }
        assert!(
            !proven(
                &v,
                "aws_route53_traffic_policy_instance",
                "traffic_policy_id",
                "Traffic policy version"
            ),
            "case {case}"
        );
    }
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][8]["expressions"]["cidr_routing_policy"][0]["location_name"] =
        json!({"references":["aws_route53_cidr_location.location.id"]});
    assert!(!proven(
        &v,
        "aws_route53_record",
        "cidr_routing_policy.location_name",
        "CIDR location"
    ));
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][8]["expressions"]["alias"][0]["zone_id"] =
        json!({"references":["aws_globalaccelerator_accelerator.accelerator.hosted_zone_id"]});
    assert!(!proven(
        &v,
        "aws_route53_record",
        "alias.name",
        "Alias target"
    ));
}

#[test]
fn route53_exclusive_indexed_lists_states_and_filtering_keep_provenance() {
    let mut v = fixture();
    let values =
        v["resource_changes"][9]["change"]["after"]["resource_record_set"][1]["alias_target"][0]
            .clone();
    let expressions =
        v["configuration"]["root_module"]["resources"][9]["expressions"]["resource_record_set"][0]
            ["alias_target"][0]
            .clone();
    v["resource_changes"][9]["change"]["after"]["resource_record_set"][1]["alias_target"]
        .as_array_mut()
        .unwrap()
        .push(values);
    v["configuration"]["root_module"]["resources"][9]["expressions"]["resource_record_set"][0]["alias_target"].as_array_mut().unwrap().push(expressions);
    let mut indexed: Value = serde_json::from_str(
        &v.to_string()
            .replace("aws_elb.classic", "aws_elb.classic[0]"),
    )
    .unwrap();
    indexed["configuration"]["root_module"]["resources"][6]["address"] = json!("aws_elb.classic");
    assert!(proven(
        &indexed,
        "aws_route53_records_exclusive",
        "resource_record_set.alias_target.dns_name",
        "Alias target"
    ));
    let graph = semantic::transform(&plan::parse(&indexed.to_string()).unwrap());
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_route53_record.record".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
    );
    let mut missing = indexed.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(6);
    let states = plan::parse_inputs(
        [("one", indexed), ("two", missing)]
            .into_iter()
            .map(|(name, v)| crate::model::PlanInput {
                state_id: crate::model::StateId::new(name).unwrap(),
                plan_json: v.to_string(),
            })
            .collect(),
    )
    .unwrap();
    for (state, parsed) in states.states {
        let g = semantic::transform(&parsed);
        let verified = g.nodes.iter().any(|n| {
            n.resource_type == "aws_route53_records_exclusive"
                && n.references
                    .iter()
                    .any(|r| r.label == "Alias target" && r.complete)
        });
        assert_eq!(verified, state.as_str() == "one");
    }
}

#[test]
fn route53_exclusive_account_checks_cover_every_nested_reference() {
    let mut v = fixture();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
    v["configuration"]["provider_config"]["aws.other"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["root_module"]["resources"][1]["provider_config_key"] = json!("aws.other");
    let verified = |v: &Value| {
        let p = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        let graph = semantic::transform(&p);
        assert!(
            graph
                .nodes
                .iter()
                .find(|n| n.resource_type == "aws_route53_records_exclusive")
                .unwrap()
                .member_owner
                .is_some()
        );
        graph.nodes.iter().any(|n| {
            n.resource_type == "aws_route53_records_exclusive"
                && n.references
                    .iter()
                    .any(|r| r.label == "Alias target" && r.complete)
        })
    };
    assert!(verified(&v));
    v["configuration"]["provider_config"]["aws.other"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::222222222222:role/Test"}}]);
    assert!(!verified(&v));
}

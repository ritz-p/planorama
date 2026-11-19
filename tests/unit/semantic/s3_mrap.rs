use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/s3-mrap-plan.json")).unwrap()
}

fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    let mut raw = crate::plan::parse(&v.to_string()).unwrap();
    for n in &mut raw.nodes {
        if let Some(c) = &mut n.provider_configuration {
            c.account = crate::model::AwsAccountScope::new(if c.key == "aws.foreign" {
                "222222222222"
            } else {
                "111111111111"
            });
        }
    }
    let evidence = raw
        .nodes
        .iter()
        .map(|n| {
            let change = v["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| {
                    r["address"] == n.address
                        && r.get("deposed").and_then(Value::as_str) == n.deposed_key.as_deref()
                })
                .unwrap();
            let c = &change["change"];
            (
                &c["after"],
                if c["after_sensitive"].is_null() {
                    &c["after_unknown"]
                } else {
                    &c["after_sensitive"]
                },
            )
        })
        .collect::<Vec<_>>();
    super::annotate(&raw.nodes, &mut raw.attributes, &evidence);
    crate::semantic::transform(&raw)
}

fn proven(g: &crate::model::ArchitectureGraph, path: &str) -> bool {
    g.nodes.iter().any(|n| {
        n.references.iter().any(|r| {
            r.attribute == path
                && Some(r.label) == super::label(&n.resource_type, path)
                && r.complete
                && !r.targets.is_empty()
                && !r.dependency
        })
    })
}

#[test]
fn json_provider_role_configuration_does_not_prove_account_identity() {
    let v = fixture();
    let g = crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap());
    for path in [
        "details.region.bucket",
        "details.name",
        "mrap",
        "route.bucket",
    ] {
        assert!(!proven(&g, path), "{path}");
    }
}

#[test]
fn complete_mrap_references_preserve_logical_independent_resources() {
    let g = graph(&fixture());
    for path in [
        "details.region.bucket",
        "details.name",
        "mrap",
        "route.bucket",
    ] {
        assert!(proven(&g, path), "{path}");
    }
    for n in &g.nodes {
        if n.resource_type
            .starts_with("aws_s3control_multi_region_access_point")
        {
            assert_eq!(n.placement, crate::model::Placement::Logical);
        }
    }
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
}

#[test]
fn collections_match_values_independently_of_set_order_and_ignore_computed_region() {
    let mut v = fixture();
    v["resource_changes"][2]["change"]["after"]["details"][0]["region"]
        .as_array_mut()
        .unwrap()
        .reverse();
    v["resource_changes"][2]["change"]["after_unknown"] =
        json!({"details":[{"region":[{"region":true},{"region":true}]}]});
    assert!(proven(&graph(&v), "details.region.bucket"));
}

#[test]
fn mrap_collections_fail_closed_without_complete_typed_current_evidence() {
    for case in 0..17 {
        let mut v = fixture();
        match case {
            0 => v["resource_changes"][0]["change"]["after"]["bucket"]=json!("different"),
            1 => v["resource_changes"][0]["change"]["after_unknown"]=json!({"bucket":true}),
            2 => v["resource_changes"][0]["change"]["after_sensitive"]=json!({"bucket":true}),
            3 => v["resource_changes"][0]["mode"]=json!("data"),
            4 => v["resource_changes"][0]["deposed"]=json!("old"),
            5 => v["resource_changes"][0]["change"]["actions"]=json!(["delete"]),
            6 => v["resource_changes"][0]["type"]=json!("aws_sqs_queue"),
            7 => v["configuration"]["root_module"]["resources"][2]["expressions"]["details"][0]["region"].as_array_mut().unwrap().pop().map(|_|()).unwrap(),
            8 => v["resource_changes"][2]["change"]["after"]["details"][0]["region"][1]["bucket"]=json!("east-bucket"),
            9 => v["resource_changes"][2]["change"]["after_unknown"]=json!({"details":true}),
            10 => v["resource_changes"][2]["change"]["after_sensitive"]=json!({"details":[{"region":[false,true]}]}),
            11 => v["resource_changes"][2]["change"]["after"]["details"][0]["region"][0]["bucket_account_id"]=json!("222222222222"),
            12 => v["resource_changes"][2]["change"]["after_unknown"]=json!({"details":[{"region":[{"bucket_account_id":true},false]}]}),
            13 => v["configuration"]["root_module"]["resources"][0]["provider_config_key"]=json!("aws.foreign"),
            14 => v["resource_changes"][0]["provider_name"]=json!("registry.terraform.io/hashicorp/google"),
            15 => v["configuration"]["root_module"]["resources"][2]["expressions"]=json!({}),
            _ => v["resource_changes"][2]["change"]["after_unknown"]=json!({"details":[{"region":[false]}]}),
        }
        assert!(!proven(&graph(&v), "details.region.bucket"), "case {case}");
    }
}

#[test]
fn policy_account_and_route_qualifiers_are_required_without_traffic_inference() {
    for case in 0..8 {
        let mut v = fixture();
        match case {
            0 => v["resource_changes"][3]["change"]["after"]["account_id"] = json!("222222222222"),
            1 => v["resource_changes"][3]["change"]["after_unknown"] = json!({"account_id":true}),
            2 => v["resource_changes"][2]["change"]["after"]["name"] = json!("other"),
            3 => v["resource_changes"][3]["change"]["after"]["details"][0]["name"] = json!("other"),
            4 => {
                v["resource_changes"][4]["change"]["after"]["route"][0]["region"] =
                    json!("eu-west-1")
            }
            5 => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"route":[{"region":true},false]})
            }
            6 => {
                v["resource_changes"][4]["change"]["after"]["route"][0]["traffic_dial_percentage"] =
                    json!(101)
            }
            _ => v["resource_changes"][4]["change"]["after"]["mrap"] = json!("different"),
        }
        let path = if case < 4 {
            "details.name"
        } else if case < 7 {
            "route.bucket"
        } else {
            "mrap"
        };
        assert!(!proven(&graph(&v), path), "case {case}");
    }
}

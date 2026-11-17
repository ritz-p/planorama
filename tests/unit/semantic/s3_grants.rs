use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/s3-grants-plan.json")).unwrap()
}

fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap())
}

fn proven(v: &Value, path: &str) -> bool {
    graph(v).nodes.iter().any(|n| {
        n.references.iter().any(|r| {
            r.attribute == path
                && Some(r.label) == super::label(&n.resource_type, path)
                && r.complete
                && !r.dependency
                && r.targets.len() == 1
        })
    })
}

#[test]
fn typed_location_role_user_and_role_grantees_preserve_independent_identity() {
    let mut v = fixture();
    for path in [
        "access_grants_location_id",
        "iam_role_arn",
        "grantee.grantee_identifier",
    ] {
        assert!(proven(&v, path), "{path}");
    }
    v["resource_changes"][3]["change"]["after"]["grantee"][0]["grantee_identifier"] =
        json!("arn:aws:iam::111111111111:role/location");
    v["configuration"]["root_module"]["resources"][3]["expressions"]["grantee"][0]["grantee_identifier"] =
        json!({"references":["aws_iam_role.location.arn"]});
    assert!(proven(&v, "grantee.grantee_identifier"));
    let g = graph(&v);
    assert_eq!(g.nodes.len(), 4);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    assert_eq!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type.starts_with("aws_s3control_access_grant"))
            .count(),
        2
    );
}

#[test]
fn proven_accounts_allow_matching_qualifiers_and_preserve_global_iam_scope() {
    let mut v = fixture();
    for i in [2, 3] {
        v["resource_changes"][i]["change"]["after"]["account_id"] = json!("111111111111");
    }
    let mut raw = crate::plan::parse(&v.to_string()).unwrap();
    for n in &mut raw.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("111111111111");
    }
    let evidence = raw
        .nodes
        .iter()
        .map(|n| {
            let c = &v["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["address"] == n.address)
                .unwrap()["change"];
            (&c["after"], &c["after_unknown"])
        })
        .collect::<Vec<_>>();
    super::annotate(&raw.nodes, &mut raw.attributes, &evidence);
    let g = crate::semantic::transform(&raw);
    for path in [
        "access_grants_location_id",
        "iam_role_arn",
        "grantee.grantee_identifier",
    ] {
        assert!(
            g.nodes
                .iter()
                .any(|n| n.references.iter().any(|r| r.attribute == path
                    && Some(r.label) == super::label(&n.resource_type, path)
                    && r.complete
                    && r.targets.len() == 1)),
            "{path}"
        );
    }
}

#[test]
fn identifiers_require_exact_current_explicit_typed_endpoints() {
    for (endpoint, consumer, field, path) in [
        (0, 2, "arn", "iam_role_arn"),
        (
            2,
            3,
            "access_grants_location_id",
            "access_grants_location_id",
        ),
        (1, 3, "arn", "grantee.grantee_identifier"),
    ] {
        for case in 0..12 {
            let mut v = fixture();
            match case {
                0 => v["resource_changes"][endpoint]["change"]["after"][field] = json!("different"),
                1 => {
                    v["resource_changes"][endpoint]["change"]["after_unknown"] = json!({field:true})
                }
                2 => {
                    v["resource_changes"][endpoint]["change"]["after_sensitive"] =
                        json!({field:true})
                }
                3 => v["resource_changes"][endpoint]["mode"] = json!("data"),
                4 => v["resource_changes"][endpoint]["deposed"] = json!("old"),
                5 => v["resource_changes"][endpoint]["change"]["actions"] = json!(["delete"]),
                6 => v["resource_changes"][endpoint]["type"] = json!("aws_sqs_queue"),
                7 => {
                    v["resource_changes"][endpoint]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                8 => {
                    v["configuration"]["root_module"]["resources"][consumer]["expressions"] =
                        json!({})
                }
                9 => v["resource_changes"][consumer]["change"]["after_unknown"] = json!(true),
                10 => v["resource_changes"][consumer]["change"]["after_sensitive"] = json!(true),
                _ => v["resource_changes"][consumer]["change"]["actions"] = json!(["delete"]),
            }
            assert!(!proven(&v, path), "{path} case {case}");
        }
    }
}

#[test]
fn directory_and_untyped_or_masked_grantees_never_become_iam_references() {
    for case in 0..9 {
        let mut v = fixture();
        match case {
            0 => {
                v["resource_changes"][3]["change"]["after"]["grantee"][0]["grantee_type"] =
                    json!("DIRECTORY_USER")
            }
            1 => {
                v["resource_changes"][3]["change"]["after"]["grantee"][0]["grantee_type"] =
                    json!("DIRECTORY_GROUP")
            }
            2 => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"grantee":[{"grantee_type":true}]})
            }
            3 => {
                v["resource_changes"][3]["change"]["after_sensitive"] =
                    json!({"grantee":[{"grantee_type":true}]})
            }
            4 => v["resource_changes"][3]["change"]["after_unknown"] = json!({"grantee":true}),
            5 => v["resource_changes"][3]["change"]["after"]["grantee"]
                .as_array_mut()
                .unwrap()
                .push(json!({"grantee_type":"IAM","grantee_identifier":"other"})),
            6 => {
                v["resource_changes"][1]["change"]["after"]["arn"] =
                    json!("AIDA-DIRECTORY-LIKE-ID");
                v["resource_changes"][3]["change"]["after"]["grantee"][0]["grantee_identifier"] =
                    json!("AIDA-DIRECTORY-LIKE-ID");
            }
            7 => {
                v["resource_changes"][1]["change"]["after"]["arn"] =
                    json!("arn:aws:iam::111111111111:role/wrong-kind");
                v["resource_changes"][3]["change"]["after"]["grantee"][0]["grantee_identifier"] =
                    json!("arn:aws:iam::111111111111:role/wrong-kind");
            }
            _ => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"grantee":[{"grantee_identifier":{}}]})
            }
        }
        assert!(!proven(&v, "grantee.grantee_identifier"), "case {case}");
    }
}

#[test]
fn provider_region_account_and_output_qualifiers_fail_closed() {
    for case in 0..8 {
        let mut v = fixture();
        match case {
            0 => v["resource_changes"][3]["change"]["after"]["account_id"] = json!("111111111111"),
            1 => v["resource_changes"][3]["change"]["after_unknown"] = json!({"account_id":true}),
            2 => {
                v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                v["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
                    json!("aws.other");
            }
            3 => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["access_grants_location_id"] =
                    json!({"references":["aws_s3control_access_grants_location.main.id"]})
            }
            4 => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["access_grants_location_id"] = json!({"references":["aws_s3control_access_grants_location.main.access_grant_id"]})
            }
            5 => v["resource_changes"][3]["change"]["after"]["account_id"] = json!("malformed"),
            6 => v["resource_changes"][2]["change"]["after"]["account_id"] = json!("222222222222"),
            _ => v["resource_changes"][2]["change"]["after_unknown"] = json!({"account_id":true}),
        }
        assert!(!proven(&v, "access_grants_location_id"), "case {case}");
    }
    let v = fixture();
    assert!(
        !graph(&v)
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.attribute == "location_scope")
    );
}

use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/dms-migration-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap())
}
fn proven(v: &Value, address: &str, path: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == address)
        .is_some_and(|n| {
            n.references.iter().any(|r| {
                r.attribute == path
                    && Some(r.label) == super::label(&n.resource_type, path)
                    && r.complete
                    && !r.dependency
                    && !r.targets.is_empty()
            })
        })
}
#[test]
fn migration_objects_preserve_typed_endpoints_and_all_engine_variants() {
    let v = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    let g = graph(&v);
    let mut checked = 0;
    for r in &raw.attributes {
        let n = &raw.nodes[r.target];
        if super::label(&n.resource_type, &r.attribute).is_some() {
            assert!(
                proven(&v, &n.address, &r.attribute),
                "{} {} {r:?}",
                n.address,
                r.attribute
            );
            checked += 1;
        }
    }
    assert!(checked >= 30, "{checked}");
    assert_eq!(g.nodes.len(), raw.nodes.len());
    assert_eq!(g.coverage.selected.resources, raw.nodes.len());
    for n in &raw.nodes {
        let nodes: Vec<_> = g.nodes.iter().filter(|a| a.address == n.address).collect();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, n.action);
    }
}
#[test]
fn migration_objects_reject_incompatible_missing_and_lifecycle_evidence() {
    let base = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    for r in &raw.attributes {
        let n = &raw.nodes[r.target];
        if super::label(&n.resource_type, &r.attribute).is_none() {
            continue;
        }
        let si = base["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == raw.nodes[r.sources[0]].address)
            .unwrap();
        let ti = base["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == n.address)
            .unwrap();
        let field = r.source_attributes_by_instance[&r.sources[0]]
            .first()
            .unwrap();
        for case in 0..10 {
            let mut v = base.clone();
            match case {
                0 => v["resource_changes"][si]["change"]["after"][field] = json!("other"),
                1 => v["resource_changes"][si]["change"]["after_unknown"] = json!({field:true}),
                2 => v["resource_changes"][si]["change"]["after_sensitive"] = json!({field:true}),
                3 => v["resource_changes"][si]["mode"] = json!("data"),
                4 => v["resource_changes"][si]["deposed"] = json!("old"),
                5 => v["resource_changes"][si]["change"]["actions"] = json!(["delete"]),
                6 => v["resource_changes"][si]["type"] = json!("aws_sqs_queue"),
                7 => {
                    v["resource_changes"][si]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                8 => v["configuration"]["root_module"]["resources"][ti]["expressions"] = json!({}),
                _ => v["resource_changes"][ti]["change"]["after_unknown"] = json!(true),
            }
            assert!(
                !proven(&v, &n.address, &r.attribute),
                "{} {} case{case}",
                n.address,
                r.attribute
            );
        }
    }
}
fn index(v: &Value, address: &str) -> usize {
    v["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["address"] == address)
        .unwrap()
}
#[test]
fn provider_settings_require_matching_exclusive_engine_branches() {
    let base = fixture();
    for resource in base["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["type"] == super::PROVIDER)
    {
        let address = resource["address"].as_str().unwrap();
        let i = index(&base, address);
        let branch = resource["change"]["after"]["settings"][0]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap();
        let field = if branch == "redshift_settings" {
            "s3_access_role_arn"
        } else {
            "certificate_arn"
        };
        let path = format!("settings.{branch}.{field}");
        for case in 0..5 {
            let mut v = base.clone();
            match case {
                0 => v["resource_changes"][i]["change"]["after"]["engine"] = json!("unknown"),
                1 => v["resource_changes"][i]["change"]["after_unknown"] = json!({"engine":true}),
                2 => {
                    let other = if branch == "oracle_settings" {
                        "mysql_settings"
                    } else {
                        "oracle_settings"
                    };
                    v["resource_changes"][i]["change"]["after"]["settings"][0][other] = json!([{}]);
                }
                3 => {
                    v["resource_changes"][i]["change"]["after_unknown"] =
                        json!({"settings":[{"redshift_settings":true,"oracle_settings":true}]})
                }
                _ => {
                    let value = &mut v["resource_changes"][i]["change"]["after"]["settings"][0]
                        [branch][0][field];
                    *value = json!([value.clone()]);
                }
            }
            assert!(!proven(&v, address, &path), "{address} {path} case{case}");
        }
    }
    let g = graph(&base);
    for (i, n) in g.nodes.iter().enumerate() {
        if super::resource(&n.resource_type) {
            assert_eq!(n.placement, crate::model::Placement::Logical);
            assert!(n.member_owner.is_none());
            assert!(
                !g.edges
                    .iter()
                    .any(|e| e.kind == crate::model::EdgeKind::Containment && e.to == i)
            );
        }
    }
}
#[test]
fn instance_profile_context_requires_both_complete_sets_and_one_explicit_vpc() {
    let base = fixture();
    let profile = "aws_dms_instance_profile.main";
    let p = index(&base, profile);
    let group = index(&base, "aws_dms_replication_subnet_group.main");
    let subnet = index(&base, "aws_subnet.second");
    let sg = index(&base, "aws_security_group.migration");
    let raw = crate::plan::parse(FIXTURE).unwrap();
    let pi = raw.nodes.iter().position(|n| n.address == profile).unwrap();
    for r in raw.attributes.iter().filter(|r| {
        r.target == pi
            && matches!(
                r.attribute.as_str(),
                "subnet_group_identifier" | "vpc_security_group_ids"
            )
    }) {
        assert!(r.service_reference_verified);
        assert_eq!(r.endpoint_network_context.len(), 1);
        assert_eq!(
            raw.nodes[r.endpoint_network_context[0]].address,
            "aws_vpc.main"
        );
    }
    for case in 0..8 {
        let mut v = base.clone();
        match case {
            0 => {
                v["resource_changes"][p]["change"]["after_unknown"] =
                    json!({"vpc_security_group_ids":true})
            }
            1 => v["resource_changes"][p]["change"]["after"]["vpc_security_group_ids"] = json!([]),
            2 => {
                v["configuration"]["root_module"]["resources"][p]["expressions"]["subnet_group_identifier"] =
                    json!({})
            }
            3 => {
                v["resource_changes"][group]["change"]["after_unknown"] = json!({"subnet_ids":true})
            }
            4 => {
                v["configuration"]["root_module"]["resources"][group]["expressions"]["subnet_ids"] =
                    json!({"references":["aws_subnet.first.id"]})
            }
            5 => {
                v["resource_changes"][subnet]["change"]["after"]["vpc_id"] = json!("vpc-other");
                v["configuration"]["root_module"]["resources"][subnet]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.other.id"]});
            }
            6 => {
                v["configuration"]["root_module"]["resources"][sg]["expressions"]["vpc_id"] =
                    json!({})
            }
            _ => v["resource_changes"][subnet]["change"]["after_unknown"] = json!({"vpc_id":true}),
        }
        assert!(
            !proven(&v, profile, "subnet_group_identifier"),
            "subnet group case{case}"
        );
        assert!(
            !proven(&v, profile, "vpc_security_group_ids"),
            "security groups case{case}"
        );
        assert!(proven(&v, profile, "kms_key_arn"));
    }
}
#[test]
fn migration_descriptors_are_complete_directional_collections_without_uri_identity() {
    let base = fixture();
    let address = "aws_dms_migration_project.main";
    let i = index(&base, address);
    let path = "source_data_provider_descriptor.data_provider_arn";
    for case in 0..4 {
        let mut v = base.clone();
        match case {
            0 => {
                v["configuration"]["root_module"]["resources"][i]["expressions"]["source_data_provider_descriptor"]
                    [1] = json!({})
            }
            1 => {
                v["resource_changes"][i]["change"]["after_unknown"] =
                    json!({"source_data_provider_descriptor":[{}, {"data_provider_arn":true}]})
            }
            2 => {
                v["resource_changes"][i]["change"]["after"]["source_data_provider_descriptor"][1]
                    ["data_provider_arn"] = json!("other")
            }
            _ => {
                let row =
                    v["resource_changes"][i]["change"]["after"]["source_data_provider_descriptor"]
                        [0]
                    .clone();
                v["resource_changes"][i]["change"]["after"]["source_data_provider_descriptor"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
        }
        assert!(!proven(&v, address, path));
        assert!(proven(
            &v,
            address,
            "target_data_provider_descriptor.data_provider_arn"
        ));
    }
    let raw = crate::plan::parse(FIXTURE).unwrap();
    let i = raw.nodes.iter().position(|n| n.address == address).unwrap();
    assert!(
        raw.attributes
            .iter()
            .filter(|r| r.target == i)
            .all(|r| r.endpoint_network_context.is_empty())
    );
    assert!(
        super::label(
            super::PROJECT,
            "schema_conversion_application_attributes.s3_bucket_path"
        )
        .is_none()
    );
    assert!(
        super::label(
            super::PROVIDER,
            "settings.redshift_settings.certificate_arn"
        )
        .is_none()
    );
    assert!(
        super::label(
            super::PROVIDER,
            "settings.doc_db_settings.s3_access_role_arn"
        )
        .is_none()
    );
}
#[test]
fn migration_objects_preserve_account_filtering_and_named_state_isolation() {
    let mut raw = crate::plan::parse(FIXTURE).unwrap();
    for n in &mut raw.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new(if n.resource_type == super::PROVIDER {
                "222222222222"
            } else {
                "111111111111"
            });
    }
    let g = crate::semantic::transform(&raw);
    assert!(
        !g.nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "DMS migration source provider" && r.complete)
    );
    let mut v = fixture();
    let i = index(&v, "aws_dms_data_provider.docdb");
    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
    v["configuration"]["root_module"]["resources"][i]["provider_config_key"] = json!("aws.other");
    assert!(!proven(
        &v,
        "aws_dms_migration_project.main",
        "source_data_provider_descriptor.data_provider_arn"
    ));
    let g = graph(&fixture());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_dms_migration_project.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    let mut missing = fixture();
    let i = index(&missing, "aws_dms_instance_profile.main");
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(i);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "first".into(),
                    json: FIXTURE.into(),
                },
                crate::NamedInput {
                    state: "second".into(),
                    json: missing.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("<metadata id=\"cross-state-relationships\">"));
    }
}

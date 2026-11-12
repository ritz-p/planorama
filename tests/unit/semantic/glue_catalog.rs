use crate::{model::Placement, plan, semantic, view};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/glue-catalog-plan.json");
const REFERENCES: &[(usize, usize, &str, &str)] = &[
    (4, 3, "registry_arn", "arn"),
    (5, 3, "registry_arn", "arn"),
    (6, 2, "federated_catalog.connection_name", "name"),
    (7, 2, "federated_catalog.connection_name", "name"),
    (7, 6, "target_redshift_catalog.catalog_arn", "arn"),
    (
        7,
        0,
        "catalog_properties.data_lake_access_properties.data_transfer_role",
        "arn",
    ),
    (
        7,
        1,
        "catalog_properties.data_lake_access_properties.kms_key",
        "arn",
    ),
    (
        7,
        0,
        "catalog_properties.iceberg_optimization_properties.role_arn",
        "arn",
    ),
    (9, 2, "federated_database.connection_name", "name"),
    (9, 8, "target_database.database_name", "name"),
    (
        10,
        4,
        "storage_descriptor.schema_reference.schema_id.schema_arn",
        "arn",
    ),
    (11, 10, "target_table.name", "name"),
    (
        11,
        4,
        "storage_descriptor.schema_reference.schema_id.schema_name",
        "schema_name",
    ),
    (
        11,
        3,
        "storage_descriptor.schema_reference.schema_id.registry_name",
        "registry_name",
    ),
    (12, 10, "view_definition.sub_objects", "arn"),
    (
        12,
        2,
        "view_definition.representations.validation_connection",
        "name",
    ),
    (13, 10, "table_name", "name"),
    (13, 0, "configuration.role_arn", "arn"),
    (14, 10, "target_table.table_name", "name"),
    (15, 10, "table_name", "name"),
    (16, 10, "table_name", "name"),
    (17, 8, "database_name", "name"),
];
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, target: usize, path: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][target]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == path
                && r.complete
                && Some(r.label)
                    == super::label(
                        v["resource_changes"][target]["type"].as_str().unwrap(),
                        path,
                    )
        })
}
fn set(root: &mut Value, path: &str, value: Value) {
    let mut at = root;
    let mut parts = path.split('.').peekable();
    while let Some(p) = parts.next() {
        if parts.peek().is_none() {
            at[p] = value;
            return;
        }
        at = &mut at[p][0];
    }
}
#[test]
fn glue_catalog_links_qualified_identities_and_retains_shared_cards() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for &(target, _, path, _) in REFERENCES {
        assert!(complete(&v, target, path), "{target} {path}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 18);
    for index in [13, 15, 16, 17, 5] {
        assert!(
            g.nodes
                .iter()
                .find(|n| n.address == v["resource_changes"][index]["address"])
                .unwrap()
                .member_owner
                .is_some(),
            "{index}"
        );
    }
    for index in [4, 6, 7, 10, 11, 14] {
        let n = g
            .nodes
            .iter()
            .find(|n| n.address == v["resource_changes"][index]["address"])
            .unwrap();
        assert!(n.member_owner.is_none(), "{index}");
        if [6, 7].contains(&index) {
            assert_eq!(n.placement, Placement::Spatial);
        }
    }
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
        assert!(!out.contains("PRIVATE_"));
        assert!(out.contains("Catalog connection"));
    }
}
#[test]
fn glue_catalog_rejects_unproven_scope_outputs_shapes_and_lifecycles() {
    for &(target, source, path, output) in REFERENCES {
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "wrong-output",
            "wrong-type",
            "mismatch",
            "delete",
            "deposed",
            "data",
            "consumer-delete",
            "consumer-deposed",
            "consumer-data",
            "provider",
            "binding",
            "region",
            "namespace",
            "ambiguous",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => set(
                    &mut v["configuration"]["root_module"]["resources"][target]["expressions"],
                    path,
                    json!({"constant_value":"literal"}),
                ),
                "ambiguous" => set(
                    &mut v["configuration"]["root_module"]["resources"][target]["expressions"],
                    path,
                    json!({"references":["var.dynamic"]}),
                ),
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] =
                        json!({path.split('.').next().unwrap():true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "wrong-output" => {
                    let a = v["resource_changes"][source]["address"]
                        .as_str()
                        .unwrap()
                        .to_string();
                    set(
                        &mut v["configuration"]["root_module"]["resources"][target]["expressions"],
                        path,
                        json!({"references":[format!("{a}.wrong")]}),
                    );
                }
                "wrong-type" => {
                    v["resource_changes"][source]["type"] = json!("aws_redshift_cluster")
                }
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("different")
                }
                "delete" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "consumer-delete" => {
                    v["resource_changes"][target]["change"]["actions"] = json!(["delete"])
                }
                "consumer-deposed" => v["resource_changes"][target]["deposed"] = json!("old"),
                "consumer-data" => v["resource_changes"][target]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "binding" | "region" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":if case=="region"{"us-west-2"}else{"us-east-1"}}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("other");
                }
                "namespace" => {
                    v["resource_changes"][source]["address"] =
                        json!("module.other.aws_glue_catalog_table.other")
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, target, path), "{target} {path} {case}");
        }
        if path.contains('.') {
            for case in ["object", "multiple"] {
                let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
                let root = path.split('.').next().unwrap();
                let block = v["resource_changes"][target]["change"]["after"][root][0].clone();
                v["resource_changes"][target]["change"]["after"][root] = if case == "object" {
                    block
                } else {
                    json!([block.clone(), block])
                };
                assert!(!complete(&v, target, path), "{path} {case}");
            }
        }
    }
}
#[test]
fn glue_catalog_names_require_catalog_database_and_schema_version_qualification() {
    for (target, path, qualifier) in [
        (7, "federated_catalog.connection_name", "catalog_id"),
        (
            9,
            "target_database.database_name",
            "target_database.catalog_id",
        ),
        (11, "target_table.name", "target_table.database_name"),
        (13, "table_name", "catalog_id"),
        (14, "target_table.table_name", "target_table.catalog_id"),
        (15, "table_name", "database_name"),
        (16, "table_name", "catalog_id"),
        (17, "database_name", "catalog_id"),
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        set(
            &mut v["resource_changes"][target]["change"]["after"],
            qualifier,
            json!("other"),
        );
        assert!(!complete(&v, target, path), "{target} {qualifier}");
    }
    for case in [
        "version",
        "version-unknown",
        "source-version-unknown",
        "version-id",
        "registry",
        "both-variants",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "version" => {
                v["resource_changes"][11]["change"]["after"]["storage_descriptor"][0]["schema_reference"]
                    [0]["schema_version_number"] = json!(2)
            }
            "version-unknown" => {
                v["resource_changes"][11]["change"]["after_unknown"] = json!({"storage_descriptor":[{"schema_reference":[{"schema_version_number":true}]}]})
            }
            "source-version-unknown" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"latest_schema_version":true})
            }
            "version-id" => {
                v["resource_changes"][11]["change"]["after"]["storage_descriptor"][0]["schema_reference"]
                    [0]["schema_version_id"] = json!("opaque-version")
            }
            "registry" => {
                v["resource_changes"][4]["change"]["after"]["registry_name"] = json!("other")
            }
            "both-variants" => {
                v["resource_changes"][11]["change"]["after"]["storage_descriptor"][0]["schema_reference"]
                    [0]["schema_id"][0]["schema_arn"] = json!("also-arn")
            }
            _ => unreachable!(),
        }
        assert!(
            !complete(
                &v,
                11,
                "storage_descriptor.schema_reference.schema_id.schema_name"
            ),
            "{case}"
        );
    }
    for value in [Value::Null, json!(false)] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][13]["change"]["after"]["configuration"][0]["enabled"] = value;
        assert!(!complete(&v, 13, "configuration.role_arn"));
        assert!(complete(&v, 13, "table_name"));
    }
}
#[test]
fn glue_catalog_view_collections_require_every_indexed_reference() {
    for case in [
        "partial",
        "duplicate",
        "unknown",
        "literal-row",
        "row-mismatch",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let path = if matches!(case, "literal-row" | "row-mismatch") {
            "view_definition.representations.validation_connection"
        } else {
            "view_definition.sub_objects"
        };
        match case {
            "partial" => {
                v["resource_changes"][12]["change"]["after"]["view_definition"][0]["sub_objects"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("unreferenced"))
            }
            "duplicate" => {
                let a=v["resource_changes"][12]["change"]["after"]["view_definition"][0]["sub_objects"][0].clone();
                v["resource_changes"][12]["change"]["after"]["view_definition"][0]["sub_objects"]
                    [1] = a;
            }
            "unknown" => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"view_definition":[{"sub_objects":[false,true]}]})
            }
            "literal-row" => {
                v["configuration"]["root_module"]["resources"][12]["expressions"]["view_definition"]
                    [0]["representations"][1]["validation_connection"] =
                    json!({"constant_value":"federated"})
            }
            "row-mismatch" => {
                v["resource_changes"][12]["change"]["after"]["view_definition"][0]["representations"]
                    [1]["validation_connection"] = json!("other")
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 12, path), "{case}");
    }
}
#[test]
fn glue_catalog_preserves_named_state_lifecycle_and_selection() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][13]["previous_address"] = json!("aws_glue_catalog_table_optimizer.old");
    v["resource_changes"][13]["change"]["actions"] = json!(["delete", "create"]);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in &parsed.nodes {
        let matched: Vec<_> = g
            .nodes
            .iter()
            .filter(|o| o.address == n.address && o.deposed_key == n.deposed_key)
            .collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].action, n.action);
        assert_eq!(matched[0].previous_address, n.previous_address);
    }
    let selected = view::single(
        &g,
        &view::Options {
            focus: Some("aws_glue_schema.used".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    for address in [
        "aws_glue_partition.partition",
        "aws_glue_catalog_table_optimizer.compact",
    ] {
        let selected = view::single(
            &g,
            &view::Options {
                focus: Some(address.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.coverage.selected.resources, 1);
    }
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(10);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
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
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    }
}

#[test]
fn glue_catalog_optimizer_rejects_unknown_type_without_losing_identity() {
    for kind in [Value::Null, json!("other")] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][13]["change"]["after"]["type"] = kind;
        assert!(!complete(&v, 13, "table_name"));
        assert!(!complete(&v, 13, "configuration.role_arn"));
        assert_eq!(graph(&v).coverage.selected.resources, 18);
    }
}

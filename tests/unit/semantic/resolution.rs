use crate::{model::Graph, plan, semantic};
use std::fmt::Write;

struct Resolution {
    graph: Graph,
    diagnostics: String,
}

fn inspect(input: &[u8]) -> Resolution {
    let raw = plan::parse(std::str::from_utf8(input).unwrap()).unwrap();
    let graph = semantic::transform(&raw).0;
    let mut diagnostics = String::new();
    for diagnostic in semantic::diagnostics::collect(&raw) {
        writeln!(
            diagnostics,
            "{}: address={:?}, attribute={:?}",
            diagnostic.reason.description(),
            diagnostic.address,
            diagnostic.attribute
        )
        .unwrap();
    }
    Resolution { graph, diagnostics }
}

#[test]
fn successful_reference_resolution_has_no_diagnostics_including_subnet_collections() {
    let input = br#"{"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc.main","type":"aws_vpc"},
        {"address":"aws_subnet.private[0]","type":"aws_subnet"},
        {"address":"aws_subnet.private[1]","type":"aws_subnet"},
        {"address":"aws_lb.app","type":"aws_lb"}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_subnet.private","expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}},
        {"address":"aws_lb.app","expressions":{"subnets":{"references":["aws_subnet.private"]}}}
    ]}}}"#;
    let output = inspect(input);
    assert!(output.diagnostics.is_empty(), "{}", output.diagnostics);
}

#[test]
fn diagnostics_explain_missing_semantic_attributes() {
    let input = br#"{"format_version":"1.2","resource_changes":[{"address":"aws_instance.app","type":"aws_instance"}]}"#;
    let output = inspect(input);
    let diagnostics = output.diagnostics;
    assert!(diagnostics.contains("missing expected attribute"));
    assert!(diagnostics.contains("subnet_id"));
}

#[test]
fn diagnostics_explain_ambiguous_parents_and_unsafe_association_lowering() {
    let input = br#"{"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc.a","type":"aws_vpc"},
        {"address":"aws_vpc.b","type":"aws_vpc"},
        {"address":"aws_subnet.private","type":"aws_subnet"},
        {"address":"aws_route_table.private","type":"aws_route_table"},
        {"address":"aws_route_table_association.private","type":"aws_route_table_association"},
        {"address":"test.consumer","type":"test"}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_subnet.private","expressions":{"vpc_id":{"references":["aws_vpc.a.id","aws_vpc.b.id"]}}},
        {"address":"aws_route_table_association.private","expressions":{"subnet_id":{"references":["aws_subnet.private.id"]},"route_table_id":{"references":["aws_route_table.private.id"]}}},
        {"address":"test.consumer","expressions":{"input":{"references":["aws_route_table_association.private.id"]}}}
    ]}}}"#;
    let output = inspect(input);
    let diagnostics = output.diagnostics;
    assert!(diagnostics.contains("ambiguous containment parent: address=\"aws_subnet.private\""));
    assert!(diagnostics.contains("additional relationships prevent association lowering: address=\"aws_route_table_association.private\""));
}

#[test]
fn iteration_meta_references_are_dynamic_not_unresolved_in_root_and_modules() {
    use serde_json::json;
    for reference in [
        "count.index",
        "each.key",
        "each.value",
        "each.value.subnet_id",
        "each.value[\"subnet_id\"]",
    ] {
        for nested in [false, true] {
            let address = if nested {
                "module.worker[0].test.item"
            } else {
                "test.item"
            };
            let resource =
                json!({"address":"test.item","expressions":{"tags":{"references":[reference]}}});
            let module = if nested {
                json!({"module_calls":{"worker":{"module":{"resources":[resource]}}}})
            } else {
                json!({"resources":[resource]})
            };
            let input = json!({"format_version":"1.2","resource_changes":[{"address":address,"type":"test"}],"configuration":{"root_module":module}}).to_string();
            let output = inspect(input.as_bytes());
            let diagnostics = output.diagnostics;
            assert!(
                diagnostics.contains("dynamic instance selection"),
                "{diagnostics}"
            );
            assert!(
                !diagnostics.contains("unresolved reference"),
                "{diagnostics}"
            );
            assert_eq!(diagnostics.lines().count(), 1);
        }
    }
}

#[test]
fn spanning_resources_diagnose_disjoint_subnet_ancestry() {
    use serde_json::json;
    for resource_type in ["aws_lb", "aws_ecs_service"] {
        for shared in [true, false] {
            let input = json!({"format_version":"1.2","resource_changes":[
                {"address":"aws_vpc.a","type":"aws_vpc"}, {"address":"aws_vpc.b","type":"aws_vpc"},
                {"address":"aws_subnet.a","type":"aws_subnet"}, {"address":"aws_subnet.b","type":"aws_subnet"},
                {"address":format!("{resource_type}.app"),"type":resource_type}
            ],"configuration":{"root_module":{"resources":[
                {"address":"aws_subnet.a","expressions":{"vpc_id":{"references":["aws_vpc.a.id"]}}},
                {"address":"aws_subnet.b","expressions":{"vpc_id":{"references":[if shared {"aws_vpc.a.id"} else {"aws_vpc.b.id"}]}}},
                {"address":format!("{resource_type}.app"),"expressions": if resource_type == "aws_lb" {
                    json!({"subnets":{"references":["aws_subnet.a.id","aws_subnet.b.id"]}})
                } else { json!({"network_configuration":[{"subnets":{"references":["aws_subnet.a.id","aws_subnet.b.id"]}}]}) }}
            ]}}}).to_string();
            let output = inspect(input.as_bytes());
            let warnings = output.diagnostics;
            if shared {
                assert!(warnings.is_empty(), "{warnings}");
            } else {
                assert!(
                    warnings.contains("ambiguous containment parent"),
                    "{warnings}"
                );
                assert!(warnings.contains(&format!("{resource_type}.app")));
                assert!(warnings.contains("subnets"));
            }
        }
    }
}

#[test]
fn mixed_semantic_candidates_report_type_mismatch_before_ambiguity() {
    use serde_json::json;
    for (resource_type, provider) in [("aws_subnet", "hashicorp/aws"), ("aws_vpc", "acme/custom")] {
        let input = json!({"format_version":"1.2","resource_changes":[
            {"address":"aws_vpc.valid","type":"aws_vpc"},
            {"address":"other.invalid","type":resource_type,"provider_name":provider},
            {"address":"aws_subnet.child","type":"aws_subnet"}
        ],"configuration":{"root_module":{"resources":[
            {"address":"aws_subnet.child","expressions":{"vpc_id":{"references":["aws_vpc.valid.id","other.invalid.id"]}}}
        ]}}}).to_string();
        let result = inspect(input.as_bytes());
        let warnings = result.diagnostics;
        assert!(
            warnings.contains("semantic endpoint type mismatch: address=\"aws_subnet.child\""),
            "{warnings}"
        );
        assert!(
            !warnings.contains("ambiguous containment parent"),
            "{warnings}"
        );
    }
}

#[test]
fn graph_only_references_keep_diagnostics_for_resources_and_module_calls() {
    use serde_json::json;
    for field in ["depends_on", "count_expression", "for_each_expression"] {
        for module_call in [false, true] {
            for resolved in [false, true] {
                let expression = if field == "depends_on" {
                    json!(["test.source"])
                } else {
                    json!({"references":["test.source.id"]})
                };
                let mut resource = json!({"address":"test.consumer"});
                let root = if module_call {
                    json!({"module_calls":{"worker":{field:expression,"module":{"resources":[resource]}}}})
                } else {
                    resource[field] = expression;
                    json!({"resources":[resource]})
                };
                let address = if module_call {
                    "module.worker.test.consumer"
                } else {
                    "test.consumer"
                };
                let mut changes = vec![json!({"address":address,"type":"test"})];
                if resolved {
                    changes.push(json!({"address":"test.source","type":"test"}));
                }
                let input = json!({"format_version":"1.2","resource_changes":changes,"configuration":{"root_module":root}}).to_string();
                let output = inspect(input.as_bytes());
                let warnings = output.diagnostics;
                if resolved {
                    assert!(warnings.is_empty(), "{warnings}");
                } else {
                    assert!(warnings.contains("unresolved reference"), "{warnings}");
                    assert!(warnings.contains(address), "{warnings}");
                    assert!(warnings.contains(field), "{warnings}");
                    assert_eq!(warnings.lines().count(), 1);
                }
            }
        }
    }
}

#[test]
fn whole_resource_dependencies_accept_count_and_for_each_instances() {
    use serde_json::json;
    for keys in [["[0]", "[1]"], ["[\"a\"]", "[\"b\"]"]] {
        for inherited in [false, true] {
            for missing in [false, true] {
                let mut dependencies = vec!["test.worker"];

                if missing {
                    dependencies.push("test.missing");
                }
                let resource = json!({"address":"test.consumer","depends_on":dependencies});
                let root = if inherited {
                    json!({"module_calls":{"child":{"depends_on":dependencies,"module":{"resources":[{"address":"test.consumer"}]}}}})
                } else {
                    json!({"resources":[resource]})
                };
                let consumer = if inherited {
                    "module.child.test.consumer"
                } else {
                    "test.consumer"
                };
                let input = json!({"format_version":"1.2","resource_changes":[
                    {"address":format!("test.worker{}", keys[0]),"type":"test"},
                    {"address":format!("test.worker{}", keys[1]),"type":"test"},
                    {"address":consumer,"type":"test"}
                ],"configuration":{"root_module":root}})
                .to_string();
                let output = inspect(input.as_bytes());
                let warnings = output.diagnostics;
                assert!(
                    !warnings.contains("multiple matching instances"),
                    "{warnings}"
                );
                if missing {
                    assert!(warnings.contains("unresolved reference"), "{warnings}");
                } else {
                    assert!(warnings.is_empty(), "{warnings}");
                }

                assert_eq!((output.graph.nodes.len(), output.graph.edges.len()), (3, 2));
            }
        }
    }
}

#[test]
fn collection_metadata_and_ecs_blocks_accept_multiple_instances() {
    use serde_json::json;
    for keys in [["[0]", "[1]"], ["[\"a\"]", "[\"b\"]"]] {
        for field in [
            "count_expression",
            "for_each_expression",
            "network_configuration",
        ] {
            for missing in [false, true] {
                let mut refs = vec!["aws_subnet.private.id"];

                if missing {
                    refs.push("aws_subnet.missing.id");
                }
                let mut consumer = json!({"address":"aws_ecs_service.app","expressions":{"network_configuration":[{"subnets":{"references":["aws_subnet.private.id"]}}]}});
                if field == "network_configuration" {
                    consumer["expressions"][field] = json!([{"subnets":{"references":refs}}]);
                } else {
                    consumer[field] = json!({"references":refs});
                }
                let input = json!({"format_version":"1.2","resource_changes":[
                    {"address":"aws_vpc.main","type":"aws_vpc"},
                    {"address":format!("aws_subnet.private{}", keys[0]),"type":"aws_subnet"},
                    {"address":format!("aws_subnet.private{}", keys[1]),"type":"aws_subnet"},
                    {"address":"aws_ecs_service.app","type":"aws_ecs_service"}
                ],"configuration":{"root_module":{"resources":[
                    {"address":"aws_subnet.private","expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}}, consumer
                ]}}}).to_string();
                let output = inspect(input.as_bytes());
                let warnings = output.diagnostics;
                assert!(
                    !warnings.contains("multiple matching instances"),
                    "{warnings}"
                );
                if missing {
                    assert!(warnings.contains("unresolved reference"), "{warnings}");
                } else {
                    assert!(warnings.is_empty(), "{warnings}");
                }
            }
        }
    }
}

#[test]
fn attribute_and_module_output_indexing_do_not_select_resource_instances() {
    use serde_json::json;
    for reference in [
        "test.source.tags[var.key]",
        "module.worker.result[var.key]",
        "module.worker.result.items[var.key]",
    ] {
        let input = json!({"format_version":"1.2","resource_changes":[
            {"address":"test.source","type":"test"},
            {"address":"module.worker.test.source","type":"test"},
            {"address":"test.consumer","type":"test"}
        ],"configuration":{"root_module":{
            "resources":[{"address":"test.consumer","expressions":{"input":{"references":[reference]}}}],
            "module_calls":{"worker":{"module":{"resources":[{"address":"test.source"}],"outputs":{"result":{"expression":{"references":["test.source.id"]}}}}}}
        }}}).to_string();
        let output = inspect(input.as_bytes());
        assert!(output.diagnostics.is_empty(), "{}", output.diagnostics);

        assert_eq!((output.graph.nodes.len(), output.graph.edges.len()), (3, 1));
    }
}

#[test]
fn whole_module_collections_are_not_ambiguous() {
    use serde_json::json;
    for modules in [
        ["module.service[0]", "module.service[1]"],
        ["module.service[\"api\"]", "module.service[\"web\"]"],
    ] {
        for missing in [false, true] {
            let mut refs = vec!["module.service"];
            if missing {
                refs.push("module.absent");
            }
            let input = json!({"format_version":"1.2","resource_changes":[
                {"address":format!("{}.test.first", modules[0]),"type":"test"},
                {"address":format!("{}.test.second", modules[0]),"type":"test"},
                {"address":format!("{}.test.first", modules[1]),"type":"test"},
                {"address":"test.consumer","type":"test"}
            ],"configuration":{"root_module":{"resources":[{"address":"test.consumer","expressions":{"input":{"references":refs}}}]}}}).to_string();
            let output = inspect(input.as_bytes());
            let warnings = output.diagnostics;
            assert!(
                !warnings.contains("multiple matching instances"),
                "{warnings}"
            );
            if missing {
                assert!(warnings.contains("unresolved reference"), "{warnings}");
            } else {
                assert!(warnings.is_empty(), "{warnings}");
            }

            assert_eq!((output.graph.nodes.len(), output.graph.edges.len()), (4, 3));
        }
    }
    let output = inspect(include_bytes!("../../fixtures/terraform-plan.json"));
    let warnings = output.diagnostics;
    assert!(
        !warnings
            .lines()
            .any(|line| line.contains("multiple matching instances")
                && line.contains("address=\"terraform_data.consumer\"")
                && line.contains("attribute=\"input\"")),
        "{warnings}"
    );
}

#[test]
fn defined_constant_aliases_are_not_missing_references() {
    use serde_json::json;
    for reference in ["local.tags", "module.constants.value", "module.child.value"] {
        let input = json!({"format_version":"1.2","resource_changes":[
            {"address":"test.consumer","type":"test"},
            {"address":"aws_instance.consumer","type":"aws_instance"},
            {"address":"module.child.test.inner","type":"test"}
        ],"configuration":{"root_module":{
            "locals":{"tags":{"constant_value":"TOP_SECRET"}},
            "resources":[
                {"address":"test.consumer","expressions":{"tags":{"references":[reference]}}},
                {"address":"aws_instance.consumer","expressions":{"subnet_id":{"references":[reference]}}}
            ],
            "module_calls":{
                "constants":{"module":{"outputs":{"value":{"expression":{"constant_value":"TOP_SECRET"}}}}},
                "child":{"expressions":{"input":{"constant_value":"TOP_SECRET"}},"module":{
                    "resources":[{"address":"test.inner","expressions":{"tags":{"references":["var.input"]}}}],
                    "outputs":{"value":{"expression":{"references":["var.input"]}}}
                }}
            }
        }}}).to_string();
        let result = inspect(input.as_bytes());
        let warnings = result.diagnostics;
        assert!(!warnings.contains("unresolved reference"), "{warnings}");
        assert!(
            warnings.contains(
                "attribute has no resolvable resource reference: address=\"aws_instance.consumer\""
            ),
            "{warnings}"
        );
        assert_eq!(warnings.lines().count(), 1, "{warnings}");
        assert!(!warnings.contains("TOP_SECRET"));
    }
}

#[test]
fn metadata_named_outputs_keep_dynamic_module_index_diagnostics() {
    use serde_json::json;
    for (name, field) in [("count", "index"), ("each", "value")] {
        for dynamic in [false, true] {
            let selector = if dynamic { "count.index" } else { "0" };
            let input = json!({"format_version":"1.2","resource_changes":[
                {"address":"module.worker[0].test.source","type":"test"},
                {"address":"test.consumer[0]","type":"test"}
            ],"configuration":{"root_module":{
                "resources":[{"address":"test.consumer","expressions":{"input":{"references":[format!("module.worker[{selector}].{name}.{field}")]}}}],
                "module_calls":{"worker":{"module":{"resources":[{"address":"test.source"}],"outputs":{name:{"expression":{"references":["test.source.id"]}}}}}}
            }}}).to_string();
            let output = inspect(input.as_bytes());
            let warnings = output.diagnostics;
            if dynamic {
                assert!(
                    warnings.contains("dynamic instance selection"),
                    "{warnings}"
                );
                assert_eq!(warnings.lines().count(), 1);
            } else {
                assert!(warnings.is_empty(), "{warnings}");
            }

            assert_eq!((output.graph.nodes.len(), output.graph.edges.len()), (2, 1));
        }
    }
}

#[test]
fn declared_variables_and_builtin_values_are_not_unresolved_resources() {
    use serde_json::json;
    for reference in [
        "var.region",
        "var.settings.name",
        "path.module",
        "path.root",
        "path.cwd",
        "terraform.workspace",
        "var.missing",
        "path.missing",
    ] {
        for nested in [false, true] {
            let module = json!({"variables":{"region":{"default":"TOP_SECRET"},"settings":{}},"resources":[
                {"address":"test.consumer","expressions":{"input":{"references":[reference]}}},
                {"address":"aws_instance.consumer","expressions":{"subnet_id":{"references":[reference]}}}
            ]});
            let (scope, root) = if nested {
                (
                    "module.child.",
                    json!({"module_calls":{"child":{"module":module}}}),
                )
            } else {
                ("", module)
            };
            let input = json!({"format_version":"1.2","resource_changes":[
                {"address":format!("{scope}test.consumer"),"type":"test"},
                {"address":format!("{scope}aws_instance.consumer"),"type":"aws_instance"}
            ],"configuration":{"root_module":root}})
            .to_string();
            let output = inspect(input.as_bytes());
            let warnings = output.diagnostics;
            if reference.ends_with("missing") {
                assert!(warnings.contains("unresolved reference"), "{warnings}");
            } else {
                assert!(!warnings.contains("unresolved reference"), "{warnings}");
                assert!(
                    warnings.contains("attribute has no resolvable resource reference"),
                    "{warnings}"
                );
                assert_eq!(warnings.lines().count(), 1);
            }
            assert!(!warnings.contains("TOP_SECRET"));
        }
    }
}

#[test]
fn absent_metadata_named_module_outputs_are_unresolved_not_iteration_context() {
    use serde_json::json;
    for output in ["count.index", "each.key", "each.value.subnet_id"] {
        let input = json!({"format_version":"1.2","resource_changes":[
            {"address":"module.worker.test.inner","type":"test"},
            {"address":"test.consumer","type":"test"}
        ],"configuration":{"root_module":{
            "resources":[{"address":"test.consumer","expressions":{"input":{"references":[format!("module.worker.{output}")]}}}],
            "module_calls":{"worker":{"module":{"resources":[{"address":"test.inner"}]}}}
        }}}).to_string();
        let result = inspect(input.as_bytes());
        let warnings = result.diagnostics;
        assert!(warnings.contains("unresolved reference"), "{warnings}");
        assert!(
            !warnings.contains("dynamic instance selection"),
            "{warnings}"
        );
        assert_eq!(warnings.lines().count(), 1);
    }
}

#[test]
fn partial_semantic_provenance_is_distinct_from_source_free_attributes() {
    use serde_json::json;
    for constant in ["local.vpc_id", "path.module", "var.vpc_id"] {
        for partial in [false, true] {
            let mut refs = vec![constant];
            if partial {
                refs.push("aws_vpc.main.id");
            }
            let input = json!({"format_version":"1.2","variables":{"vpc_id":{"value":"TOP_SECRET"}},"resource_changes":[
                {"address":"aws_vpc.main","type":"aws_vpc"},
                {"address":"aws_subnet.child","type":"aws_subnet"}
            ],"configuration":{"root_module":{
                "locals":{"vpc_id":{"constant_value":"TOP_SECRET"}},
                "resources":[{"address":"aws_subnet.child","expressions":{"vpc_id":{"references":refs}}}]
            }}}).to_string();
            let output = inspect(input.as_bytes());
            let warnings = output.diagnostics;
            assert_eq!(warnings.lines().count(), 1, "{warnings}");
            assert_eq!(
                warnings.contains("only part of the attribute resolves to resource references"),
                partial,
                "{warnings}"
            );
            assert_eq!(
                warnings.contains("attribute has no resolvable resource reference"),
                !partial,
                "{warnings}"
            );
            assert!(warnings.contains("address=\"aws_subnet.child\", attribute=\"vpc_id\""));
            assert!(!warnings.contains("TOP_SECRET"));

            assert!(
                output
                    .graph
                    .edges
                    .iter()
                    .all(|edge| edge.kind != crate::model::EdgeKind::Containment)
            );
        }
    }
}

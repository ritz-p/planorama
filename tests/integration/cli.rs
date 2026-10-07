mod support;
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_planorama"))
}

#[test]
fn stdin_to_stdout_is_svg_and_diagnostics_stay_on_stderr() {
    let output = support::run(include_bytes!("../../examples/plan.json"));
    assert!(output.status.success());
    assert!(output.stdout.starts_with(b"<svg "));
    assert!(output.stdout.ends_with(b"</svg>\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn invalid_json_fails_without_svg() {
    let output = support::run(b"broken");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid JSON"));
}

#[test]
fn refuses_to_overwrite_input() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/plan.json");
    let output = cli().args([path, "-o", path]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("different files"));
    assert_eq!(
        std::fs::read(path).unwrap(),
        include_bytes!("../../examples/plan.json")
    );
}

#[test]
fn help_succeeds_and_unknown_flags_fail() {
    assert!(cli().arg("--help").output().unwrap().status.success());
    assert!(!cli().arg("--unknown").output().unwrap().status.success());
}

#[test]
fn address_format_validation_rejects_missing_and_unknown_values() {
    for args in [vec!["--address-format"], vec!["--address-format", "local"]] {
        let output = cli().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("qualified or terraform"));
    }
}

#[test]
fn selected_addresses_keep_modules_indices_and_full_titles() {
    use std::io::Write;
    use std::process::Stdio;
    let addresses = [
        "aws_vpc.main",
        "module.app.aws_instance.web",
        "module.net.module.app[0].aws_instance.web[1]",
        "module.net.aws_subnet.private[\"a&b\"]",
    ];
    let changes: Vec<_> = addresses
        .iter()
        .map(|address| serde_json::json!({"address":address,"type":"aws_instance"}))
        .collect();
    let input = serde_json::json!({"format_version":"1.2","resource_changes":changes}).to_string();
    let run = |format: Option<&str>| {
        let mut command = cli();
        command.args(["-", "-o", "-"]);
        if let Some(format) = format {
            command.args(["--address-format", format]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    let default = run(None);
    assert_eq!(default, run(Some("qualified")));
    for (svg, root) in [
        (default, "module.root.aws_vpc.main"),
        (run(Some("terraform")), "aws_vpc.main"),
    ] {
        assert!(svg.contains(&format!(">{root}</text>")));
        assert!(svg.contains(">module.app.aws_instance.web</text>"));
        assert!(svg.contains("<title>module.net.module.app[0].aws_instance.web[1]"));
        assert!(svg.contains("<title>module.net.aws_subnet.private[&quot;a&amp;b&quot;]"));
        assert!(svg.contains("data-terraform-address=\"aws_vpc.main\""));
    }
}

#[test]
fn optional_diagnostics_distinguish_failures_without_changing_svg_or_leaking_values() {
    use serde_json::json;
    let cases = [
        (
            json!({"references":["aws_subnet.missing.id"]}),
            json!({}),
            "unresolved reference",
        ),
        (
            json!({"references":["aws_subnet.private.id"]}),
            json!({}),
            "multiple matching instances",
        ),
        (
            json!({"references":["aws_subnet.private[count.index].id"]}),
            json!({}),
            "dynamic instance selection",
        ),
        (
            json!({"references":["local.a"]}),
            json!({"a":{"references":["local.b"]},"b":{"references":["local.a"]}}),
            "alias/local/module resolution cycle",
        ),
        (
            json!({"references":["aws_vpc.main.id"]}),
            json!({}),
            "semantic endpoint type mismatch",
        ),
        (
            json!({"constant_value":"TOP_SECRET"}),
            json!({}),
            "attribute has no resolvable resource reference",
        ),
    ];
    for (expression, locals, reason) in cases {
        let input = json!({"format_version":"1.2","resource_changes":[
            {"address":"aws_vpc.main","type":"aws_vpc"},
            {"address":"aws_subnet.private[0]","type":"aws_subnet"},
            {"address":"aws_subnet.private[1]","type":"aws_subnet"},
            {"address":"aws_instance.app","type":"aws_instance","change":{"after":{"password":"TOP_SECRET"}}}
        ],"configuration":{"root_module":{"locals":locals,"resources":[
            {"address":"aws_subnet.private","expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}},
            {"address":"aws_instance.app","expressions":{"subnet_id":expression,"password":{"constant_value":"TOP_SECRET"}}}
        ]}}}).to_string();
        let normal = support::run(input.as_bytes());
        let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
        assert!(output.status.success());
        assert_eq!(output.stdout, normal.stdout);
        assert!(normal.stderr.is_empty());
        let diagnostics = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostics.contains(reason), "{diagnostics}");
        assert!(diagnostics.contains("address=\"aws_instance.app\""));
        assert!(diagnostics.contains("attribute=\"subnet_id\""));
        assert!(!diagnostics.contains("TOP_SECRET"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("TOP_SECRET"));
        assert_eq!(
            diagnostics.as_bytes(),
            support::run_with_args(input.as_bytes(), &["--diagnostics"]).stderr
        );
    }
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
    let output = support::run_with_args(input, &["--diagnostics"]);
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, support::run(input).stdout);
}

#[test]
fn diagnostics_explain_missing_semantic_attributes() {
    let input = br#"{"format_version":"1.2","resource_changes":[{"address":"aws_instance.app","type":"aws_instance"}]}"#;
    let output = support::run_with_args(input, &["--diagnostics"]);
    assert!(output.status.success());
    let diagnostics = String::from_utf8(output.stderr).unwrap();
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
    let output = support::run_with_args(input, &["--diagnostics"]);
    assert!(output.status.success());
    let diagnostics = String::from_utf8(output.stderr).unwrap();
    assert!(diagnostics.contains("ambiguous containment parent: address=\"aws_subnet.private\""));
    assert!(diagnostics.contains("additional relationships prevent association lowering: address=\"aws_route_table_association.private\""));
    assert_eq!(output.stdout, support::run(input).stdout);
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
            let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
            assert!(output.status.success());
            let diagnostics = String::from_utf8(output.stderr).unwrap();
            assert!(
                diagnostics.contains("dynamic instance selection"),
                "{diagnostics}"
            );
            assert!(
                !diagnostics.contains("unresolved reference"),
                "{diagnostics}"
            );
            assert_eq!(diagnostics.lines().count(), 1);
            assert_eq!(output.stdout, support::run(input.as_bytes()).stdout);
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
            let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
            assert!(output.status.success());
            let warnings = String::from_utf8(output.stderr).unwrap();
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
            assert_eq!(output.stdout, support::run(input.as_bytes()).stdout);
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
        let result = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
        assert!(result.status.success());
        let warnings = String::from_utf8(result.stderr).unwrap();
        assert!(
            warnings.contains("semantic endpoint type mismatch: address=\"aws_subnet.child\""),
            "{warnings}"
        );
        assert!(
            !warnings.contains("ambiguous containment parent"),
            "{warnings}"
        );
        assert_eq!(result.stdout, support::run(input.as_bytes()).stdout);
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
                let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
                assert!(output.status.success());
                let warnings = String::from_utf8(output.stderr).unwrap();
                if resolved {
                    assert!(warnings.is_empty(), "{warnings}");
                } else {
                    assert!(warnings.contains("unresolved reference"), "{warnings}");
                    assert!(warnings.contains(address), "{warnings}");
                    assert!(warnings.contains(field), "{warnings}");
                    assert_eq!(warnings.lines().count(), 1);
                }
                assert_eq!(output.stdout, support::run(input.as_bytes()).stdout);
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
                let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
                assert!(output.status.success());
                let warnings = String::from_utf8(output.stderr).unwrap();
                assert!(
                    !warnings.contains("multiple matching instances"),
                    "{warnings}"
                );
                if missing {
                    assert!(warnings.contains("unresolved reference"), "{warnings}");
                } else {
                    assert!(warnings.is_empty(), "{warnings}");
                }
                let svg = String::from_utf8(output.stdout).unwrap();
                assert!(svg.contains("3 resources, 2 reference edges"));
                assert_eq!(svg.as_bytes(), support::run(input.as_bytes()).stdout);
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
                let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
                assert!(output.status.success());
                let warnings = String::from_utf8(output.stderr).unwrap();
                assert!(
                    !warnings.contains("multiple matching instances"),
                    "{warnings}"
                );
                if missing {
                    assert!(warnings.contains("unresolved reference"), "{warnings}");
                } else {
                    assert!(warnings.is_empty(), "{warnings}");
                }
                assert_eq!(output.stdout, support::run(input.as_bytes()).stdout);
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
        let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
        assert!(output.status.success());
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let svg = String::from_utf8(output.stdout).unwrap();
        assert!(svg.contains("3 resources, 1 reference edges"));
        assert_eq!(svg.as_bytes(), support::run(input.as_bytes()).stdout);
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
            let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
            assert!(output.status.success());
            let warnings = String::from_utf8(output.stderr).unwrap();
            assert!(
                !warnings.contains("multiple matching instances"),
                "{warnings}"
            );
            if missing {
                assert!(warnings.contains("unresolved reference"), "{warnings}");
            } else {
                assert!(warnings.is_empty(), "{warnings}");
            }
            let svg = String::from_utf8(output.stdout).unwrap();
            assert!(svg.contains("4 resources, 3 reference edges"));
            assert_eq!(svg.as_bytes(), support::run(input.as_bytes()).stdout);
        }
    }
    let output = support::run_with_args(
        include_bytes!("../fixtures/terraform-plan.json"),
        &["--diagnostics"],
    );
    assert!(output.status.success());
    let warnings = String::from_utf8(output.stderr).unwrap();
    assert!(
        !warnings
            .lines()
            .any(|line| line.contains("multiple matching instances")
                && line.contains("address=\"terraform_data.consumer\"")
                && line.contains("attribute=\"input\"")),
        "{warnings}"
    );
}

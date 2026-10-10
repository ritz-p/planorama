mod support;

#[test]
fn vpc_workloads_keep_subnet_references_and_show_derived_placement() {
    let svg = render(include_bytes!("../fixtures/vpc-workloads-plan.json"));
    assert_eq!(svg.matches("data-placement=\"inferred\"").count(), 3);
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 5);
    assert_eq!(svg.matches("data-edge-kind=\"dependency\"").count(), 6);
    assert_eq!(
        svg,
        include_str!("../../examples/vpc-workloads.svg").replace("\r\n", "\n")
    );
}

#[test]
fn inferred_placement_notes_use_provenance_and_fit_below_long_addresses() {
    let input = include_bytes!("../fixtures/subnet-groups-plan.json");
    let svg = render(input);
    assert_eq!(svg.matches("data-placement=\"inferred\"").count(), 2);
    assert_eq!(
        svg,
        include_str!("../../examples/subnet-groups.svg").replace("\r\n", "\n")
    );
    let direct = render(include_bytes!("../fixtures/network-scope-plan.json"));
    assert!(!direct.contains("inferred placement"));
    let long_input = String::from_utf8(input.to_vec()).unwrap().replace(
        "aws_db_subnet_group.main",
        "aws_db_subnet_group.very_long_resource_address_that_requires_two_lines_and_truncation",
    );
    let svg = render(long_input.as_bytes());
    let doc = roxmltree::Document::parse(&svg).unwrap();
    for note in doc
        .descendants()
        .filter(|n| n.attribute("data-placement") == Some("inferred"))
    {
        let card = note.parent().unwrap();
        let rect = card.children().find(|n| n.has_tag_name("rect")).unwrap();
        let y = |n: roxmltree::Node| n.attribute("y").unwrap().parse::<usize>().unwrap();
        let height = rect.attribute("height").unwrap().parse::<usize>().unwrap();
        assert!(y(note) >= y(rect) + 80);
        assert!(y(note) < y(rect) + height - 12);
        assert!(
            card.children()
                .filter(|n| n.has_tag_name("text") && n.attribute("font-size") == Some("13"))
                .all(|n| y(n) + 10 < y(note))
        );
    }
}

#[test]
fn relationship_patterns_are_distinct_and_nesting_stays_line_free() {
    for (input, kind, width, dash) in [
        (
            include_bytes!("../../examples/plan.json").as_slice(),
            "dependency",
            "1.5",
            None,
        ),
        (
            include_bytes!("../fixtures/association-plan.json").as_slice(),
            "association",
            "2",
            Some("7 4"),
        ),
        (
            include_bytes!("../fixtures/security-groups-plan.json").as_slice(),
            "connection",
            "2.5",
            Some("2 4"),
        ),
    ] {
        let svg = render(input);
        let paths: Vec<_> = svg
            .lines()
            .filter(|line| {
                line.starts_with("<path ") && line.contains(&format!("data-edge-kind=\"{kind}\""))
            })
            .collect();
        assert!(!paths.is_empty());
        for path in paths {
            assert!(path.contains(&format!("stroke-width=\"{width}\"")));
            match dash {
                Some(dash) => assert!(path.contains(&format!("stroke-dasharray=\"{dash}\""))),
                None => assert!(!path.contains("stroke-dasharray")),
            }
        }
        assert!(
            svg.contains("Thin solid: dependency. Dashed: association. Thick dotted: connection.")
        );
    }
    let svg = render(include_bytes!("../fixtures/containment-plan.json"));
    assert!(svg.contains("<g data-edge-kind=\"containment\""));
    assert!(
        !svg.lines()
            .any(|line| line.starts_with("<path ")
                && line.contains("data-edge-kind=\"containment\""))
    );
}

#[test]
fn resource_ids_survive_reordering_insertions_and_label_changes() {
    let addresses = [
        "module.app[0].test_item.x[\"a/b & 日本語\"]",
        "test_item.x[0]",
        "test_item.x[1]",
    ];
    let mut resources: Vec<_> = addresses
        .iter()
        .map(|address| {
            serde_json::json!({
                "address": address, "type": "test_item", "mode": "managed",
                "change": {"actions": ["no-op"]}
            })
        })
        .collect();
    let mut deposed = resources[0].clone();
    deposed["deposed"] = "old/key".into();
    resources.push(deposed);
    let make = |resources: &Vec<serde_json::Value>| {
        serde_json::json!({"format_version":"1.2","resource_changes":resources}).to_string()
    };
    let ids = |svg: &str| {
        svg.lines()
            .filter_map(|line| line.strip_prefix("<g id=\"resource-"))
            .map(|line| line.split('"').next().unwrap().to_owned())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let original = ids(&render(make(&resources).as_bytes()));
    assert_eq!(original.len(), 4);
    assert!(
        original
            .iter()
            .all(|id| id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    );
    resources.reverse();
    assert_eq!(original, ids(&render(make(&resources).as_bytes())));
    resources.push(
        serde_json::json!({"address":"aaa.extra","type":"aaa","change":{"actions":["create"]}}),
    );
    let changed = support::run_with_args(
        make(&resources).as_bytes(),
        &["--address-format", "terraform"],
    );
    assert!(changed.status.success());
    let inserted = ids(&String::from_utf8(changed.stdout).unwrap());
    assert!(original.is_subset(&inserted));
    assert_eq!(inserted.len(), 5);
}

#[test]
fn synthetic_components_are_distinct_and_link_to_original_changes() {
    let input = include_bytes!("../fixtures/components-plan.json");
    let svg = render(input);
    assert_eq!(svg.matches("data-entity-kind=\"synthetic\"").count(), 1);
    assert_eq!(
        svg.matches("data-component-id=\"logical:load_balancer:aws_lb.app\"")
            .count(),
        4
    );
    assert!(svg.contains("Logical component: Load balancer: aws_lb.app"));
    assert!(svg.contains("update: aws_lb_listener.https"));
    assert!(svg.contains("replace: aws_lb_listener_rule.api"));
    assert!(svg.contains("data-previous-address=\"aws_lb_listener_rule.old\""));
    assert_eq!(svg.matches("data-source-address=").count(), 3);
    assert!(!svg.contains("data-container="));
    assert!(svg.contains("unchanged (1)"));
    assert!(svg.contains("create (2)"));
    assert!(svg.contains("id=\"plan-checks\""));
    assert!(svg.contains("id=\"plan-status\""));
    assert!(!svg.contains("TOP_SECRET"));
    assert_eq!(svg, render(input));
}

#[test]
fn referenced_ids_with_different_planned_values_keep_route_cards() {
    let mut input: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/routes-plan.json")).unwrap();
    input["resource_changes"][2]["change"]["after"]["gateway_id"] =
        "TOP_SECRET_OTHER_GATEWAY".into();
    let bytes = input.to_string();
    let output = support::run_with_args(bytes.as_bytes(), &["--diagnostics"]);
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!svg.contains("association; replace: aws_route.default"));
    assert!(svg.contains("data-terraform-address=\"aws_route.default\""));
    assert!(svg.contains("association; create: aws_route.second"));
    assert!(stderr.contains("planned endpoint value is not proven"));
    assert!(!format!("{svg}{stderr}").contains("TOP_SECRET"));
}

#[test]
fn route_fallback_diagnostics_explain_rejections_without_exposing_values() {
    for (case, reason) in [
        (0, "attribute has no resolvable resource reference"),
        (1, "unsupported route target"),
        (2, "missing expected attribute"),
        (3, "additional relationships prevent association lowering"),
        (4, "multiple route target attributes"),
    ] {
        let mut input: serde_json::Value =
            serde_json::from_slice(include_bytes!("../fixtures/routes-plan.json")).unwrap();
        let expressions = &mut input["configuration"]["root_module"]["resources"][0]["expressions"];
        match case {
            0 => expressions["gateway_id"] = serde_json::json!({"constant_value":"TOP_SECRET"}),
            1 => {
                expressions.as_object_mut().unwrap().remove("gateway_id");
                expressions["vpc_endpoint_id"] = serde_json::json!({"constant_value":"TOP_SECRET"});
            }
            2 => {
                expressions
                    .as_object_mut()
                    .unwrap()
                    .remove("route_table_id");
            }
            3 => {
                input["configuration"]["root_module"]["resources"][1]["expressions"]["extra"] =
                    serde_json::json!({"references":["aws_route.default.id"]})
            }
            _ => expressions["nat_gateway_id"] = serde_json::json!({"constant_value":"TOP_SECRET"}),
        }
        let bytes = input.to_string();
        let output = support::run_with_args(bytes.as_bytes(), &["--diagnostics"]);
        assert!(output.status.success());
        assert_eq!(output.stdout, render(bytes.as_bytes()).as_bytes());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr
                .lines()
                .any(|line| line.contains("aws_route.default") && line.contains(reason)),
            "{stderr}"
        );
        assert!(!stderr.contains("TOP_SECRET"));
    }
    let output = support::run_with_args(
        include_bytes!("../fixtures/routes-plan.json"),
        &["--diagnostics"],
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr
            .lines()
            .any(|line| line.contains("aws_route.default") || line.contains("aws_route.second"))
    );
}

#[test]
fn routes_render_as_individually_inspectable_relationships() {
    let input = include_bytes!("../fixtures/routes-plan.json");
    let svg = render(input);
    assert!(svg.contains("association; replace: aws_route.default"));
    assert!(svg.contains("association; create: aws_route.second"));
    assert!(svg.contains("aws_route.literal"));
    assert_eq!(svg.matches("data-edge-kind=\"association\"").count(), 2);
    assert!(svg.contains("data-action-reason=\"replace_because_cannot_update\""));
    assert!(!svg.contains("TOP_SECRET"));
    assert_eq!(svg, render(input));
}

#[test]
fn drift_relevance_exposes_safe_paths_and_unresolved_diagnostics() {
    let mut input: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/drift-plan.json")).unwrap();
    input["relevant_attributes"] = serde_json::json!([
        {"resource":"terraform_data.planned","attribute":["network",0,"id"]},
        {"resource":"terraform_data.missing","attribute":["TOP_SECRET"]}
    ]);
    let bytes = input.to_string();
    let output = support::run_with_args(bytes.as_bytes(), &["--diagnostics"]);
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    let diagnostics = String::from_utf8(output.stderr).unwrap();
    assert_eq!(svg.matches("data-plan-relevant=\"true\"").count(), 1);
    assert!(svg.contains("data-relevant-attributes=\"[[&quot;network&quot;,0,&quot;id&quot;]]\""));
    assert!(svg.contains("data-drift=\"delete\""));
    assert!(diagnostics.contains("matched=false"));
    assert!(!format!("{svg}{diagnostics}").contains("TOP_SECRET"));
}

#[test]
fn check_summary_preserves_actions_and_redacts_evaluated_messages() {
    let input = include_bytes!("../fixtures/checks-plan.json");
    let svg = render(input);
    assert!(svg.contains(
        "Checks (objects): 1 pass / 1 fail / 1 error / 1 unknown / 1 other / 0 unavailable"
    ));
    assert!(svg.contains("terraform_data.app[0]: fail"));
    assert!(svg.contains("terraform_data.app[1]: pass"));
    assert!(svg.contains("future&lt;&amp;status"));
    assert!(svg.contains("update (1)"));
    assert!(svg.contains("unchanged (1)"));
    assert!(svg.contains("id=\"plan-status\""));
    assert!(svg.contains("transform=\"translate(0 24)\""));
    let output = support::run_with_args(input, &["--diagnostics"]);
    assert!(output.status.success());
    assert_eq!(svg.as_bytes(), output.stdout);
    assert!(!svg.contains("TOP_SECRET"));
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("TOP_SECRET")
    );
    let mut plan: serde_json::Value = serde_json::from_slice(input).unwrap();
    plan["checks"].as_array_mut().unwrap().reverse();
    plan["checks"][4]["instances"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(render(plan.to_string().as_bytes()), svg);
    let empty =
        render(br#"{"format_version":"1.2","resource_changes":[],"checks":[{"status":"error"}]}"#);
    assert!(empty.contains("0 pass / 0 fail / 1 error"));
    assert!(empty.contains("No resources to display"));
}

#[test]
fn sensitive_replacement_keys_do_not_reach_cards_edges_or_diagnostics() {
    for fixture in [
        include_bytes!("../fixtures/replacement-reasons-plan.json").as_slice(),
        include_bytes!("../fixtures/association-plan.json").as_slice(),
    ] {
        for field in ["before_sensitive", "after_sensitive"] {
            let mut input: serde_json::Value = serde_json::from_slice(fixture).unwrap();
            for resource in input["resource_changes"].as_array_mut().unwrap() {
                resource["change"]["replace_paths"] =
                    serde_json::json!([["tokens", "TOP_SECRET_PATH_KEY"], ["public_attribute"]]);
                resource["change"][field] = serde_json::json!({"tokens":true});
            }
            let result = support::run_with_args(input.to_string().as_bytes(), &["--diagnostics"]);
            assert!(result.status.success());
            let svg = String::from_utf8(result.stdout).unwrap();
            assert!(!svg.contains("TOP_SECRET"));
            assert!(
                !String::from_utf8(result.stderr)
                    .unwrap()
                    .contains("TOP_SECRET")
            );
            assert!(svg.contains("data-replace-paths=\"[[&quot;public_attribute&quot;]]\""));
        }
    }
}

#[test]
fn replacement_reasons_are_escaped_and_visible_on_cards_and_lowered_edges() {
    let input = include_bytes!("../fixtures/replacement-reasons-plan.json");
    let svg = render(input);
    assert!(svg.contains("reason: replace_because_tainted"));
    assert!(svg.contains("replacement paths: [[&quot;subnet_id&quot;]"));
    assert!(svg.contains("future&lt;&amp;reason"));
    assert!(!svg.contains("future<&reason"));
    assert!(!svg.contains("TOP_SECRET"));
    assert!(svg.contains("replace (4)"));
    assert_eq!(svg, render(input));
    let mut input: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/association-plan.json")).unwrap();
    input["resource_changes"][2]["action_reason"] =
        serde_json::json!("replace_because_cannot_update");
    input["resource_changes"][2]["change"] =
        serde_json::json!({"actions":["delete","create"],"replace_paths":[["subnet_id"]]});
    let svg = render(input.to_string().as_bytes());
    assert!(svg.contains("data-action-reason=\"replace_because_cannot_update\""));
    assert!(svg.contains("data-replace-paths=\"[[&quot;subnet_id&quot;]]\""));
    assert!(svg.contains("association; replace: aws_route_table_association.private"));
    assert!(svg.contains("replacement paths:"));
}

#[test]
fn plan_status_survives_lowering_without_changing_resource_rendering() {
    let mut input: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/association-plan.json")).unwrap();
    for key in ["applyable", "complete", "errored"] {
        input.as_object_mut().unwrap().remove(key);
    }
    let baseline = render(input.to_string().as_bytes());
    assert!(!baseline.contains("id=\"plan-status\""));
    for (applyable, complete, errored) in [
        (true, true, false),
        (true, false, false),
        (false, false, true),
        (false, true, false),
    ] {
        input["applyable"] = applyable.into();
        input["complete"] = complete.into();
        input["errored"] = errored.into();
        let bytes = input.to_string();
        let svg = render(bytes.as_bytes());
        assert_eq!(svg, render(bytes.as_bytes()));
        for (key, value) in [
            ("applyable", applyable),
            ("complete", complete),
            ("errored", errored),
        ] {
            assert!(svg.contains(&format!("data-plan-{key}=\"{value}\"")));
            assert!(svg.contains(&format!("{key}: {value}")));
        }
        let start = svg.find("<g id=\"plan-status\"").unwrap();
        let end = start + svg[start..].find("</g>\n").unwrap() + "</g>\n".len();
        assert_eq!(format!("{}{}", &svg[..start], &svg[end..]), baseline);
    }
    let svg = render(br#"{"format_version":"1.2","resource_changes":[],"complete":false}"#);
    assert!(svg.contains("applyable: unknown complete: false errored: unknown"));
    assert!(!svg.contains("data-plan-applyable"));
    assert!(!svg.contains("data-plan-errored"));
}

#[test]
fn mixed_literal_security_group_ids_keep_dependency_edges_and_redacted_diagnostics() {
    let mut input: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/security-groups-plan.json")).unwrap();
    let change = &mut input["resource_changes"][4]["change"];
    change["after"] = serde_json::json!({"vpc_security_group_ids":[null,"TOP_SECRET_LITERAL"]});
    change["after_unknown"] = serde_json::json!({"vpc_security_group_ids":[true,false]});
    let bytes = input.to_string();
    let svg = render(bytes.as_bytes());
    assert_eq!(svg.matches("data-edge-kind=\"connection\"").count(), 2);
    assert!(svg.contains("aws_security_group.web → aws_instance.app (dependency)"));
    let result = support::run_with_args(bytes.as_bytes(), &["--diagnostics"]);
    assert!(result.status.success());
    assert_eq!(svg.as_bytes(), result.stdout);
    let diagnostics = String::from_utf8(result.stderr).unwrap();
    assert!(diagnostics.contains("only part of the attribute resolves to resource references"));
    assert!(!diagnostics.contains("TOP_SECRET"));
    assert!(!svg.contains("TOP_SECRET"));
}

#[test]
fn security_groups_fixture_preserves_policy_cards_and_inspectable_connections() {
    let input = include_bytes!("../fixtures/security-groups-plan.json");
    let svg = render(input);
    assert!(svg.contains("7 resources (7 cards), 10 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"connection\"").count(), 4);
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 6);
    for (group, target) in [
        ("web", "aws_instance.app"),
        ("shared", "aws_instance.app"),
        ("web", "aws_lb.app"),
        ("shared", "aws_ecs_service.app"),
    ] {
        assert!(svg.contains(&format!(
            "aws_security_group.{group} → {target} (connection)"
        )));
        assert!(svg.contains(&format!(
            "data-terraform-address=\"aws_security_group.{group}\""
        )));
    }
    assert_eq!(svg, render(input));
}

#[test]
fn drift_fixture_keeps_apply_actions_and_reports_drift_without_values() {
    let input = include_bytes!("../fixtures/drift-plan.json");
    let svg = render(input);
    assert!(svg.contains("data-drift=\"delete\""));
    assert!(svg.contains("data-drift=\"update\""));
    assert!(svg.contains("create (1)"));
    assert!(svg.contains("delete (0)"));
    assert!(!svg.contains("TOP_SECRET"));
    let output = support::run_with_args(input, &["--diagnostics"]);
    assert!(output.status.success());
    assert_eq!(svg.as_bytes(), output.stdout);
    let diagnostics = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        diagnostics
            .matches("Terraform-reported resource drift")
            .count(),
        2
    );
    assert!(!diagnostics.contains("TOP_SECRET"));
    assert_eq!(svg, render(input));
}

#[test]
fn lifecycle_fixture_distinguishes_import_forget_and_destroy_without_values() {
    let input = include_bytes!("../fixtures/lifecycle-plan.json");
    let svg = render(input);
    assert_eq!(svg.matches("data-import=\"true\"").count(), 2);
    assert!(svg.contains("data-state-removal=\"forget\""));
    assert!(svg.contains("data-state-removal=\"create then forget\""));
    assert!(svg.contains("import / update"));
    assert!(svg.contains("delete (1)"));
    assert!(!svg.contains("TOP_SECRET"));
    let diagnostic = support::run_with_args(input, &["--diagnostics"]);
    assert!(diagnostic.status.success());
    assert!(!String::from_utf8_lossy(&diagnostic.stderr).contains("TOP_SECRET"));
    assert_eq!(svg.as_bytes(), diagnostic.stdout);
    assert_eq!(svg, render(input));
}

#[test]
fn moved_metadata_survives_cards_and_lowered_relationships_and_is_escaped() {
    let mut input: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/association-plan.json")).unwrap();
    input["resource_changes"][0]["previous_address"] =
        serde_json::json!("aws_subnet.old[\"<a>&\"]");
    input["resource_changes"][2]["previous_address"] =
        serde_json::json!("module.old.aws_route_table_association.private");
    let svg = render(input.to_string().as_bytes());
    assert!(svg.contains("data-previous-address=\"aws_subnet.old[&quot;&lt;a&gt;&amp;&quot;]\""));
    assert!(
        svg.contains("data-previous-address=\"module.old.aws_route_table_association.private\"")
    );
    assert_eq!(svg.matches("data-previous-address=").count(), 2);
    assert!(svg.contains("data-terraform-address=\"aws_subnet.private\""));
    assert!(!svg.contains("<a>"));
}

#[test]
fn deposed_fixture_preserves_every_change_card_and_action() {
    let input = include_bytes!("../fixtures/deposed-plan.json");
    let svg = render(input);
    assert_eq!(svg.matches("<g id=\"resource-").count(), 4);
    assert_eq!(svg.matches("data-deposed-key=").count(), 2);
    assert!(svg.contains("delete (2)"));
    assert!(svg.contains("create (1)"));
    assert_eq!(svg, render(input));
}

#[test]
fn committed_svg_samples_match_the_current_renderer() {
    for (input, expected) in [
        (
            include_bytes!("../fixtures/components-plan.json").as_slice(),
            include_str!("../../examples/components.svg"),
        ),
        (
            include_bytes!("../fixtures/routes-plan.json").as_slice(),
            include_str!("../../examples/routes.svg"),
        ),
        (
            include_bytes!("../fixtures/checks-plan.json").as_slice(),
            include_str!("../../examples/checks.svg"),
        ),
        (
            include_bytes!("../fixtures/spanning-dense-plan.json").as_slice(),
            include_str!("../../examples/spanning-dense.svg"),
        ),
        (
            include_bytes!("../fixtures/dense-architecture-plan.json").as_slice(),
            include_str!("../../examples/dense-architecture.svg"),
        ),
        (
            include_bytes!("../../examples/plan.json").as_slice(),
            include_str!("../../examples/diagram.svg"),
        ),
        (
            include_bytes!("../../examples/bundling-plan.json").as_slice(),
            include_str!("../../examples/bundling.svg"),
        ),
        (
            include_bytes!("../fixtures/terraform-plan.json").as_slice(),
            include_str!("../../examples/terraform.svg"),
        ),
        (
            include_bytes!("../fixtures/association-plan.json").as_slice(),
            include_str!("../../examples/association.svg"),
        ),
        (
            include_bytes!("../fixtures/containment-plan.json").as_slice(),
            include_str!("../../examples/containment.svg"),
        ),
        (
            include_bytes!("../../examples/data-plan.json").as_slice(),
            include_str!("../../examples/data.svg"),
        ),
    ] {
        assert_eq!(render(input), expected.replace("\r\n", "\n"));
    }
}

#[test]
fn cli_emits_containment_metadata_without_dropping_resources_or_dependencies() {
    let svg = render(include_bytes!("../fixtures/containment-plan.json"));
    assert!(svg.contains("3 resources (3 cards), 3 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 2);
    assert_eq!(svg.matches("marker-end=").count(), 0);
    assert_eq!(svg.matches("data-container=").count(), 2);
}

#[test]
fn association_resources_are_lowered_by_the_cli() {
    let svg = render(include_bytes!("../fixtures/association-plan.json"));
    assert!(svg.contains("3 resources (2 cards), 1 relationships"));
    assert!(svg.contains("aws_subnet.private → aws_route_table.private"));
    assert!(!svg.contains("<title>aws_route_table_association.private</title>"));
    assert!(svg.contains("association; create: aws_route_table_association.private"));
    assert!(svg.contains("create (3)"));
    assert!(!svg.contains("reference edges"));
    assert!(!svg.contains("dependency → dependent"));
    assert!(!svg.contains("Arrows point from dependencies"));
    assert!(svg.contains("data-edge-kind=\"association\""));
}

fn render(input: &[u8]) -> String {
    let output = support::run(input);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn output_is_deterministic_and_empty_plans_render() {
    let input = include_bytes!("../../examples/plan.json");
    assert_eq!(render(input), render(input));
    assert!(render(br#"{"format_version":"1.0","resource_changes":[]}"#).contains("No resources"));
}

#[test]
fn escapes_untrusted_labels_and_omits_values() {
    let input = br#"{"format_version":"1.0","resource_changes":[{"address":"test.<script>&\"","type":"test","change":{"actions":["create"],"after":{"password":"TOP_SECRET"}}}]}"#;
    let svg = render(input);
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("&lt;script&gt;&amp;&quot;"));
    assert!(!svg.contains("TOP_SECRET"));
}

#[test]
fn bundled_fixture_preserves_individual_dependency_arrows() {
    let svg = render(include_bytes!("../../examples/bundling-plan.json"));
    assert!(svg.contains("5 resources, 6 reference edges"));
    assert_eq!(svg.matches("marker-end=").count(), 6);
    assert!(svg.contains("<circle "));
    for name in ["a", "b", "c"] {
        assert!(svg.contains(&format!(
            "<title>terraform_data.seed → terraform_data.{name}</title>"
        )));
        assert!(svg.contains(&format!(
            "<title>terraform_data.{name} → terraform_data.aggregate</title>"
        )));
    }
}
#[test]
fn multi_container_fixture_renders_one_card_per_resource_and_subnet_connections() {
    let output = support::run(include_bytes!("../fixtures/multi-container-plan.json"));
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    assert_eq!(svg.matches("<g id=\"resource-").count(), 6);
    assert_eq!(svg.matches("data-edge-kind=\"connection\"").count(), 5);
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 4);
    assert_eq!(
        svg,
        include_str!("../../examples/multi-container.svg").replace("\r\n", "\n")
    );
}

#[test]
fn explicit_network_scopes_render_as_nested_architecture() {
    let input = include_bytes!("../fixtures/network-scope-plan.json");
    let result = support::run(input);
    assert!(result.status.success());
    let svg = String::from_utf8(result.stdout).unwrap();
    assert!(svg.contains("7 resources (7 cards), 6 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 6);
    assert_eq!(svg.matches("marker-end=").count(), 0);
    assert_eq!(svg.as_bytes(), support::run(input).stdout);
}

#[test]
fn rds_subnet_group_fixture_preserves_cards_and_infers_common_scope() {
    let input = include_bytes!("../fixtures/rds-subnet-group-plan.json");
    let svg = render(input);
    assert!(svg.contains("6 resources (6 cards), 9 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 5);
    assert_eq!(svg.matches("<g id=\"resource-").count(), 6);
    assert_eq!(svg, render(input));
}

#[test]
fn mixed_aws_relationships_render_actions_without_helper_cards() {
    let input = include_bytes!("../fixtures/aws-relationships-plan.json");
    let svg = render(input);
    assert!(svg.contains("9 resources (6 cards), 7 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"association\"").count(), 3);
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 4);
    for (kind, action) in [
        ("aws_route_table_association", "create"),
        ("aws_lb_target_group_attachment", "replace"),
        ("aws_vpc_endpoint_route_table_association", "delete"),
    ] {
        assert!(!svg.contains(&format!("<title>{kind}.main</title>")));
        assert!(svg.contains(&format!("association; {action}: {kind}.main")));
        assert!(svg.contains(&format!("{action} (1)")));
    }
    assert_eq!(svg, render(input));
}

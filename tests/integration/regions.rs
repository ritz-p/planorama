use super::render;

#[test]
fn verified_region_overrides_survive_selection_and_rendering() {
    for input in [
        include_bytes!("../fixtures/aws-region-overrides/plan.json").as_slice(),
        include_bytes!("../fixtures/compatibility/1.16.6/overrides/plan.json").as_slice(),
    ] {
        for args in [
            vec![],
            vec!["--changed-only"],
            vec!["--format", "html"],
            vec!["--focus", "aws_subnet.paris", "--focus-depth", "0"],
            vec![
                "--focus",
                "aws_cloudwatch_log_group.paris",
                "--focus-depth",
                "0",
            ],
        ] {
            let output = super::support::run_with_args(input, &args);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(text.contains("eu-west-3"));
            if args.contains(&"aws_cloudwatch_log_group.paris") {
                assert!(text.contains("data-region=\"eu-west-3\""));
                assert!(text.contains("data-deployment-scope="));
            } else {
                assert!(text.contains("data-region=\"eu-west-3\""));
                assert!(text.contains("aws_vpc.paris") && text.contains("aws_subnet.paris"));
            }
        }
        let mut value: serde_json::Value = serde_json::from_slice(input).unwrap();
        value["resource_changes"][1]["change"]["after"]["region"] = serde_json::json!("us-west-2");
        value["configuration"]["root_module"]["resources"][1]["expressions"]["region"]["constant_value"] =
            serde_json::json!("us-west-2");
        let svg = render(value.to_string().as_bytes());
        assert!(svg.contains("aws_vpc.paris") && svg.contains("aws_subnet.paris"));
        assert!(svg.contains("data-region=\"eu-west-3\""));
        assert!(!svg.contains("data-region=\"us-west-2\""));
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let subnet = doc
            .descendants()
            .find(|n| n.attribute("data-terraform-address") == Some("aws_subnet.paris"))
            .unwrap();
        assert!(
            !subnet
                .ancestors()
                .any(|n| n.attribute("data-region").is_some())
        );
    }
}

#[test]
fn verified_region_overrides_remain_independent_across_states_and_modules() {
    let path = format!(
        "{}/tests/fixtures/aws-region-overrides/plan.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let additional_path = format!(
        "{}/tests/fixtures/compatibility/1.16.6/overrides/plan.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"))
        .args([
            "--state",
            &format!("first={path}"),
            "--state",
            &format!("second={additional_path}"),
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let doc = roxmltree::Document::parse(&text).unwrap();
    for id in ["first", "second"] {
        let state = doc
            .descendants()
            .find(|n| n.attribute("data-state-id") == Some(id))
            .unwrap();
        assert_eq!(
            state
                .descendants()
                .filter(|n| n.attribute("data-region") == Some("eu-west-3"))
                .count(),
            1
        );
        assert!(
            state
                .descendants()
                .any(|n| n.text() == Some("AWS eu-west-3"))
        );
    }
    for input in [
        include_str!("../fixtures/aws-region-overrides/plan.json"),
        include_str!("../fixtures/compatibility/1.16.6/overrides/plan.json"),
    ] {
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        for resource in value["resource_changes"].as_array_mut().unwrap() {
            resource["address"] = serde_json::json!(format!(
                "module.regional.{}",
                resource["address"].as_str().unwrap()
            ));
        }
        value.as_object_mut().unwrap().remove("planned_values");
        let module = value["configuration"]["root_module"].take();
        value["configuration"]["root_module"] =
            serde_json::json!({"module_calls":{"regional":{"module":module}}});
        let output = super::support::run_with_args(
            value.to_string().as_bytes(),
            &["--modules", "module.regional"],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("data-region=\"eu-west-3\"") && text.contains("AWS eu-west-3"));
    }
}

#[test]
fn mixed_logical_and_log_group_regions_keep_separate_state_ids_in_svg_and_html() {
    let path = format!(
        "{}/tests/fixtures/logical-regions-plan.json",
        env!("CARGO_MANIFEST_DIR")
    );
    for format in ["svg", "html"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"))
            .args([
                "--state",
                &format!("first={path}"),
                "--state",
                &format!("second={path}"),
                "--format",
                format,
                "-o",
                "-",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let start = text.find("<svg").unwrap();
        let end = text.rfind("</svg>").unwrap() + 6;
        let doc = roxmltree::Document::parse(&text[start..end]).unwrap();
        let mut ids = std::collections::BTreeSet::new();
        for state in ["first", "second"] {
            let section = doc
                .descendants()
                .find(|n| n.attribute("data-state-id") == Some(state))
                .unwrap();
            let rows: Vec<_> = section
                .descendants()
                .filter(|n| n.attribute("data-terraform-address").is_some())
                .collect();
            assert_eq!(rows.len(), 7);
            assert_eq!(
                section
                    .descendants()
                    .filter(|n| n.attribute("data-logical-group").is_some())
                    .count(),
                1
            );
            for row in rows {
                assert!(ids.insert(row.attribute("id").unwrap()));
                assert_eq!(
                    row.attribute("data-placement") == Some("logical"),
                    row.attribute("data-terraform-address") == Some("aws_iam_policy.access")
                );
            }
            assert_eq!(
                section
                    .descendants()
                    .filter(|n| n.attribute("data-region") == Some("ap-northeast-1"))
                    .count(),
                1
            );
        }
        assert_eq!(ids.len(), 14);
        assert!(!text.contains("PRIVATE_") && !text.contains("eu-west-3"));
    }
}

#[test]
fn region_panels_are_deterministic_and_resources_are_not_duplicated() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/regions-plan.json")).unwrap();
    let svg = render(input.to_string().as_bytes());
    let document = roxmltree::Document::parse(&svg).unwrap();
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.attribute("data-deployment-scope").is_some())
            .count(),
        3
    );
    for address in input["resource_changes"].as_array().unwrap() {
        assert_eq!(
            document
                .descendants()
                .filter(|n| n.attribute("data-terraform-address") == address["address"].as_str())
                .count(),
            1
        );
    }
    assert!(
        svg.contains("AWS ap-northeast-1")
            && svg.contains("AWS us-east-1")
            && svg.contains("AWS Global")
    );
    assert!(!svg.contains("us-west-2"));
    input["resource_changes"].as_array_mut().unwrap().reverse();
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(svg, render(input.to_string().as_bytes()));
    assert_eq!(
        svg,
        include_str!("../../examples/regions.svg").replace("\r\n", "\n")
    );
}

#[test]
fn named_states_keep_distinct_region_panels() {
    let path = format!(
        "{}/tests/fixtures/regions-plan.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"))
        .args([
            "--state",
            &format!("first={path}"),
            "--state",
            &format!("second={path}"),
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    let document = roxmltree::Document::parse(&svg).unwrap();
    for id in ["first", "second"] {
        let state = document
            .descendants()
            .find(|n| n.attribute("data-state-id") == Some(id))
            .unwrap();
        assert_eq!(
            state
                .descendants()
                .filter(|n| n.attribute("data-region") == Some("ap-northeast-1"))
                .count(),
            1
        );
        assert_eq!(
            state
                .descendants()
                .filter(|n| n.attribute("data-deployment-scope").is_some())
                .count(),
            3
        );
    }
}

#[test]
fn regional_state_examples_include_singletons_in_svg_and_html() {
    for inputs in [
        vec![
            ("first", "single-region", 1),
            ("second", "single-region", 1),
        ],
        vec![
            ("application", "application", 2),
            ("network", "network", 3),
            ("singleton", "single-region", 1),
        ],
    ] {
        let render = |format| {
            let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"));
            for (state, fixture, _) in &inputs {
                command.args([
                    "--state",
                    &format!(
                        "{state}={}/tests/fixtures/multi/{fixture}.json",
                        env!("CARGO_MANIFEST_DIR")
                    ),
                ]);
            }
            let output = command
                .args(["--format", format, "-o", "-"])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).unwrap()
        };
        let svg = render("svg");
        assert!(render("html").contains(&svg));
        let document = roxmltree::Document::parse(&svg).unwrap();
        let number =
            |node: roxmltree::Node, name| node.attribute(name).unwrap().parse::<usize>().unwrap();
        let mut identities = std::collections::BTreeSet::new();
        for (state_id, _, count) in &inputs {
            let state = document
                .descendants()
                .find(|node| node.attribute("data-state-id") == Some(*state_id))
                .unwrap();
            let panels: Vec<_> = state
                .descendants()
                .filter(|node| node.attribute("data-deployment-scope").is_some())
                .collect();
            assert_eq!(panels.len(), 1);
            let panel = panels[0];
            assert_eq!(panel.attribute("data-region"), Some("ap-northeast-1"));
            assert_eq!(panel.attribute("data-scope-provider"), Some("AWS"));
            let frame = panel
                .children()
                .find(|node| node.has_tag_name("rect"))
                .unwrap();
            let cards: Vec<_> = state
                .descendants()
                .filter(|node| node.attribute("data-terraform-address").is_some())
                .collect();
            assert_eq!(cards.len(), *count);
            for card in cards {
                assert!(identities.insert(card.attribute("id").unwrap()));
                assert!(panel.range().start < card.range().start);
                let bounds = card
                    .children()
                    .find(|node| node.has_tag_name("rect"))
                    .unwrap();
                if card.attribute("data-deposed-key").is_some() {
                    assert!(number(bounds, "y") >= number(frame, "y") + number(frame, "height"));
                    continue;
                }
                assert!(number(bounds, "x") >= number(frame, "x"));
                assert!(number(bounds, "y") >= number(frame, "y") + 36);
                assert!(
                    number(bounds, "x") + number(bounds, "width")
                        <= number(frame, "x") + number(frame, "width")
                );
                assert!(
                    number(bounds, "y") + number(bounds, "height")
                        <= number(frame, "y") + number(frame, "height")
                );
            }
            for path in state.descendants().filter(|node| {
                node.has_tag_name("path") && node.attribute("data-edge-kind").is_some()
            }) {
                assert!(panel.range().start < path.range().start);
            }
            let nested = state
                .children()
                .find(|node| node.has_tag_name("svg"))
                .unwrap();
            assert!(number(frame, "x") + number(frame, "width") <= number(nested, "width"));
            assert!(
                number(nested, "y") + number(nested, "height")
                    <= number(document.root_element(), "height")
            );
        }
        assert_eq!(
            document
                .descendants()
                .filter(|node| node.attribute("data-deployment-scope").is_some())
                .count(),
            inputs.len()
        );
        if inputs.len() == 3 {
            assert_eq!(
                svg,
                include_str!("../../examples/regional-states.svg").replace("\r\n", "\n")
            );
            assert_eq!(svg.matches("id=\"relationship-legend\"").count(), 1);
        }
    }
    let input = include_bytes!("../fixtures/multi/single-region.json");
    let svg = render(input);
    assert_eq!(svg.matches("data-region=\"ap-northeast-1\"").count(), 1);
    let html = super::support::run_with_args(input, &["--format", "html"]);
    assert!(html.status.success());
    assert!(String::from_utf8(html.stdout).unwrap().contains(&svg));
}

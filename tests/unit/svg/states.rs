use super::*;

#[test]
fn account_panels_remain_state_local_and_identical_in_html() {
    let input = include_str!("../../fixtures/accounts-plan.json");
    let graph = crate::semantic::transform(&crate::plan::parse(input).unwrap());
    let architecture = crate::model::architecture::Architecture {
        states: BTreeMap::from([
            (StateId::new("first").unwrap(), graph.clone()),
            (StateId::new("second").unwrap(), graph),
        ]),
        relationships: vec![crate::model::architecture::CrossStateEdge {
            from: Endpoint {
                state: StateId::new("first").unwrap(),
                address: "aws_s3_bucket.east".into(),
            },
            to: Endpoint {
                state: StateId::new("second").unwrap(),
                address: "aws_s3_bucket.other".into(),
            },
            provenance: crate::model::architecture::CrossStateProvenance::TerraformRemoteState {
                remote: "data.terraform_remote_state.first".into(),
                output: "bucket".into(),
                output_action: Some(crate::model::Action::Create),
            },
        }],
    };
    let viewed = crate::view::apply(&architecture, &Default::default()).unwrap();
    let svg = render_states(&viewed, super::super::AddressFormat::Qualified);
    let html = crate::html::render_states(&viewed, super::super::AddressFormat::Qualified);
    assert!(html.contains(&svg));
    let document = roxmltree::Document::parse(&svg).unwrap();
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.has_tag_name("path") && n.attribute("data-cross-state") == Some("true"))
            .count(),
        1
    );
    let mut ids = std::collections::BTreeSet::new();
    for node in document.descendants() {
        if let Some(id) = node.attribute("id") {
            assert!(ids.insert(id));
        }
    }
    for state in ["first", "second"] {
        let section = document
            .descendants()
            .find(|n| n.attribute("data-state-id") == Some(state))
            .unwrap();
        assert_eq!(
            section
                .descendants()
                .filter(|n| n.attribute("data-account-panel") == Some("true"))
                .count(),
            2
        );
        assert_eq!(
            section
                .descendants()
                .filter(|n| n.attribute("data-region") == Some("us-east-1"))
                .count(),
            3
        );
        assert!(
            section
                .descendants()
                .any(|n| n.attribute("data-global") == Some("true")
                    && n.attribute("data-account") == Some("123456789012"))
        );
    }
    let mut reordered: serde_json::Value = serde_json::from_str(input).unwrap();
    reordered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let reversed = crate::semantic::transform(&crate::plan::parse(&reordered.to_string()).unwrap());
    assert_eq!(architecture.states.values().next().unwrap(), &reversed);
}

#[test]
fn namespacing_only_rewrites_fragment_reference_attributes() {
    let input = r##"<svg><g id="paint" data-action-reason="url(#ticket)"><title>url(#ticket)</title></g><path fill="url(#paint)" stroke="url(#paint)" clip-path="url(#paint)" mask="url(#paint)" marker-end="url(#paint)"/><use href="#paint"/></svg>"##;
    let scoped = namespace(input, "state-x-");
    assert!(scoped.contains("data-action-reason=\"url(#ticket)\""));
    assert!(scoped.contains("<title>url(#ticket)</title>"));
    assert_eq!(scoped.matches("url(#state-x-paint)").count(), 5);
    assert!(scoped.contains("href=\"#state-x-paint\""));
}

#[test]
fn shared_legend_collects_later_state_actions_and_omits_empty_height() {
    use crate::model::architecture::Architecture;
    let empty = crate::semantic::transform(
        &crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[]}"#).unwrap(),
    );
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap();
    for resource in input["resource_changes"].as_array_mut().unwrap() {
        resource["change"]["actions"] = serde_json::json!(["delete"]);
    }
    let later = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap());
    let first_id = StateId::new("a").unwrap();
    let later_id = StateId::new("z").unwrap();
    for first in [empty.clone(), crate::semantic::transform(&crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.single","type":"test"}]}"#).unwrap())] {
        let architecture = Architecture {
            states: BTreeMap::from([(first_id.clone(), first), (later_id.clone(), later.clone())]),
            relationships: Vec::new(),
        };
        let viewed = crate::view::apply(&architecture, &Default::default()).unwrap();
        let svg = render_states(&viewed, super::super::AddressFormat::Qualified);
        let html = crate::html::render_states(&viewed, super::super::AddressFormat::Qualified);
        assert!(html.contains(&svg));
        let document = roxmltree::Document::parse(&svg).unwrap();
        let legends: Vec<_> = document.descendants().filter(|node| node.attribute("id") == Some("relationship-legend")).collect();
        assert_eq!(legends.len(), 1);
        let legend = legends[0];
        assert_eq!(legend.parent(), Some(document.root_element()));
        let actions: std::collections::BTreeSet<_> = legend.descendants().filter_map(|node| node.attribute("data-legend-action")).collect();
        assert_eq!(actions, ["none", "unchanged", "delete"].into_iter().collect());
        assert_eq!(legend.descendants().filter(|node| node.attribute("data-legend-entry").is_some()).count(), 5);
    }
    let architecture = Architecture {
        states: BTreeMap::from([(first_id, empty.clone()), (later_id, empty)]),
        relationships: Vec::new(),
    };
    let viewed = crate::view::apply(&architecture, &Default::default()).unwrap();
    let svg = render_states(&viewed, super::super::AddressFormat::Qualified);
    assert!(!svg.contains("relationship-legend"));
    let document = roxmltree::Document::parse(&svg).unwrap();
    let first = document
        .descendants()
        .find(|node| node.has_tag_name("svg") && node.attribute("y").is_some())
        .unwrap();
    assert_eq!(first.attribute("y"), Some("100"));
}

#[test]
fn mixed_width_state_legends_use_shared_content_width_outside_cross_state_gutters() {
    let graphs: BTreeMap<_, _> = [
        (
            "compact",
            include_str!("../../fixtures/association-plan.json"),
        ),
        ("wide", include_str!("../../fixtures/terraform-plan.json")),
    ]
    .into_iter()
    .map(|(name, input)| {
        let graph = crate::semantic::transform(&crate::plan::parse(input).unwrap());
        (StateId::new(name).unwrap(), graph)
    })
    .collect();
    let endpoint = |name: &str| {
        let state = StateId::new(name).unwrap();
        let address = graphs[&state]
            .nodes
            .iter()
            .find(|node| node.placement != crate::model::Placement::Logical)
            .unwrap()
            .address
            .clone();
        Endpoint { state, address }
    };
    let relationship = crate::model::architecture::CrossStateEdge {
        from: endpoint("compact"),
        to: endpoint("wide"),
        provenance: crate::model::architecture::CrossStateProvenance::TerraformRemoteState {
            remote: "data.terraform_remote_state.compact".into(),
            output: "value".into(),
            output_action: None,
        },
    };
    let architecture = crate::model::architecture::Architecture {
        states: graphs,
        relationships: vec![relationship],
    };
    let viewed = crate::view::apply(&architecture, &Default::default()).unwrap();
    let rendered = render_states(&viewed, super::super::AddressFormat::Qualified);
    let document = roxmltree::Document::parse(&rendered).unwrap();
    let number =
        |node: roxmltree::Node, name| node.attribute(name).unwrap().parse::<usize>().unwrap();
    let allocated_width = number(document.root_element(), "width") - 80;
    let mut maximum_width = 1120;
    for (id, graph) in &viewed.states {
        let local_layout = Layout::new(graph);
        let local = super::super::render(graph, &local_layout);
        let independent = roxmltree::Document::parse(&local).unwrap();
        let state_width = number(independent.root_element(), "width");
        maximum_width = maximum_width.max(state_width);
        let state = document
            .descendants()
            .find(|node| node.attribute("data-state-id") == Some(id.as_str()))
            .unwrap();
        let nested = state
            .children()
            .find(|node| node.has_tag_name("svg"))
            .unwrap();
        assert_eq!(number(nested, "width"), allocated_width);
        assert!(!nested.descendants().any(|node| {
            node.attribute("id")
                .is_some_and(|id| id.ends_with("relationship-legend"))
        }));
        assert!(number(nested, "height") < number(independent.root_element(), "height"));
    }
    let actual = legend_shape(document.root_element());
    assert_eq!(actual.0, allocated_width - 80);
    assert_eq!(
        document
            .descendants()
            .filter(|node| node.attribute("id") == Some("relationship-legend"))
            .count(),
        1
    );
    for sample in document.descendants().filter(|node| {
        node.attribute("data-legend-kind").is_some() && node.attribute("marker-end").is_some()
    }) {
        let marker = sample
            .attribute("marker-end")
            .unwrap()
            .strip_prefix("url(#")
            .unwrap()
            .trim_end_matches(')');
        assert!(
            document
                .descendants()
                .any(|node| node.attribute("id") == Some(marker))
        );
    }
    assert_eq!(number(document.root_element(), "width"), maximum_width + 80);
    assert_eq!(
        document
            .descendants()
            .filter(|node| node.attribute("data-cross-state") == Some("true"))
            .count(),
        1
    );
}

fn legend_shape(root: roxmltree::Node<'_, '_>) -> (usize, usize, Vec<(String, String)>) {
    let legend = root
        .descendants()
        .find(|node| {
            node.attribute("id")
                .is_some_and(|id| id.ends_with("relationship-legend"))
        })
        .unwrap();
    let rectangle = legend
        .children()
        .find(|node| node.has_tag_name("rect"))
        .unwrap();
    let entries = legend
        .descendants()
        .filter_map(|node| {
            node.attribute("data-legend-entry").map(|name| {
                (
                    name.to_owned(),
                    node.attribute("transform").unwrap().to_owned(),
                )
            })
        })
        .collect();
    (
        rectangle.attribute("width").unwrap().parse().unwrap(),
        rectangle.attribute("height").unwrap().parse().unwrap(),
        entries,
    )
}

#[test]
fn cross_state_paths_avoid_intervening_cards_and_describe_dependencies() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"aws_instance.a","type":"aws_instance"},{"address":"aws_instance.b","type":"aws_instance"}],"configuration":{"provider_config":{"aws":{"full_name":"hashicorp/aws","expressions":{"region":{"constant_value":"ap-northeast-1"}}}},"root_module":{"resources":[{"address":"aws_instance.a","provider_config_key":"aws"},{"address":"aws_instance.b","provider_config_key":"aws","expressions":{"input":{"references":["aws_instance.a.id"]}}}]}}}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw);
    graph.0.components.push(crate::model::SyntheticComponent {
        entity: crate::model::ArchitectureEntity::synthetic(
            "test",
            "group",
            graph
                .nodes
                .iter()
                .flat_map(|n| n.entity.provenance.clone())
                .collect(),
        ),
        id: "test-group".into(),
        kind: "test".into(),
        label: "Test group".into(),
        members: vec!["aws_instance.a".into(), "aws_instance.b".into()],
    });
    let first = StateId::new("first").unwrap();
    let second = StateId::new("second").unwrap();
    let local = Layout::new(&graph);
    assert_eq!(local.scopes.len(), 1);
    assert!(local.bounds[0].right() < local.bounds[1].right());
    let index = super::super::relationship_index::Index::new(&graph, &local);
    let sections = super::super::Sections::without_legend(
        &graph,
        &local,
        &index,
        super::super::AddressFormat::Qualified,
        1120,
    );
    let (_, height) = sections.dimensions(&local);
    let legend_height = super::super::legend::Legend::new(&graph, 1120).height;
    let offset = 100 + legend_height + sections.height();
    let obstacles: Vec<_> = [offset, offset + height + 60]
        .into_iter()
        .flat_map(|offset| {
            local
                .bounds
                .iter()
                .copied()
                .chain(local.scopes.iter().map(|panel| panel.bounds.header(36)))
                .map(move |b| {
                    let mut b = b;
                    b.origin.y += offset;
                    b
                })
        })
        .collect();
    let endpoint_y = offset + local.bounds[0].origin.y + local.header_heights[0] / 2;
    let mut architecture = crate::model::architecture::Architecture {
        states: BTreeMap::from([(first.clone(), graph.clone()), (second.clone(), graph)]),
        relationships: vec![crate::model::architecture::CrossStateEdge {
            from: Endpoint {
                state: first,
                address: "aws_instance.a".into(),
            },
            to: Endpoint {
                state: second,
                address: "aws_instance.a".into(),
            },
            provenance: crate::model::architecture::CrossStateProvenance::TerraformRemoteState {
                remote: "data.terraform_remote_state.x".into(),
                output: "value".into(),
                output_action: None,
            },
        }],
    };
    let svg = render_states(
        &crate::view::apply(&architecture, &Default::default()).unwrap(),
        super::super::AddressFormat::Qualified,
    );
    assert!(svg.contains("2 named states with 1 cross-state dependencies"));
    let document = roxmltree::Document::parse(&svg).unwrap();
    let number = |node: roxmltree::Node, attribute| {
        node.attribute(attribute).unwrap().parse::<usize>().unwrap()
    };
    let translated_y = |node: roxmltree::Node| {
        node.ancestors()
            .filter_map(|parent| parent.attribute("transform"))
            .map(|value| {
                value
                    .strip_prefix("translate(")
                    .unwrap()
                    .trim_end_matches(')')
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
            })
            .sum::<usize>()
    };
    for state in document
        .descendants()
        .filter(|node| node.attribute("data-state-id").is_some())
    {
        let nested = state
            .children()
            .find(|node| node.has_tag_name("svg"))
            .unwrap();
        for card in nested
            .descendants()
            .filter(|node| node.attribute("data-terraform-address").is_some())
            .flat_map(|node| node.children().filter(|node| node.has_tag_name("rect")))
        {
            let top = translated_y(card) + number(card, "y");
            assert!(top >= 148);
            assert!(top + number(card, "height") <= number(nested, "height"));
        }
    }
    let line = svg
        .lines()
        .find(|l| l.contains("data-cross-state=\"true\""))
        .unwrap();
    let path = line
        .split(" d=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let tokens: Vec<_> = path.split_whitespace().collect();
    let points: Vec<Point> = tokens
        .chunks_exact(3)
        .map(|p| Point {
            x: p[1].parse().unwrap(),
            y: p[2].parse().unwrap(),
        })
        .collect();
    assert!(points.len() > 4);
    assert_eq!(points[0].y, endpoint_y);
    assert_eq!(points.last().unwrap().y, endpoint_y + height + 60);
    for segment in points.windows(2) {
        let (a, b) = (segment[0], segment[1]);
        for obstacle in &obstacles {
            let crosses = if a.x == b.x {
                a.x > obstacle.origin.x
                    && a.x < obstacle.right()
                    && a.y.max(b.y) > obstacle.origin.y
                    && a.y.min(b.y) < obstacle.origin.y + obstacle.height
            } else {
                a.y > obstacle.origin.y
                    && a.y < obstacle.origin.y + obstacle.height
                    && a.x.max(b.x) > obstacle.origin.x
                    && a.x.min(b.x) < obstacle.right()
            };
            assert!(!crosses, "{segment:?} crosses {obstacle:?}");
        }
    }
    let mut parallel = architecture.relationships[0].clone();
    let crate::model::architecture::CrossStateProvenance::TerraformRemoteState { output, .. } =
        &mut parallel.provenance;
    *output = "second_value".into();
    architecture.relationships.push(parallel);
    let svg = render_states(
        &crate::view::apply(&architecture, &Default::default()).unwrap(),
        super::super::AddressFormat::Qualified,
    );
    let paths: std::collections::BTreeSet<_> = svg
        .lines()
        .filter(|line| line.contains("data-cross-state=\"true\""))
        .map(|line| {
            line.split(" d=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
        })
        .collect();
    assert_eq!(paths.len(), 2);
    assert!(svg.contains("outputs.second_value"));
    assert!(svg.contains("outputs.value"));
    architecture.relationships.reverse();
    assert_eq!(
        svg,
        render_states(
            &crate::view::apply(&architecture, &Default::default()).unwrap(),
            super::super::AddressFormat::Qualified
        )
    );
}

#[test]
fn named_states_reflow_indexes_at_shared_width_and_keep_cross_state_offsets() {
    use crate::model::architecture::{Architecture, CrossStateEdge, CrossStateProvenance};
    use crate::svg::AddressFormat;

    let mut narrow_input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/relationship-index-plan.json")).unwrap();
    let mut wide_input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap();
    let mut previous = String::from("aws_route_table.private");
    for i in 0..3 {
        let address = format!("test.wide_{i}");
        wide_input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"address": address, "type": "test"}));
        wide_input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"address": address, "expressions": {
                "input": {"references": [format!("{previous}.id")]}
            }}));
        previous = address;
    }
    let graph = |input: &serde_json::Value| {
        crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap())
    };
    let narrow = graph(&narrow_input);
    let standalone = crate::svg::render(&narrow, &Layout::new(&narrow));
    let standalone_document = roxmltree::Document::parse(&standalone).unwrap();
    let standalone_root = standalone_document.root_element();
    let first = StateId::new("first").unwrap();
    let second = StateId::new("second").unwrap();
    let mut architecture = Architecture {
        states: BTreeMap::from([
            (first.clone(), narrow),
            (second.clone(), graph(&wide_input)),
        ]),
        relationships: ["primary", "secondary"]
            .into_iter()
            .map(|output| CrossStateEdge {
                from: Endpoint {
                    state: first.clone(),
                    address: "aws_route_table.private".into(),
                },
                to: Endpoint {
                    state: second.clone(),
                    address: "aws_route_table.private".into(),
                },
                provenance: CrossStateProvenance::TerraformRemoteState {
                    remote: "data.terraform_remote_state.first".into(),
                    output: output.into(),
                    output_action: None,
                },
            })
            .collect(),
    };
    let render = |architecture: &Architecture| {
        render_states(
            &crate::view::apply(architecture, &Default::default()).unwrap(),
            AddressFormat::Qualified,
        )
    };
    let svg = render(&architecture);
    let document = roxmltree::Document::parse(&svg).unwrap();
    let number =
        |node: roxmltree::Node, key| node.attribute(key).unwrap().parse::<usize>().unwrap();
    let position = |node: roxmltree::Node| {
        let values: Vec<usize> = node
            .attribute("transform")
            .unwrap()
            .trim_start_matches("translate(")
            .trim_end_matches(')')
            .split_whitespace()
            .map(|value| value.parse().unwrap())
            .collect();
        (values[0], values[1])
    };
    let offset_y = |node: roxmltree::Node| {
        node.ancestors()
            .map(|ancestor| {
                ancestor
                    .attribute("transform")
                    .map_or(0, |_| position(ancestor).1)
                    + if ancestor.has_tag_name("svg") {
                        ancestor.attribute("y").map_or(0, |_| number(ancestor, "y"))
                    } else {
                        0
                    }
            })
            .sum::<usize>()
    };
    let states: Vec<_> = document
        .descendants()
        .filter(|node| node.attribute("data-state-id").is_some())
        .map(|state| {
            state
                .children()
                .find(|node| node.has_tag_name("svg"))
                .unwrap()
        })
        .collect();
    assert_eq!(states.len(), 2);
    let (narrow, wide) = (states[0], states[1]);
    let width = number(wide, "width");
    assert!(width > number(standalone_root, "width"));
    assert_eq!(number(narrow, "width"), width);
    assert_eq!(number(document.root_element(), "width"), width + 80 + 12);
    assert_eq!(
        number(narrow, "y"),
        100 + legend_shape(document.root_element()).1 + 12
    );
    assert!(number(narrow, "height") < number(standalone_root, "height"));
    assert_eq!(
        number(wide, "y"),
        number(narrow, "y") + number(narrow, "height") + 60
    );
    assert_eq!(
        number(document.root_element(), "height"),
        number(wide, "y") + number(wide, "height") + 20
    );
    let cards = |root: roxmltree::Node| {
        root.descendants()
            .filter(|node| node.attribute("data-relationship-index").is_some())
            .map(|index| {
                let card = index.parent().unwrap();
                let rectangle = card
                    .children()
                    .find(|node| node.has_tag_name("rect"))
                    .unwrap();
                (
                    position(card),
                    number(rectangle, "width"),
                    number(rectangle, "height"),
                )
            })
            .collect::<Vec<_>>()
    };
    let original_cards = cards(standalone_root);
    let reflowed_cards = cards(narrow);
    assert_eq!(original_cards.len(), 5);
    assert_eq!(reflowed_cards.len(), 5);
    assert!(original_cards[2].0.1 > original_cards[0].0.1);
    for (i, &(position, card_width, card_height)) in reflowed_cards.iter().enumerate() {
        assert_eq!(
            (card_width, card_height),
            (original_cards[i].1, original_cards[i].2)
        );
        assert_eq!(position, (16 + i * (card_width + 20), 48));
        assert!(40 + position.0 + card_width <= width - 56);
    }
    let section = narrow
        .descendants()
        .find(|node| node.attribute("data-annotation-section") == Some("Relationship Resources"))
        .unwrap();
    let rectangle = section
        .children()
        .find(|node| node.has_tag_name("rect"))
        .unwrap();
    assert_eq!(
        number(rectangle, "height"),
        48 + reflowed_cards.iter().map(|card| card.2).max().unwrap() + 16
    );
    for (state, count) in [(narrow, 21), (wide, 1)] {
        let entries: Vec<_> = state
            .descendants()
            .filter_map(|node| node.attribute("data-relationship-entry"))
            .map(|value| value.parse::<usize>().unwrap())
            .collect();
        assert_eq!(entries, (1..=count).collect::<Vec<_>>());
        let markers: std::collections::BTreeSet<_> = state
            .descendants()
            .filter_map(|node| node.attribute("data-relationship-marker"))
            .map(|value| value.parse::<usize>().unwrap())
            .collect();
        assert_eq!(markers, entries.into_iter().collect());
    }
    let geometry = |root: roxmltree::Node| {
        root.descendants()
            .find(|node| node.attribute("data-architecture").is_some())
            .unwrap()
            .descendants()
            .filter(|node| matches!(node.tag_name().name(), "rect" | "path" | "circle"))
            .map(|node| {
                ["d", "x", "y", "width", "height", "cx", "cy", "r"]
                    .map(|key| node.attribute(key).map(str::to_owned))
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(geometry(narrow), geometry(standalone_root));
    let architecture_y = |root: roxmltree::Node| {
        let group = root
            .descendants()
            .find(|node| node.attribute("data-architecture").is_some())
            .unwrap();
        offset_y(group) - root.attribute("y").map_or(0, |_| number(root, "y"))
    };
    assert_eq!(
        number(narrow, "height") - architecture_y(narrow),
        number(standalone_root, "height") - architecture_y(standalone_root)
    );
    let lines: Vec<_> = document
        .descendants()
        .filter(|node| node.attribute("data-cross-state") == Some("true"))
        .collect();
    assert_eq!(lines.len(), 2);
    for (i, line) in lines.into_iter().enumerate() {
        let tokens: Vec<_> = line.attribute("d").unwrap().split_whitespace().collect();
        let points: Vec<_> = tokens
            .chunks_exact(3)
            .map(|point| Point {
                x: point[1].parse().unwrap(),
                y: point[2].parse().unwrap(),
            })
            .collect();
        assert_eq!(
            points.iter().map(|point| point.x).max(),
            Some(width + 30 + i * 12)
        );
        for (attribute, point) in [
            ("data-source-port", points[0]),
            ("data-target-port", *points.last().unwrap()),
        ] {
            let card = document
                .descendants()
                .find(|node| node.attribute("id") == line.attribute(attribute))
                .unwrap();
            let rectangle = card
                .children()
                .find(|node| node.has_tag_name("rect"))
                .unwrap();
            assert_eq!(point.x, number(rectangle, "x") + number(rectangle, "width"));
            assert_eq!(
                point.y,
                offset_y(card) + number(rectangle, "y") + number(rectangle, "height") / 2
            );
        }
    }
    for input in [&mut narrow_input, &mut wide_input] {
        input["resource_changes"].as_array_mut().unwrap().reverse();
        input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .reverse();
    }
    architecture.states =
        BTreeMap::from([(second, graph(&wide_input)), (first, graph(&narrow_input))]);
    architecture.relationships.reverse();
    assert_eq!(render(&architecture), svg);
}

#[test]
fn cross_state_obstacles_only_relax_the_endpoint_and_its_ancestors() {
    let rectangle = |x, y, width, height| Bounds {
        origin: Point { x, y },
        width,
        height,
    };
    let state = StateObstacles {
        scopes: Vec::new(),
        markers: vec![rectangle(270, 219, 24, 18)],
        bounds: vec![
            rectangle(50, 100, 400, 300),
            rectangle(80, 180, 150, 96),
            rectangle(500, 120, 300, 300),
        ],
        headers: vec![60, 96, 60],
        parents: vec![None, Some(0), None],
    };
    let obstacles = state.for_endpoint(1);
    assert_eq!(obstacles[0], state.bounds[0].header(60));
    assert_eq!(obstacles[2], state.bounds[2]);
    assert_eq!(obstacles[3], state.markers[0]);
    let route = crate::layout::route_to_margin(
        Point { x: 230, y: 228 },
        Point { x: 900, y: 228 },
        &obstacles,
    );
    for unrelated in [state.bounds[2], state.markers[0]] {
        for segment in route.windows(2) {
            let (a, b) = (segment[0], segment[1]);
            let crosses = if a.x == b.x {
                a.x > unrelated.origin.x
                    && a.x < unrelated.right()
                    && a.y.max(b.y) > unrelated.origin.y
                    && a.y.min(b.y) < unrelated.origin.y + unrelated.height
            } else {
                a.y > unrelated.origin.y
                    && a.y < unrelated.origin.y + unrelated.height
                    && a.x.max(b.x) > unrelated.origin.x
                    && a.x.min(b.x) < unrelated.right()
            };
            assert!(!crosses, "{segment:?}");
        }
    }
}

use super::*;
use serde_json::{Value, json};

fn inputs() -> Vec<NamedInput> {
    [
        ("producer", include_str!("../fixtures/multi/producer.json")),
        ("consumer", include_str!("../fixtures/multi/consumer.json")),
    ]
    .into_iter()
    .map(|(state, json)| NamedInput {
        state: state.into(),
        json: json.into(),
    })
    .collect()
}

fn mapping() -> RemoteStateMapping {
    RemoteStateMapping {
        consumer: "consumer".into(),
        remote: "data.terraform_remote_state.network".into(),
        producer: "producer".into(),
    }
}

fn options() -> Options {
    Options {
        remote_state_mappings: vec![mapping()],
        ..Default::default()
    }
}

fn edit(input: &mut NamedInput, change: impl FnOnce(&mut Value)) {
    let mut value = serde_json::from_str(&input.json).unwrap();
    change(&mut value);
    input.json = value.to_string();
}

#[test]
fn mapped_generation_matches_shared_cli_pipeline_and_resumes_with_filters() {
    for format in [OutputFormat::Svg, OutputFormat::Html] {
        for selection in 0..5 {
            let make_options = || Options {
                format,
                focus: (selection > 0).then(|| {
                    if selection == 4 {
                        "producer:aws_subnet.unrelated"
                    } else {
                        "consumer:aws_instance.app"
                    }
                    .into()
                }),
                focus_depth: (selection == 1 || selection == 4).then_some(0),
                changed_only: selection == 3,
                ..options()
            };
            let mut input = inputs();
            edit(&mut input[0], |value| {
                value["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"address":"aws_subnet.unrelated","type":"aws_subnet"}))
            });
            let opts = make_options();
            let plans = parse(&input).unwrap();
            let architecture = crate::model::cross_state::Architecture {
                states: plans
                    .states
                    .iter()
                    .map(|(state, plan)| (state.clone(), semantic::transform(plan)))
                    .collect(),
                relationships: semantic::cross_state::resolve(
                    &plans,
                    &[mapping().validate().unwrap()],
                )
                .edges,
            };
            let view = view::apply(
                &architecture,
                &view::Options {
                    focus: opts.focus.clone(),
                    depth: opts.focus_depth,
                    changed_only: opts.changed_only,
                    ..Default::default()
                },
            )
            .unwrap();
            let expected = match format {
                OutputFormat::Svg => svg::render_states(&view, svg::Options::default()),
                OutputFormat::Html => crate::html::render_states(&view, svg::Options::default()),
            };
            let generated = generate_named(&input, &opts).unwrap();
            let expected = crate::html::generation::attach(
                expected,
                &view::Options {
                    focus: opts.focus.clone(),
                    depth: opts.focus_depth,
                    changed_only: opts.changed_only,
                    ..Default::default()
                },
                svg::Options::default(),
                false,
                view.states.keys().map(|id| id.as_str()).collect(),
                vec![serde_json::to_value(mapping()).unwrap()],
            );
            assert_eq!(generated.content, expected);
            assert_eq!(
                generated
                    .content
                    .contains("<path data-edge-kind=\"dependency\" data-cross-state=\"true\""),
                selection != 4
            );
            for units in [1, 4096] {
                let owned = input
                    .iter()
                    .map(|input| NamedInput {
                        state: input.state.clone(),
                        json: input.json.clone(),
                    })
                    .collect();
                let mut job = crate::Generator::new_named(owned, make_options()).unwrap();
                let result = loop {
                    if let Some(result) = job.step(units).unwrap() {
                        break result;
                    }
                };
                assert_eq!(result.content, expected);
                assert_eq!(result.diagnostics, generated.diagnostics);
            }
        }
    }
}

#[test]
fn mappings_reject_invalid_unknown_and_conflicting_targets_without_echoing_input() {
    for (field, value, error) in [
        ("consumer", "missing", Error::UnknownRemoteStateMapping),
        ("producer", "missing", Error::UnknownRemoteStateMapping),
        ("consumer", "SECRET:name", Error::InvalidRemoteStateMapping),
        ("remote", "", Error::InvalidRemoteStateMapping),
        ("remote", "SECRET\nvalue", Error::InvalidRemoteStateMapping),
        (
            "remote",
            "data.terraform_remote_state.missing",
            Error::InvalidRemoteStateMapping,
        ),
        (
            "remote",
            "aws_instance.app",
            Error::InvalidRemoteStateMapping,
        ),
    ] {
        let mut opts = options();
        let mapping = &mut opts.remote_state_mappings[0];
        match field {
            "consumer" => mapping.consumer = value.into(),
            "producer" => mapping.producer = value.into(),
            _ => mapping.remote = value.into(),
        }
        let actual = generate_named(&inputs(), &opts).err().unwrap();
        assert_eq!(actual, error);
        assert!(!actual.to_string().contains("SECRET"));
    }
    let mut duplicate = options();
    duplicate.remote_state_mappings.push(mapping());
    assert_eq!(
        generate_named(&inputs(), &duplicate).unwrap().content,
        generate_named(&inputs(), &options()).unwrap().content
    );
    duplicate.remote_state_mappings[1].producer = "consumer".into();
    assert_eq!(
        generate_named(&inputs(), &duplicate).err(),
        Some(Error::AmbiguousRemoteStateMapping)
    );
    for change in ["provider", "mode", "deposed"] {
        let mut input = inputs();
        edit(&mut input[1], |value| {
            let remote = &mut value["resource_changes"][0];
            match change {
                "provider" => remote["provider_name"] = json!("example.org/private/terraform"),
                "mode" => remote["mode"] = json!("managed"),
                _ => remote["deposed"] = json!("old"),
            }
        });
        assert_eq!(
            generate_named(&input, &options()).err(),
            Some(Error::InvalidRemoteStateMapping)
        );
        let plans = parse(&input).unwrap();
        let cross = semantic::cross_state::resolve(&plans, &[mapping().validate().unwrap()]);
        assert!(cross.edges.is_empty());
    }
    assert_eq!(
        crate::generate(&inputs()[0].json, &options()).err(),
        Some(Error::InvalidOptions)
    );
    assert!(matches!(
        crate::Generator::new(inputs()[0].json.clone(), options()),
        Err(Error::InvalidOptions)
    ));
}

#[test]
fn missing_outputs_and_incomplete_provenance_use_fixed_shared_diagnostics() {
    let mut input = inputs();
    input.push(NamedInput {
        state: "unmapped".into(),
        json: input[1].json.clone(),
    });
    edit(&mut input[0], |value| {
        value["configuration"]["root_module"]["outputs"]
            .as_object_mut()
            .unwrap()
            .remove("subnet");
        value["configuration"]["root_module"]["outputs"]["both"] =
            json!({"expression":{"constant_value":"SECRET credential"}});
    });
    let result = generate_named(&input, &options()).unwrap();
    assert!(result.diagnostics.contains(&"unmapped remote state".into()));
    assert!(!result.content.contains("data-cross-state=\"true\""));
    assert!(
        result
            .diagnostics
            .contains(&"producer output is missing".into())
    );
    assert!(
        result
            .diagnostics
            .contains(&"producer output provenance is unresolved".into())
    );
    assert!(!serde_json::to_string(&result).unwrap().contains("SECRET"));
    let unmapped = generate_named(&input, &Options::default()).unwrap();
    assert!(
        !unmapped
            .diagnostics
            .iter()
            .any(|message| message.contains("producer output") || message.contains("remote state"))
    );
}

#[test]
fn mapped_special_addresses_remain_escaped_and_state_qualified() {
    let mut input = inputs();
    let remote = "data.terraform_remote_state.network[\"</script><svg/onload=alert(1)>&=\\\"\"]";
    input[1].json = input[1].json.replace(
        "data.terraform_remote_state.network",
        &serde_json::to_string(remote).unwrap()
            [1..serde_json::to_string(remote).unwrap().len() - 1],
    );
    let mut opts = options();
    opts.remote_state_mappings[0].remote = remote.into();
    opts.format = OutputFormat::Html;
    input.push(NamedInput {
        state: "unrelated".into(),
        json: input[0].json.clone(),
    });
    edit(&mut input[0], |value| {
        value["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_subnet.app", "type":"aws_subnet", "deposed":"old", "change":{"actions":["delete"]}}));
    });
    let result = generate_named(&input, &opts).unwrap();
    assert!(result.content.contains("data-cross-state=\"true\""));
    assert!(!result.content.contains("</script><svg/onload"));
    assert!(!result.content.contains("data-source-state=\"unrelated\""));
    assert!(result.content.contains("data-source-state=\"producer\""));
    assert!(result.content.contains("-current\" data-target="));
}

#[test]
fn mapping_options_are_bounded_and_strictly_deserialized() {
    for json in [
        r#"{"remote_state_mappings":[{"consumer":"c","remote":"r","producer":"p","extra":1}]}"#,
        r#"{"remote_state_mappings":[{"consumer":"c","producer":"p"}]}"#,
        r#"{"remote_state_mappings":null}"#,
    ] {
        assert!(serde_json::from_str::<Options>(json).is_err());
    }
    for mappings in [
        vec![mapping(); 257],
        vec![RemoteStateMapping {
            remote: "x".repeat(4097),
            ..mapping()
        }],
        vec![
            RemoteStateMapping {
                remote: "x".repeat(4096),
                ..mapping()
            };
            16
        ],
    ] {
        let opts = Options {
            remote_state_mappings: mappings,
            ..Default::default()
        };
        assert_eq!(
            generate_named(&inputs(), &opts).err(),
            Some(Error::ResourceLimit)
        );
        assert!(matches!(
            crate::Generator::new_named(inputs(), opts),
            Err(Error::ResourceLimit)
        ));
    }
}

#[test]
fn mapped_fanout_counts_against_aggregate_edges_before_selection() {
    for size in [32, 33] {
        refuse_fanout(size);
    }
}

fn refuse_fanout(size: usize) {
    let mut input = inputs();
    edit(&mut input[0], |value| {
        value["resource_changes"] = json!(
            (0..size)
                .map(|i| json!({"address":format!("aws_subnet.s{i}"),"type":"aws_subnet"}))
                .collect::<Vec<_>>()
        );
        value["configuration"]["root_module"]["outputs"] = json!({"all":{"expression":{"references":(0..size).map(|i| format!("aws_subnet.s{i}.id")).collect::<Vec<_>>()}}});
    });
    edit(&mut input[1], |value| {
        let remote = value["resource_changes"][0].clone();
        let mut resources = vec![remote];
        resources.extend(
            (0..size)
                .map(|i| json!({"address":format!("aws_instance.c{i}"),"type":"aws_instance"})),
        );
        value["resource_changes"] = json!(resources);
        value["configuration"]["root_module"]["resources"] = json!((0..size).map(|i| json!({"address":format!("aws_instance.c{i}"),"expressions":{"subnet_id":{"references":["data.terraform_remote_state.network.outputs.all"]}}})).collect::<Vec<_>>());
    });
    let mut job = crate::Generator::new_named(
        input,
        Options {
            focus: Some("consumer:aws_instance.c0".into()),
            focus_depth: Some(0),
            ..options()
        },
    )
    .unwrap();
    for _ in 0..10000 {
        match job.step(1) {
            Ok(None) => continue,
            Err(Error::ResourceLimit) => {
                assert_eq!(job.step(1).err(), Some(Error::GenerationClosed));
                return;
            }
            _ => panic!("cross-state fanout must hit the aggregate edge budget"),
        }
    }
    panic!("bounded job must finish");
}

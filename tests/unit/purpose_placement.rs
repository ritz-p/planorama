use crate::{layout::Layout, model::Placement, plan, semantic, svg, view};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fmt::Write, fs};

const TYPES: [&str; 7] = [
    "aws_ssm_parameter",
    "aws_cloudwatch_metric_alarm",
    "aws_iam_role",
    "aws_iam_user",
    "aws_iam_group",
    "aws_iam_openid_connect_provider",
    "aws_iam_saml_provider",
];

fn eligible(node: &crate::model::Node) -> bool {
    node.provider.is_aws() && TYPES.contains(&node.resource_type.as_str())
}

fn monitoring() -> Value {
    json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_db_instance.database","type":"aws_db_instance","change":{"actions":["create"],"after":{"identifier":"PRIVATE_DATABASE"}}},
        {"address":"aws_cloudwatch_metric_alarm.cpu","type":"aws_cloudwatch_metric_alarm","change":{"actions":["update"],"after":{"namespace":"AWS/RDS","metric_name":"CPUUtilization","dimensions":{"DBInstanceIdentifier":"PRIVATE_DATABASE"}}}},
        {"address":"aws_cloudwatch_metric_alarm.literal","type":"aws_cloudwatch_metric_alarm","change":{"actions":["create"],"after":{"namespace":"AWS/RDS","metric_name":"CPUUtilization","dimensions":{"DBInstanceIdentifier":"PRIVATE_DATABASE"}}}}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_cloudwatch_metric_alarm.cpu","expressions":{"dimensions":{"references":["aws_db_instance.database.id"]}}}
    ]}}})
}

#[test]
fn explicit_spatial_types_preserve_semantics_references_and_selection() {
    for input in [
        include_str!("../fixtures/viewer-plan.json").to_owned(),
        monitoring().to_string(),
        include_str!("../fixtures/iam-plan.json").to_owned(),
        include_str!("../fixtures/identity-providers-plan.json").to_owned(),
        include_str!("../fixtures/ssm-plan.json").to_owned(),
    ] {
        let original = semantic::transform(&plan::parse(&input).unwrap());
        let snapshot = original.clone();
        for focus in std::iter::once(None)
            .chain(original.nodes.iter().map(|node| Some(node.address.clone())))
        {
            let options = view::Options {
                depth: focus.as_ref().map(|_| 0),
                focus,
                ..Default::default()
            };
            let default = view::single(&original, &options).unwrap();
            let spatial = view::single(
                &original,
                &view::Options {
                    spatial_types: TYPES.into_iter().map(String::from).collect(),
                    ..options
                },
            )
            .unwrap();
            let mut restored = spatial.0.clone();
            for (before, after) in default.nodes.iter().zip(&mut restored.0.nodes) {
                assert_eq!(
                    after.placement,
                    if eligible(before) {
                        Placement::Spatial
                    } else {
                        before.placement
                    }
                );
                let rows = |node: &crate::model::Node| {
                    view::references::rows(node)
                        .iter()
                        .map(|row| row.label(false))
                        .collect::<Vec<_>>()
                };
                assert_eq!(rows(before), rows(after));
                after.placement = before.placement;
                for (before, after) in before.references.iter().zip(&mut after.references) {
                    after.preserve_spatial = before.preserve_spatial;
                    for (before, after) in before.targets.iter().zip(&mut after.targets) {
                        after.placement = before.placement;
                    }
                }
            }
            assert_eq!(restored, default.0);
            assert_eq!(original, snapshot);
        }
    }
}

#[test]
fn spatial_cross_state_references_keep_qualified_targets() {
    use crate::model::architecture::{
        Architecture, CrossStateEdge, CrossStateProvenance, Endpoint, StateId,
    };
    use std::collections::BTreeMap;
    let input = r#"{"format_version":"1.2","resource_changes":[{"address":"aws_iam_role.same","type":"aws_iam_role"}]}"#;
    let one = StateId::new("one").unwrap();
    let two = StateId::new("two").unwrap();
    let original = Architecture {
        states: BTreeMap::from([
            (
                one.clone(),
                semantic::transform(&plan::parse(input).unwrap()),
            ),
            (
                two.clone(),
                semantic::transform(&plan::parse(input).unwrap()),
            ),
        ]),
        relationships: vec![CrossStateEdge {
            from: Endpoint {
                state: one,
                address: "aws_iam_role.same".into(),
            },
            to: Endpoint {
                state: two,
                address: "aws_iam_role.same".into(),
            },
            provenance: CrossStateProvenance::TerraformRemoteState {
                remote: "data.terraform_remote_state.roles".into(),
                output: "role".into(),
                output_action: None,
            },
        }],
    };
    let projected = view::apply(
        &original,
        &view::Options {
            focus: Some("two:aws_iam_role.same".into()),
            depth: Some(0),
            spatial_types: vec!["aws_iam_role".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(projected.relationships(), original.relationships);
    let svg = svg::render_states(&projected, svg::Options::default());
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let links: Vec<_> = doc
        .descendants()
        .filter(|node| node.attribute("data-reference-attribute").is_some())
        .collect();
    assert_eq!(links.len(), 1);
    let target = links[0]
        .attribute("href")
        .unwrap()
        .strip_prefix('#')
        .unwrap();
    let target = doc
        .descendants()
        .find(|node| node.attribute("id") == Some(target))
        .unwrap();
    assert!(
        target
            .ancestors()
            .any(|node| node.attribute("data-state-id") == Some("one"))
    );
    for graph in original.states.values() {
        assert_eq!(graph.nodes[0].placement, Placement::Logical);
        assert!(graph.nodes[0].references.is_empty());
    }
}

#[test]
fn spatial_projection_preserves_all_deployment_scopes() {
    use crate::model::{AwsAccountScope, RegionScope, architecture::DeploymentScope};
    let input = include_str!("../fixtures/identity-providers-plan.json");
    for global in [false, true] {
        for region in [None, Some(RegionScope("us-east-1".into()))] {
            for account in [None, AwsAccountScope::new("123456789012")] {
                let mut graph = semantic::transform(&plan::parse(input).unwrap());
                for node in &mut graph.0.nodes {
                    node.entity.scope = Some(DeploymentScope {
                        provider: "registry.terraform.io/hashicorp/aws".into(),
                        region: region.clone(),
                        account: account.clone(),
                        global,
                    });
                }
                let original = graph.clone();
                let projected = view::single(
                    &graph,
                    &view::Options {
                        spatial_types: TYPES.into_iter().map(String::from).collect(),
                        ..Default::default()
                    },
                )
                .unwrap();
                for (before, after) in graph.nodes.iter().zip(&projected.nodes) {
                    assert_eq!(before.entity, after.entity);
                    assert_eq!(before.provider_configuration, after.provider_configuration);
                }
                assert_eq!(graph, original);
                assert!(!svg::render(&projected, &Layout::new(&projected)).contains("PRIVATE_"));
            }
        }
    }
}

#[test]
fn spatial_allowlist_is_exact_and_state_scoped() {
    use crate::model::architecture::{Architecture, StateId};
    use std::collections::BTreeMap;
    let input = json!({"format_version":"1.2","resource_changes":[
        {"address":"module.a.aws_iam_role.same","type":"aws_iam_role","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"]}},
        {"address":"module.b.aws_iam_role.same","type":"aws_iam_role","provider_name":"registry.terraform.io/foreign/aws","change":{"actions":["delete"]}},
        {"address":"data.aws_ssm_parameter.same","mode":"data","type":"aws_ssm_parameter","change":{"actions":["read"]}},
        {"address":"aws_iam_role.old","type":"aws_iam_role","deposed":"old","change":{"actions":["delete"]}},
        {"address":"aws_iam_role.old","type":"aws_iam_role","change":{"actions":["create"]}},
        {"address":"aws_iam_policy.policy","type":"aws_iam_policy","change":{"actions":["create"]}},
        {"address":"aws_iam_unknown.unknown","type":"aws_iam_unknown","change":{"actions":["create"]}}
    ]});
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let architecture = Architecture {
        states: BTreeMap::from([
            (StateId::new("one").unwrap(), graph.clone()),
            (StateId::new("two").unwrap(), graph),
        ]),
        relationships: Vec::new(),
    };
    for modules in [
        vec![],
        vec!["one:module.a".into()],
        vec!["two:module.b".into()],
    ] {
        let default = view::apply(
            &architecture,
            &view::Options {
                modules: modules.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let spatial = view::apply(
            &architecture,
            &view::Options {
                spatial_types: TYPES.into_iter().map(String::from).collect(),
                modules,
                ..Default::default()
            },
        )
        .unwrap();
        for (state, graph) in &spatial.states {
            assert_eq!(graph.nodes.len(), default.states[state].nodes.len());
            for (before, after) in default.states[state].nodes.iter().zip(&graph.nodes) {
                assert_eq!(before.entity, after.entity);
                assert_eq!(before.provider, after.provider);
                assert_eq!(before.mode, after.mode);
                assert_eq!(before.action, after.action);
                assert_eq!(
                    after.placement,
                    if eligible(before) {
                        Placement::Spatial
                    } else {
                        before.placement
                    }
                );
            }
        }
    }
    for invalid in [
        "aws_iam_policy",
        "aws_iam_role_policy",
        "aws_iam_instance_profile",
        "aws_iam_group_membership",
        "aws_cloudwatch_composite_alarm",
        "aws_ssm_document",
        "AWS_IAM_ROLE",
        "aws_iam_role ",
        "secret",
    ] {
        let options = view::Options {
            spatial_types: vec![invalid.into()],
            ..Default::default()
        };
        assert!(view::apply(&architecture, &options).is_err());
        assert_eq!(
            crate::generate(
                &input.to_string(),
                &crate::Options {
                    spatial_types: vec![invalid.into()],
                    ..Default::default()
                }
            )
            .err(),
            Some(crate::Error::InvalidOptions)
        );
    }
}

fn repeat(input: &Value, count: usize) -> Value {
    fn rename(value: &mut Value, addresses: &[&str], copy: usize) {
        match value {
            Value::String(text) => {
                if let Some(address) = addresses.iter().find(|address| {
                    text.strip_prefix(**address)
                        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
                }) {
                    *text = format!("{address}_copy{copy}{}", &text[address.len()..]);
                }
            }
            Value::Array(values) => {
                for value in values {
                    rename(value, addresses, copy);
                }
            }
            Value::Object(values) => {
                for value in values.values_mut() {
                    rename(value, addresses, copy);
                }
            }
            _ => {}
        }
    }
    if count == 1 {
        return input.clone();
    }
    let mut addresses: Vec<_> = input["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|resource| resource["address"].as_str().unwrap())
        .collect();
    addresses.sort_by_key(|address| std::cmp::Reverse(address.len()));
    let mut output = input.clone();
    output["resource_changes"] = json!([]);
    output["configuration"]["root_module"]["resources"] = json!([]);
    for copy in 0..count {
        let mut part = input.clone();
        rename(&mut part, &addresses, copy);
        for field in ["/resource_changes", "/configuration/root_module/resources"] {
            output
                .pointer_mut(field)
                .unwrap()
                .as_array_mut()
                .unwrap()
                .extend(
                    part.pointer(field)
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .iter()
                        .cloned(),
                );
        }
    }
    output
}

#[test]
#[ignore = "purpose-dependent placement review; writes comparison SVGs and measurements"]
fn compare_purpose_placements() {
    let directory = std::path::Path::new("output/purpose-placement");
    fs::create_dir_all(directory).unwrap();
    let mut report = String::from(
        "| Scenario | Copies | Resources / relationships | View | Spatial / logical | Canvas | Visible reference rows |\n| --- | ---: | --- | --- | --- | --- | ---: |\n",
    );
    let scenarios = [
        (
            "infrastructure",
            serde_json::from_str(include_str!("../fixtures/viewer-plan.json")).unwrap(),
        ),
        ("monitoring", monitoring()),
        (
            "iam",
            serde_json::from_str(include_str!("../fixtures/iam-plan.json")).unwrap(),
        ),
        (
            "federation",
            serde_json::from_str(include_str!("../fixtures/identity-providers-plan.json")).unwrap(),
        ),
        (
            "configuration",
            serde_json::from_str(include_str!("../fixtures/ssm-plan.json")).unwrap(),
        ),
    ];
    let mut covered = BTreeSet::new();
    for (name, input) in scenarios {
        for copies in [1, 4] {
            let input = repeat(&input, copies);
            let original = semantic::transform(&plan::parse(&input.to_string()).unwrap());
            let targets: BTreeSet<_> = original
                .nodes
                .iter()
                .filter(|node| eligible(node))
                .flat_map(|node| node.entity.provenance.iter().cloned())
                .collect();
            for node in &original.nodes {
                if eligible(node) {
                    covered.insert(node.resource_type.clone());
                    assert_eq!(node.placement, Placement::Logical);
                    assert!(node.member_owner.is_none());
                    assert!(node.routes.owner.is_none());
                }
            }
            for independent in [false, true] {
                let projected = view::single(
                    &original,
                    &view::Options {
                        spatial_types: if independent {
                            TYPES.into_iter().map(String::from).collect()
                        } else {
                            Vec::new()
                        },
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(projected.nodes.len(), original.nodes.len());
                assert_eq!(projected.edges, original.edges);
                assert_eq!(projected.relationships, original.relationships);
                assert_eq!(projected.coverage, original.coverage);
                for (before, after) in original.nodes.iter().zip(&projected.nodes) {
                    assert_eq!(before.entity, after.entity);
                    assert_eq!(before.action, after.action);
                    assert_eq!(before.role, after.role);
                }
                let output = svg::render(&projected, &Layout::new(&projected));
                assert_eq!(
                    original,
                    view::single(&original, &view::Options::default())
                        .unwrap()
                        .0
                );
                assert!(!output.contains("PRIVATE_"));
                assert!(!output.contains("SYNTHETIC_PARAMETER_NOT_FOR_OUTPUT"));
                let doc = roxmltree::Document::parse(&output).unwrap();
                assert_eq!(
                    doc.descendants()
                        .filter(|node| node.attribute("data-terraform-address").is_some())
                        .count(),
                    original.nodes.len()
                );
                for href in doc.descendants().filter_map(|node| node.attribute("href")) {
                    let target = href.strip_prefix('#').expect("expected a fragment href");
                    assert!(
                        doc.descendants()
                            .any(|node| node.attribute("id") == Some(target))
                    );
                }
                let root = doc.root_element();
                let logical = projected
                    .nodes
                    .iter()
                    .filter(|node| node.placement == Placement::Logical)
                    .count();
                let rows = projected
                    .nodes
                    .iter()
                    .map(|node| view::references::rows(node).len())
                    .sum::<usize>();
                assert_eq!(
                    rows,
                    original
                        .nodes
                        .iter()
                        .map(|node| view::references::rows(node).len())
                        .sum::<usize>()
                );
                for node in &projected.nodes {
                    for reference in &node.references {
                        for target in &reference.targets {
                            if independent && targets.contains(&target.id) {
                                assert_eq!(target.placement, Placement::Spatial);
                            }
                        }
                    }
                }
                let variant = if independent {
                    "independent"
                } else {
                    "logical"
                };
                writeln!(report, "| {name} | {copies} | {} / {} | {variant} | {} / {logical} | {} × {} | {rows} |",
                    projected.nodes.len(), projected.edges.len(), projected.nodes.len() - logical,
                    root.attribute("width").unwrap(), root.attribute("height").unwrap()).unwrap();
                fs::write(
                    directory.join(format!("{name}-{copies}-{variant}.svg")),
                    output,
                )
                .unwrap();
            }
        }
    }
    assert_eq!(covered, TYPES.into_iter().map(String::from).collect());
    fs::write(directory.join("measurements.md"), report).unwrap();
}

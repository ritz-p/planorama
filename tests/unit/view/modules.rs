use crate::{plan, semantic, view};
use serde_json::json;

fn graph() -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&json!({
        "format_version":"1.2",
        "resource_changes": [
            {"address":"module.app[0].aws_instance.web", "type":"aws_instance", "change":{"actions":["update"]}},
            {"address":"module.app[0].aws_instance.web", "deposed":"old", "type":"aws_instance", "change":{"actions":["delete"]}},
            {"address":"module.app[1].module.db.data.aws_ami.image", "mode":"data", "type":"aws_ami", "change":{"actions":["no-op"]}},
            {"address":"module.application.aws_instance.other", "type":"aws_instance", "change":{"actions":["no-op"]}}
        ],
        "configuration":{"root_module":{"module_calls":{"empty":{"module":{}}}}}
    }).to_string()).unwrap())
}

#[test]
fn module_union_intersection_and_identity() {
    let source = graph();
    let options = view::Options {
        modules: vec!["module.app".into()],
        ..Default::default()
    };
    let selected = view::single(&source, &options).unwrap();
    assert_eq!(selected.coverage.input.resources, 4);
    assert_eq!(selected.coverage.selected.resources, 3);
    assert_eq!(selected.nodes.len(), 3);
    let duplicate = view::single(
        &source,
        &view::Options {
            modules: vec![
                "module.app[0]".into(),
                "module.app".into(),
                "module.app".into(),
            ],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes, duplicate.nodes);
    assert_eq!(selected.coverage, duplicate.coverage);
    let changed = view::single(
        &source,
        &view::Options {
            changed_only: true,
            ..options
        },
    )
    .unwrap();
    assert_eq!(changed.coverage.selected.resources, 2);
    let empty = view::single(
        &source,
        &view::Options {
            modules: vec!["module.empty".into()],
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(empty.coverage.selected.resources, 0);
    assert!(
        view::single(
            &source,
            &view::Options {
                modules: vec!["module.missing".into()],
                ..Default::default()
            }
        )
        .is_err()
    );
    let focus = view::single(
        &source,
        &view::Options {
            modules: vec!["module.app".into()],
            focus: Some("module.application.aws_instance.other".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(focus.nodes.is_empty());
}

#[test]
fn modules_require_unambiguous_state() {
    use crate::model::{StateId, cross_state::Architecture};
    let all = Architecture {
        states: [
            (StateId::new("a").unwrap(), graph()),
            (StateId::new("b").unwrap(), graph()),
        ]
        .into(),
        relationships: vec![],
    };
    assert!(
        view::apply(
            &all,
            &view::Options {
                modules: vec!["module.app".into()],
                ..Default::default()
            }
        )
        .err()
        .unwrap()
        .contains("ambiguous")
    );
    let selected = view::apply(
        &all,
        &view::Options {
            modules: vec!["a:module.app".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.states[&StateId::new("a").unwrap()].nodes.len(), 3);
    assert!(
        selected.states[&StateId::new("b").unwrap()]
            .nodes
            .is_empty()
    );
}

#[test]
fn modules_keep_containment_context_but_not_external_dependencies() {
    let raw = plan::parse(&json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc.main","type":"aws_vpc","change":{"actions":["no-op"]}},
        {"address":"aws_subnet.main","type":"aws_subnet","change":{"actions":["no-op"]}},
        {"address":"module.app.aws_instance.web","type":"aws_instance","change":{"actions":["create"]}},
        {"address":"module.app.aws_iam_role.role","type":"aws_iam_role","change":{"actions":["create"]}}
    ],"configuration":{"root_module":{
        "resources":[{"address":"aws_vpc.main"},{"address":"aws_subnet.main","expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}}],
        "module_calls":{"app":{"expressions":{"subnet":{"references":["aws_subnet.main.id"]}},"module":{
            "variables":{"subnet":{}},"resources":[{"address":"aws_instance.web","expressions":{"subnet_id":{"references":["var.subnet"]}}},{"address":"aws_iam_role.role"}]
        }}}
    }}}).to_string()).unwrap();
    let graph = semantic::transform(&raw);
    let selected = view::single(
        &graph,
        &view::Options {
            modules: vec!["module.app".into()],
            explain_selection: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 4);
    assert_eq!(selected.coverage.selection.as_ref().unwrap().direct(), 2);
    assert!(selected.nodes.iter().any(|n| n.address == "aws_vpc.main"));
    assert!(
        selected
            .nodes
            .iter()
            .any(|n| n.address == "aws_subnet.main")
    );
}

#[test]
fn module_selection_preserves_lowered_and_embedded_source_accounting() {
    for input in [
        include_str!("../../fixtures/association-plan.json"),
        include_str!("../../fixtures/routes-plan.json"),
        include_str!("../../fixtures/messaging-plan.json"),
    ] {
        let mut json: serde_json::Value = serde_json::from_str(input).unwrap();
        for resource in json["resource_changes"].as_array_mut().unwrap() {
            resource["address"] = json!(format!(
                "module.app.{}",
                resource["address"].as_str().unwrap()
            ));
        }
        let config = json["configuration"]["root_module"].take();
        json["configuration"]["root_module"] = json!({"module_calls":{"app":{"module":config}}});
        let graph = semantic::transform(&plan::parse(&json.to_string()).unwrap());
        let full = view::single(&graph, &view::Options::default()).unwrap();
        let selected = view::single(
            &graph,
            &view::Options {
                modules: vec!["module.app".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(full.coverage.selected, selected.coverage.selected);
        assert_eq!(full.nodes, selected.nodes);
        assert_eq!(full.relationships, selected.relationships);
    }
}

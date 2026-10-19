use crate::{layout::Layout, model::Placement, plan, semantic, svg, view};
use serde_json::{Value, json};

fn input() -> Value {
    json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_db_instance.database","type":"aws_db_instance","change":{"actions":["create"],"after":{"identifier":"PRIVATE_DATABASE"}}},
        {"address":"aws_cloudwatch_metric_alarm.cpu","type":"aws_cloudwatch_metric_alarm","change":{"actions":["update"],"after":{"namespace":"AWS/RDS","metric_name":"CPUUtilization","dimensions":{"DBInstanceIdentifier":"PRIVATE_DATABASE"}}}},
        {"address":"data.aws_cloudwatch_log_group.logs","type":"aws_cloudwatch_log_group","mode":"data","change":{"actions":["read"]}},
        {"address":"aws_ecs_service.app","type":"aws_ecs_service"}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_cloudwatch_metric_alarm.cpu","expressions":{"dimensions":{"references":["aws_db_instance.database.id"]}}},
        {"address":"aws_ecs_service.app","expressions":{"tags":{"references":["data.aws_cloudwatch_log_group.logs.name"]}}}
    ]}}})
}

#[test]
fn monitoring_requires_verified_values_and_preserves_native_edges() {
    for case in 0..12 {
        let mut value = input();
        match case {
            1 => {
                value["resource_changes"][1]["change"]["after_unknown"] = json!({"dimensions":true})
            }
            2 => {
                value["resource_changes"][1]["change"]["after"]["dimensions"]["DBInstanceIdentifier"] =
                    json!("other")
            }
            3 => value["resource_changes"][1]["change"]["after"]["namespace"] = json!("AWS/EC2"),
            4 => {
                value["configuration"]["root_module"]["resources"][0] = json!({"address":"aws_cloudwatch_metric_alarm.cpu","depends_on":["aws_db_instance.database"]})
            }
            5 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["dimensions"] = json!({"references":["aws_db_instance.database.id","aws_db_instance.missing.id"]})
            }
            6 => value["resource_changes"][0]["change"]["after_unknown"] = json!(true),
            7 => {
                value["resource_changes"][1]["change"]["after"]["dimensions"]["Other"] =
                    json!("PRIVATE_OTHER")
            }
            8 => {
                value["resource_changes"][1]["change"]["after"]["metric_query"] =
                    json!([{"id":"PRIVATE_QUERY"}])
            }
            9 => value["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            10 => {
                value["resource_changes"][1]["change"]["after"]["metric_query"] = json!([]);
                value["resource_changes"][1]["change"]["after_unknown"]["metric_query"] = json!([]);
            }
            11 => {
                value["resource_changes"][1]["change"]["after_unknown"]["metric_query"] =
                    json!([true])
            }
            _ => {}
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert_eq!(graph.edges.len(), parsed.edges.len());
        let database = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_db_instance.database")
            .unwrap();
        assert_eq!(
            database
                .references
                .iter()
                .any(|r| r.label == "Monitored by"),
            matches!(case, 0 | 10),
            "case {case}"
        );
        assert_eq!(
            semantic::diagnostics::collect(&parsed)
                .iter()
                .any(|d| d.reason == crate::model::DiagnosticReason::UnverifiedMonitoring),
            !matches!(case, 0 | 10)
        );
        let layout = Layout::new(&graph);
        assert!(graph.edges.iter().zip(&layout.paths).all(|(edge, path)| {
            path.is_empty() || graph.nodes[edge.from].resource_type == "aws_cloudwatch_log_group"
        }));
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|n| n.placement == Placement::Logical)
                .count(),
            1
        );
        let output = svg::render(&graph, &layout);
        assert!(!output.contains("PRIVATE_"));
        assert_eq!(
            output.contains("Monitors: unresolved"),
            !matches!(case, 0 | 10)
        );
        let doc = roxmltree::Document::parse(&output).unwrap();
        for href in doc.descendants().filter_map(|n| n.attribute("href")) {
            assert!(
                doc.descendants()
                    .any(|n| n.attribute("id") == href.strip_prefix('#'))
            );
        }
        let focused = view::single(
            &graph,
            &view::Options {
                focus: Some(database.address.clone()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        let output = svg::render(&focused, &Layout::new(&focused));
        assert_eq!(output.contains("Unavailable"), matches!(case, 0 | 10));
    }
}

#[test]
fn deposed_alarms_keep_identity_without_synthesized_monitoring() {
    let mut value = input();
    let mut alarm = value["resource_changes"][1].clone();
    alarm["deposed"] = json!("old");
    alarm["change"]["actions"] = json!(["delete"]);
    alarm["change"]["after"] = Value::Null;
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(alarm);
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let old = graph
        .nodes
        .iter()
        .find(|n| n.deposed_key.is_some())
        .unwrap();
    assert_eq!(old.placement, Placement::Logical);
    assert!(old.references.is_empty());
    assert!(
        !semantic::diagnostics::collect(&parsed)
            .iter()
            .any(|d| d.reason == crate::model::DiagnosticReason::UnverifiedMonitoring)
    );
    let database = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_db_instance.database")
        .unwrap();
    assert_eq!(database.references.len(), 1);
    assert!(database.references[0].targets[0].id.deposed_key.is_none());
    let output = svg::render(&graph, &Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    let old = doc
        .descendants()
        .find(|n| n.attribute("data-deposed-key") == Some("old"))
        .unwrap();
    assert!(
        !old.descendants()
            .any(|n| n.attribute("data-resource-references").is_some())
    );
}

#[test]
fn multiple_alarms_consumers_and_states_keep_distinct_identities() {
    let mut value = input();
    let mut alarm = value["resource_changes"][1].clone();
    alarm["address"] = json!(format!(
        "aws_cloudwatch_metric_alarm.{}",
        "long_name".repeat(30)
    ));
    let mut config = value["configuration"]["root_module"]["resources"][0].clone();
    config["address"] = alarm["address"].clone();
    value["resource_changes"].as_array_mut().unwrap().extend([
        alarm,
        json!({"address":"aws_ecs_service.second","type":"aws_ecs_service"}),
    ]);
    let mut service = value["configuration"]["root_module"]["resources"][1].clone();
    service["address"] = json!("aws_ecs_service.second");
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .extend([config, service]);
    let render = |value: &Value| {
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        assert_eq!(
            graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_db_instance.database")
                .unwrap()
                .references
                .len(),
            2
        );
        let architecture = crate::model::architecture::Architecture {
            states: ["first", "second"]
                .into_iter()
                .map(|state| (crate::model::StateId::new(state).unwrap(), graph.clone()))
                .collect(),
            relationships: Vec::new(),
        };
        svg::render_states(
            &view::apply(&architecture, &Default::default()).unwrap(),
            crate::svg::AddressFormat::default(),
        )
    };
    let output = render(&value);
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-placement") == Some("logical"))
            .count(),
        4
    );
    for href in doc.descendants().filter_map(|n| n.attribute("href")) {
        assert!(
            doc.descendants()
                .any(|n| n.attribute("id") == href.strip_prefix('#'))
        );
    }
    value["resource_changes"].as_array_mut().unwrap().reverse();
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(output, render(&value));
}

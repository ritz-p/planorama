use super::*;

#[test]
fn embedded_routes_preserve_room_for_concentrated_numbered_attachments() {
    use serde_json::json;
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/routes-plan.json")).unwrap();
    input["configuration"]["provider_config"] = json!({"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}});
    let mut resources = Vec::new();
    for (address, kind) in [
        ("aws_lb_target_group.app", "aws_lb_target_group"),
        ("aws_instance.worker", "aws_instance"),
    ] {
        resources.push(json!({"address": address, "type": kind, "change": {"actions": ["create"],"after":{"id":"instance","arn":"group","target_type":"instance"}}}));
        input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(json!({"address":address,"provider_config_key":"aws"}));
    }
    for index in 0..12 {
        let address = format!("aws_lb_target_group_attachment.worker_{index}");
        resources.push(json!({"address": address, "type": "aws_lb_target_group_attachment", "change": {"actions": ["create"],"after":{"target_group_arn":"group","target_id":"instance"}}}));
        input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(
                json!({"address": address, "provider_config_key":"aws", "expressions": {
                    "target_group_arn": {"references": ["aws_lb_target_group.app.arn"]},
                    "target_id": {"references": ["aws_instance.worker.id"]}
                }}),
            );
    }
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .extend(resources);
    let graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap()).0;
    let layout = Layout::new(&graph);
    let numbers = crate::layout::relationship_markers::numbers(&graph);
    let mut endpoints = std::collections::BTreeSet::new();
    let mut attachments = std::collections::BTreeSet::new();
    let mut numbered = 0;
    for (index, edge) in graph.edges.iter().enumerate() {
        if numbers[index][1].is_none() {
            continue;
        }
        numbered += 1;
        if let Some(change) = &edge.change {
            if change
                .address
                .starts_with("aws_lb_target_group_attachment.")
            {
                assert!(attachments.insert(change.address.as_str()));
            }
        }
        assert!(numbers[index].iter().all(Option::is_some));
        let path = &layout.paths[index];
        assert!(path.len() >= 2);
        let pair = [0, 1].map(|end| crate::layout::routing_shared::refinement::terminal(path, end));
        for (node, port) in [edge.from, edge.to].into_iter().zip(pair) {
            assert!(endpoints.insert((node, port.point.x, port.point.y)));
        }
        assert!(crate::layout::routing_shared::ports::valid_with_clearances(
            path,
            pair[0],
            pair[1],
            &crate::layout::routing_shared::spacing::barriers(&graph, &layout, index),
            crate::layout::relationship_markers::terminals(
                &graph,
                index,
                numbers[index],
                pair.map(|port| port.side)
            ),
        ));
    }
    assert_eq!(numbered, 14);
    assert_eq!(attachments.len(), 12);
    for index in 0..12 {
        assert!(
            attachments.contains(format!("aws_lb_target_group_attachment.worker_{index}").as_str())
        );
    }
}

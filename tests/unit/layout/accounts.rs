use crate::{layout::Layout, plan, semantic};

#[test]
fn conflicting_account_containment_is_not_grouped_under_either_account() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/regions-plan.json")).unwrap();
    for (key, id) in [
        ("aws.tokyo", "123456789012"),
        ("aws.virginia", "210987654321"),
    ] {
        input["configuration"]["provider_config"][key]["expressions"]["assume_role"] = serde_json::json!([{"role_arn":{"constant_value":format!("arn:aws:iam::{id}:role/Fixture")}}]);
    }
    input["configuration"]["root_module"]["resources"][3]["provider_config_key"] =
        serde_json::json!("aws.virginia");
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let layout = Layout::new(&graph);
    let vpc = graph
        .nodes
        .iter()
        .position(|n| n.address == "aws_vpc.tokyo")
        .unwrap();
    assert!(
        layout
            .scopes
            .iter()
            .all(|p| !super::tests::inside(layout.bounds[vpc], p.bounds))
    );
    assert_eq!(layout.containment.parents.iter().flatten().count(), 4);
}

#[test]
fn nested_accounts_preserve_partial_scopes_filtering_and_routes() {
    let input = include_str!("../../fixtures/accounts-plan.json");
    let architecture = semantic::transform(&plan::parse(input).unwrap());
    let graph = crate::view::single(&architecture, &Default::default()).unwrap();
    let layout = Layout::new(&graph);
    let accounts: Vec<_> = layout.scopes.iter().filter(|s| s.account_panel).collect();
    assert_eq!(accounts.len(), 2);
    assert_eq!(
        layout
            .scopes
            .iter()
            .filter(|s| s.scope.region.is_some())
            .count(),
        4
    );
    for panel in layout
        .scopes
        .iter()
        .filter(|s| !s.account_panel && s.scope.account.is_some())
    {
        let parent = accounts
            .iter()
            .find(|a| a.scope.account == panel.scope.account)
            .unwrap();
        assert!(super::tests::inside(panel.bounds, parent.bounds));
        assert!(panel.bounds.origin.y >= parent.bounds.origin.y + 40);
    }
    for (i, node) in graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.placement != crate::model::Placement::Logical)
    {
        let account = node.entity.scope.as_ref().and_then(|s| s.account.as_ref());
        let parent = accounts
            .iter()
            .find(|a| a.scope.account.as_ref() == account);
        if account.is_some() {
            assert!(super::tests::inside(
                layout.bounds[i],
                parent.unwrap().bounds
            ));
        } else {
            assert!(
                accounts
                    .iter()
                    .all(|a| !super::tests::inside(layout.bounds[i], a.bounds))
            );
        }
    }
    assert_eq!(layout.paths.iter().filter(|p| p.len() > 1).count(), 2);
    for path in &layout.paths {
        for panel in &layout.scopes {
            for segment in path.windows(2) {
                let header = panel.bounds.header(36);
                assert!(!crate::layout::routing_shared::crosses(
                    segment[0], segment[1], header
                ));
            }
        }
    }
    let filtered = crate::view::single(
        &architecture,
        &crate::view::Options {
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    let filtered_layout = Layout::new(&filtered);
    assert_eq!(filtered_layout.scopes.len(), 2);
    assert!(filtered_layout.scopes[0].account_panel);
    assert!(filtered_layout.scopes[1].scope.region.is_some());
    let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
    value["resource_changes"][0]["change"]["actions"] = serde_json::json!(["no-op"]);
    let unchanged = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let empty = crate::view::single(
        &unchanged,
        &crate::view::Options {
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(Layout::new(&empty).scopes.is_empty());
}

#[test]
fn account_only_and_global_only_still_receive_panels() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/accounts-plan.json")).unwrap();
    for (address, panels) in [
        ("aws_s3_bucket.unknown_region", 1),
        ("aws_cloudfront_distribution.global", 2),
        ("aws_iam_role.logical", 0),
    ] {
        let mut input = source.clone();
        input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .retain(|n| n["address"] == address);
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        let viewed = crate::view::single(&graph, &Default::default()).unwrap();
        assert_eq!(Layout::new(&viewed).scopes.len(), panels);
    }
}

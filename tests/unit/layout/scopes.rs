use crate::{
    layout::{Bounds, Layout},
    plan, semantic,
};

#[test]
fn unrelated_root_columns_and_regions_refine_beside_affinity() {
    use crate::layout::{Point, containers::placement};
    use std::collections::BTreeSet;
    let sizes = vec![(100, 80); 5];
    let original = vec![
        Point { x: 0, y: 0 },
        Point { x: 140, y: 0 },
        Point { x: 140, y: 120 },
        Point { x: 280, y: 0 },
        Point { x: 280, y: 120 },
    ];
    for roots in [vec![0, 1, 2, 3, 4], vec![1, 2, 3, 4]] {
        let mut offsets = original.clone();
        if !roots.contains(&0) {
            for &root in &roots {
                offsets[root].x -= 140;
            }
        }
        let movable = super::movable_roots(&roots, &BTreeSet::from([0]), &offsets);
        assert_eq!(movable, vec![1, 2, 3, 4]);
        placement::refine(&movable, &[(1, 4), (2, 3)], &sizes, &mut offsets, 200, 40);
        assert_eq!(offsets[0], original[0]);
        assert_eq!(offsets[1].y, offsets[4].y);
        assert_eq!(offsets[2].y, offsets[3].y);
    }
}

pub(super) fn inside(inner: Bounds, outer: Bounds) -> bool {
    inner.origin.x >= outer.origin.x
        && inner.origin.y >= outer.origin.y
        && inner.right() <= outer.right()
        && inner.origin.y + inner.height <= outer.origin.y + outer.height
}

#[test]
fn singleton_regions_follow_displayed_spatial_resources_after_filtering() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/multi/single-region.json")).unwrap();
    input["configuration"]["provider_config"]["aws.other"] = serde_json::json!({
        "full_name":"hashicorp/aws", "alias":"other", "expressions":{"region":{"constant_value":"us-east-1"}}
    });
    for (address, resource_type, provider, action) in [
        (
            "aws_s3_bucket.second",
            "aws_s3_bucket",
            "aws.archive",
            "no-op",
        ),
        ("aws_s3_bucket.other", "aws_s3_bucket", "aws.other", "no-op"),
        (
            "aws_cloudwatch_log_group.other",
            "aws_cloudwatch_log_group",
            "aws.other",
            "update",
        ),
    ] {
        input["resource_changes"].as_array_mut().unwrap().push(serde_json::json!({"address":address,"type":resource_type,"change":{"actions":[action]}}));
        input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"address":address,"provider_config_key":provider}));
    }
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let full = crate::view::single(&graph, &Default::default()).unwrap();
    let full_layout = Layout::new(&full);
    assert_eq!(full_layout.scopes.len(), 2);
    for (region, count) in [("ap-northeast-1", 2), ("us-east-1", 1)] {
        let panel = full_layout
            .scopes
            .iter()
            .find(|panel| panel.scope.region.as_ref().unwrap().0 == region)
            .unwrap();
        assert_eq!(
            full_layout
                .bounds
                .iter()
                .filter(|bounds| bounds.width > 0 && inside(**bounds, panel.bounds))
                .count(),
            count
        );
    }
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    let layout = Layout::new(&filtered);
    assert_eq!(filtered.nodes.len(), 2);
    assert_eq!(layout.scopes.len(), 1);
    let panel = &layout.scopes[0];
    assert_eq!(panel.scope.region.as_ref().unwrap().0, "ap-northeast-1");
    assert_eq!(
        layout
            .bounds
            .iter()
            .filter(|bounds| bounds.width > 0 && inside(**bounds, panel.bounds))
            .count(),
        1
    );
    let original = full_layout
        .scopes
        .iter()
        .find(|p| p.scope == panel.scope)
        .unwrap();
    assert!(
        panel.bounds.width * panel.bounds.height < original.bounds.width * original.bounds.height
    );

    input["resource_changes"][0]["change"]["actions"] = serde_json::json!(["no-op"]);
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    let layout = Layout::new(&filtered);
    assert_eq!(filtered.nodes.len(), 1);
    assert_eq!(
        filtered.nodes[0].placement,
        crate::model::Placement::Logical
    );
    assert!(layout.scopes.is_empty());
    let mut unscoped = (*filtered).clone();
    for node in &mut unscoped.nodes {
        node.entity.scope = None;
    }
    let unscoped_layout = Layout::new(&unscoped);
    assert_eq!(
        (layout.width, layout.height),
        (unscoped_layout.width, unscoped_layout.height)
    );
}

#[test]
fn a_single_region_encloses_one_resource_or_a_complete_nested_subtree() {
    let raw = plan::parse(include_str!("../../fixtures/regions-plan.json")).unwrap();
    let graph = semantic::transform(&raw);
    for (focus, count) in [("aws_vpc.tokyo_second", 1), ("aws_instance.tokyo", 3)] {
        let viewed = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some(focus.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        let layout = Layout::new(&viewed);
        assert_eq!(viewed.nodes.len(), count);
        assert_eq!(layout.scopes.len(), 1);
        let panel = &layout.scopes[0];
        assert_eq!(panel.scope.region.as_ref().unwrap().0, "ap-northeast-1");
        assert!(
            layout
                .bounds
                .iter()
                .all(|bounds| inside(*bounds, panel.bounds))
        );
        assert_eq!(
            layout.containment.parents.iter().flatten().count(),
            count - 1
        );
    }
}

#[test]
fn regions_preserve_containment_global_unknown_and_cross_region_routes() {
    let raw = plan::parse(include_str!("../../fixtures/regions-plan.json")).unwrap();
    let graph = semantic::transform(&raw);
    let layout = Layout::new(&graph);
    assert_eq!(layout.scopes.len(), 3);
    let node = |address: &str| {
        graph
            .nodes
            .iter()
            .position(|n| n.address == address)
            .unwrap()
    };
    for (name, region) in [("tokyo", "ap-northeast-1"), ("virginia", "us-east-1")] {
        let panel = layout
            .scopes
            .iter()
            .find(|p| p.scope.region.as_ref().is_some_and(|r| r.0 == region))
            .unwrap();
        let vpc = node(&format!("aws_vpc.{name}"));
        let subnet = node(&format!("aws_subnet.{name}"));
        let instance = node(&format!("aws_instance.{name}"));
        assert_eq!(layout.containment.parents[subnet], Some(vpc));
        assert_eq!(layout.containment.parents[instance], Some(subnet));
        assert!(inside(layout.bounds[vpc], panel.bounds));
        assert!(inside(layout.bounds[subnet], layout.bounds[vpc]));
        assert!(inside(layout.bounds[instance], layout.bounds[subnet]));
        if name == "tokyo" {
            assert!(inside(
                layout.bounds[node("aws_vpc.tokyo_second")],
                panel.bounds
            ));
        }
    }
    let global = layout
        .scopes
        .iter()
        .find(|p| p.scope.region.is_none())
        .unwrap();
    for address in ["aws_cloudfront_distribution.app", "aws_route53_zone.app"] {
        assert!(inside(layout.bounds[node(address)], global.bounds));
    }
    assert_eq!(layout.bounds[node("aws_iam_role.app")].width, 0);
    for address in ["aws_instance.unknown", "aws_unclassified.app"] {
        assert!(
            layout
                .scopes
                .iter()
                .all(|p| !inside(layout.bounds[node(address)], p.bounds))
        );
    }
    let edge = graph
        .edges
        .iter()
        .position(|e| e.from == node("aws_instance.tokyo") && e.to == node("aws_instance.virginia"))
        .unwrap();
    assert!(!layout.paths[edge].is_empty());
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        for segment in path.windows(2) {
            assert!(segment[0].x == segment[1].x || segment[0].y == segment[1].y);
            for panel in &layout.scopes {
                assert!(!crate::layout::routing_shared::crosses(
                    segment[0],
                    segment[1],
                    panel.bounds.header(36)
                ));
            }
            for (index, bounds) in layout.bounds.iter().enumerate() {
                if index != edge.from
                    && index != edge.to
                    && !layout.containment.is_ancestor(index, edge.from)
                    && !layout.containment.is_ancestor(index, edge.to)
                {
                    assert!(!crate::layout::routing_shared::crosses(
                        segment[0], segment[1], *bounds
                    ));
                }
            }
        }
    }
}

#[test]
fn inconsistent_subtrees_remain_outside_panels_without_reparenting() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/regions-plan.json")).unwrap();
    value["configuration"]["root_module"]["resources"][3]["provider_config_key"] =
        serde_json::json!("aws.unknown");
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
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
            .all(|p| !inside(layout.bounds[vpc], p.bounds))
    );
    assert_eq!(
        layout
            .containment
            .parents
            .iter()
            .filter(|p| p.is_some())
            .count(),
        4
    );
}

use crate::{layout::Layout, plan, semantic, svg, view};
use std::collections::BTreeSet;

#[test]
fn data_icon_fixture_preserves_external_styling_and_matches_deterministic_example() {
    let input = include_str!("../../fixtures/data-icons-plan.json");
    let raw = plan::parse(input).unwrap();
    let graph = view::single(&semantic::transform(&raw), &Default::default()).unwrap();
    let output = svg::render(&graph, &Layout::new(&graph));
    let document = roxmltree::Document::parse(&output).unwrap();
    let expected = [
        ("data.aws_subnets.shared", Some("#planorama-icon-aws-vpc")),
        (
            "data.aws_ami_ids.available",
            Some("#planorama-icon-aws-ec2"),
        ),
        ("data.aws_instances.shared", Some("#planorama-icon-aws-ec2")),
        ("data.aws_lbs.shared", Some("#planorama-icon-aws-elb")),
        ("data.aws_iam_roles.shared", Some("#planorama-icon-aws-iam")),
        (
            "data.aws_security_groups.shared",
            Some("#planorama-icon-aws-vpc"),
        ),
        ("data.aws_s3_objects.assets", Some("#planorama-icon-aws-s3")),
        (
            "data.aws_lambda_alias.live",
            Some("#planorama-icon-aws-lambda"),
        ),
        (
            "data.aws_eks_clusters.shared",
            Some("#planorama-icon-aws-eks"),
        ),
        (
            "data.aws_db_snapshot.backup",
            Some("#planorama-icon-aws-rds"),
        ),
        (
            "data.aws_elasticache_replication_group.shared",
            Some("#planorama-icon-aws-cache"),
        ),
        (
            "data.aws_cloudfront_origin_access_control.shared",
            Some("#planorama-icon-aws-cloudfront"),
        ),
        (
            "data.aws_apigatewayv2_apis.shared",
            Some("#planorama-icon-aws-api"),
        ),
        ("aws_lambda_alias.app", Some("#planorama-icon-aws-lambda")),
        ("aws_s3_bucket.assets", Some("#planorama-icon-aws-s3")),
        ("data.aws_subnets.foreign", None),
        ("data.aws_future_widget.unknown", None),
    ];
    assert_eq!(graph.nodes.len(), expected.len());
    assert_eq!(raw.nodes.len(), expected.len() + 3);
    let symbols: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("symbol"))
        .map(|node| node.attribute("id").unwrap())
        .collect();
    let expected_symbols: BTreeSet<_> = expected
        .iter()
        .filter_map(|(_, icon)| icon.map(|href| href.strip_prefix('#').unwrap()))
        .collect();
    assert_eq!(symbols.len(), expected_symbols.len());
    assert_eq!(
        symbols.into_iter().collect::<BTreeSet<_>>(),
        expected_symbols
    );
    assert!(
        !document
            .descendants()
            .any(|node| node.has_tag_name("image"))
    );
    for (address, icon) in expected {
        let card = document
            .descendants()
            .find(|node| node.attribute("data-terraform-address") == Some(address))
            .unwrap();
        let uses: Vec<_> = card
            .children()
            .filter(|node| node.has_tag_name("use"))
            .map(|node| node.attribute("href").unwrap())
            .collect();
        assert_eq!(uses, icon.into_iter().collect::<Vec<_>>(), "{address}");
        let is_data = address.starts_with("data.");
        assert_eq!(
            card.descendants()
                .any(|node| node.attribute("data-external") == Some("true")),
            is_data,
            "{address}"
        );
        let border = card
            .children()
            .find(|node| {
                node.attribute("data-resource-underline") == Some("true")
                    || (node.has_tag_name("rect") && node.attribute("data-mode").is_some())
            })
            .unwrap();
        assert_eq!(
            border.attribute("stroke-dasharray"),
            is_data.then_some("6 4"),
            "{address}"
        );
    }
    for address in [
        "data.aws_region.current",
        "data.aws_caller_identity.current",
        "data.aws_iam_policy_document.app",
    ] {
        assert!(
            !document
                .descendants()
                .any(|node| node.attribute("data-terraform-address") == Some(address)),
            "{address}"
        );
    }
    let snapshot = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data-icons.svg");
    assert_eq!(
        output,
        std::fs::read_to_string(snapshot)
            .unwrap()
            .replace("\r\n", "\n")
    );
    let mut reordered: serde_json::Value = serde_json::from_str(input).unwrap();
    reordered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let raw = plan::parse(&reordered.to_string()).unwrap();
    let graph = view::single(&semantic::transform(&raw), &Default::default()).unwrap();
    assert_eq!(output, svg::render(&graph, &Layout::new(&graph)));
}

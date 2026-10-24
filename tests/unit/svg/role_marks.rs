use crate::{NamedInput, Options, OutputFormat, generate, generate_named};

const INPUT: &str = include_str!("../../fixtures/role-shapes-plan.json");

#[test]
fn role_marks_preserve_provider_boundaries_and_fit_inside_individual_cards() {
    let output = generate(INPUT, &Options::default()).unwrap();
    let doc = roxmltree::Document::parse(&output.content).unwrap();
    let marks: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-role-shape").is_some())
        .collect();
    assert_eq!(marks.len(), 4);
    for mark in marks {
        let card = mark.parent().unwrap();
        let address = card.attribute("data-terraform-address").unwrap();
        let expected =
            if address.contains("aws_instance.") || address.contains("aws_lambda_function.") {
                "execution"
            } else if address == "aws_s3_bucket.archive" {
                "data-store"
            } else {
                assert_eq!(address, "aws_nat_gateway.egress");
                "network"
            };
        assert_eq!(mark.attribute("data-role-shape"), Some(expected));
        let rect = card
            .children()
            .find(|n| n.has_tag_name("rect") && n.attribute("data-service-badge").is_none())
            .unwrap();
        let number =
            |n: roxmltree::Node, key: &str| n.attribute(key).unwrap().parse::<usize>().unwrap();
        let position: Vec<usize> = mark
            .attribute("transform")
            .unwrap()
            .strip_prefix("translate(")
            .unwrap()
            .strip_suffix(')')
            .unwrap()
            .split_whitespace()
            .map(|n| n.parse().unwrap())
            .collect();
        assert!(position[0] > number(rect, "x"));
        assert!(position[0] + 22 < number(rect, "x") + number(rect, "width"));
        assert!(position[1] > number(rect, "y"));
        assert!(position[1] + 18 < number(rect, "y") + number(rect, "height"));
        let badge = card
            .children()
            .find(|n| n.attribute("data-service-badge").is_some())
            .unwrap();
        assert!(number(badge, "x") + number(badge, "width") < position[0]);
        for text in card.children().filter(|n| {
            n.has_tag_name("text")
                && n.attribute("font-size") == Some("13")
                && n.attribute("data-service-name").is_none()
        }) {
            assert!(number(text, "y") >= position[1] + 18 + 13);
        }
        assert_eq!(mark.attribute("pointer-events"), Some("none"));
    }
}

#[test]
fn named_state_svg_and_html_share_roles_without_changing_resource_identity() {
    let inputs = [
        NamedInput {
            state: "one".into(),
            json: INPUT.into(),
        },
        NamedInput {
            state: "two".into(),
            json: INPUT.into(),
        },
    ];
    for format in [OutputFormat::Svg, OutputFormat::Html] {
        let output = generate_named(
            &inputs,
            &Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        let content = &output.content;
        let svg = &content[content.find("<svg").unwrap()..content.rfind("</svg>").unwrap() + 6];
        let doc = roxmltree::Document::parse(svg).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("data-role-shape").is_some())
                .count(),
            8
        );
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("data-terraform-address").is_some())
                .count(),
            16
        );
        let ids: Vec<_> = doc
            .descendants()
            .filter_map(|n| n.attribute("id"))
            .collect();
        assert_eq!(
            ids.len(),
            ids.iter().collect::<std::collections::BTreeSet<_>>().len()
        );
        assert!(!svg.contains("<symbol"));
    }
}

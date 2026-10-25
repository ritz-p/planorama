use crate::{NamedInput, Options, OutputFormat, generate, generate_named};

const INPUT: &str = include_str!("../../fixtures/role-shapes-plan.json");

#[test]
fn resource_headers_reclaim_role_mark_space() {
    let output = generate(INPUT, &Options::default()).unwrap();
    assert!(!output.content.contains("data-role-shape"));
    assert!(!output.content.contains("Visual role:"));
    let doc = roxmltree::Document::parse(&output.content).unwrap();
    for heading in doc
        .descendants()
        .filter(|n| n.attribute("data-resource-type-heading").is_some())
    {
        let card = heading.parent().unwrap();
        if let Some(rect) = card
            .children()
            .find(|n| n.has_tag_name("rect") && n.attribute("data-service-badge").is_none())
        {
            let x = |n: roxmltree::Node| n.attribute("x").unwrap().parse::<usize>().unwrap();
            assert_eq!(x(heading), x(rect) + 12);
        }
    }
}

#[test]
fn named_state_svg_and_html_omit_marks_without_changing_resource_identity() {
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
            0
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

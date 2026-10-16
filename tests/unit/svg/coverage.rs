use crate::{html, layout::Layout, plan, semantic, svg, view};

fn summaries(text: &str) -> Vec<serde_json::Value> {
    let start = text.find("<svg ").unwrap();
    let end = text.rfind("</svg>").unwrap() + 6;
    let doc = roxmltree::Document::parse(&text[start..end]).unwrap();
    doc.descendants()
        .filter(|node| node.attribute("data-view-coverage") == Some("true"))
        .map(|node| serde_json::from_str(node.text().unwrap()).unwrap())
        .collect()
}

#[test]
fn filtered_summary_agrees_between_html_and_svg_without_excluded_identities() {
    let raw = plan::parse(include_str!("../../fixtures/containment-plan.json")).unwrap();
    let graph = semantic::transform(&raw);
    let selected = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_vpc.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let layout = Layout::new(&selected);
    let svg = svg::render(&selected, &layout);
    let html = html::render(&selected, &layout, svg::AddressFormat::Qualified);
    assert_eq!(summaries(&svg), summaries(&html));
    let summary = &summaries(&svg)[0];
    assert_eq!(summary["input"]["resources"], raw.nodes.len());
    assert_eq!(summary["selected"]["resources"], 1);
    assert_eq!(summary["excluded"], raw.nodes.len() - 1);
    assert_eq!(summary["focused"], true);
    assert!(!summary.to_string().contains("aws_instance.app"));
    assert!(svg.contains("Input completeness: unknown"));
}

#[test]
fn aggregate_reconciles_named_states_and_preserves_empty_state_counts() {
    let raw = plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"terraform_data.x","type":"terraform_data"}]}"#).unwrap();
    let long = "a".repeat(64);
    let architecture = crate::model::cross_state::Architecture {
        states: [&long[..], "empty"]
            .into_iter()
            .map(|id| {
                (
                    crate::model::StateId::new(id).unwrap(),
                    semantic::transform(&raw),
                )
            })
            .collect(),
        relationships: Vec::new(),
    };
    let selected = view::apply(
        &architecture,
        &view::Options {
            focus: Some(format!("{long}:terraform_data.x")),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let svg = svg::render_states(&selected, svg::AddressFormat::Qualified);
    let html = html::render_states(&selected, svg::AddressFormat::Qualified);
    let values = summaries(&svg);
    assert_eq!(values, summaries(&html));
    assert_eq!(values.len(), 3);
    assert_eq!(values[0]["input"]["resources"], 2);
    assert_eq!(values[0]["selected"]["resources"], 1);
    assert_eq!(values[2]["selected"]["resources"], 0);
    assert!(svg.contains("not applicable"));
}

#[test]
fn measured_coverage_wraps_and_distinguishes_unavailable_from_zero() {
    let card = super::card(std::iter::once(("", &Default::default())));
    assert!(card.content.contains("analysis unavailable"));
    let count = card.content.matches("<text ").count();
    assert_eq!(card.height, count * 20 + 32);
    let raw =
        plan::parse(r#"{"format_version":"1.2","complete":false,"resource_changes":[]}"#).unwrap();
    let graph = semantic::transform(&raw);
    let card = super::card(std::iter::once(("", &graph.coverage)));
    assert!(card.content.contains("input 0 | selected 0 | excluded 0"));
    assert!(card.content.contains("Input completeness: incomplete"));
    assert!(!card.content.contains("analysis unavailable"));
}

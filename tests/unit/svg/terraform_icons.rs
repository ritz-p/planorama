use crate::{layout::Layout, plan, semantic, svg};
use std::collections::{BTreeMap, BTreeSet};

fn terraform_plan() -> serde_json::Value {
    serde_json::json!({"format_version":"1.2","resource_changes":[
        {"address":"terraform_data.main","type":"terraform_data","mode":"managed","provider_name":"terraform.io/builtin/terraform"},
        {"address":"data.terraform_remote_state.shared","type":"terraform_remote_state","mode":"data","provider_name":"terraform.io/builtin/terraform"}
    ]})
}

fn assert_self_contained(document: &roxmltree::Document<'_>) {
    let mut ids = BTreeSet::new();
    for id in document.descendants().filter_map(|n| n.attribute("id")) {
        assert!(ids.insert(id), "duplicate id: {id}");
    }
    for node in document.descendants().filter(|n| n.is_element()) {
        assert!(!node.has_tag_name("image"));
        for attribute in node.attributes() {
            let target = if attribute.name() == "href" {
                Some(attribute.value().strip_prefix('#').unwrap())
            } else if attribute.value().starts_with("url(") {
                Some(
                    attribute
                        .value()
                        .strip_prefix("url(#")
                        .unwrap()
                        .strip_suffix(')')
                        .unwrap(),
                )
            } else {
                None
            };
            if let Some(target) = target {
                assert!(ids.contains(target), "missing local target: {target}");
            }
        }
    }
}

fn assert_builtin_notice(output: &str, notice: &str) {
    assert!(notice.contains("Planorama built-in provider icon, version 1.0.0."));
    assert!(notice.contains("MIT License"));
    assert!(notice.contains("Copyright (c) 2026 Planorama contributors"));
    assert!(notice.contains(include_str!("../../../assets/icons/terraform/LICENSE.txt")));
    for removed in [
        "HashiCorp Flight Icons",
        "MPL-2.0",
        "Mozilla Public License",
        "unconfirmed",
    ] {
        assert!(
            !output.contains(removed),
            "unexpected bundled notice: {removed}"
        );
    }
}

#[test]
fn builtin_and_aws_icons_preserve_provider_boundaries_and_external_styling() {
    let mut input = terraform_plan();
    input["resource_changes"].as_array_mut().unwrap().extend([
        serde_json::json!({"address":"aws_instance.app","type":"aws_instance"}),
        serde_json::json!({"address":"data.aws_ami.current","type":"aws_ami","mode":"data"}),
        serde_json::json!({"address":"terraform_data.foreign","type":"terraform_data","provider_name":"registry.terraform.io/example/terraform"}),
        serde_json::json!({"address":"data.terraform_remote_state.foreign","type":"terraform_remote_state","mode":"data","provider_name":"registry.terraform.io/example/terraform"}),
        serde_json::json!({"address":"terraform_future.main","type":"terraform_future","provider_name":"terraform.io/builtin/terraform"}),
        serde_json::json!({"address":"custom_resource.main","type":"custom_resource"}),
    ]);
    let raw = plan::parse(&input.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    let layout = Layout::new(&graph);
    for format in [svg::AddressFormat::Qualified, svg::AddressFormat::Terraform] {
        let output = svg::render_with_format(&graph, &layout, format);
        let document = roxmltree::Document::parse(&output).unwrap();
        assert_self_contained(&document);
        let symbols: BTreeSet<_> = document
            .descendants()
            .filter(|n| n.has_tag_name("symbol"))
            .map(|n| n.attribute("id").unwrap())
            .collect();
        assert_eq!(
            symbols,
            BTreeSet::from(["planorama-icon-terraform", "planorama-icon-aws-ec2"])
        );
        for (address, icon) in [
            ("terraform_data.main", Some("#planorama-icon-terraform")),
            (
                "data.terraform_remote_state.shared",
                Some("#planorama-icon-terraform"),
            ),
            ("aws_instance.app", Some("#planorama-icon-aws-ec2")),
            ("data.aws_ami.current", Some("#planorama-icon-aws-ec2")),
            ("terraform_data.foreign", None),
            ("data.terraform_remote_state.foreign", None),
            ("terraform_future.main", None),
            ("custom_resource.main", None),
        ] {
            let card = document
                .descendants()
                .find(|n| n.attribute("data-terraform-address") == Some(address))
                .unwrap();
            let uses: Vec<_> = card
                .children()
                .filter(|n| n.has_tag_name("use"))
                .map(|n| n.attribute("href").unwrap())
                .collect();
            assert_eq!(uses, icon.into_iter().collect::<Vec<_>>(), "{address}");
            let is_data = address.starts_with("data.");
            let mode = Some(if is_data { "data" } else { "managed" });
            let stroke = if card.attribute("data-placement") == Some("logical") {
                assert_eq!(card.attribute("data-mode"), mode);
                assert_eq!(card.attribute("data-external"), is_data.then_some("true"));
                let group = card.parent_element().unwrap();
                assert_eq!(group.attribute("data-logical-group"), Some("true"));
                assert_eq!(group.attribute("data-mode"), mode);
                assert_eq!(
                    group.children().any(|n| {
                        n.has_tag_name("text")
                            && n.text()
                                .is_some_and(|text| text.contains("data / external"))
                    }),
                    is_data
                );
                card.children()
                    .find(|n| {
                        n.has_tag_name("line")
                            && n.attribute("data-resource-underline") == Some("true")
                    })
                    .unwrap()
            } else {
                let border = card
                    .children()
                    .find(|n| n.has_tag_name("rect") && n.attribute("data-mode").is_some())
                    .unwrap();
                assert_eq!(border.attribute("data-mode"), mode);
                let external = card
                    .children()
                    .find(|n| n.attribute("data-external") == Some("true"));
                assert_eq!(external.is_some(), is_data, "{address}");
                if let Some(external) = external {
                    assert!(
                        external
                            .children()
                            .any(|n| n.has_tag_name("text") && n.text() == Some("external"))
                    );
                }
                border
            };
            assert_eq!(
                stroke.attribute("stroke-dasharray"),
                is_data.then_some("6 4")
            );
        }
        let notices: Vec<_> = document
            .descendants()
            .filter(|n| {
                n.has_tag_name("metadata")
                    && n.parent_element().is_some_and(|p| p.has_tag_name("defs"))
            })
            .filter_map(|n| n.text())
            .collect();
        assert_eq!(notices.len(), 1);
        assert_builtin_notice(&output, notices[0]);
        for provider in ["Planorama built-in provider icon", "AWS Architecture Icons"] {
            assert_eq!(notices[0].matches(provider).count(), 1);
        }
    }
}

#[test]
fn terraform_only_diagrams_embed_only_the_terraform_notice() {
    let raw = plan::parse(&terraform_plan().to_string()).unwrap();
    let graph = semantic::transform(&raw);
    let output = svg::render(&graph, &Layout::new(&graph));
    let document = roxmltree::Document::parse(&output).unwrap();
    assert_self_contained(&document);
    let notices: Vec<_> = document
        .descendants()
        .filter(|n| {
            n.has_tag_name("metadata") && n.parent_element().is_some_and(|p| p.has_tag_name("defs"))
        })
        .filter_map(|n| n.text())
        .collect();
    assert_eq!(notices.len(), 1);
    assert_builtin_notice(&output, notices[0]);
    assert!(!notices[0].contains("AWS Architecture Icons"));
    assert!(!output.contains("planorama-icon-aws-"));
}

#[test]
fn named_states_scope_shared_terraform_symbols_and_their_references() {
    let raw = plan::parse(&terraform_plan().to_string()).unwrap();
    let graph = semantic::transform(&raw);
    let architecture = crate::model::architecture::Architecture {
        states: BTreeMap::from([
            (crate::model::StateId::new("one").unwrap(), graph.clone()),
            (crate::model::StateId::new("two").unwrap(), graph),
        ]),
        relationships: Vec::new(),
    };
    let view = crate::view::apply(&architecture, &Default::default()).unwrap();
    let output = svg::render_states(&view, svg::AddressFormat::Qualified);
    let document = roxmltree::Document::parse(&output).unwrap();
    assert_self_contained(&document);
    let sections: Vec<_> = document
        .descendants()
        .filter(|n| n.attribute("data-state-id").is_some())
        .collect();
    assert_eq!(sections.len(), 2);
    for section in sections {
        let symbols: Vec<_> = section
            .descendants()
            .filter(|n| n.has_tag_name("symbol"))
            .collect();
        assert_eq!(symbols.len(), 1);
        let symbol_id = symbols[0].attribute("id").unwrap();
        assert!(symbol_id.ends_with("planorama-icon-terraform"));
        assert_ne!(symbol_id, "planorama-icon-terraform");
        let expected = format!("#{symbol_id}");
        let uses: Vec<_> = section
            .descendants()
            .filter(|n| {
                n.has_tag_name("use")
                    && n.parent_element()
                        .is_some_and(|p| p.attribute("data-terraform-address").is_some())
            })
            .collect();
        assert_eq!(uses.len(), 2);
        for icon in uses {
            assert_eq!(icon.attribute("href"), Some(expected.as_str()));
        }
    }
}

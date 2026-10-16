use super::cli;

fn svg(text: &str) -> &str {
    &text[text.find("<svg ").unwrap()..text.rfind("</svg>").unwrap() + 6]
}

#[test]
fn hidden_relationship_explanations_preserve_graph_and_html_options() {
    let root = tempfile::tempdir().unwrap();
    for inputs in [
        vec!["tests/fixtures/checks-plan.json"],
        vec!["tests/fixtures/association-plan.json"],
        vec![
            "--state",
            "one=tests/fixtures/association-plan.json",
            "--state",
            "two=tests/fixtures/association-plan.json",
        ],
        vec![
            "--state",
            "producer=tests/fixtures/multi/producer.json",
            "--state",
            "consumer=tests/fixtures/multi/consumer.json",
            "--remote-state",
            "consumer:data.terraform_remote_state.network=producer",
        ],
    ] {
        let mut hidden_svg = None;
        for format in ["svg", "html"] {
            let command = || {
                let mut command = cli();
                command.args(&inputs).args([
                    "-o",
                    "-",
                    "--format",
                    format,
                    "--icon-mode",
                    "service-name",
                    "--address-format",
                    "terraform",
                ]);
                command
            };
            let plain = command().output().unwrap();
            assert!(plain.status.success());
            let plain = String::from_utf8(plain.stdout).unwrap();
            let report = root.path().join("coverage.md");
            let layout_report = root.path().join("layout.json");
            let hidden = command()
                .arg("--hide-relationship-legend")
                .arg("--coverage-report")
                .arg(&report)
                .arg("--layout-diagnostics-json")
                .arg(&layout_report)
                .output()
                .unwrap();
            assert!(
                hidden.status.success(),
                "{}",
                String::from_utf8_lossy(&hidden.stderr)
            );
            let hidden = String::from_utf8(hidden.stdout).unwrap();
            let original = roxmltree::Document::parse(svg(&plain)).unwrap();
            let compact = roxmltree::Document::parse(svg(&hidden)).unwrap();
            assert!(!svg(&hidden).contains("Relationship lines"));
            assert!(!svg(&hidden).contains("data-legend-entry"));
            assert!(!svg(&hidden).contains("Dashed blue: cross-state dependency"));
            let height = |doc: &roxmltree::Document<'_>| {
                doc.root_element()
                    .attribute("height")
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
            };
            if plain.contains("Relationship lines") {
                assert!(height(&compact) < height(&original));
            } else {
                assert_eq!(height(&compact), height(&original));
            }
            let records = |doc: &roxmltree::Document<'_>| {
                doc.descendants()
                    .filter(|n| {
                        n.attribute("data-edge-kind").is_some()
                            || n.attribute("data-terraform-address").is_some()
                            || n.attribute("data-relationship-entry").is_some()
                            || n.attribute("data-relationship-marker").is_some()
                    })
                    .map(|n| {
                        n.attributes()
                            .filter(|a| !matches!(a.name(), "d" | "transform"))
                            .map(|a| (a.name().to_owned(), a.value().to_owned()))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(records(&original), records(&compact));
            let paths = |doc: &roxmltree::Document<'_>| {
                doc.descendants()
                    .filter(|n| {
                        n.has_tag_name("path")
                            && n.attribute("data-edge-kind").is_some()
                            && n.attribute("data-cross-state").is_none()
                    })
                    .map(|n| n.attribute("d").unwrap().to_owned())
                    .collect::<Vec<_>>()
            };
            assert_eq!(paths(&original), paths(&compact));
            for label in [
                "create (",
                "update (",
                "delete (",
                "replace (",
                "unchanged (",
            ] {
                assert_eq!(plain.matches(label).count(), hidden.matches(label).count());
            }
            assert_eq!(
                plain.matches(">Relationship Resources</text>").count(),
                hidden.matches(">Relationship Resources</text>").count()
            );
            let layout: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&layout_report).unwrap()).unwrap();
            assert_eq!(
                layout["canvas"]["height"].as_u64().unwrap() as usize,
                height(&compact)
            );
            assert!(
                std::fs::read_to_string(&report)
                    .unwrap()
                    .contains("# Terraform coverage report")
            );
            if let Some(expected) = &hidden_svg {
                assert_eq!(expected, svg(&hidden));
            } else {
                hidden_svg = Some(svg(&hidden).to_owned());
            }
        }
    }
    assert!(
        String::from_utf8(cli().arg("--help").output().unwrap().stdout)
            .unwrap()
            .contains("--hide-relationship-legend")
    );
}

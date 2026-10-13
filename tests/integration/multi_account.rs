#[path = "../../examples/multi-account/generator.rs"]
mod generator;

#[test]
fn multi_account_inputs_and_committed_topology_are_current() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let inputs = generator::generate(
        &crate::generator::generate(&root.join("examples/terraform-large")).unwrap(),
    )
    .unwrap();
    let mut count = 0;
    for (state, generated) in inputs {
        assert_eq!(
            generated,
            std::fs::read_to_string(root.join(format!("examples/multi-account/{state}.json")))
                .unwrap()
                .replace("\r\n", "\n")
        );
        let value: serde_json::Value = serde_json::from_str(&generated).unwrap();
        count += value["resource_changes"].as_array().unwrap().len();
    }
    assert_eq!(count, 250);
    let svg = std::fs::read_to_string(root.join("examples/multi-account/diagram.svg")).unwrap();
    let document = roxmltree::Document::parse(&svg).unwrap();
    let main = document
        .descendants()
        .find(|n| n.attribute("data-state-id") == Some("application-a"))
        .unwrap();
    assert_eq!(
        main.descendants()
            .filter(|n| n.attribute("data-relationship-entry").is_some())
            .count(),
        38
    );
    assert_eq!(
        main.descendants()
            .filter(|n| n.attribute("data-account-panel").is_some())
            .count(),
        2
    );
}

#[test]
#[ignore = "large combined rendering snapshot; run explicitly with --release"]
fn multi_account_example_reproduces_inputs_svg_and_html_with_state_scoped_identity() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let inputs = generator::generate(
        &crate::generator::generate(&root.join("examples/terraform-large")).unwrap(),
    )
    .unwrap();
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"));
    let mut count = 0;
    for (state, input) in &inputs {
        let path = root.join(format!("examples/multi-account/{state}.json"));
        assert_eq!(
            *input,
            std::fs::read_to_string(&path)
                .unwrap()
                .replace("\r\n", "\n")
        );
        let json: serde_json::Value = serde_json::from_str(input).unwrap();
        count += json["resource_changes"].as_array().unwrap().len();
        command.args(["--state", &format!("{state}={}", path.display())]);
    }
    assert!((150..=250).contains(&count), "{count}");
    for mapping in generator::MAPPINGS {
        command.args(["--remote-state", mapping]);
    }
    command.args(["-o", "-"]);
    let start = std::time::Instant::now();
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = String::from_utf8(output.stdout).unwrap();
    eprintln!(
        "multi-account SVG generation: {:?}; bytes={}",
        start.elapsed(),
        svg.len()
    );
    assert_eq!(
        svg,
        std::fs::read_to_string(root.join("examples/multi-account/diagram.svg"))
            .unwrap()
            .replace("\r\n", "\n")
    );
    let document = roxmltree::Document::parse(&svg).unwrap();
    let mut ids = std::collections::BTreeSet::new();
    for node in document.descendants() {
        if let Some(id) = node.attribute("id") {
            assert!(ids.insert(id), "duplicate {id}");
        }
    }
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.attribute("id") == Some("relationship-legend"))
            .count(),
        1
    );
    assert!(
        document
            .descendants()
            .any(|n| n.attribute("data-legend-action") == Some("delete"))
    );
    let cross = document
        .descendants()
        .filter(|n| n.has_tag_name("path") && n.attribute("data-cross-state") == Some("true"))
        .count();
    assert!(cross >= 5, "{cross}");
    for state in generator::STATES {
        let section = document
            .descendants()
            .find(|n| n.attribute("data-state-id") == Some(state))
            .unwrap();
        assert!(
            section
                .descendants()
                .any(|n| n.attribute("data-account-panel") == Some("true"))
        );
        assert!(
            section
                .descendants()
                .any(|n| n.attribute("data-region") == Some("us-east-1"))
        );
        assert!(
            section
                .descendants()
                .any(|n| n.attribute("data-global") == Some("true"))
        );
        assert!(
            section
                .descendants()
                .any(|n| n.attribute("data-terraform-address")
                    == Some("module.network.aws_vpc.main"))
        );
    }
    let start = std::time::Instant::now();
    let output = command.args(["--format", "html"]).output().unwrap();
    assert!(output.status.success());
    let html = String::from_utf8(output.stdout).unwrap();
    eprintln!(
        "multi-account HTML generation: {:?}; bytes={}",
        start.elapsed(),
        html.len()
    );
    let embedded = &html[html.find("<svg ").unwrap()..html.rfind("</svg>").unwrap() + 6];
    let html_document = roxmltree::Document::parse(embedded).unwrap();
    let paths = |doc: &roxmltree::Document| {
        doc.descendants()
            .filter(|n| n.has_tag_name("path") && n.attribute("data-edge-kind").is_some())
            .map(|n| n.attribute("d").unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(paths(&document), paths(&html_document));
    assert_eq!(
        html,
        std::fs::read_to_string(root.join("examples/multi-account/diagram.html"))
            .unwrap()
            .replace("\r\n", "\n")
    );
}

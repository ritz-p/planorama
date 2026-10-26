use super::support;
use serde_json::{Value, json};

fn html(input: &[u8], args: &[&str]) -> String {
    let mut flags = vec!["--format", "html"];
    flags.extend_from_slice(args);
    let output = support::run_with_args(input, &flags);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn generation_data(document: &str) -> Value {
    serde_json::from_str(
        document
            .split("<script id=\"viewer-generation\" type=\"application/json\">")
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn generation_information_is_effective_allowlisted_and_shared() {
    let input = include_str!("../fixtures/viewer-plan.json");
    let default = html(input.as_bytes(), &[]);
    let metadata = generation_data(&default);
    assert_eq!(metadata["generator"]["version"], env!("CARGO_PKG_VERSION"));
    let revision = metadata["generator"]["revision"]
        .as_str()
        .unwrap_or("unavailable");
    let version = support::run_with_args(b"", &["--version"]);
    assert!(
        String::from_utf8(version.stdout)
            .unwrap()
            .contains(&format!("(revision: {revision})"))
    );
    assert!(metadata["setting_origins"].is_null());
    assert_eq!(metadata["selected_view"], false);
    assert_eq!(metadata["settings"]["relationship_legend"], "full");
    assert!(metadata["settings"]["focus_depth"].is_null());
    let shared = planorama::generate(
        input,
        &planorama::Options {
            format: planorama::OutputFormat::Html,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(generation_data(&shared.content), metadata);
    let filtered = html(
        input.as_bytes(),
        &[
            "--changed-only",
            "--address-format",
            "terraform",
            "--omit-relationship-legend",
        ],
    );
    let metadata = generation_data(&filtered);
    assert_eq!(metadata["selected_view"], true);
    assert_eq!(metadata["settings"]["changed_only"], true);
    assert_eq!(metadata["settings"]["address_format"], "terraform");
    assert_eq!(metadata["settings"]["relationship_legend"], "omitted");
    assert_eq!(
        metadata["settings"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "address_format",
            "changed_only",
            "focus",
            "focus_depth",
            "html_diagnostics",
            "html_semantic_diagnostics",
            "icon_mode",
            "modules",
            "relationship_legend",
            "remote_state_mappings",
            "spatial_types",
            "states"
        ]
    );
    assert!(!metadata.to_string().contains("resource_changes"));
    assert!(!metadata.to_string().contains("viewer-plan.json"));
}

fn diagram(html: &str) -> &str {
    let start = html.find("<svg ").unwrap();
    let end = html.rfind("</svg>").unwrap() + "</svg>\n".len();
    &html[start..end]
}

fn data(html: &str) -> Vec<Value> {
    let script = html
        .split("<script id=\"viewer-data\" type=\"application/json\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    serde_json::from_str(script).unwrap()
}

fn semantic_data(html: &str) -> Value {
    serde_json::from_str(
        html.split("<script id=\"viewer-semantic-diagnostics\" type=\"application/json\">")
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn semantic_diagnostics_are_opt_in_selected_safe_and_do_not_change_svg() {
    let address = "test.item[\"</script><script>alert(1)</script>\"]";
    let input = json!({"format_version":"1.2", "resource_changes":[
        {"address":address,"type":"test","change":{"actions":["create"],"after":{"secret":"NEVER_EMBED_VALUE"}}},
        {"address":"test.excluded","type":"test","change":{"actions":["no-op"]}}
    ]}).to_string();
    let baseline = html(input.as_bytes(), &["--changed-only"]);
    assert!(!baseline.contains("<script id=\"viewer-semantic-diagnostics\""));
    let enabled = html(
        input.as_bytes(),
        &["--changed-only", "--html-semantic-diagnostics"],
    );
    assert_eq!(diagram(&baseline), diagram(&enabled));
    assert!(!enabled.contains("test.excluded"));
    assert!(!enabled.contains("NEVER_EMBED_VALUE"));
    assert!(!enabled.contains("<script>alert"));
    assert_eq!(semantic_data(&enabled)["records"][0]["address"], address);
    assert_eq!(semantic_data(&enabled)["records"][0]["mode"], "managed");
    let rejected = support::run_with_args(input.as_bytes(), &["--html-semantic-diagnostics"]);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("requires --format html"));
}

#[test]
fn semantic_reference_reasons_keep_paths_and_deterministic_order() {
    let mut input = json!({"format_version":"1.2", "resource_changes":[
        {"address":"test.a","type":"test"}, {"address":"test.b","type":"test"}
    ],"configuration":{"root_module":{"resources":[
        {"address":"test.a","expressions":{"missing":{"references":["test.absent.id"]}}},
        {"address":"test.b","expressions":{"dynamic":{"references":["each.value"]}}}
    ]}}});
    let first = semantic_data(&html(
        input.to_string().as_bytes(),
        &["--html-semantic-diagnostics"],
    ));
    let records = first["records"].as_array().unwrap();
    assert!(
        records
            .iter()
            .any(|r| r["reason"] == "unresolved_reference" && r["attribute"] == "missing")
    );
    assert!(
        records
            .iter()
            .any(|r| r["category"] == "dynamic" && r["attribute"] == "dynamic")
    );
    input["resource_changes"].as_array_mut().unwrap().reverse();
    let second = semantic_data(&html(
        input.to_string().as_bytes(),
        &["--html-semantic-diagnostics"],
    ));
    assert_eq!(first, second);
}

#[test]
fn html_keeps_selected_svg_and_indexes_only_visible_resources() {
    let input = include_bytes!("../fixtures/lifecycle-plan.json");
    for args in [
        vec![],
        vec!["--changed-only"],
        vec!["--address-format", "terraform"],
    ] {
        let document = html(input, &args);
        assert!(document.starts_with("<!doctype html>"));
        let svg = support::run_with_args(input, &args);
        assert_eq!(diagram(&document).as_bytes(), svg.stdout);
        let parsed = roxmltree::Document::parse(diagram(&document)).unwrap();
        for resource in data(&document) {
            let id = resource["id"].as_str().unwrap();
            assert!(
                parsed.descendants().any(|n| n.attribute("id") == Some(id)),
                "{id}"
            );
        }
    }
    let input = json!({"format_version":"1.2", "resource_changes":[
        {"address":"test.visible", "type":"test", "change":{"actions":["create"]}},
        {"address":"test.hidden", "type":"test", "change":{"actions":["no-op"]}}
    ]});
    let document = html(input.to_string().as_bytes(), &["--changed-only"]);
    assert!(!document.contains("test.hidden"));
    assert_eq!(data(&document).len(), 1);
}

#[test]
fn metadata_round_trips_hostile_long_indexed_addresses_without_values_or_markup() {
    let address = format!(
        "module.{}.test.item[\"</script><script>alert('&')</script>\\\"\u{2028}\"]",
        "long_".repeat(40)
    );
    let input = json!({"format_version":"1.2", "resource_changes":[
        {"address":address, "type":"test", "change":{"actions":["create"],"after":{"secret":"ATTRIBUTE_SECRET"}}},
        {"address":"data.test.item", "type":"test", "mode":"data"},
        {"address":"test.item", "type":"test"}
    ]});
    let document = html(input.to_string().as_bytes(), &[]);
    assert!(!document.contains("ATTRIBUTE_SECRET"));
    assert!(!document.contains("<script>alert"));
    let focused = html(
        input.to_string().as_bytes(),
        &["--focus", &address, "--focus-depth", "0"],
    );
    assert_eq!(generation_data(&focused)["settings"]["focus"], address);
    assert_eq!(generation_data(&focused)["settings"]["focus_depth"], 0);
    assert!(!focused.contains("<script>alert"));
    assert!(!focused.contains("ATTRIBUTE_SECRET"));
    assert_eq!(document.matches("</script>").count(), 4);
    let diagnostics = html(input.to_string().as_bytes(), &["--html-diagnostics"]);
    assert!(!diagnostics.contains("ATTRIBUTE_SECRET"));
    assert!(!diagnostics.contains("<script>alert"));
    assert_eq!(diagnostics.matches("</script>").count(), 5);
    let resources = data(&document);
    assert!(resources.iter().any(|r| r["address"] == address));
    assert!(resources.iter().any(|r| r["mode"] == "data"));
    assert!(resources.iter().any(|r| r["mode"] == "managed"));
    let svg = roxmltree::Document::parse(diagram(&document)).unwrap();
    assert_eq!(resources.len(), 3);
    for r in resources {
        assert!(
            svg.descendants()
                .any(|n| n.attribute("id") == r["id"].as_str())
        );
    }
}

#[test]
fn named_state_html_preserves_namespaces_and_cross_state_paths() {
    let args = [
        "--state",
        "network=tests/fixtures/multi/network.json",
        "--state",
        "application=tests/fixtures/multi/application.json",
        "-o",
        "-",
    ];
    let svg = super::cli().args(args).output().unwrap();
    let output = super::cli()
        .args(args)
        .args(["--format", "html"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let document = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        generation_data(&document)["settings"]["states"],
        json!(["application", "network"])
    );
    assert_eq!(diagram(&document).as_bytes(), svg.stdout);
    let resources = data(&document);
    assert!(resources.iter().any(|r| r["state"] == "network"));
    assert!(resources.iter().any(|r| r["state"] == "application"));
    let svg = roxmltree::Document::parse(diagram(&document)).unwrap();
    for r in resources {
        assert!(
            svg.descendants()
                .any(|n| n.attribute("id") == r["id"].as_str())
        );
    }
}

#[test]
fn format_is_explicit_and_validated() {
    let input = include_bytes!("../../examples/plan.json");
    assert_eq!(
        support::run(input).stdout,
        support::run_with_args(input, &["--format", "svg"]).stdout
    );
    for args in [vec!["--format"], vec!["--format", "pdf"]] {
        let output = support::run_with_args(input, &args);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("svg or html"));
    }
    let help = super::cli().arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&help.stdout).contains("--format"));
}

#[test]
fn html_omits_route_constants_without_changing_paths_or_identities() {
    let input = include_bytes!("../fixtures/routes-plan.json");
    let document = html(input, &[]);
    let standalone = String::from_utf8(support::run(input).stdout).unwrap();
    let original = roxmltree::Document::parse(&standalone).unwrap();
    let viewer = roxmltree::Document::parse(diagram(&document)).unwrap();
    let metadata = original
        .descendants()
        .find(|n| n.attribute("id") == Some("route-presentations"))
        .unwrap();
    let routes: Value = serde_json::from_str(metadata.text().unwrap()).unwrap();
    let mut count = 0;
    for route in routes.as_array().unwrap() {
        for entry in route["entries"].as_array().unwrap() {
            if let Some(value) = entry["destination"].as_str() {
                count += 1;
                assert!(!document.contains(value), "{value}");
            }
        }
    }
    assert!(count > 0);
    let paths = |doc: &roxmltree::Document<'_>| {
        doc.descendants()
            .filter(|n| n.has_tag_name("path") && n.attribute("data-edge-kind").is_some())
            .map(|n| {
                [
                    "d",
                    "data-source",
                    "data-target",
                    "data-directionality",
                    "stroke",
                    "marker-end",
                ]
                .map(|a| n.attribute(a).map(str::to_owned))
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(paths(&original), paths(&viewer));
    assert!(document.contains("Destination: omitted"));
}

#[test]
fn representative_html_matches_the_renderer() {
    let output = html(include_bytes!("../fixtures/viewer-plan.json"), &[]);
    let normalize = |document: &str| {
        let revision = &generation_data(document)["generator"]["revision"];
        document.replacen(&format!("\"revision\":{revision}"), "\"revision\":null", 1)
    };
    assert_eq!(
        normalize(&output),
        normalize(include_str!("../../examples/viewer.html"))
    );
    let svg = roxmltree::Document::parse(diagram(&output)).unwrap();
    let entries: Vec<_> = svg
        .descendants()
        .filter(|node| node.attribute("data-legend-entry").is_some())
        .collect();
    assert_eq!(entries.len(), 5);
    let columns: std::collections::BTreeSet<_> = entries
        .iter()
        .map(|node| {
            node.attribute("transform")
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
        })
        .collect();
    assert!(columns.len() >= 2);
}

#[test]
fn html_diagnostics_are_explicit_bounded_and_do_not_change_svg() {
    let input = include_bytes!("../fixtures/aws-relationships-plan.json");
    let plain = html(input, &[]);
    for flags in [
        vec!["--html-diagnostics"],
        vec!["--html-diagnostics", "--changed-only"],
    ] {
        let output = html(input, &flags);
        let metadata = output
            .split("<script id=\"viewer-diagnostics\" type=\"application/json\">")
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap();
        let data: Value = serde_json::from_str(metadata).unwrap();
        assert_eq!(data["schema_version"], 1);
        assert_eq!(data["available"], true);
        assert!(data["hotspots"].as_array().unwrap().len() <= 200);
        let baseline = html(input, &flags[1..]);
        assert_eq!(diagram(&baseline), diagram(&output));
    }
    assert!(!plain.contains("id=\"viewer-diagnostics\""));
    assert!(
        !support::run_with_args(input, &["--html-diagnostics"])
            .status
            .success()
    );
}

#[test]
fn relationship_navigation_is_scoped_to_each_state_and_only_existing_targets() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"))
        .args([
            "--format",
            "html",
            "-o",
            "-",
            "--state",
            "left=tests/fixtures/association-plan.json",
            "--state",
            "right=tests/fixtures/association-plan.json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8(output.stdout).unwrap();
    let document = roxmltree::Document::parse(diagram(&output)).unwrap();
    let ids: Vec<_> = document
        .descendants()
        .filter_map(|node| node.attribute("id"))
        .collect();
    assert_eq!(
        ids.len(),
        ids.iter().collect::<std::collections::BTreeSet<_>>().len()
    );
    let entries: Vec<_> = document
        .descendants()
        .filter(|node| node.attribute("data-relationship-entry") == Some("1"))
        .collect();
    assert_eq!(entries.len(), 2);
    assert_ne!(entries[0].attribute("id"), entries[1].attribute("id"));
    for state in document
        .descendants()
        .filter(|node| node.attribute("data-state-id").is_some())
    {
        let local_ids: Vec<_> = state
            .descendants()
            .filter_map(|node| node.attribute("id"))
            .collect();
        for node in state.descendants() {
            if let Some(ids) = node.attribute("data-relationship-ids") {
                for id in ids.split_whitespace() {
                    assert!(local_ids.contains(&id));
                }
            }
            if node.attribute("data-relationship-select").is_some()
                || node.attribute("data-relationship-endpoint").is_some()
            {
                assert!(
                    local_ids.contains(&node.attribute("href").unwrap().strip_prefix('#').unwrap())
                );
            }
        }
    }
    let filtered = html(
        include_bytes!("../fixtures/association-plan.json"),
        &["--focus", "aws_subnet.private", "--focus-depth", "0"],
    );
    let document = roxmltree::Document::parse(diagram(&filtered)).unwrap();
    assert!(
        !document
            .descendants()
            .any(|node| node.attribute("data-relationship-select").is_some())
    );
}

use super::cli;
use serde_json::Value;

#[test]
fn layout_reports_preserve_svg_and_match_text_and_named_state_canvas() {
    let root = std::env::temp_dir().join(format!("planorama-layout-report-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let report = root.join("report.json");
    for named in [false, true] {
        let mut command = cli();
        if named {
            command.args(["--state", "example=examples/plan.json"]);
        } else {
            command.arg("examples/plan.json");
        }
        let output = command
            .args([
                "-o",
                "-",
                "--diagnostics",
                "--layout-diagnostics",
                "--layout-diagnostics-json",
            ])
            .arg(&report)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let svg = String::from_utf8(output.stdout).unwrap();
        let document = roxmltree::Document::parse(&svg).unwrap();
        let json: Value = serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(
            json["canvas"]["width"].as_u64().unwrap().to_string(),
            document.root_element().attribute("width").unwrap()
        );
        assert_eq!(
            json["canvas"]["height"].as_u64().unwrap().to_string(),
            document.root_element().attribute("height").unwrap()
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("layout diagnostics"));
        assert_eq!(
            json["summary"]["routed_paths"].as_u64().unwrap() as usize,
            document
                .descendants()
                .filter(|n| n.has_tag_name("path") && n.attribute("data-edge-kind").is_some())
                .count()
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn diagnostics_destinations_cannot_overwrite_inputs_or_diagrams() {
    for args in [
        vec!["--layout-diagnostics-json", "examples/plan.json"],
        vec![
            "-o",
            "diagram.svg",
            "--layout-diagnostics-json",
            "./diagram.svg",
        ],
        vec!["--layout-diagnostics-json", "-"],
    ] {
        assert!(
            !cli()
                .arg("examples/plan.json")
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}

#[cfg(unix)]
#[test]
fn dangling_output_symlinks_are_rejected_before_either_file_is_written() {
    let root = std::env::temp_dir().join(format!("planorama-layout-alias-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for report_is_link in [false, true] {
        let diagram = root.join(format!("diagram-{report_is_link}.svg"));
        let report = root.join(format!("report-{report_is_link}.json"));
        let (link, target) = if report_is_link {
            (&report, &diagram)
        } else {
            (&diagram, &report)
        };
        std::os::unix::fs::symlink(target.file_name().unwrap(), link).unwrap();
        let output = cli()
            .arg("examples/plan.json")
            .arg("-o")
            .arg(&diagram)
            .arg("--layout-diagnostics-json")
            .arg(&report)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("output symlink"));
        assert!(!target.exists());
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn case_only_output_names_follow_filesystem_identity() {
    let root = std::env::temp_dir().join(format!("planorama-layout-case-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let diagram = root.join("Diagram.svg");
    let report = root.join("diagram.svg");
    std::fs::write(&report, b"probe").unwrap();
    let aliases = diagram.exists();
    std::fs::remove_file(&report).unwrap();
    for named in [false, true] {
        let mut command = cli();
        if named {
            command.args(["--state", "example=examples/plan.json"]);
        } else {
            command.arg("examples/plan.json");
        }
        let output = command
            .arg("-o")
            .arg(&diagram)
            .arg("--layout-diagnostics-json")
            .arg(&report)
            .output()
            .unwrap();
        if aliases {
            assert!(!output.status.success());
            assert!(String::from_utf8_lossy(&output.stderr).contains("different files"));
            assert!(std::fs::read(&report).unwrap().is_empty());
        } else {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            serde_json::from_slice::<Value>(&std::fs::read(&report).unwrap()).unwrap();
            let svg = std::fs::read_to_string(&diagram).unwrap();
            roxmltree::Document::parse(&svg).unwrap();
            std::fs::remove_file(&diagram).unwrap();
        }
        std::fs::remove_file(&report).unwrap();
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cross_state_report_uses_final_gutter_coordinates() {
    let report = std::env::temp_dir().join(format!(
        "planorama-cross-layout-{}.json",
        std::process::id()
    ));
    let output = cli()
        .args([
            "--state",
            "producer=tests/fixtures/multi/producer.json",
            "--state",
            "consumer=tests/fixtures/multi/consumer.json",
            "--remote-state",
            "consumer:data.terraform_remote_state.network=producer",
            "-o",
            "-",
            "--layout-diagnostics-json",
        ])
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    let svg = String::from_utf8(output.stdout).unwrap();
    let document = roxmltree::Document::parse(&svg).unwrap();
    let links: Vec<_> = document
        .descendants()
        .filter(|n| n.attribute("data-cross-state") == Some("true"))
        .collect();
    let reported: Vec<_> = json["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["identity"]["cross_state"] == true)
        .collect();
    assert!(!reported.is_empty());
    assert_eq!(links.len(), reported.len());
    for edge in reported {
        let mut expected = String::new();
        for (i, p) in edge["polyline"].as_array().unwrap().iter().enumerate() {
            expected.push_str(&format!(
                "{} {} {} ",
                if i == 0 { "M" } else { "L" },
                p[0],
                p[1]
            ));
        }
        assert!(
            links
                .iter()
                .any(|n| n.attribute("d") == Some(expected.trim()))
        );
    }
    assert!(!json.to_string().contains("TOP_SECRET"));
    std::fs::remove_file(report).unwrap();
}

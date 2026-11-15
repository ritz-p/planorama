use super::support;
use serde_json::{Value, json};

fn records(html: &str) -> Vec<Value> {
    let data = html
        .split("<script id=\"viewer-semantic-diagnostics\" type=\"application/json\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    let value: Value = serde_json::from_str(data).unwrap();
    value["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["reason"].as_str().unwrap().starts_with("cloudflare_"))
        .cloned()
        .collect()
}

fn svg(html: &str) -> &str {
    &html[html.find("<svg ").unwrap()..html.rfind("</svg>").unwrap() + 6]
}

#[test]
fn cloudflare_diagnostics_share_codes_and_messages_without_changing_selection_or_geometry() {
    let input = include_bytes!("../fixtures/cloudflare-captured/create.json");
    let plain = support::run_with_args(input, &["--format", "html"]);
    let diagnosed = support::run_with_args(
        input,
        &[
            "--format",
            "html",
            "--diagnostics",
            "--html-semantic-diagnostics",
        ],
    );
    assert!(plain.status.success() && diagnosed.status.success());
    let plain = String::from_utf8(plain.stdout).unwrap();
    let html = String::from_utf8(diagnosed.stdout).unwrap();
    assert_eq!(svg(&plain), svg(&html));
    let findings = records(&html);
    assert_eq!(findings.len(), 3);
    assert!(
        findings
            .iter()
            .all(|r| r["reason"] == "cloudflare_unknown_value")
    );
    let stderr = String::from_utf8(diagnosed.stderr).unwrap();
    for finding in findings {
        assert!(stderr.contains(finding["message"].as_str().unwrap()));
        assert!(stderr.contains(finding["address"].as_str().unwrap()));
    }
    let document = roxmltree::Document::parse(svg(&html)).unwrap();
    let references = document
        .descendants()
        .find(|n| n.attribute("id") == Some("resource-references"))
        .unwrap();
    let references: Value = serde_json::from_str(references.text().unwrap()).unwrap();
    assert_eq!(
        references
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["issues"] == json!(["cloudflare_unknown_value"]))
            .count(),
        3
    );
    let selected = support::run_with_args(
        input,
        &[
            "--format",
            "html",
            "--html-semantic-diagnostics",
            "--focus",
            "cloudflare_dns_record.www",
            "--focus-depth",
            "0",
        ],
    );
    assert!(selected.status.success());
    let selected = records(&String::from_utf8(selected.stdout).unwrap());
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0]["address"], "cloudflare_dns_record.www");
    assert_eq!(selected[0]["attribute"], "zone_id");
}

#[test]
fn cloudflare_named_state_diagnostics_and_sensitive_values_keep_their_boundaries() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"))
        .args([
            "--state",
            "left=tests/fixtures/cloudflare-captured/create.json",
            "--state",
            "right=tests/fixtures/cloudflare-captured/create.json",
            "--format",
            "html",
            "--html-semantic-diagnostics",
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let findings = records(&String::from_utf8(output.stdout).unwrap());
    for state in ["left", "right"] {
        assert_eq!(findings.iter().filter(|r| r["state"] == state).count(), 3);
    }
    let mut plan: Value =
        serde_json::from_str(include_str!("../fixtures/cloudflare-dns-plan.json")).unwrap();
    let record = plan["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == "cloudflare_dns_record.www")
        .unwrap();
    record["change"]["after"]["zone_id"] = json!("PRIVATE_DIAGNOSTIC_VALUE");
    record["change"]["after_sensitive"] = json!({"zone_id":true});
    let input = plan.to_string();
    for format in ["svg", "html"] {
        let mut flags = vec!["--format", format, "--diagnostics"];
        if format == "html" {
            flags.push("--html-semantic-diagnostics");
        }
        let output = support::run_with_args(input.as_bytes(), &flags);
        assert!(output.status.success());
        let content = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!content.contains("PRIVATE_"));
        assert!(!stderr.contains("PRIVATE_"));
        assert!(content.contains("cloudflare_sensitive_value"));
        assert!(stderr.contains("Cloudflare reference requires a sensitive value"));
        if format == "html" {
            assert!(
                records(&content)
                    .iter()
                    .any(|r| r["reason"] == "cloudflare_sensitive_value")
            );
        }
    }
}

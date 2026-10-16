use super::*;

fn graph() -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"terraform_data.one","type":"terraform_data"}]}"#).unwrap())
}

#[test]
fn opt_in_and_record_byte_limits_are_explicit_without_partial_identities() {
    let mut graph = graph();
    assert!(bridge(std::iter::once(("", &graph.coverage))).is_none());
    graph.0.coverage.embed_findings = true;
    let mut finding = graph.coverage.findings[0].clone();
    finding.message = "test";
    graph.0.coverage.findings = (0..1100)
        .map(|i| {
            let mut f = finding.clone();
            f.attribute = Some(format!("field_{i:04}"));
            f
        })
        .collect();
    let value = bridge(std::iter::once(("one", &graph.coverage))).unwrap();
    assert_eq!(value["total"], 1100);
    assert_eq!(value["records"].as_array().unwrap().len(), 1000);
    assert_eq!(value["omitted"], 100);
    assert_eq!(value["records_complete"], false);
    graph.0.coverage.findings[0].attribute = Some("</script>".repeat(40000));
    graph.0.coverage.findings.truncate(2);
    let value = bridge(std::iter::once(("one", &graph.coverage))).unwrap();
    assert_eq!(value["omitted"], 1);
    assert_eq!(value["records"][0]["attribute"], "field_0001");
    assert!(crate::html::safe_json(&value).len() <= 256 * 1024);
}

#[test]
fn state_and_deposed_keys_survive_serialization_and_zero_is_available() {
    let mut graph = graph();
    graph.0.coverage.embed_findings = true;
    let mut old = graph.coverage.resources.keys().next().unwrap().clone();
    old.deposed_key = Some("old".into());
    let resource = graph.coverage.resources.values().next().unwrap().clone();
    graph.0.coverage.resources.insert(old.clone(), resource);
    let mut finding = graph.coverage.findings[0].clone();
    finding.source = old;
    graph.0.coverage.findings.push(finding);
    let value =
        bridge([("alpha", &graph.coverage), ("beta", &graph.coverage)].into_iter()).unwrap();
    let records = value["records"].as_array().unwrap();
    assert_eq!(records.len(), 4);
    assert_eq!(records[0]["state"], "alpha");
    assert_eq!(records[1]["deposed"], "old");
    assert_eq!(records[2]["state"], "beta");
    graph.0.coverage.findings.clear();
    let value = bridge(std::iter::once(("", &graph.coverage))).unwrap();
    assert_eq!(value["total"], 0);
    assert_eq!(value["available"], true);
    assert_eq!(value["records_complete"], true);
    assert_eq!(value["input_complete"], Value::Null);
}

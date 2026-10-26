use super::*;

#[test]
fn changed_path_bridge_bounds_the_aggregate_and_discloses_omissions() {
    let records: Vec<_> = (0..100)
        .map(|i| {
            json!({"id": i, "changed_paths": {
                "available": true, "paths": ["<".repeat(1000)], "truncated": false
            }})
        })
        .collect();
    let output = document(String::new(), records, vec![], None, None);
    let text = output
        .split("<script id=\"viewer-data\" type=\"application/json\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    assert!(!text.contains('<'));
    let records: Vec<Value> = serde_json::from_str(text).unwrap();
    let retained: usize = records
        .iter()
        .filter(|r| r["changed_paths"]["available"] == true)
        .map(|r| safe_json(&r["changed_paths"]).len())
        .sum();
    assert!(retained <= 256 * 1024);
    assert!(records.iter().any(
        |r| r["changed_paths"]["truncated"] == true && r["changed_paths"]["available"] == false
    ));
}

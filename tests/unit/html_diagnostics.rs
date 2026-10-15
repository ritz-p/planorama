use super::*;

#[test]
fn distinct_findings_keep_global_coordinates_and_identity_when_grouped() {
    let report = json!({"schema_version":1,"scope":"whole_diagram", "limits":{"findings_complete":true},
    "edges":[{"id":"a","identity":{"source":{"state":"two","id":"terraform:a"}}}],
    "findings":[
        {"category":"crossing","edge":"a","peer":"b","location":{"point":[100,900]}},
        {"category":"crossing","edge":"a","peer":"b","location":{"point":[101,901]}},
        {"category":"crossing","edge":"a","peer":"c","location":{"point":[100,900]}},
        {"category":"independent_overlap","edge":"a","location":{"horizontal":true,"coordinates":[900,900],"interval":[100,200],"visible_gap":-2}},
        {"category":"parallel_proximity","edge":"a","location":{"horizontal":false,"coordinates":[100,110],"interval":[900,1000],"visible_gap":8}},
        {"category":"visible_border_clearance","edge":"a","horizontal":true,"coordinate":900,"interval":[100,200],"below_minimum":true}
    ]});
    let result = bridge(&report);
    let groups = result["hotspots"].as_array().unwrap();
    assert_eq!(groups.len(), 5);
    assert_eq!(groups[0]["count"], 2);
    assert_eq!(groups[0]["bounds"], json!([100.0, 900.0, 1.0, 1.0]));
    assert_eq!(groups[0]["paths"][0]["identity"]["source"]["state"], "two");
    assert_eq!(groups[2]["severity"], "violation");
    assert_eq!(groups[3]["severity"], "advisory");
    assert_eq!(groups[3]["bounds"], json!([100.0, 900.0, 10.0, 100.0]));
    assert_eq!(groups[4]["bounds"], json!([100.0, 900.0, 100.0, 0.0]));
}

#[test]
fn bounded_bridge_counts_omissions_and_preserves_incomplete_analysis() {
    let findings: Vec<_> = (0..250).map(|i| json!({"category":"crossing","edge":format!("edge-{i}"),"location":{"point":[1,2]}})).collect();
    let result = bridge(
        &json!({"schema_version":1,"scope":"whole_diagram","findings":findings,"limits":{"findings_complete":false}}),
    );
    assert_eq!(result["hotspots"].as_array().unwrap().len(), 200);
    assert_eq!(result["omitted"], 50);
    assert_eq!(result["total"], 250);
    assert_eq!(result["complete"], false);
    assert_eq!(bridge(&Value::Null)["available"], false);
}

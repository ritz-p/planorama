use super::*;

#[test]
fn embedded_values_are_not_interpreted_as_template_slots() {
    let address = "/* VIEWER_JS */<!-- DIAGRAM -->&\u{2028}\u{2029}東京";
    let resources = vec![json!({"address": address, "changed_paths": []})];
    let svg = "<svg><!-- VIEWER_DATA --><!-- DIAGRAM --></svg>";
    let html = document(svg.into(), resources.clone(), vec![], None, None);
    assert!(html.contains(svg));
    let data = html
        .split("<script id=\"viewer-data\" type=\"application/json\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(data).unwrap(),
        Value::Array(resources)
    );
    assert!(!data.contains(['<', '>', '&', '\u{2028}', '\u{2029}']));
    assert_eq!(html.matches("<script>/* VIEWER_JS */</script>").count(), 0);
    assert!(!viewer_script().contains("/* VIEWER_MODULES */"));
    assert_eq!(
        html,
        document(
            svg.into(),
            vec![json!({"address": address, "changed_paths": []})],
            vec![],
            None,
            None
        )
    );
}

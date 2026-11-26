use super::*;
use serde_json::json;

#[test]
fn scalar_nested_collections_and_unknown_are_distinguished() {
    let result = collect(
        &json!({"before":{"a":1,"removed":true,"nested":{"x":1},"list":[1,2],"unknown":4},
        "after":{"a":2,"added":null,"nested":{"x":2},"list":[1],"unknown":null},
        "after_unknown":{"unknown":true},"replace_paths":[["nested","x"]]}),
    );
    assert!(result.available);
    assert_eq!(
        result.paths,
        [
            "$[\"a\"]",
            "$[\"added\"]",
            "$[\"list\"][1]",
            "$[\"nested\"][\"x\"]",
            "$[\"removed\"]"
        ]
    );
    assert_eq!(result.unknown_paths, ["$[\"unknown\"]"]);
    assert_eq!(result.replacement_paths, ["$[\"nested\"][\"x\"]"]);
    assert!(result.replacement_available);
}

#[test]
fn unavailable_sensitive_and_empty_are_not_conflated() {
    assert!(!collect(&json!({"after":{}})).available);
    let empty = collect(&json!({"before":{},"after":{}}));
    assert!(empty.available && empty.paths.is_empty() && !empty.sensitive_omitted);
    for mask in ["before_sensitive", "after_sensitive"] {
        let mut change = json!({
            "before": {"PRIVATE_MAP": {"SECRET_KEY": 1, "label": "old"}, "safe": 1},
            "after": {"PRIVATE_MAP": {"SECRET_KEY": 2, "label": "new"}, "safe": 2},
            "replace_paths": [["PRIVATE_MAP", "SECRET_KEY"], ["safe"]]
        });
        change[mask] = json!({"PRIVATE_MAP": {"SECRET_KEY": true}});
        let result = collect(&change);
        let serialized = serde_json::to_string(&result).unwrap();
        assert!(!serialized.contains("PRIVATE_MAP"), "{mask}: {serialized}");
        assert!(!serialized.contains("SECRET_KEY"), "{mask}: {serialized}");
        assert_eq!(result.paths, [r#"$["safe"]"#]);
        assert_eq!(result.replacement_paths, [r#"$["safe"]"#]);
        assert!(result.sensitive_omitted);
    }
    assert_eq!(
        collect(&json!({"before":null,"after":{"new":1}})).paths,
        ["$[\"new\"]"]
    );
    assert_eq!(
        collect(&json!({"before":{"old":1},"after":null})).paths,
        ["$[\"old\"]"]
    );
}

#[test]
fn paths_are_bounded_and_escaped_without_values() {
    let after: serde_json::Map<_, _> = (0..1000)
        .map(|i| (format!("key{i}"), json!("VALUE_SECRET")))
        .collect();
    let result = collect(&json!({"before":{},"after":after}));
    assert!(result.truncated);
    assert!(result.paths.len() <= PATH_LIMIT);
    assert!(serde_json::to_string(&result).unwrap().len() <= BYTE_LIMIT);
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("VALUE_SECRET")
    );
    let hostile = collect(&json!({"before":{},"after":{"</script>\"\n":"SECRET"}}));
    assert_eq!(hostile.paths, ["$[\"</script>\\\"\\n\"]"]);
    let large = collect(&json!({"before":{},"after":{"x".repeat(9000):1}}));
    assert!(large.truncated && large.paths.is_empty());
}

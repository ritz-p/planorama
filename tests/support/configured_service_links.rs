use crate::model::TerraformPlan;
use serde_json::{Value, json};

pub(crate) fn verified(graph: &crate::model::Graph, address: &str, path: &str, label: Option<&str>) -> bool {
    let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
    node.references.iter().any(|r| r.attribute == path && r.complete && !r.dependency
        && !r.targets.is_empty() && Some(r.label) == label)
}

pub(crate) fn check(
    json: &str,
    parse: fn(&str) -> Result<TerraformPlan, String>,
    label: fn(&str, &str) -> Option<&'static str>,
    paths: &[(&str, &str)],
    endpoint: (&str, &str),
) {
    let fixture: Value = serde_json::from_str(json).unwrap();
    let verified = |value: &Value, address: &str, path: &str| {
        let graph = crate::semantic::transform(&parse(&value.to_string()).unwrap());
        verified(&graph, address, path, label(address.split('.').next().unwrap(), path))
    };
    for &(address, path) in paths {
        assert!(verified(&fixture, address, path), "{address}/{path}");
        for mask in ["after_sensitive", "after_unknown"] {
            let mut changed = fixture.clone();
            change(&mut changed, address)["change"][mask] = json!(true);
            assert!(!verified(&changed, address, path), "{address}/{path}: {mask}");
        }
    }
    let (consumer, path) = paths[0];
    let (source, field) = endpoint;
    for (key, v) in [("actions", json!(["delete"])), ("after_sensitive", json!(true))] {
        let mut changed = fixture.clone();
        change(&mut changed, source)["change"][key] = v;
        assert!(!verified(&changed, consumer, path), "{source} {key}");
    }
    let mut wrong_mode = fixture.clone();
    change(&mut wrong_mode, source)["mode"] = json!("data");
    assert!(!verified(&wrong_mode, consumer, path));
    let mut deposed = fixture.clone();
    change(&mut deposed, source)["deposed"] = json!("old");
    assert!(!verified(&deposed, consumer, path));
    let mut mismatched = fixture.clone();
    change(&mut mismatched, source)["change"]["after"][field] = json!("different");
    assert!(!verified(&mismatched, consumer, path));
    let mut replacing = fixture;
    let row = change(&mut replacing, source);
    row["change"]["actions"] = json!(["delete","create"]);
    row["change"]["after_unknown"] = json!({field: true});
    assert!(!verified(&replacing, consumer, path));
}

fn change<'a>(fixture: &'a mut Value, address: &str) -> &'a mut Value {
    fixture["resource_changes"].as_array_mut().unwrap().iter_mut()
        .find(|row| row["address"] == address).unwrap()
}

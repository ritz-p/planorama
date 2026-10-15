use serde_json::{Value, json};
use std::collections::BTreeMap;

const LIMIT: usize = 200;

#[cfg(test)]
#[path = "../../tests/unit/html_diagnostics.rs"]
mod tests;

fn region(finding: &Value) -> Option<[f64; 4]> {
    let location = finding.get("location").unwrap_or(finding);
    if let Some(point) = location["point"].as_array() {
        return Some([point.first()?.as_f64()?, point.get(1)?.as_f64()?, 0.0, 0.0]);
    }
    let low = location["interval"][0].as_f64()?;
    let high = location["interval"][1].as_f64()?;
    let a = location["coordinates"][0]
        .as_f64()
        .or_else(|| location["coordinate"].as_f64())?;
    let b = location["coordinates"][1].as_f64().unwrap_or(a);
    Some(if location["horizontal"].as_bool()? {
        [low, a.min(b), high - low, (a - b).abs()]
    } else {
        [a.min(b), low, (a - b).abs(), high - low]
    })
}

pub(super) fn bridge(report: &Value) -> Value {
    if report["schema_version"] != 1 || report["scope"] != "whole_diagram" {
        return json!({"schema_version": 1, "available": false});
    }
    let edges: BTreeMap<_, _> = report["edges"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|edge| Some((edge["id"].as_str()?, &edge["identity"])))
        .collect();
    let mut groups: Vec<Value> = Vec::new();
    let mut keys = BTreeMap::new();
    let mut omitted = 0;
    let mut total = 0;
    for finding in report["findings"].as_array().into_iter().flatten() {
        total += 1;
        let Some(bounds) = region(finding) else {
            omitted += 1;
            continue;
        };
        let hard = finding["category"] == "independent_overlap"
            || (finding["category"] == "visible_border_clearance"
                && finding["below_minimum"] == true);
        let key = json!([
            finding["category"],
            finding["edge"],
            finding["peer"],
            hard,
            (bounds[0] / 32.0).floor(),
            (bounds[1] / 32.0).floor()
        ])
        .to_string();
        let gap = finding["location"]["visible_gap"].as_f64();
        if let Some(&index) = keys.get(&key) {
            let group: &mut Value = &mut groups[index];
            group["count"] = json!(group["count"].as_u64().unwrap_or(0) + 1);
            let old: [f64; 4] = std::array::from_fn(|i| group["bounds"][i].as_f64().unwrap_or(0.0));
            let x = old[0].min(bounds[0]);
            let y = old[1].min(bounds[1]);
            group["bounds"] = json!([
                x,
                y,
                (old[0] + old[2]).max(bounds[0] + bounds[2]) - x,
                (old[1] + old[3]).max(bounds[1] + bounds[3]) - y
            ]);
            if let Some(gap) = gap {
                group["minimum_visible_gap"] =
                    json!(gap.min(group["minimum_visible_gap"].as_f64().unwrap_or(gap)));
            }
            continue;
        }
        if groups.len() == LIMIT {
            omitted += 1;
            continue;
        }
        keys.insert(key, groups.len());
        let paths: Vec<_> = ["edge", "peer"]
            .into_iter()
            .filter_map(|field| {
                let id = finding[field].as_str()?;
                Some(json!({"id": id, "identity": edges.get(id),
                "segment": finding[if field == "edge" { "segment" } else { "peer_segment" }]}))
            })
            .collect();
        groups.push(json!({"kind": finding["category"], "severity": if hard { "violation" } else { "advisory" },
            "bounds": bounds, "count": 1, "paths": paths, "minimum_visible_gap": gap}));
    }
    json!({"schema_version": 1, "available": true, "scope": "whole_diagram", "limit": LIMIT,
        "total": total, "omitted": omitted, "complete": report["limits"]["findings_complete"] == true,
        "hotspots": groups})
}

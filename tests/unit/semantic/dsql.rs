use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/dsql-plan.json");
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap())
}
fn verified(v: &Value, t: usize, p: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == p
                && r.complete
                && Some(r.label)
                    == super::label(v["resource_changes"][t]["type"].as_str().unwrap(), p)
        })
}
#[test]
fn explicit_peers_require_complete_region_and_account_identity() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (1, "kms_encryption_key"),
        (1, "multi_region_properties.clusters"),
        (3, "identifier"),
        (3, "clusters"),
        (4, "identifier"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    v["configuration"]["provider_config"]["aws.west"]["expressions"]["assume_role"][0]["role_arn"]
        ["constant_value"] = json!("arn:aws:iam::999999999999:role/Test");
    assert!(!verified(&v, 3, "clusters"));
    assert!(!verified(&v, 1, "multi_region_properties.clusters"));
    v["resource_changes"][1]["change"]["after"]["kms_encryption_key"] = json!("AWS_OWNED_KMS_KEY");
    assert!(!verified(&v, 1, "kms_encryption_key"));
}

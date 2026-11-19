use super::*;
use crate::model::{Action, Directionality, EdgeChange, TerraformEntityId};

#[test]
fn provenance_metadata_round_trips_evidence_identity_and_deposed_keys() {
    let current = TerraformEntityId {
        address: "cloudflare_worker.api[\"x&<y>\"]".into(),
        deposed_key: None,
    };
    let consumer = TerraformEntityId {
        address: "cloudflare_workers_route.api".into(),
        deposed_key: None,
    };
    let old = TerraformEntityId {
        address: current.address.clone(),
        deposed_key: Some("old-instance".into()),
    };
    let proofs = [
        (
            RelationshipProvenance::Reference {
                from: old.clone(),
                to: consumer.clone(),
            },
            "reference",
            None,
        ),
        (
            RelationshipProvenance::ConfiguredValue {
                from: current.clone(),
                to: consumer.clone(),
                attribute: "zone_id",
            },
            "configured-value",
            Some("typed-configured-value"),
        ),
        (
            RelationshipProvenance::Resource {
                source: old,
                change: EdgeChange {
                    directionality: Directionality::Directed,
                    address: current.address.clone(),
                    previous_address: None,
                    metadata: Default::default(),
                    action: Action::Delete,
                },
                deployment: None,
            },
            "resource",
            None,
        ),
    ];
    for (proof, kind, confidence) in proofs {
        let value = relationship_provenance(&proof);
        assert_eq!(value["kind"], kind);
        assert_eq!(
            value.get("confidence").and_then(serde_json::Value::as_str),
            confidence
        );
        if confidence.is_none() {
            assert!(value.get("confidence").is_none());
        }
        match kind {
            "reference" => {
                assert_eq!(value["from_deposed_key"], "old-instance");
                assert!(value["to_deposed_key"].is_null());
            }
            "resource" => assert_eq!(value["deposed_key"], "old-instance"),
            _ => {
                assert_eq!(value["attribute"], "zone_id");
                assert_eq!(value["from"], current.address);
            }
        }
        let xml = format!(
            "<metadata>{}</metadata>",
            crate::svg::escape(&value.to_string())
        );
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let decoded: serde_json::Value =
            serde_json::from_str(doc.root_element().text().unwrap()).unwrap();
        assert_eq!(decoded, value);
    }
}

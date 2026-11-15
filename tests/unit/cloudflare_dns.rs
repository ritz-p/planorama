use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../fixtures/cloudflare-dns-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn linked(v: &Value, address: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| r.kind == crate::model::ReferenceKind::Membership && r.complete)
}
#[test]
fn zones_are_administrative_references_without_physical_containment() {
    let v = fixture();
    for i in [1, 2, 3, 5] {
        assert!(linked(
            &v,
            v["resource_changes"][i]["address"].as_str().unwrap()
        ));
    }
    let g = graph(&v);
    assert_eq!(g.nodes.len(), 6);
    assert!(
        g.edges
            .iter()
            .all(|e| e.kind == crate::model::EdgeKind::Dependency)
    );
    let record = g
        .nodes
        .iter()
        .find(|n| n.address == "cloudflare_dns_record.www")
        .unwrap();
    assert_eq!(
        record
            .entity
            .cloudflare_scope
            .as_ref()
            .unwrap()
            .account
            .as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(record.placement, crate::model::Placement::Logical);
    assert!(record.member_owner.is_none());
    assert!(!format!("{g:?}").contains("PRIVATE_"));
}
#[test]
fn dns_membership_requires_exact_current_unmasked_identity_and_provenance() {
    let address = "cloudflare_dns_record.www";
    for (path, value) in [
        (
            "/resource_changes/1/provider_name",
            json!("example/cloudflare"),
        ),
        (
            "/resource_changes/1/change/after/id",
            json!("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"),
        ),
        (
            "/resource_changes/1/change/after/id",
            json!("ffffffffffffffffffffffffffffffff"),
        ),
        ("/resource_changes/1/change/after_unknown/id", json!(true)),
        (
            "/resource_changes/2/change/after_sensitive/zone_id",
            json!(true),
        ),
        ("/resource_changes/1/change/actions", json!(["delete"])),
        (
            "/configuration/root_module/resources/2/expressions/zone_id/references",
            json!(["cloudflare_zone.example.name"]),
        ),
        (
            "/configuration/root_module/resources/2/expressions/zone_id/references",
            json!(["cloudflare_zone.example.id", "cloudflare_zone.other.id"]),
        ),
        (
            "/configuration/root_module/resources/2/expressions/zone_id/references",
            json!([]),
        ),
    ] {
        let mut v = fixture();
        let (parent, field) = path.rsplit_once('/').unwrap();
        v.pointer_mut(parent).unwrap()[field] = value;
        assert!(!linked(&v, address), "{path}");
    }
}
#[test]
fn duplicate_current_ids_fail_but_deposed_instances_do_not_claim_membership() {
    let mut v = fixture();
    let mut duplicate = v["resource_changes"][1].clone();
    duplicate["address"] = json!("cloudflare_zone.duplicate");
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(!linked(&v, "cloudflare_dns_record.www"));
    v["resource_changes"][6]["deposed"] = json!("old");
    v["resource_changes"][6]["change"]["actions"] = json!(["delete"]);
    assert!(linked(&v, "cloudflare_dns_record.www"));
    let g = graph(&v);
    assert_eq!(g.nodes.len(), 7);
}
#[test]
fn singleton_zone_data_uses_id_but_list_data_is_not_an_owner() {
    let mut v = fixture();
    for collection in ["resource_changes", "configuration/root_module/resources"] {
        let row = v.pointer_mut(&format!("/{collection}/1")).unwrap();
        row["address"] = json!("data.cloudflare_zone.example");
        row["mode"] = json!("data");
    }
    v["configuration"]["root_module"]["resources"][1]["expressions"] =
        json!({"zone_id": {"constant_value": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}});
    v["resource_changes"][1]["change"]["actions"] = json!(["read"]);
    v["resource_changes"][1]["change"]["after"]["zone_id"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    v["configuration"]["root_module"]["resources"][2]["expressions"]["zone_id"]["references"] =
        json!(["data.cloudflare_zone.example.id"]);
    assert!(linked(&v, "cloudflare_dns_record.www"));
    v["configuration"]["root_module"]["resources"][2]["expressions"]["zone_id"]["references"] =
        json!(["data.cloudflare_zone.example.zone_id"]);
    assert!(linked(&v, "cloudflare_dns_record.www"));
    v["resource_changes"][1]["type"] = json!("cloudflare_zones");
    assert!(!linked(&v, "cloudflare_dns_record.www"));
}

#[test]
fn record_changes_moves_deposed_and_named_states_remain_independent() {
    let mut v = fixture();
    v["resource_changes"][2]["change"]["actions"] = json!(["delete", "create"]);
    v["resource_changes"][2]["previous_address"] = json!("cloudflare_dns_record.old");
    let mut old = v["resource_changes"][2].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    old["change"]["after"] = Value::Null;
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let inputs = ["first", "second"].map(|state| crate::model::PlanInput {
        state_id: crate::model::StateId::new(state).unwrap(),
        plan_json: v.to_string(),
    });
    let plans = plan::parse_inputs(inputs.into()).unwrap();
    assert_eq!(plans.states.len(), 2);
    for plan in plans.states.values() {
        let g = semantic::transform(plan);
        let records: Vec<_> = g
            .nodes
            .iter()
            .filter(|n| n.address == "cloudflare_dns_record.www")
            .collect();
        assert_eq!(records.len(), 2);
        let current = records.iter().find(|n| n.deposed_key.is_none()).unwrap();
        assert_eq!(current.action, crate::model::Action::Replace);
        assert_eq!(
            current.previous_address.as_deref(),
            Some("cloudflare_dns_record.old")
        );
        assert!(
            current
                .references
                .iter()
                .any(|r| r.kind == crate::model::ReferenceKind::Membership && r.complete)
        );
        assert!(
            records
                .iter()
                .find(|n| n.deposed_key.is_some())
                .unwrap()
                .references
                .is_empty()
        );
    }
}

#[test]
fn singleton_zone_data_checks_both_identity_fields_for_each_output() {
    for output in ["id", "zone_id"] {
        let mut base = fixture();
        for collection in ["resource_changes", "configuration/root_module/resources"] {
            let row = base.pointer_mut(&format!("/{collection}/1")).unwrap();
            row["address"] = json!("data.cloudflare_zone.example");
            row["mode"] = json!("data");
        }
        base["resource_changes"][1]["change"]["after"]["zone_id"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        base["configuration"]["root_module"]["resources"][1]["expressions"] = json!({});
        base["configuration"]["root_module"]["resources"][2]["expressions"]["zone_id"]["references"] =
            json!([format!("data.cloudflare_zone.example.{output}")]);
        assert!(linked(&base, "cloudflare_dns_record.www"));
        for field in ["id", "zone_id"] {
            for (section, value) in [
                ("after", json!("ffffffffffffffffffffffffffffffff")),
                ("after", Value::Null),
                ("after_unknown", json!(true)),
                ("after_sensitive", json!(true)),
            ] {
                let mut v = base.clone();
                v["resource_changes"][1]["change"][section][field] = value;
                assert!(
                    !linked(&v, "cloudflare_dns_record.www"),
                    "{output}/{section}/{field}"
                );
                let g = graph(&v);
                let record = g
                    .nodes
                    .iter()
                    .find(|n| n.address == "cloudflare_dns_record.www")
                    .unwrap();
                assert!(
                    record
                        .entity
                        .cloudflare_scope
                        .as_ref()
                        .unwrap()
                        .account
                        .is_none()
                );
            }
        }
    }
}

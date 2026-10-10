use super::*;
use serde_json::json;

#[test]
fn indexed_addresses_keep_ambiguity_independent_of_node_order() {
    let input: Value =
        serde_json::from_str(include_str!("../../fixtures/checks-plan.json")).unwrap();
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    let current = raw.nodes[0].clone();
    let mut deposed = current.clone();
    deposed.deposed_key = Some("old".into());
    for mut nodes in [
        vec![current.clone(), current.clone()],
        vec![current.clone(), deposed.clone()],
        vec![deposed.clone()],
        vec![deposed.clone(), current.clone(), deposed],
    ] {
        nodes.push(raw.nodes[1].clone());
        let expected = parse(&input["checks"], &nodes);
        assert!(
            !expected
                .iter()
                .flat_map(|c| &c.instances)
                .any(|i| i.resource.as_deref() == Some(current.address.as_str()))
        );
        assert!(
            expected
                .iter()
                .flat_map(|c| &c.instances)
                .any(|i| i.resource.as_deref() == Some(raw.nodes[1].address.as_str()))
        );
        nodes.reverse();
        assert_eq!(parse(&input["checks"], &nodes), expected);
    }
}

#[test]
fn module_and_data_instances_require_exact_dynamic_addresses() {
    for (address, static_address, mode) in [
        (
            "module.app[\"a\"].terraform_data.item[0]",
            "module.app.terraform_data.item",
            "managed",
        ),
        (
            "module.app[0].data.terraform_data.item[\"a\"]",
            "module.app.data.terraform_data.item",
            "data",
        ),
    ] {
        let input = json!({"format_version":"1.2","resource_changes":[{"address":address,"type":"terraform_data","mode":mode,"change":{"actions":["read"]}}],"checks":[{"address":{"kind":"resource","mode":mode,"type":"terraform_data","to_display":static_address},"status":"unknown","instances":[{"address":{"to_display":address},"status":"unknown"}]}]});
        let raw = crate::plan::parse(&input.to_string()).unwrap();
        assert_eq!(
            raw.checks[0].instances[0].resource.as_deref(),
            Some(address)
        );
    }
}

#[test]
fn checks_keep_statuses_and_exact_resource_associations_without_values() {
    let raw = crate::plan::parse(include_str!("../../fixtures/checks-plan.json")).unwrap();
    assert_eq!(raw.checks.len(), 5);
    for status in ["pass", "fail", "error", "unknown", "future<&status"] {
        assert!(
            raw.checks
                .iter()
                .any(|check| check.status.as_deref() == Some(status))
        );
    }
    let failed = raw
        .checks
        .iter()
        .find(|check| check.status.as_deref() == Some("fail"))
        .unwrap();
    assert!(
        failed
            .instances
            .iter()
            .any(
                |instance| instance.resource.as_deref() == Some("terraform_data.app[0]")
                    && instance.status.as_deref() == Some("fail")
            )
    );
    assert!(
        failed
            .instances
            .iter()
            .any(
                |instance| instance.resource.as_deref() == Some("terraform_data.app[1]")
                    && instance.status.as_deref() == Some("pass")
            )
    );
    let graph = crate::semantic::transform(&raw);
    assert_eq!(graph.checks, raw.checks);
    assert!(!format!("{raw:?} {graph:?}").contains("TOP_SECRET"));
    assert_eq!(raw.nodes[0].action, crate::model::Action::Update);
}

#[test]
fn unfamiliar_shapes_are_safe_and_results_are_deterministic() {
    for checks in [json!(null), json!(false), json!({"future":"TOP_SECRET"})] {
        assert!(parse(&checks, &[]).is_empty());
    }
    let mut checks = json!([false, null, {"status":42,"instances":[null,{"status":false,"problems":[{"message":"TOP_SECRET"}]}]}, {"status":"unknown"}]);
    let expected = parse(&checks, &[]);
    assert_eq!(expected.len(), 2);
    assert_eq!(expected[0].status, None);
    assert_eq!(expected[0].instances[0].status, None);
    assert!(!format!("{expected:?}").contains("TOP_SECRET"));
    checks.as_array_mut().unwrap().reverse();
    assert_eq!(parse(&checks, &[]), expected);
}

#[test]
fn conflicting_or_ambiguous_addresses_are_not_associated() {
    let input: Value =
        serde_json::from_str(include_str!("../../fixtures/checks-plan.json")).unwrap();
    for (field, value) in [
        ("kind", "check"),
        ("mode", "data"),
        ("type", "aws_instance"),
        ("to_display", "terraform_data.other"),
    ] {
        let mut changed = input.clone();
        changed["checks"][0]["address"][field] = value.into();
        let raw = crate::plan::parse(&changed.to_string()).unwrap();
        assert!(
            raw.checks
                .iter()
                .flat_map(|c| &c.instances)
                .all(|i| i.resource.is_none())
        );
    }
    let mut changed = input.clone();
    let mut deposed = changed["resource_changes"][0].clone();
    deposed["deposed"] = "old".into();
    changed["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(deposed);
    let raw = crate::plan::parse(&changed.to_string()).unwrap();
    assert!(
        !raw.checks
            .iter()
            .flat_map(|c| &c.instances)
            .any(|i| i.resource.as_deref() == Some("terraform_data.app[0]"))
    );
    changed["checks"][0]["instances"][1]["address"]["to_display"] = "terraform_data.app".into();
    let raw = crate::plan::parse(&changed.to_string()).unwrap();
    assert!(
        raw.checks
            .iter()
            .flat_map(|c| &c.instances)
            .all(|i| i.resource.is_none())
    );
}

#[test]
fn lowered_relationships_keep_resource_check_identity() {
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap();
    let address = input["resource_changes"][2]["address"]
        .as_str()
        .unwrap()
        .to_owned();
    input["checks"] = json!([{"address":{"kind":"resource","mode":"managed","type":"aws_route_table_association","to_display":address},"status":"fail","instances":[{"address":{"to_display":address},"status":"fail"}]}]);
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert!(!graph.nodes.iter().any(|n| n.address == address));
    assert_eq!(
        graph.checks[0].instances[0].resource.as_deref(),
        Some(address.as_str())
    );
}

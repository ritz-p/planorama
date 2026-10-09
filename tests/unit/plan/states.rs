use super::*;
use crate::model::StateId;

fn input(id: &str) -> PlanInput {
    PlanInput {
        state_id: StateId::new(id).unwrap(),
        plan_json: include_str!("../../fixtures/multi/network.json").into(),
    }
}

#[test]
fn state_partitions_preserve_local_change_identity_and_ignore_input_order() {
    let first = parse_inputs(vec![input("network"), input("application")]).unwrap();
    let reversed = parse_inputs(vec![input("application"), input("network")]).unwrap();
    assert_eq!(first, reversed);
    assert_eq!(first.states.len(), 2);
    for graph in first.states.values() {
        let copies: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.address == "aws_security_group.app")
            .collect();
        assert_eq!(copies.len(), 2);
        assert!(
            copies
                .iter()
                .any(|n| n.deposed_key.as_deref() == Some("old"))
        );
        assert!(copies.iter().any(|n| n.deposed_key.is_none()));
    }
}

#[test]
fn duplicate_ids_and_invalid_plans_fail_with_state_context() {
    assert!(
        parse_inputs(vec![input("same"), input("same")])
            .unwrap_err()
            .contains("duplicate state ID: same")
    );
    let mut bad = input("broken");
    bad.plan_json = "broken".into();
    assert!(
        parse_inputs(vec![input("good"), bad])
            .unwrap_err()
            .contains("state \"broken\": invalid JSON")
    );
    for name in ["", "a=b", "a b", "a\nb", "日本語"] {
        assert!(StateId::new(name).is_err());
    }
    assert!(StateId::new(&"a".repeat(65)).is_err());
}

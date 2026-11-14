use super::*;
use crate::{plan, semantic, view};

#[test]
fn shared_accounting_selection_and_details_survive_lowering_and_filtering() {
    for (input, focus) in [
        (
            include_str!("../fixtures/containment-plan.json"),
            "aws_instance.app",
        ),
        (
            include_str!("../fixtures/routes-plan.json"),
            "aws_route.default",
        ),
        (
            include_str!("../fixtures/association-plan.json"),
            "aws_route_table_association.private",
        ),
    ] {
        let options = view::Options {
            explain_selection: true,
            focus: Some(focus.into()),
            depth: Some(0),
            ..Default::default()
        };
        let graph =
            view::single(&semantic::transform(&plan::parse(input).unwrap()), &options).unwrap();
        let coverage = &graph.coverage;
        let report = render(std::iter::once(("", coverage)), 1000);
        assert_eq!(
            report["summary"]["input"]["resources"],
            coverage.input.resources
        );
        assert_eq!(
            report["summary"]["selected"]["resources"],
            coverage.selected.resources
        );
        assert_eq!(
            report["summary"]["selected_findings"],
            json!(coverage.selected_findings)
        );
        assert_eq!(
            report["summary"]["selection"]["direct"],
            coverage.selection.as_ref().unwrap().direct()
        );
        let records = report["details"]["resources"]["records"]
            .as_array()
            .unwrap();
        assert_eq!(records.len(), coverage.selected.resources);
        let unique: std::collections::BTreeSet<_> = records.iter().map(|r| r.to_string()).collect();
        assert_eq!(unique.len(), records.len());
        for record in records {
            assert!(
                coverage
                    .resources
                    .keys()
                    .any(|id| json!(id.address) == record["address"]
                        && json!(id.deposed_key) == record["deposed"])
            );
        }
        let markdown = super::super::report::render(std::iter::once(("", coverage)), &options);
        assert!(markdown.contains(&format!(
            "| Resource instances | {} | {} |",
            coverage.input.resources, coverage.selected.resources
        )));
        let bounded = render(std::iter::once(("", coverage)), 1);
        assert_eq!(
            bounded["details"]["resources"]["records"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            bounded["details"]["resources"]["omitted"],
            coverage.selected.resources - 1
        );
        assert_eq!(
            bounded["details"]["resources"]["truncated"],
            coverage.selected.resources > 1
        );
        let disabled = render(std::iter::once(("", coverage)), 0);
        assert_eq!(disabled["details"]["resources"]["records"], json!([]));
        assert_eq!(
            disabled["details"]["resources"]["omitted"],
            coverage.selected.resources
        );
        assert!(!disabled.to_string().contains("aws_instance.app"));
    }
}

#[test]
fn states_lifecycle_modes_and_findings_are_deterministic_and_bounded() {
    let mut coverage = Coverage {
        collected: true,
        input_complete: Some(false),
        ..Default::default()
    };
    for (address, deposed, mode) in [
        ("terraform_data.x", None, crate::model::EntityMode::Managed),
        (
            "terraform_data.x",
            Some("old"),
            crate::model::EntityMode::Managed,
        ),
        (
            "data.terraform_data.x",
            None,
            crate::model::EntityMode::Data,
        ),
    ] {
        let id = crate::model::TerraformEntityId {
            address: address.into(),
            deposed_key: deposed.map(Into::into),
        };
        coverage.resources.insert(
            id.clone(),
            super::super::Resource {
                mode,
                account_applicable: false,
                unknown_account: false,
                region_applicable: false,
                unknown_region: false,
            },
        );
        coverage.findings.push(super::super::Finding {
            source: id,
            attribute: Some("target".into()),
            code: "unresolved_reference",
            category: "unresolved",
            message: "resource reference could not be resolved",
        });
    }
    coverage.input.resources = 3;
    coverage.retain(&coverage.resources.keys().cloned().collect());
    let first = render([("z", &coverage), ("a", &coverage)].into_iter(), 1000);
    let mut reversed = coverage.clone();
    reversed.findings.reverse();
    let second = render([("a", &reversed), ("z", &reversed)].into_iter(), 1000);
    assert_eq!(first, second);
    assert_eq!(first["summary"]["selected"]["resources"], 6);
    assert_eq!(first["summary"]["input_complete"], false);
    assert_eq!(first["summary"]["selection"]["collected"], false);
    let records = first["details"]["resources"]["records"].as_array().unwrap();
    assert!(records.iter().any(|r| r["deposed"] == "old"));
    assert!(records.iter().any(|r| r["mode"] == "data"));
    assert_eq!(first["details"]["findings"]["total"], 6);
    assert_eq!(first["details"]["findings"]["records_complete"], true);
    let bounded = render([("a", &coverage), ("z", &coverage)].into_iter(), 2);
    assert_eq!(
        bounded["details"]["findings"]["records"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(bounded["details"]["findings"]["omitted"], 4);
    assert_eq!(bounded["details"]["findings"]["records_complete"], false);
}

#[test]
fn unavailable_empty_and_unknown_input_are_distinct() {
    let unavailable = Coverage::default();
    let report = render(std::iter::once(("", &unavailable)), 10);
    assert_eq!(report["summary"]["collected"], false);
    assert!(report["summary"]["input"].is_null());
    assert!(report["details"]["findings"]["total"].is_null());
    assert_eq!(report["details"]["findings"]["records_complete"], false);
    for complete in [None, Some(false), Some(true)] {
        let empty = Coverage {
            collected: true,
            input_complete: complete,
            ..Default::default()
        };
        let report = render(std::iter::once(("", &empty)), 10);
        assert_eq!(report["summary"]["collected"], true);
        assert_eq!(report["summary"]["input"]["resources"], 0);
        assert_eq!(report["summary"]["input_complete"], json!(complete));
        assert_eq!(report["details"]["findings"]["records_complete"], true);
        let mixed = render([("a", &empty), ("b", &unavailable)].into_iter(), 10);
        assert!(mixed["summary"]["input"].is_null());
        assert_eq!(mixed["states"][0]["summary"]["input"]["resources"], 0);
    }
}

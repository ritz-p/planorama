use super::*;
use crate::{Options, OutputFormat, generate};

const FIXTURE: &str = include_str!("../fixtures/viewer-plan.json");

#[test]
fn verified_account_evidence_is_removed_from_visible_and_hidden_serialization() {
    let input = include_str!("../fixtures/accounts-plan.json");
    let raw = crate::plan::evidence::tests::parse_fixture(input).unwrap();
    let graph = crate::semantic::transform(&raw);
    for anonymize in [false, true] {
        let projected = crate::view::single(
            &graph,
            &crate::view::Options {
                anonymize,
                ..Default::default()
            },
        )
        .unwrap();
        let layout = crate::layout::Layout::new(&projected);
        let svg = crate::svg::render(&projected, &layout);
        let html = crate::html::render(&projected, &layout, crate::svg::Options::default());
        for output in [svg, html] {
            for original in ["123456789012", "210987654321"] {
                assert_eq!(
                    output.contains(original),
                    !anonymize,
                    "verified account {original}"
                );
                if anonymize {
                    use std::fmt::Write;
                    let hex = original.bytes().fold(String::new(), |mut hex, byte| {
                        write!(&mut hex, "{byte:02x}").unwrap();
                        hex
                    });
                    assert!(!output.contains(&hex));
                }
            }
            assert_eq!(output.contains("account_1"), anonymize);
        }
    }
}

#[test]
fn anonymous_projection_keeps_topology_and_resolves_references() {
    for fixture in [
        FIXTURE,
        include_str!("../fixtures/routes-plan.json"),
        include_str!("../fixtures/cloudflare-applications-plan.json"),
        include_str!("../fixtures/organization-deployments-plan.json"),
    ] {
        let raw = crate::plan::parse(fixture).unwrap();
        let graph = crate::semantic::transform(&raw);
        let ordinary = crate::view::single(&graph, &crate::view::Options::default()).unwrap();
        let private = crate::view::single(
            &graph,
            &crate::view::Options {
                anonymize: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(ordinary.nodes.len(), private.nodes.len());
        assert_eq!(ordinary.relationships.len(), private.relationships.len());
        assert_eq!(ordinary.components.len(), private.components.len());
        assert_eq!(
            ordinary
                .edges
                .iter()
                .map(|e| (e.from, e.to, e.kind))
                .collect::<Vec<_>>(),
            private
                .edges
                .iter()
                .map(|e| (e.from, e.to, e.kind))
                .collect::<Vec<_>>()
        );
        for (a, b) in ordinary.nodes.iter().zip(&private.nodes) {
            assert_eq!(a.resource_type, b.resource_type);
            assert_eq!(a.placement, b.placement);
            assert_eq!(
                crate::view::routes::owner(&ordinary, a),
                crate::view::routes::owner(&private, b)
            );
            assert_eq!(
                a.references.iter().map(|r| r.targets.len()).sum::<usize>(),
                b.references.iter().map(|r| r.targets.len()).sum::<usize>()
            );
        }
        for relation in &private.relationships {
            assert!(private.entities().any(|e| e.id == relation.from));
            assert!(private.entities().any(|e| e.id == relation.to));
        }
    }
}

#[test]
fn anonymized_generation_is_repeatable_and_shared_by_resumable_paths() {
    for format in [OutputFormat::Svg, OutputFormat::Html] {
        let options = || Options {
            anonymize: true,
            format,
            ..Default::default()
        };
        let result = generate(FIXTURE, &options()).unwrap();
        assert_eq!(
            result.content,
            generate(FIXTURE, &options()).unwrap().content
        );
        assert!(result.content.contains("aws_lb.resource_"));
        let mut generator = crate::Generator::new(FIXTURE.into(), options()).unwrap();
        loop {
            if let Some(sliced) = generator.step(4096).unwrap() {
                assert_eq!(result.content, sliced.content);
                break;
            }
        }
    }
}

#[test]
fn full_mode_rejects_optional_identifier_reports_and_invalid_values() {
    for format in [OutputFormat::Svg, OutputFormat::Html] {
        for (routing, semantic) in [(true, false), (false, true)] {
            let options = Options {
                anonymize: true,
                format,
                html_diagnostics: routing,
                html_semantic_diagnostics: semantic,
                ..Default::default()
            };
            assert_eq!(
                generate(FIXTURE, &options).err(),
                Some(crate::Error::InvalidOptions)
            );
            assert!(matches!(
                crate::Generator::new(FIXTURE.into(), options),
                Err(crate::Error::InvalidOptions)
            ));
        }
    }
    for invalid in ["null", "1", "\"true\""] {
        assert!(serde_json::from_str::<Options>(&format!("{{\"anonymize\":{invalid}}}")).is_err());
    }
}

#[test]
fn pseudonyms_preserve_nested_module_identity_and_account_equality() {
    let mut map = Projection::default();
    let a =
        "module.customer_secret[\"blue.secret\"].module.private_secret.aws_instance.private_secret";
    let b =
        "module.customer_secret[\"blue.secret\"].module.private_secret.aws_instance.other_secret";
    let first = map.address(a);
    assert_eq!(first, map.address(a));
    let second = map.address(b);
    assert_ne!(first, second);
    assert_eq!(
        crate::model::module_of(&first),
        crate::model::module_of(&second)
    );
    assert!(!first.contains("secret"));
    assert_eq!(first.matches("module.").count(), 2);
    let mut account = AwsAccountScope::new("123456789012").unwrap();
    let mut same = account.clone();
    let mut other = AwsAccountScope::new("987654321098").unwrap();
    map.account(&mut account);
    map.account(&mut same);
    map.account(&mut other);
    assert_eq!(account, same);
    assert_ne!(account, other);
    assert_eq!(account.as_str(), "account_1");
}

#[test]
fn input_names_in_the_pseudonym_namespace_are_not_reused() {
    let input = r#"{"format_version":"1.2","resource_changes":[
        {"address":"terraform_data.resource_1","type":"terraform_data","change":{"actions":["create"]}},
        {"address":"terraform_data.resource_2","type":"terraform_data","change":{"actions":["create"]}}
    ]}"#;
    let inputs = [
        crate::NamedInput {
            state: "state_1".into(),
            json: input.into(),
        },
        crate::NamedInput {
            state: "state_2".into(),
            json: input.into(),
        },
    ];
    let output = crate::generate_named(
        &inputs,
        &Options {
            anonymize: true,
            format: OutputFormat::Html,
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    for original in [
        "terraform_data.resource_1",
        "terraform_data.resource_2",
        "\"state_1\"",
        "\"state_2\"",
    ] {
        assert!(!output.contains(original), "{original}");
    }
    assert!(output.contains("terraform_data.resource_3"));
    assert!(output.contains("state_3"));
    assert!(output.contains("state_4"));
}

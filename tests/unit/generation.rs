use super::*;

#[test]
fn slices_preserve_svg_html_and_diagnostics() {
    for json in [
        include_str!("../fixtures/viewer-plan.json"),
        include_str!("../fixtures/aws-basic-plan.json"),
        include_str!("../fixtures/terraform-plan.json"),
    ] {
        for format in [OutputFormat::Svg, OutputFormat::Html] {
            let options = || Options {
                format,
                icon_mode: crate::IconMode::ServiceName,
                ..Default::default()
            };
            let expected = generate(json, &options()).unwrap();
            for sizes in [&[1][..], &[7, 1, 64, 3][..], &[4096][..]] {
                let mut generator = Generator::new(json.to_owned(), options()).unwrap();
                let mut steps = 0;
                let actual = loop {
                    let result = generator.step(sizes[steps % sizes.len()]).unwrap();
                    steps += 1;
                    assert!(
                        steps < 100_000_000,
                        "generation must make progress: {} bytes",
                        json.len()
                    );
                    if let Some(result) = result {
                        break result;
                    }
                };
                assert_eq!(actual.content, expected.content);
                assert_eq!(actual.diagnostics, expected.diagnostics);
                assert!(generator.future.is_none());
                assert_eq!(generator.step(1).err(), Some(Error::GenerationClosed));
            }
        }
    }
}

#[test]
fn resumable_accepts_the_same_leading_boms_as_the_parser() {
    let json = format!(
        "\u{feff}\u{feff}{}",
        include_str!("../fixtures/viewer-plan.json")
    );
    let expected = generate(&json, &Options::default()).unwrap();
    let mut job = Generator::new(json, Options::default()).unwrap();
    loop {
        if let Some(actual) = job.step(4096).unwrap() {
            assert_eq!(actual.content, expected.content);
            break;
        }
    }
}

#[test]
fn resumable_options_are_bounded_before_retaining_the_job() {
    let json = "{}";
    for options in [
        Options {
            modules: vec![String::new(); 257],
            ..Default::default()
        },
        Options {
            spatial_types: vec![String::new(); 257],
            ..Default::default()
        },
        Options {
            focus: Some("x".repeat(MAX_BROWSER_OPTIONS_BYTES + 1)),
            ..Default::default()
        },
        Options {
            modules: vec!["x".repeat(MAX_BROWSER_OPTIONS_BYTES)],
            focus: Some("x".into()),
            ..Default::default()
        },
    ] {
        assert!(matches!(
            Generator::new(json.into(), options),
            Err(Error::ResourceLimit)
        ));
    }
    assert!(
        Generator::new(
            json.into(),
            Options {
                modules: vec!["x".repeat(MAX_BROWSER_OPTIONS_BYTES / 256); 256],
                spatial_types: vec![String::new(); 256],
                ..Default::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn cancellation_and_failure_release_state() {
    let json = include_str!("../fixtures/viewer-plan.json");
    for steps in [0, 1, 10, 100] {
        let mut generator = Generator::new(json.into(), Options::default()).unwrap();
        assert_eq!(generator.step(0).err(), Some(Error::InvalidOptions));
        for _ in 0..steps {
            if generator.step(1).unwrap().is_some() {
                break;
            }
        }
        generator.cancel();
        generator.cancel();
        assert!(generator.future.is_none());
        assert_eq!(generator.step(1).err(), Some(Error::GenerationClosed));
    }
    let mut generator = Generator::new("invalid".into(), Options::default()).unwrap();
    assert_eq!(generator.step(4096).err(), Some(Error::InvalidJson));
    assert!(generator.future.is_none());
    assert!(generate(json, &Options::default()).is_ok());
}

#[test]
fn browser_budgets_fail_without_returning_partial_output() {
    assert_eq!(
        Generator::new(" ".repeat(MAX_BROWSER_INPUT_BYTES + 1), Options::default())
            .err()
            .map(|e| e.to_string()),
        Some(Error::ResourceLimit.to_string())
    );
    let resources: Vec<_> = (0..257).map(|i| serde_json::json!({
        "address": format!("terraform_data.item{i}"), "mode": "managed", "type": "terraform_data",
        "name": format!("item{i}"), "provider_name": "terraform.io/builtin/terraform",
        "change": {"actions": ["create"], "before": null, "after": {"input": i}}
    })).collect();
    let json =
        serde_json::json!({"format_version":"1.2", "resource_changes":resources}).to_string();
    let mut job = Generator::new(json, Options::default()).unwrap();
    loop {
        match job.step(1) {
            Ok(None) => {}
            Err(Error::ResourceLimit) => break,
            _ => panic!("over-budget input must not return a diagram"),
        }
    }
    assert!(job.future.is_none());
    assert_eq!(job.step(1).err(), Some(Error::GenerationClosed));
}

#[test]
fn public_errors_do_not_disclose_input_or_selection() {
    for (input, options, expected) in [
        ("secret", Options::default(), Error::InvalidJson),
        (
            r#"{"format_version":"secret"}"#,
            Options::default(),
            Error::UnsupportedInput,
        ),
        (
            include_str!("../fixtures/viewer-plan.json"),
            Options {
                focus: Some("secret".into()),
                ..Default::default()
            },
            Error::SelectionFailed,
        ),
        (
            include_str!("../fixtures/viewer-plan.json"),
            Options {
                focus_depth: Some(1),
                ..Default::default()
            },
            Error::InvalidOptions,
        ),
    ] {
        assert_eq!(generate(input, &options).err(), Some(expected));
        assert!(!expected.to_string().contains("secret"));
    }
    assert!(serde_json::from_str::<Options>(r#"{"provider_plan":"secret"}"#).is_err());
}

#[test]
fn bounded_input_and_static_diagnostics() {
    assert_eq!(
        generate(&" ".repeat(MAX_INPUT_BYTES + 1), &Options::default()).err(),
        Some(Error::InputTooLarge)
    );
    let result = generate(
        include_str!("../fixtures/viewer-plan.json"),
        &Options::default(),
    )
    .unwrap();
    assert!(result.content.contains("<svg"));
    assert!(
        result
            .diagnostics
            .iter()
            .all(|message| !message.contains("aws_"))
    );
}

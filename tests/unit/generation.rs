use super::*;

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

use crate::{Error, IconMode, Options, OutputFormat, generate};

#[test]
fn artwork_free_defaults_and_explicit_icon_rejection() {
    assert_eq!(IconMode::default(), IconMode::ServiceName);
    let json = include_str!("../fixtures/viewer-plan.json");
    for format in [OutputFormat::Svg, OutputFormat::Html] {
        let result = generate(
            json,
            &Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!result.content.contains("<symbol"));
        assert!(!result.content.contains("planorama-icon-aws"));
        assert!(!result.content.contains("planorama-icon-terraform"));
    }
    assert_eq!(
        generate(
            json,
            &Options {
                icon_mode: IconMode::Icon,
                ..Default::default()
            }
        )
        .err(),
        Some(Error::InvalidOptions)
    );
}

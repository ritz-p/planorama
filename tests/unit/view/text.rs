use super::{width, wrap, wrap_address};

#[test]
fn addresses_prefer_component_boundaries() {
    assert_eq!(
        wrap_address("module.network.aws_instance.app", 30),
        ["module.network.aws_instance.", "app"]
    );
    assert_eq!(
        wrap_address(r#"aws_instance.app["a.b"].id"#, 20),
        ["aws_instance.", r#"app["a.b"].id"#]
    );
    assert_eq!(
        wrap_address(r#"aws_instance.app["a\".b"].id"#, 22),
        ["aws_instance.", r#"app["a\".b"].id"#]
    );
}

#[test]
fn addresses_preserve_every_character_at_all_widths() {
    for text in [
        "",
        "module.root.aws_instance.app[0]",
        "network:module.network[0].module.private.aws_subnet.app[12]",
        r#"Uses: module.network["東京.  西"].aws_instance.app["a\\\".b"]"#,
        "Unavailable — Dependency: aws_instance.app (deposed: abc123)",
        "aws_instance.an_unusually_long_resource_name_without_any_separator",
        "Moved from: module.東京.aws_instance.サーバー[0]",
    ] {
        for limit in 0..=40 {
            let lines = wrap_address(text, limit);
            assert_eq!(lines.concat(), text, "{text:?}, limit {limit}");
            if !text.is_empty() {
                assert!(lines.iter().all(|line| !line.is_empty()));
            }
            for line in &lines {
                assert!(
                    width(line) <= limit || line.chars().count() == 1,
                    "{line:?}, limit {limit}"
                );
            }
            assert_eq!(lines, wrap_address(text, limit));
        }
    }
}

#[test]
fn hard_wrapping_keeps_quote_state_across_lines() {
    assert_eq!(
        wrap_address(r#"r.x["abcdefghij.klmnopqrst"].y"#, 12),
        ["r.", r#"x["abcdefghi"#, r#"j.klmnopqrst"#, r#""].y"#]
    );
}

#[test]
fn prose_keeps_width_based_wrapping() {
    assert_eq!(
        wrap("module.network.aws_instance.app", 30),
        ["module.network.aws_instance.ap", "p"]
    );
    assert_eq!(wrap("日本語abc", 5), ["日本", "語abc"]);
    assert_eq!(wrap("", 0), [""]);
}

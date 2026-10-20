use super::{cli, support};
use std::process::Output;

const INPUT: &[u8] = br#"{"format_version":"1.2","resource_changes":[
    {"address":"module.app.terraform_data.a","type":"terraform_data","change":{"actions":["create"]}},
    {"address":"module.db.terraform_data.b","type":"terraform_data","change":{"actions":["no-op"]}},
    {"address":"terraform_data.root","type":"terraform_data","change":{"actions":["create"]}}
]}"#;

#[test]
fn spatial_types_replace_layers_and_clear_explicitly() {
    let input = include_bytes!("../fixtures/identity-providers-plan.json");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spatial.toml");
    std::fs::write(
        &path,
        r#"
[generation]
spatial-types = ["aws_iam_role"]
[presets.providers]
spatial-types = ["aws_iam_openid_connect_provider", "aws_iam_saml_provider"]
"#,
    )
    .unwrap();
    for format in ["svg", "html"] {
        for (overrides, expected) in [
            (vec![], vec!["--spatial-type", "aws_iam_role"]),
            (
                vec!["--preset", "providers"],
                vec![
                    "--spatial-type",
                    "aws_iam_openid_connect_provider",
                    "--spatial-type",
                    "aws_iam_saml_provider",
                ],
            ),
            (
                vec!["--preset", "providers", "--spatial-type", "aws_iam_role"],
                vec!["--spatial-type", "aws_iam_role"],
            ),
            (vec!["--no-spatial-types"], vec![]),
            (
                vec![
                    "--spatial-type",
                    "aws_iam_role",
                    "--no-spatial-types",
                    "--spatial-type",
                    "aws_iam_saml_provider",
                ],
                vec!["--spatial-type", "aws_iam_saml_provider"],
            ),
        ] {
            let mut args = vec!["--config", path.to_str().unwrap(), "--format", format];
            args.extend(overrides);
            let mut direct = vec!["--format", format];
            direct.extend(expected);
            same(
                support::run_with_args(input, &args),
                support::run_with_args(input, &direct),
            );
        }
        let default = support::run_with_args(input, &["--format", format]);
        let spatial = support::run_with_args(
            input,
            &["--format", format, "--spatial-type", "aws_iam_role"],
        );
        assert_ne!(default.stdout, spatial.stdout);
    }
    for args in [
        vec!["--spatial-type"],
        vec!["--spatial-type", "aws_iam_policy"],
        vec!["--spatial-type", "aws_iam_role,aws_iam_user"],
    ] {
        assert!(!support::run_with_args(input, &args).status.success());
    }
    std::fs::write(
        &path,
        "[generation]\nspatial-types = [\"private-invalid-type\"]",
    )
    .unwrap();
    let rejected = support::run_with_args(input, &["--config", path.to_str().unwrap()]);
    assert!(!rejected.status.success());
    assert!(!String::from_utf8_lossy(&rejected.stderr).contains("private-invalid-type"));
}

fn configured(source: &str, args: &[&str]) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("planorama.toml");
    std::fs::write(&path, source).unwrap();
    let mut options = vec!["--config", path.to_str().unwrap()];
    options.extend_from_slice(args);
    support::run_with_args(INPUT, &options)
}

fn same(actual: Output, expected: Output) {
    for output in [&actual, &expected] {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(actual.stdout, expected.stdout);
}

#[test]
fn configuration_common_presets_and_cli_match_svg_and_html() {
    let source = r#"
[generation]
format = "html"
icon-mode = "service-name"
address-format = "terraform"
hide-relationship-legend = true
changed-only = true
modules = ["module.app"]
[presets.database]
modules = ["module.db"]
changed-only = false
[presets.all]
modules = []
changed-only = false
hide-relationship-legend = false
"#;
    for format in ["svg", "html"] {
        let common = [
            "--format",
            format,
            "--icon-mode",
            "service-name",
            "--address-format",
            "terraform",
        ];
        let mut expected = common.to_vec();
        expected.extend([
            "--hide-relationship-legend",
            "--changed-only",
            "--modules",
            "module.app",
        ]);
        same(
            configured(source, &["--format", format]),
            support::run_with_args(INPUT, &expected),
        );

        let mut expected = common.to_vec();
        expected.extend(["--hide-relationship-legend", "--modules", "module.db"]);
        same(
            configured(source, &["--preset", "database", "--format", format]),
            support::run_with_args(INPUT, &expected),
        );

        same(
            configured(source, &["--preset", "all", "--format", format]),
            support::run_with_args(INPUT, &common),
        );

        let mut expected = common.to_vec();
        expected.extend(["--hide-relationship-legend", "--modules", "module.app"]);
        same(
            configured(
                source,
                &[
                    "--preset",
                    "database",
                    "--format",
                    format,
                    "--modules",
                    "module.app",
                ],
            ),
            support::run_with_args(INPUT, &expected),
        );

        let mut expected = common.to_vec();
        expected.extend(["--modules", "module.app", "module.db"]);
        same(
            configured(
                source,
                &[
                    "--preset",
                    "database",
                    "--format",
                    format,
                    "--no-hide-relationship-legend",
                    "--modules",
                    "module.app",
                    "--modules",
                    "module.db",
                ],
            ),
            support::run_with_args(INPUT, &expected),
        );

        same(
            configured(
                source,
                &[
                    "--format",
                    format,
                    "--no-changed-only",
                    "--no-hide-relationship-legend",
                    "--no-modules",
                ],
            ),
            support::run_with_args(INPUT, &common),
        );
    }
}

#[test]
fn configuration_focus_replaces_and_clears_inherited_depth() {
    let source = r#"
[generation]
focus = ["module.app.terraform_data.a"]
focus-depth = 0
[presets.database]
focus = ["module.db.terraform_data.b"]
[presets.all]
focus = []
"#;
    same(
        configured(source, &[]),
        support::run_with_args(
            INPUT,
            &[
                "--focus",
                "module.app.terraform_data.a",
                "--focus-depth",
                "0",
            ],
        ),
    );
    same(
        configured(source, &["--preset", "database"]),
        support::run_with_args(
            INPUT,
            &[
                "--focus",
                "module.db.terraform_data.b",
                "--focus-depth",
                "0",
            ],
        ),
    );
    same(
        configured(
            source,
            &[
                "--preset",
                "database",
                "--focus",
                "terraform_data.root",
                "--focus-depth",
                "2",
            ],
        ),
        support::run_with_args(
            INPUT,
            &["--focus", "terraform_data.root", "--focus-depth", "2"],
        ),
    );
    same(
        configured(source, &["--preset", "all"]),
        support::run_with_args(INPUT, &[]),
    );
    same(
        configured(source, &["--no-focus"]),
        support::run_with_args(INPUT, &[]),
    );
}

#[test]
fn configuration_booleans_can_be_disabled_and_cli_order_wins() {
    let source = r#"
[generation]
format = "html"
diagnostics = true
timings = true
html-diagnostics = true
html-semantic-diagnostics = true
layout-diagnostics = true
"#;
    let output = configured(
        source,
        &[
            "--format",
            "svg",
            "--no-diagnostics",
            "--no-timings",
            "--no-html-diagnostics",
            "--no-html-semantic-diagnostics",
            "--no-layout-diagnostics",
        ],
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    same(output, support::run_with_args(INPUT, &[]));
    let output = configured(
        "[generation]\nchanged-only = true",
        &["--changed-only", "--no-changed-only"],
    );
    same(output, support::run_with_args(INPUT, &[]));
    same(
        configured(
            "[generation]\nchanged-only = false",
            &["--no-changed-only", "--changed-only"],
        ),
        support::run_with_args(INPUT, &["--changed-only"]),
    );
    same(
        configured(source, &[]),
        support::run_with_args(
            INPUT,
            &[
                "--format",
                "html",
                "--diagnostics",
                "--timings",
                "--html-diagnostics",
                "--html-semantic-diagnostics",
                "--layout-diagnostics",
            ],
        ),
    );
}

#[test]
fn configuration_errors_are_bounded_and_do_not_echo_values() {
    for (source, args, message) in [
        ("secret = 'DO_NOT_ECHO'", vec![], "unknown config key"),
        (
            "[generation]\nsecret = 'DO_NOT_ECHO'",
            vec![],
            "unknown generation setting",
        ),
        (
            "[generation]\nformat = 'DO_NOT_ECHO'",
            vec![],
            "invalid config setting 'format'",
        ),
        (
            "[generation]\nchanged-only = 'DO_NOT_ECHO'",
            vec![],
            "invalid config setting 'changed-only'",
        ),
        (
            "[generation]\nmodules = ['ok', 1]",
            vec![],
            "invalid config setting 'modules'",
        ),
        (
            "[generation]\nfocus-depth = -1",
            vec![],
            "invalid config setting 'focus-depth'",
        ),
        ("[generation]\nfocus = ['a', 'b']", vec![], "at most one"),
        (
            "[generation]\nformat = 'DO_NOT_ECHO",
            vec![],
            "invalid TOML",
        ),
        (
            "[presets.unused]\noutput = 'DO_NOT_ECHO'",
            vec![],
            "unknown generation setting",
        ),
        ("", vec!["--preset", "DO_NOT_ECHO"], "preset does not exist"),
        (
            "[generation]\nhtml-diagnostics = true",
            vec![],
            "requires --format html",
        ),
        (
            "[generation]\nhtml-semantic-diagnostics = true",
            vec![],
            "requires --format html",
        ),
        ("[generation]\nfocus-depth = 2", vec![], "requires --focus"),
        ("", vec!["--list-modules"], "cannot be combined"),
    ] {
        let output = configured(source, &args);
        assert!(!output.status.success(), "{source}");
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(message), "{error}");
        assert!(!error.contains("DO_NOT_ECHO"), "{error}");
    }
    for args in [
        vec!["--config"],
        vec!["--preset"],
        vec!["--preset", "missing"],
        vec!["--config", "a", "--config", "b"],
        vec!["--preset", "a", "--preset", "b"],
        vec!["--config", "missing-config.toml"],
    ] {
        assert!(!cli().args(args).output().unwrap().status.success());
    }
}

#[test]
fn configuration_is_explicit_and_cannot_be_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("planorama.toml");
    std::fs::write(&path, "not valid TOML").unwrap();
    let output = cli()
        .current_dir(dir.path())
        .args([
            concat!(env!("CARGO_MANIFEST_DIR"), "/examples/plan.json"),
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    std::fs::write(&path, "[generation]").unwrap();
    for option in ["-o", "--coverage-report", "--layout-diagnostics-json"] {
        let output = support::run_with_args(
            INPUT,
            &[
                "--config",
                path.to_str().unwrap(),
                option,
                path.to_str().unwrap(),
            ],
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("different files"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[generation]");
    }
    same(configured("", &[]), support::run_with_args(INPUT, &[]));
}

#[test]
fn configuration_relative_path_default_output_and_named_states() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("input.json"), INPUT).unwrap();
    let source = "[generation]\nformat = 'html'\n[presets.selected]\nmodules = ['app:module.app']\nfocus = ['app:module.app.terraform_data.a']\nfocus-depth = 0";
    std::fs::write(dir.path().join("settings.toml"), source).unwrap();
    let actual = cli()
        .current_dir(dir.path())
        .args([
            "--state",
            "app=input.json",
            "--preset",
            "selected",
            "--config",
            "settings.toml",
        ])
        .output()
        .unwrap();
    assert!(
        actual.status.success(),
        "{}",
        String::from_utf8_lossy(&actual.stderr)
    );
    let expected = cli()
        .current_dir(dir.path())
        .args([
            "--state",
            "app=input.json",
            "--format",
            "html",
            "--modules",
            "app:module.app",
            "--focus",
            "app:module.app.terraform_data.a",
            "--focus-depth",
            "0",
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(expected.status.success());
    assert_eq!(
        std::fs::read(dir.path().join("diagram.html")).unwrap(),
        expected.stdout
    );
    let actual = cli()
        .current_dir(dir.path())
        .args([
            "input.json",
            "--format",
            "svg",
            "--config",
            "settings.toml",
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    same(actual, support::run_with_args(INPUT, &[]));
}

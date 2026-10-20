use super::cli;

#[test]
fn json_only_roles_never_assign_accounts_or_expose_provider_values() {
    let output = cli()
        .args(["tests/fixtures/accounts-plan.json", "-o", "-"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    assert!(!svg.contains("data-account-panel"));
    assert!(!svg.contains("123456789012"));
    assert!(!svg.contains("arn:aws:"));
    assert!(svg.contains("us-east-1"));
}

#[test]
fn evidence_cli_rejects_ambiguous_state_bindings_and_protects_evidence_files() {
    let input = "tests/fixtures/accounts-plan.json";
    for args in [
        vec![input, "--provider-plan"],
        vec![input, "--provider-plan", "-"],
        vec![input, "--provider-plan", input, "--provider-plan", input],
        vec![input, "--state-provider-plan", "a=x"],
        vec!["--state", "a=x", "--provider-plan", input],
        vec!["--state", "a=x", "--state-provider-plan", "b=x"],
        vec![
            "--state",
            "a=x",
            "--state-provider-plan",
            "a=x",
            "--state-provider-plan",
            "a=y",
        ],
        vec![input, "--provider-plan", input, "-o", input],
        vec![
            input,
            "--provider-plan",
            input,
            "--layout-diagnostics-json",
            input,
        ],
    ] {
        let output = cli().args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty());
    }
    assert_eq!(
        std::fs::read(input).unwrap(),
        include_bytes!("../fixtures/accounts-plan.json")
    );
}

#[cfg(unix)]
#[test]
fn terraform_subprocess_errors_are_private_and_temporary_copies_are_removed() {
    use std::os::unix::fs::PermissionsExt;
    let work = tempfile::tempdir().unwrap();
    let binary = work.path().join("terraform");
    std::fs::write(&binary, r#"#!/bin/sh
test -z "${TF_CLI_ARGS+x}${TF_CLI_ARGS_show+x}${TF_LOG+x}${TF_LOG_PATH+x}" || exit 99
echo 'private-role-arn-and-credential' >&2
if [ "$1" = version ]; then
  if [ "$TEST_FAILURE" = version ]; then exit 1; fi
  printf '%s' '{"terraform_version":"1.16.6","provider_selections":{"registry.terraform.io/hashicorp/aws":"5.100.0"}}'
elif [ "$1" = show ]; then
  printf '%s' "$3" > copy-path
  test "$3" != saved.plan || exit 98
  cmp "$3" saved.plan || exit 97
  if [ "$TEST_FAILURE" = show ]; then exit 1; fi
  printf '%s' '{}'
else
  exit 96
fi
"#).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(work.path().join("saved.plan"), b"private-plan-content").unwrap();
    std::fs::write(
        work.path().join("plan.json"),
        r#"{"format_version":"1.2","terraform_version":"1.16.6","resource_changes":[]}"#,
    )
    .unwrap();
    for failure in ["version", "show", "json"] {
        let output = cli()
            .current_dir(work.path())
            .env("PATH", format!("{}:/usr/bin:/bin", work.path().display()))
            .env("TEST_FAILURE", failure)
            .env("TF_CLI_ARGS", "private-arguments")
            .env("TF_CLI_ARGS_show", "private-arguments")
            .env("TF_LOG", "TRACE")
            .env("TF_LOG_PATH", work.path().join("must-not-exist"))
            .args(["plan.json", "--provider-plan", "saved.plan", "-o", "-"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!error.contains("private"));
        assert!(!error.contains(&work.path().display().to_string()));
        assert!(
            error.contains(match failure {
                "version" => "version check failed",
                "show" => "terraform show failed",
                _ => "does not match",
            }),
            "{error}"
        );
        if failure != "version" {
            let copy = std::fs::read_to_string(work.path().join("copy-path")).unwrap();
            assert!(!std::path::Path::new(&copy).exists());
        }
        assert!(!work.path().join("must-not-exist").exists());
        assert_eq!(
            std::fs::read(work.path().join("saved.plan")).unwrap(),
            b"private-plan-content"
        );
    }
}

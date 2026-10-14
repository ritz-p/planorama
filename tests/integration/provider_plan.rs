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

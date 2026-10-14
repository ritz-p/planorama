use super::*;
use std::io::Write;

fn archive(files: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    if !files.iter().any(|(name, _)| *name == ".terraform.lock.hcl") {
        zip.start_file(
            ".terraform.lock.hcl",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(
            b"provider \"registry.terraform.io/hashicorp/aws\" { version = \"5.100.0\" }\n",
        )
        .unwrap();
    }
    for (name, content) in files {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn fixture() -> Value {
    json!({"format_version":"1.2","terraform_version":"1.9.8",
        "resource_changes":[{"address":"module.child[0].aws_vpc.main","type":"aws_vpc"}],
        "configuration":{"provider_config":{"aws.prod":{"full_name":"hashicorp/aws","alias":"prod","expressions":{"region":{"constant_value":"us-east-1"},"assume_role":[{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]}}},
        "root_module":{"module_calls":{"child":{"module":{"resources":[{"address":"aws_vpc.main","provider_config_key":"aws.prod"}]}}}}}})
}

fn snapshot(source: &str) -> Vec<u8> {
    archive(&[
        ("tfplan", "fixture"),
        ("tfconfig/modules.json", r#"[{"Key":""},{"Key":"child"}]"#),
        ("tfconfig/m-/main.tf", source),
        ("tfconfig/m-child/main.tf", ""),
    ])
}

const STATIC: &str = r#"provider "aws" {
  alias = "prod"
  assume_role { role_arn = "arn:aws:iam::123456789012:role/Test" }
}"#;

pub(crate) fn parse_fixture(json: &str) -> Result<TerraformPlan, String> {
    let mut value: Value = serde_json::from_str(json).unwrap();
    value["terraform_version"] = json!("1.9.8");
    let mut modules = BTreeSet::new();
    collect_modules(&value["configuration"]["root_module"], "", &mut modules)?;
    let mut sources: BTreeMap<String, String> = modules
        .iter()
        .map(|key| (key.clone(), String::new()))
        .collect();
    for (key, provider) in value["configuration"]["provider_config"]
        .as_object()
        .unwrap()
    {
        let (module, local) = key.split_once(':').unwrap_or(("", key));
        let module = module
            .split('.')
            .filter(|part| *part != "module")
            .collect::<Vec<_>>()
            .join(".");
        let (local, alias) = local
            .split_once('.')
            .map(|(l, a)| (l, Some(a)))
            .unwrap_or((local, None));
        let source = sources.get_mut(&module).unwrap();
        source.push_str(&format!("provider {local:?} {{\n"));
        if let Some(alias) = alias {
            source.push_str(&format!("alias = {alias:?}\n"));
        }
        for name in ["assume_role", "assume_role_with_web_identity"] {
            for role in provider["expressions"][name]
                .as_array()
                .into_iter()
                .flatten()
            {
                let expression = &role["role_arn"];
                let arn = expression["constant_value"]
                    .as_str()
                    .filter(|_| expression.get("references").is_none());
                source.push_str(&format!(
                    "{name} {{ role_arn = {} }}\n",
                    arn.map(|s| serde_json::to_string(s).unwrap())
                        .unwrap_or("var.unresolved".into())
                ));
            }
        }
        source.push_str("}\n");
    }
    let manifest = serde_json::to_string(
        &modules
            .iter()
            .map(|key| json!({"Key":key}))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut files = vec![
        ("tfplan".to_owned(), "fixture".to_owned()),
        ("tfconfig/modules.json".into(), manifest),
    ];
    files.extend(
        sources
            .into_iter()
            .map(|(key, source)| (format!("tfconfig/m-{key}/main.tf"), source)),
    );
    let archive = archive(
        &files
            .iter()
            .map(|(name, source)| (name.as_str(), source.as_str()))
            .collect::<Vec<_>>(),
    );
    let json = value.to_string();
    let mut plan = crate::plan::parse(&json)?;
    Evidence::validate(&archive, &json, json.as_bytes())?.apply(&mut plan);
    Ok(plan)
}

#[test]
fn verified_static_snapshot_resolves_exact_module_binding_only() {
    let value = fixture();
    let json = value.to_string();
    let mut plan = crate::plan::parse(&json).unwrap();
    assert!(
        plan.nodes[0]
            .provider_configuration
            .as_ref()
            .unwrap()
            .account
            .is_none()
    );
    Evidence::validate(&snapshot(STATIC), &json, json.as_bytes())
        .unwrap()
        .apply(&mut plan);
    let binding = plan.nodes[0].provider_configuration.as_ref().unwrap();
    assert_eq!(binding.account.as_ref().unwrap().as_str(), "123456789012");
    assert_eq!(binding.key, "aws.prod");
    assert_eq!(binding.region.as_ref().unwrap().0, "us-east-1");
    assert!(!format!("{plan:?}").contains("arn:aws:"));
}

#[test]
fn omitted_dynamic_and_unresolved_roles_never_prove_an_account() {
    let json = fixture().to_string();
    for source in [
        STATIC.replace("  alias", "  dynamic \"assume_role\" {\n for_each = var.roles\n content { role_arn = assume_role.value }\n }\n  alias"),
        STATIC.replace("\"arn:aws:iam::123456789012:role/Test\"", "var.role"),
        STATIC.replace("123456789012", "210987654321"),
        STATIC.replace("prod", "other"),
    ] {
        let mut plan = crate::plan::parse(&json).unwrap();
        Evidence::validate(&snapshot(&source), &json, json.as_bytes()).unwrap().apply(&mut plan);
        assert!(plan.nodes[0].provider_configuration.as_ref().unwrap().account.is_none());
    }
}

#[test]
fn stale_missing_modules_and_overrides_are_rejected() {
    let json = fixture().to_string();
    assert!(Evidence::validate(&snapshot(STATIC), &json, b"{}").is_err());
    for files in [
        vec![
            ("tfplan", "x"),
            ("tfconfig/modules.json", r#"[{"Key":""}]"#),
            ("tfconfig/m-/main.tf", STATIC),
        ],
        vec![
            ("tfplan", "x"),
            ("tfconfig/modules.json", r#"[{"Key":""},{"Key":"child"}]"#),
            ("tfconfig/m-/override.tf", STATIC),
            ("tfconfig/m-child/main.tf", ""),
        ],
    ] {
        assert!(Evidence::validate(&archive(&files), &json, json.as_bytes()).is_err());
    }
}

#[test]
fn json_static_and_dynamic_provider_declarations() {
    let good = br#"{"provider":{"aws":[{"alias":"prod","assume_role":[{"role_arn":"arn:aws:iam::123456789012:role/Test"}]}]}}"#;
    assert!(json_providers(good).unwrap()[0].2.is_some());
    let dynamic = br#"{"provider":{"aws":{"assume_role":{"role_arn":"arn:aws:iam::123456789012:role/Test"},"dynamic":{"assume_role":{"for_each":"${var.roles}"}}}}}"#;
    assert!(json_providers(dynamic).unwrap()[0].2.is_none());
}

#[test]
fn complete_chains_conflicts_and_state_local_evidence_are_independent() {
    let mut input = fixture();
    let first = parse_fixture(&input.to_string()).unwrap();
    input["configuration"]["provider_config"]["aws.prod"]["expressions"]["assume_role"][0]["role_arn"]
        ["constant_value"] = json!("arn:aws:iam::210987654321:role/Test");
    let second = parse_fixture(&input.to_string()).unwrap();
    assert_ne!(
        first.nodes[0]
            .provider_configuration
            .as_ref()
            .unwrap()
            .account,
        second.nodes[0]
            .provider_configuration
            .as_ref()
            .unwrap()
            .account
    );
    input["configuration"]["provider_config"]["aws.prod"]["expressions"]["assume_role"]
        .as_array_mut()
        .unwrap()
        .push(json!({"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}));
    let conflict = parse_fixture(&input.to_string()).unwrap();
    let binding = conflict.nodes[0].provider_configuration.as_ref().unwrap();
    assert!(binding.account.is_none());
    assert!(binding.account_conflict);
    input["configuration"]["provider_config"]["aws.prod"]["expressions"]["assume_role"][0]["role_arn"]
        ["constant_value"] = json!("arn:aws:iam::123456789012:role/First");
    let same = parse_fixture(&input.to_string()).unwrap();
    assert_eq!(
        same.nodes[0]
            .provider_configuration
            .as_ref()
            .unwrap()
            .account
            .as_ref()
            .unwrap()
            .as_str(),
        "123456789012"
    );
}

#[test]
fn module_local_provider_does_not_borrow_root_alias() {
    let mut input = fixture();
    let provider = input["configuration"]["provider_config"]["aws.prod"].clone();
    input["configuration"]["provider_config"]["module.child:aws.prod"] = provider;
    input["configuration"]["provider_config"]["module.child:aws.prod"]["expressions"]["assume_role"]
        [0]["role_arn"]["constant_value"] = json!("arn:aws:iam::210987654321:role/Child");
    input["configuration"]["root_module"]["module_calls"]["child"]["module"]["resources"][0]["provider_config_key"] =
        json!("module.child:aws.prod");
    let parsed = parse_fixture(&input.to_string()).unwrap();
    let binding = parsed.nodes[0].provider_configuration.as_ref().unwrap();
    assert_eq!(binding.key, "module.child:aws.prod");
    assert_eq!(binding.account.as_ref().unwrap().as_str(), "210987654321");
}

#[test]
fn unexamined_provider_version_cannot_establish_completeness() {
    let json = fixture().to_string();
    let files = [
        ("tfplan", "fixture"),
        (
            ".terraform.lock.hcl",
            "provider \"registry.terraform.io/hashicorp/aws\" { version = \"6.0.0\" }",
        ),
        ("tfconfig/modules.json", r#"[{"Key":""},{"Key":"child"}]"#),
        ("tfconfig/m-/main.tf", STATIC),
        ("tfconfig/m-child/main.tf", ""),
    ];
    let error = Evidence::validate(&archive(&files), &json, json.as_bytes())
        .err()
        .unwrap();
    assert!(error.contains("5.100.0"));
}

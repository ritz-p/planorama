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
        if *name == "tfplan" && *content == "fixture" {
            zip.write_all(b"\x08\x03\x72\x051.9.8").unwrap();
        } else {
            zip.write_all(content.as_bytes()).unwrap();
        }
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
            ("tfplan", "fixture"),
            ("tfconfig/modules.json", r#"[{"Key":""}]"#),
            ("tfconfig/m-/main.tf", STATIC),
        ],
        vec![
            ("tfplan", "fixture"),
            ("tfconfig/modules.json", r#"[{"Key":""},{"Key":"child"}]"#),
            ("tfconfig/m-/override.tf", STATIC),
            ("tfconfig/m-child/main.tf", ""),
        ],
        vec![
            ("tfplan", "fixture"),
            (
                "tfconfig/modules.json",
                r#"[{"Key":""},{"Key":""},{"Key":"child"}]"#,
            ),
            ("tfconfig/m-/main.tf", STATIC),
            ("tfconfig/m-child/main.tf", ""),
        ],
        vec![
            ("tfplan", "fixture"),
            ("tfconfig/modules.json", r#"[{"Key":""},{"Key":"child"}]"#),
            ("tfconfig/m-/main.tf", STATIC),
        ],
        vec![
            ("tfplan", "fixture"),
            ("tfconfig/modules.json", r#"[{"Key":""},{"Key":"child"}]"#),
            ("tfconfig/m-/main.tf", "provider \"aws\" {"),
            ("tfconfig/m-child/main.tf", ""),
        ],
    ] {
        assert!(Evidence::validate(&archive(&files), &json, json.as_bytes()).is_err());
    }
    let mut duplicate = archive(&[
        ("tfplan", "fixture"),
        ("tfconfig/modules.json", r#"[{"Key":""},{"Key":"child"}]"#),
        ("tfconfig/m-/main.tf", STATIC),
        ("tfconfig/m-/else.tf", ""),
        ("tfconfig/m-child/main.tf", ""),
    ]);
    for index in 0..duplicate.len() - 7 {
        if &duplicate[index..index + 7] == b"else.tf" {
            duplicate[index..index + 7].copy_from_slice(b"main.tf");
        }
    }
    assert!(Evidence::validate(&duplicate, &json, json.as_bytes()).is_err());
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

#[test]
fn core_binary_archive_json_and_lock_are_independently_checked() {
    for core in ["1.9.8", "1.16.6"] {
        let mut input = fixture();
        input["terraform_version"] = json!(core);
        let json = input.to_string();
        let header = format!("\u{8}\u{3}r{}{core}", char::from(core.len() as u8));
        let saved = archive(&[
            ("tfplan", &header),
            ("tfconfig/modules.json", r#"[{"Key":""},{"Key":"child"}]"#),
            ("tfconfig/m-/main.tf", STATIC),
            ("tfconfig/m-child/main.tf", ""),
        ]);
        let version = json!({"terraform_version":core,"provider_selections":{"registry.terraform.io/hashicorp/aws":"5.100.0"}});
        Evidence::validate_runtime(&json, &serde_json::to_vec(&version).unwrap()).unwrap();
        let evidence = Evidence::validate(&saved, &json, json.as_bytes()).unwrap();
        let mut parsed = crate::plan::parse(&json).unwrap();
        evidence.apply(&mut parsed);
        assert_eq!(
            parsed.nodes[0]
                .provider_configuration
                .as_ref()
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .as_str(),
            "123456789012"
        );
        let mut unrelated = input.clone();
        unrelated["output_changes"] = json!({"unrelated":{"actions":["create"],"after":"private"}});
        assert!(
            Evidence::validate(&saved, &json, &serde_json::to_vec(&unrelated).unwrap()).is_err()
        );
        Evidence::validate(&saved, &json, &serde_json::to_vec_pretty(&input).unwrap()).unwrap();
        for wrong in ["1.9.7", "1.16.5", "secret-value"] {
            let mut version = version.clone();
            version["terraform_version"] = json!(wrong);
            let error = Evidence::validate_runtime(&json, &serde_json::to_vec(&version).unwrap())
                .unwrap_err();
            assert!(!error.contains(wrong));
            assert!(validate_header(header.as_bytes(), wrong).is_none());
        }
        for provider in ["6.0.0", "5.99.0", "secret-value"] {
            let mut version = version.clone();
            version["provider_selections"]["registry.terraform.io/hashicorp/aws"] = json!(provider);
            assert!(
                Evidence::validate_runtime(&json, &serde_json::to_vec(&version).unwrap()).is_err()
            );
        }
    }
    for header in [
        b"".as_slice(),
        b"\x08\x04\x72\x051.9.8",
        b"\x08\x03",
        b"\x08\x03\x72\x051.9.8\x72\x051.9.8",
        b"\x08\x03\x72\x051.9.8\x08\x03",
        b"\x08\x03\x72\xff",
        b"\x08\x03\x72\x051.9.8\x00",
        b"\x08\x03\x72\x051.9.8\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff",
    ] {
        assert!(validate_header(header, "1.9.8").is_none());
    }
    let mut unsupported = fixture();
    unsupported["terraform_version"] = json!("unverified-private-version");
    let json = unsupported.to_string();
    let error = Evidence::validate(&snapshot(STATIC), &json, json.as_bytes())
        .err()
        .unwrap();
    assert!(error.contains("unsupported Terraform"));
    assert!(!error.contains("unverified-private-version"));
    assert!(Evidence::validate_runtime(&json, br#"{"terraform_version":"unverified-private-version","provider_selections":{"registry.terraform.io/hashicorp/aws":"5.100.0"}}"#).is_err());
    assert!(
        Evidence::validate_runtime(&fixture().to_string(), b"private-invalid-response").is_err()
    );
}

#[test]
fn evidence_keeps_data_current_deposed_and_removed_objects_distinct() {
    let mut input = fixture();
    input["configuration"]["root_module"]["module_calls"]["child"]["module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(
            json!({"address":"data.aws_vpc.main","mode":"data","provider_config_key":"aws.prod"}),
        );
    input["resource_changes"].as_array_mut().unwrap().extend([
        json!({"address":"module.child[0].data.aws_vpc.main","mode":"data","type":"aws_vpc","change":{"actions":["read"]}}),
        json!({"address":"module.child[0].aws_vpc.main","type":"aws_vpc","deposed":"old","change":{"actions":["delete"]}}),
        json!({"address":"module.child[0].aws_vpc.removed","type":"aws_vpc","change":{"actions":["delete"]}}),
    ]);
    let parsed = parse_fixture(&input.to_string()).unwrap();
    assert_eq!(parsed.nodes.len(), 4);
    for node in &parsed.nodes {
        let account = node
            .provider_configuration
            .as_ref()
            .and_then(|binding| binding.account.as_ref());
        assert_eq!(
            account.is_some(),
            node.deposed_key.is_none() && !node.address.ends_with(".removed"),
            "{}",
            node.address
        );
    }
}

#[test]
fn entire_provider_and_nested_authentication_must_be_known_literals() {
    assert!(
        hcl_providers(include_bytes!("../../fixtures/provider-plan/main.tf")).unwrap()[0]
            .2
            .is_some()
    );
    for extra in [
        "profile = var.profile",
        "region = \"${var.region}\"",
        "new_authentication = \"secret\"",
        "unknown_role { role_arn = \"arn:aws:iam::123456789012:role/Test\" }",
        "endpoints { sts = var.endpoint }",
        "default_tags { tags = { Name = var.name } }",
        "assume_role {\n role_arn = \"arn:aws:iam::123456789012:role/Test\"\n external_id = var.id\n }",
        "assume_role {\n role_arn = \"arn:aws:iam::123456789012:role/Test\"\n unknown_auth = \"secret\"\n }",
        "assume_role {\n role_arn = \"arn:aws:iam::123456789012:role/Test\"\n nested {}\n }",
    ] {
        let source = STATIC.replace("  alias", &format!("  {extra}\n  alias"));
        assert!(
            hcl_providers(source.as_bytes()).unwrap()[0].2.is_none(),
            "{extra}"
        );
    }
    for extra in [
        json!({"profile":"${var.profile}"}),
        json!({"new_authentication":"secret"}),
        json!({"endpoints":{"sts":"${var.endpoint}"}}),
        json!({"assume_role":{"role_arn":"arn:aws:iam::123456789012:role/Test","new_auth":"secret"}}),
    ] {
        let mut declaration =
            json!({"assume_role":{"role_arn":"arn:aws:iam::123456789012:role/Test"}});
        declaration
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let body = json!({"provider":{"aws":declaration}});
        assert!(
            json_providers(&serde_json::to_vec(&body).unwrap()).unwrap()[0]
                .2
                .is_none()
        );
    }
    let literal = STATIC.replace("  alias", "  profile = \"local\"\n default_tags { tags = { Name = \"test\" } }\n endpoints { sts = \"http://127.0.0.1:1234\" }\n  alias");
    assert!(hcl_providers(literal.as_bytes()).unwrap()[0].2.is_some());
}

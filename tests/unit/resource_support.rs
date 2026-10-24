use std::{collections::BTreeMap, fmt::Write, fs, path::Path};

use crate::{icons, model, plan, provider, semantic};

fn sources(path: &Path, result: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, result);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            result.insert(
                path.to_str().unwrap().replace('\\', "/"),
                fs::read_to_string(path).unwrap(),
            );
        }
    }
}

fn types(source: &str) -> Vec<String> {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Types(Vec<String>);
    impl<'ast> Visit<'ast> for Types {
        fn visit_lit_str(&mut self, literal: &'ast syn::LitStr) {
            let value = literal.value();
            if (value.starts_with("aws_") || value.starts_with("terraform_"))
                && !value.ends_with('_')
                && value
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            {
                self.0.push(value);
            }
        }

        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            fn tokens(stream: proc_macro2::TokenStream, types: &mut Types) {
                for token in stream {
                    match token {
                        proc_macro2::TokenTree::Group(group) => tokens(group.stream(), types),
                        proc_macro2::TokenTree::Literal(literal) => {
                            if let Ok(literal) = syn::parse_str::<syn::LitStr>(&literal.to_string())
                            {
                                types.visit_lit_str(&literal);
                            }
                        }
                        _ => {}
                    }
                }
            }
            tokens(node.tokens.clone(), self);
        }
    }
    let mut types = Types::default();
    types.visit_file(&syn::parse_file(source).unwrap());
    types.0
}

fn matrix() -> String {
    let inventory: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/aws-inventory/6.70.0.json")).unwrap();
    let baseline: BTreeMap<_, _> = inventory["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| (entry["type"].as_str().unwrap(), entry))
        .collect();
    let mut files = BTreeMap::new();
    sources(Path::new("src/provider"), &mut files);
    let remote = "src/plan/remote.rs";
    files.insert(remote.into(), fs::read_to_string(remote).unwrap());
    let mut catalog = BTreeMap::<String, Vec<&str>>::new();
    for (path, source) in &files {
        for resource_type in types(source) {
            let paths = catalog.entry(resource_type).or_default();
            if !paths.contains(&path.as_str()) {
                paths.push(path);
            }
        }
    }
    let mut output = String::from(include_str!("resource_support_intro.txt")).replace("\r\n", "\n");
    for (source, prefix, title) in [
        (
            "registry.terraform.io/hashicorp/aws",
            "aws_",
            "HashiCorp AWS",
        ),
        (
            "terraform.io/builtin/terraform",
            "terraform_",
            "Terraform built-in",
        ),
    ] {
        writeln!(output, "\n## {title}\n\nProvider source: `{source}`.\n").unwrap();
        if prefix == "aws_" {
            writeln!(output, "Baseline: AWS Provider **{}**, commit `{}`; **{} managed resource types**. [Inventory and update procedure](aws-inventory.md). Counts describe types, not input instances.\n", inventory["version"].as_str().unwrap(), inventory["commit"].as_str().unwrap(), baseline.len()).unwrap();
        }
        output.push_str("| Resource type | Role (managed / data) | Display (managed / data) | Icon / service name | Deployment scope | Baseline / basic display notes | Conditional rule source references |\n| --- | --- | --- | --- | --- | --- | --- |\n");
        for (resource_type, paths) in &catalog {
            if !resource_type.starts_with(prefix) {
                continue;
            }
            let input = serde_json::json!({"format_version":"1.2", "resource_changes": [
                {"address":format!("{resource_type}.example"), "type":resource_type,
                 "provider_name":source, "mode":"managed", "change":{"actions":["create"]}},
                {"address":format!("data.{resource_type}.example"), "type":resource_type,
                 "provider_name":source, "mode":"data", "change":{"actions":["read"]}}
            ]});
            let mut plan = plan::parse(&input.to_string()).unwrap();
            for entity in &mut plan.nodes {
                entity.provider_configuration = Some(model::ProviderConfiguration {
                    key: "example".into(),
                    alias: None,
                    region: Some(model::RegionScope("us-east-1".into())),
                    account: None,
                    account_conflict: false,
                });
            }
            let entity = &plan.nodes[0];
            let scope = match provider::scope(entity) {
                _ if matches!(
                    resource_type.as_str(),
                    "aws_wafv2_web_acl" | "aws_wafv2_rule_group"
                ) =>
                {
                    "Global / Regional / Unknown (scope-dependent)"
                }
                Some(scope) if scope.global => "Global",
                Some(scope) if scope.region.is_some() => "Regional (conditional)",
                _ => "Unknown",
            };
            let graph = super::data::visible(semantic::base_graph(&plan));
            let display = |mode| {
                graph
                    .nodes
                    .iter()
                    .find(|node| node.mode == mode)
                    .map_or("Omitted", |node| match node.placement {
                        model::Placement::Logical => "Logical",
                        model::Placement::Spatial => "Spatial",
                        model::Placement::Unresolved => "Spatial (unresolved)",
                    })
            };
            let role = provider::classify(&entity.provider, resource_type);
            let data_role = graph
                .nodes
                .iter()
                .find(|node| node.mode == model::EntityMode::Data)
                .map_or(role, |node| node.role);
            let icon = icons::icon_for(
                icons::Provider::from_identity(&entity.provider),
                resource_type,
            )
            .map_or_else(
                || {
                    super::service_name(resource_type).map_or_else(
                        || "—".into(),
                        |name| format!("Neutral card / {name} (no matching service artwork in package 07312026)"),
                    )
                },
                |icon| {
                    format!(
                        "{icon:?} / {}",
                        super::service_name(resource_type).unwrap_or(icon.service_name())
                    )
                },
            );
            let canonical = super::aliases::canonical(resource_type);
            let paths = catalog.get(canonical).unwrap_or(paths);
            let rules: Vec<_> = paths
                .iter()
                .filter(|path| {
                    !matches!(
                        path.rsplit('/').next().unwrap(),
                        "classification.rs"
                            | "scope.rs"
                            | "icons.rs"
                            | "data.rs"
                            | "terraform.rs"
                            | "basic_entries.rs"
                            | "basic.rs"
                            | "aliases.rs"
                    )
                })
                .map(|path| {
                    let label = path
                        .strip_prefix("src/provider/")
                        .unwrap_or_else(|| path.strip_prefix("src/").unwrap());
                    format!("[{label}](../{path})")
                })
                .collect();
            let mut notes = Vec::new();
            if let Some(entry) = baseline.get(resource_type.as_str()) {
                notes.push(format!("[Managed resource](https://github.com/hashicorp/terraform-provider-aws/blob/{}/{})", inventory["commit"].as_str().unwrap(), entry["documentation"].as_str().unwrap()));
                if entry["dataSource"] == true {
                    notes.push("Also a registered data source".into());
                }
                if entry["deprecated"] == true {
                    notes.push("Deprecated".into());
                }
                if let Some(alias) = entry["aliasOf"].as_str() {
                    notes.push(format!("Verified managed alias of `{alias}`; [mode limits and evidence](aws-inventory.md#verified-resource-aliases)"));
                }
                if scope == "Unknown" {
                    notes.push(format!(
                        "Scope unknown: {}",
                        super::basic::unknown_scope_reason(resource_type)
                            .unwrap_or("Missing verified scope evidence")
                    ));
                }
                if super::classification::classify(canonical) == model::ResourceRole::Unknown {
                    notes.push(if super::icons::icon_for(resource_type).is_some() {
                        "Previously icon-only / incomplete basic classification".into()
                    } else {
                        "Previously generic fallback".into()
                    });
                    notes.push(
                        "Basic display only; no new dedicated relationship or verified owner"
                            .into(),
                    );
                } else {
                    notes.push("Existing classification and presentation retained".into());
                }
            } else if prefix == "aws_" {
                notes.push(
                    "Outside managed baseline (data-only or historical implementation type)".into(),
                );
            }
            writeln!(
                output,
                "| `{resource_type}` | {role:?} / {data_role:?} | {} / {} | {icon} | {scope} | {} | {} |",
                display(model::EntityMode::Managed),
                display(model::EntityMode::Data),
                notes.join("; "),
                if rules.is_empty() {
                    "—".into()
                } else {
                    rules.join(", ")
                }
            )
            .unwrap();
        }
    }
    output
}

#[test]
fn discovers_type_literals_in_rust_expressions_and_macros_only() {
    let source = r##"
        // "aws_comment" "a quote: \""
        fn rule(value: &str) -> bool {
            let _ = "an escaped quote: \"";
            matches!(value, "aws_vpc" | r#"aws_subnet"#)
                || value == "terraform_data"
                || value.starts_with("aws_iam_")
        }
    "##;
    assert_eq!(types(source), ["aws_vpc", "aws_subnet", "terraform_data"]);
}

#[test]
fn generated_resource_support_is_current() {
    let generated = matrix();
    let path = Path::new("docs/resource-support.md");
    if std::env::var_os("UPDATE_RESOURCE_SUPPORT").as_deref() == Some(std::ffi::OsStr::new("1")) {
        fs::write(path, &generated).unwrap();
    }
    assert_eq!(
        fs::read_to_string(path).unwrap().replace("\r\n", "\n"),
        generated,
        "Run sh scripts/update-resource-support.sh inside the development container"
    );
}

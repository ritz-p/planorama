use crate::model::{AwsAccountScope, ProviderIdentity, TerraformPlan};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

#[cfg(test)]
#[path = "../../tests/unit/plan/evidence.rs"]
pub(crate) mod tests;

pub(crate) struct Evidence(BTreeMap<String, (Option<AwsAccountScope>, bool)>);

const VERIFIED_PAIRS: &[(&str, &str)] = &[("1.9.8", "5.100.0"), ("1.16.6", "5.100.0")];

impl Evidence {
    pub(crate) fn validate_runtime(input: &str, version: &[u8]) -> Result<(), String> {
        let input: Value = serde_json::from_str(input.trim_start_matches('\u{feff}'))
            .map_err(|_| "provider plan: invalid input JSON")?;
        let version: Value = serde_json::from_slice(version)
            .map_err(|_| "provider plan: invalid Terraform version response")?;
        let core = input["terraform_version"].as_str().unwrap_or("");
        let provider = version["provider_selections"]["registry.terraform.io/hashicorp/aws"]
            .as_str()
            .unwrap_or("");
        if !VERIFIED_PAIRS.contains(&(core, provider)) {
            return Err("provider plan: unsupported Terraform / installed AWS Provider pair; see docs/provider-evidence.md".into());
        }
        if version["terraform_version"] != input["terraform_version"] {
            return Err("provider plan: Terraform binary version does not match input JSON".into());
        }
        Ok(())
    }

    pub(crate) fn validate(archive: &[u8], input: &str, shown: &[u8]) -> Result<Self, String> {
        let input: Value = serde_json::from_str(input.trim_start_matches('\u{feff}'))
            .map_err(|_| "provider plan: invalid input JSON")?;
        let shown: Value = serde_json::from_slice(shown)
            .map_err(|_| "provider plan: invalid terraform show JSON")?;
        if input != shown {
            return Err("provider plan does not match input plan JSON".into());
        }
        let core = input["terraform_version"].as_str().unwrap_or("");
        if !VERIFIED_PAIRS.iter().any(|(version, _)| *version == core) {
            return Err("provider plan: unsupported Terraform archive version".into());
        }
        Self::snapshot(archive, &input["configuration"], core)
    }

    fn snapshot(archive: &[u8], configuration: &Value, core: &str) -> Result<Self, String> {
        let invalid = || "provider plan: invalid or incomplete configuration snapshot".to_owned();
        let mut zip = zip::ZipArchive::new(Cursor::new(archive)).map_err(|_| invalid())?;
        if central_entry_count(archive, zip.central_directory_start()) != Some(zip.len()) {
            return Err(invalid());
        }
        let mut files = BTreeMap::new();
        let mut total = 0u64;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|_| invalid())?;
            let name = entry.name().to_owned();
            if files.contains_key(&name) || entry.size() > 64 * 1024 * 1024 {
                return Err(invalid());
            }
            total = total.checked_add(entry.size()).ok_or_else(invalid)?;
            if total > 128 * 1024 * 1024 {
                return Err(invalid());
            }
            let mut bytes = Vec::new();
            (&mut entry)
                .take(64 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| invalid())?;
            if bytes.len() as u64 != entry.size() {
                return Err(invalid());
            }
            files.insert(name, bytes);
        }
        validate_header(files.get("tfplan").ok_or_else(invalid)?, core).ok_or(
            "provider plan: archive version does not match input JSON or supported format",
        )?;
        let locks = std::str::from_utf8(files.get(".terraform.lock.hcl").ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        let locks = hcl::parse(locks).map_err(|_| invalid())?;
        let aws: Vec<_> = locks
            .blocks()
            .filter(|block| {
                block.identifier() == "provider"
                    && block.labels.len() == 1
                    && block.labels[0].as_str() == "registry.terraform.io/hashicorp/aws"
            })
            .collect();
        let supported = match aws.as_slice() {
            [provider] => {
                let versions: Vec<_> = provider
                    .body
                    .attributes()
                    .filter(|a| a.key() == "version")
                    .collect();
                matches!(versions.as_slice(), [version] if matches!(&version.expr,
                    hcl::Expression::String(provider) if VERIFIED_PAIRS.contains(&(core, provider.as_str()))))
            }
            _ => false,
        };
        if !supported {
            return Err("provider plan evidence requires hashicorp/aws 5.100.0 in the embedded dependency lock".into());
        }
        let manifest: Value =
            serde_json::from_slice(files.get("tfconfig/modules.json").ok_or_else(invalid)?)
                .map_err(|_| invalid())?;
        let mut modules = BTreeSet::new();
        for module in manifest.as_array().ok_or_else(invalid)? {
            if !modules.insert(module["Key"].as_str().ok_or_else(invalid)?.to_owned()) {
                return Err(invalid());
            }
        }
        let mut expected = BTreeSet::new();
        collect_modules(&configuration["root_module"], "", &mut expected)?;
        if modules != expected {
            return Err(invalid());
        }
        let mut declarations = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for (path, bytes) in &files {
            let Some(path) = path.strip_prefix("tfconfig/m-") else {
                continue;
            };
            let (module, name) = path.split_once('/').ok_or_else(invalid)?;
            if !modules.contains(module) || name.contains('/') || name.contains('\\') {
                return Err(invalid());
            }
            seen.insert(module.to_owned());
            if name == "override.tf"
                || name == "override.tf.json"
                || name.ends_with("_override.tf")
                || name.ends_with("_override.tf.json")
            {
                return Err(
                    "provider plan: override files are not supported as account evidence".into(),
                );
            }
            let providers = if name.ends_with(".tf.json") {
                json_providers(bytes)?
            } else if name.ends_with(".tf") {
                hcl_providers(bytes)?
            } else {
                return Err(invalid());
            };
            for (local, alias, expressions) in providers {
                let prefix = if module.is_empty() {
                    String::new()
                } else {
                    format!(
                        "{}:",
                        module
                            .split('.')
                            .map(|part| format!("module.{part}"))
                            .collect::<Vec<_>>()
                            .join(".")
                    )
                };
                let key = format!(
                    "{prefix}{local}{}",
                    alias.map(|a| format!(".{a}")).unwrap_or_default()
                );
                if declarations.insert(key, expressions).is_some() {
                    return Err("provider plan: duplicate provider declaration".into());
                }
            }
        }
        if seen != modules {
            return Err(invalid());
        }
        let mut accounts = BTreeMap::new();
        for (key, provider) in configuration["provider_config"]
            .as_object()
            .ok_or_else(invalid)?
        {
            let identity =
                ProviderIdentity::from_source(provider["full_name"].as_str().unwrap_or("unknown"));
            let Some(Some(expressions)) = declarations.get(key) else {
                continue;
            };
            let agrees = ["assume_role", "assume_role_with_web_identity"]
                .iter()
                .all(|name| {
                    role_targets(&expressions[name]) == role_targets(&provider["expressions"][name])
                });
            if agrees {
                accounts.insert(
                    key.clone(),
                    super::providers::static_account(&identity, expressions),
                );
            }
        }
        Ok(Self(accounts))
    }

    pub(crate) fn apply(&self, plan: &mut TerraformPlan) {
        for node in &mut plan.nodes {
            if node.deposed_key.is_some() || !node.provider.is_aws() {
                continue;
            }
            if let Some(binding) = &mut node.provider_configuration {
                if let Some((account, conflict)) = self.0.get(&binding.key) {
                    binding.account = account.clone();
                    binding.account_conflict = *conflict;
                }
            }
        }
    }
}

fn central_entry_count(archive: &[u8], offset: u64) -> Option<usize> {
    let mut bytes = archive.get(usize::try_from(offset).ok()?..)?;
    let mut count = 0;
    while bytes.starts_with(b"PK\x01\x02") {
        let header = bytes.get(..46)?;
        let length = [28, 30, 32].iter().fold(46usize, |size, &index| {
            size + usize::from(u16::from_le_bytes([header[index], header[index + 1]]))
        });
        bytes = bytes.get(length..)?;
        count += 1;
    }
    (bytes.starts_with(b"PK\x05\x06") || bytes.starts_with(b"PK\x06\x06")).then_some(count)
}

fn validate_header(mut bytes: &[u8], core: &str) -> Option<()> {
    fn varint(bytes: &mut &[u8]) -> Option<u64> {
        let mut value = 0;
        for shift in (0..70).step_by(7) {
            let (&byte, rest) = bytes.split_first()?;
            *bytes = rest;
            if shift == 63 && byte > 1 {
                return None;
            }
            value |= u64::from(byte & 127) << shift;
            if byte & 128 == 0 {
                return Some(value);
            }
        }
        None
    }
    let mut format = None;
    let mut version = None;
    while !bytes.is_empty() {
        let tag = varint(&mut bytes)?;
        let field = tag >> 3;
        let wire = tag & 7;
        if field == 0 || field > 0x1fff_ffff {
            return None;
        }
        match (field, wire) {
            (1, 0) if format.is_none() => format = Some(varint(&mut bytes)?),
            (14, 2) if version.is_none() => {
                let len = usize::try_from(varint(&mut bytes)?).ok()?;
                version = Some(std::str::from_utf8(bytes.get(..len)?).ok()?);
                bytes = bytes.get(len..)?;
            }
            (1 | 14, _) => return None,
            (_, 0) => {
                varint(&mut bytes)?;
            }
            (_, 1) => bytes = bytes.get(8..)?,
            (_, 2) => {
                let len = usize::try_from(varint(&mut bytes)?).ok()?;
                bytes = bytes.get(len..)?;
            }
            (_, 5) => bytes = bytes.get(4..)?,
            _ => return None,
        }
    }
    (format == Some(3) && version == Some(core)).then_some(())
}

fn collect_modules(module: &Value, key: &str, found: &mut BTreeSet<String>) -> Result<(), String> {
    if !module.is_object() {
        return Err("provider plan: missing module configuration".into());
    }
    found.insert(key.into());
    if let Some(calls) = module.get("module_calls") {
        for (name, call) in calls
            .as_object()
            .ok_or("provider plan: invalid module calls")?
        {
            collect_modules(
                &call["module"],
                &if key.is_empty() {
                    name.clone()
                } else {
                    format!("{key}.{name}")
                },
                found,
            )?;
        }
    }
    Ok(())
}

type Declaration = (String, Option<String>, Option<Value>);

fn role_targets(blocks: &Value) -> Vec<Value> {
    blocks
        .as_array()
        .into_iter()
        .flatten()
        .map(|block| block["role_arn"].clone())
        .collect()
}

fn literal(value: &str) -> bool {
    !value.contains("${") && !value.contains("%{")
}

fn literal_expression(expression: &hcl::Expression) -> bool {
    use hcl::Expression;
    match expression {
        Expression::Null | Expression::Bool(_) | Expression::Number(_) => true,
        Expression::String(value) => literal(value),
        Expression::Array(items) => items.iter().all(literal_expression),
        Expression::Object(items) => items.iter().all(|(key, value)| {
            let key = match key {
                hcl::expr::ObjectKey::Identifier(_) => true,
                hcl::expr::ObjectKey::Expression(Expression::String(key)) => literal(key),
                _ => false,
            };
            key && literal_expression(value)
        }),
        _ => false,
    }
}

fn literal_body(body: &hcl::Body) -> Option<Value> {
    let mut value = serde_json::Map::new();
    for attribute in body.attributes() {
        if !literal_expression(&attribute.expr) {
            return None;
        }
        let literal = serde_json::to_value(hcl::Value::from(attribute.expr.clone())).ok()?;
        if value.insert(attribute.key().into(), literal).is_some() {
            return None;
        }
    }
    let mut blocks = BTreeSet::new();
    for block in body.blocks() {
        if !block.labels.is_empty()
            || (value.contains_key(block.identifier()) && !blocks.contains(block.identifier()))
        {
            return None;
        }
        blocks.insert(block.identifier());
        value
            .entry(block.identifier())
            .or_insert_with(|| json!([]))
            .as_array_mut()?
            .push(literal_body(&block.body)?);
    }
    Some(Value::Object(value))
}

fn matches_type(value: &Value, kind: &Value) -> bool {
    match kind.as_str() {
        Some("string") => {
            value.as_str().is_some_and(literal) || value.is_boolean() || value.is_number()
        }
        Some("bool") => value.is_boolean(),
        Some("number") => value.is_number(),
        _ => match kind[0].as_str() {
            Some("list" | "set") => value
                .as_array()
                .is_some_and(|items| items.iter().all(|item| matches_type(item, &kind[1]))),
            Some("map") => value.as_object().is_some_and(|items| {
                items
                    .iter()
                    .all(|(key, item)| literal(key) && matches_type(item, &kind[1]))
            }),
            _ => false,
        },
    }
}

fn matches_schema(value: &Value, schema: &Value, provider: bool) -> bool {
    value.as_object().is_some_and(|items| {
        items.iter().all(|(key, value)| {
            if provider && key == "alias" {
                return value.as_str().is_some_and(literal);
            }
            if let Some(kind) = schema["attributes"].get(key) {
                return matches_type(value, kind);
            }
            let Some(block) = schema["blocks"].get(key) else {
                return false;
            };
            match value {
                Value::Array(items) => items.iter().all(|item| matches_schema(item, block, false)),
                Value::Object(_) => matches_schema(value, block, false),
                _ => false,
            }
        })
    })
}

fn complete_provider(value: &Value) -> bool {
    static SCHEMA: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    let schema = SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!("evidence/aws-5.100.0.json"))
            .expect("embedded provider schema")
    });
    matches_schema(value, schema, true)
}

fn hcl_providers(bytes: &[u8]) -> Result<Vec<Declaration>, String> {
    let source =
        std::str::from_utf8(bytes).map_err(|_| "provider plan: invalid configuration encoding")?;
    let body = hcl::parse(source).map_err(|_| "provider plan: invalid HCL configuration")?;
    let mut result = Vec::new();
    for block in body
        .blocks()
        .filter(|block| block.identifier() == "provider")
    {
        if block.labels.len() != 1 {
            return Err("provider plan: invalid provider declaration".into());
        }
        let local = block.labels[0].as_str().to_owned();
        let mut alias = None;
        let mut expressions = json!({});
        let mut complete = literal_body(&block.body)
            .as_ref()
            .is_some_and(complete_provider);
        for attribute in block.body.attributes() {
            if attribute.key() == "alias" {
                if let hcl::Expression::String(value) = &attribute.expr {
                    if !literal(value) {
                        return Err("provider plan: nonliteral provider alias".into());
                    }
                    alias = Some(value.clone());
                } else {
                    return Err("provider plan: nonliteral provider alias".into());
                }
            }
            if ["assume_role", "assume_role_with_web_identity"].contains(&attribute.key()) {
                complete = false;
            }
        }
        for nested in block.body.blocks() {
            let name = nested.identifier();
            if name == "dynamic" {
                complete = false;
            }
            if !["assume_role", "assume_role_with_web_identity"].contains(&name) {
                continue;
            }
            let roles: Vec<_> = nested
                .body
                .attributes()
                .filter(|a| a.key() == "role_arn")
                .collect();
            let value = match roles.as_slice() {
                [attribute] => match &attribute.expr {
                    hcl::Expression::String(value) if literal(value) => Some(value),
                    _ => None,
                },
                _ => None,
            };
            if let Some(value) = value {
                if expressions.get(name).is_none() {
                    expressions[name] = json!([]);
                }
                expressions[name]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"role_arn":{"constant_value":value}}));
            } else {
                complete = false;
            }
        }
        result.push((local, alias, complete.then_some(expressions)));
    }
    Ok(result)
}

fn json_providers(bytes: &[u8]) -> Result<Vec<Declaration>, String> {
    let body: Value =
        serde_json::from_slice(bytes).map_err(|_| "provider plan: invalid JSON configuration")?;
    let Some(providers) = body.get("provider") else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for (local, declarations) in providers
        .as_object()
        .ok_or("provider plan: unsupported JSON providers")?
    {
        let declarations = match declarations {
            Value::Array(items) => items.iter().collect::<Vec<_>>(),
            Value::Object(_) => vec![declarations],
            _ => return Err("provider plan: unsupported JSON provider declaration".into()),
        };
        for declaration in declarations {
            let mut expressions = json!({});
            let mut complete = complete_provider(declaration);
            let alias = declaration
                .get("alias")
                .map(|v| {
                    v.as_str()
                        .filter(|s| literal(s))
                        .map(str::to_owned)
                        .ok_or("provider plan: nonliteral provider alias")
                })
                .transpose()?;
            for name in ["assume_role", "assume_role_with_web_identity"] {
                let Some(blocks) = declaration.get(name) else {
                    continue;
                };
                let blocks = match blocks {
                    Value::Array(items) => items.iter().collect::<Vec<_>>(),
                    Value::Object(_) => vec![blocks],
                    _ => {
                        complete = false;
                        continue;
                    }
                };
                expressions[name] = json!([]);
                for block in blocks {
                    if let Some(role) = block["role_arn"].as_str().filter(|s| literal(s)) {
                        expressions[name]
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"role_arn":{"constant_value":role}}));
                    } else {
                        complete = false;
                    }
                }
            }
            result.push((local.clone(), alias, complete.then_some(expressions)));
        }
    }
    Ok(result)
}

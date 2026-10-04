use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::path::Path;

pub fn generate(root: &Path) -> Result<String, String> {
    let mut changes = Vec::new();
    let module = module(root, "", &mut changes)?;
    let fixture = json!({
        "format_version": "1.2",
        "terraform_version": "1.9.8",
        "resource_changes": changes,
        "configuration": {"root_module": module}
    });
    serde_json::to_string_pretty(&fixture)
        .map(|text| text + "\n")
        .map_err(|error| error.to_string())
}

fn module(path: &Path, scope: &str, changes: &mut Vec<Value>) -> Result<Value, String> {
    let source = path.join("main.tf.json");
    let text = std::fs::read_to_string(&source)
        .map_err(|error| format!("{}: {error}", source.display()))?;
    let config: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let mut resources = Vec::new();
    for (block, mode, prefix, action) in [
        ("resource", "managed", "", "create"),
        ("data", "data", "data.", "no-op"),
    ] {
        for (resource_type, definitions) in config[block].as_object().into_iter().flatten() {
            for (name, attributes) in definitions
                .as_object()
                .ok_or("resource definitions must be objects")?
            {
                if attributes.get("for_each").is_some() {
                    return Err("fixture generator supports literal count, not for_each".into());
                }
                let address = format!("{prefix}{resource_type}.{name}");
                let count = match attributes.get("count") {
                    Some(value) => value
                        .as_u64()
                        .ok_or("fixture count must be a nonnegative integer")?,
                    None => 1,
                };
                let mut expressions = Map::new();
                for (key, value) in attributes
                    .as_object()
                    .ok_or("resource attributes must be objects")?
                {
                    if !matches!(
                        key.as_str(),
                        "count" | "depends_on" | "provider" | "lifecycle"
                    ) {
                        let generated = match (resource_type.as_str(), key.as_str()) {
                            ("aws_ecs_service", "network_configuration") => {
                                network_configuration(value)?
                            }
                            _ => expression(value)?,
                        };
                        expressions.insert(key.clone(), generated);
                    }
                }
                let mut resource = json!({"address":address,"mode":mode,"type":resource_type,"name":name,"expressions":expressions});
                if let Some(count) = attributes.get("count") {
                    resource["count_expression"] = expression(count)?;
                }
                if let Some(dependencies) = attributes.get("depends_on") {
                    resource["depends_on"] = dependencies.clone();
                }
                resources.push(resource);
                for index in 0..count {
                    let suffix = match attributes.get("count") {
                        Some(_) => format!("[{index}]"),
                        None => String::new(),
                    };
                    changes.push(json!({"address":format!("{scope}{address}{suffix}"),"mode":mode,"type":resource_type,"name":name,"change":{"actions":[action]}}));
                }
            }
        }
    }
    let mut calls = Map::new();
    for (name, call) in config["module"].as_object().into_iter().flatten() {
        for argument in ["count", "for_each", "depends_on", "providers"] {
            if call.get(argument).is_some() {
                return Err(format!(
                    "fixture generator does not support module meta-argument {argument}: {scope}module.{name}"
                ));
            }
        }
        let relative = call["source"].as_str().ok_or("module source is required")?;
        if !relative.starts_with("./") {
            return Err("fixture modules must be local".into());
        }
        let mut expressions = Map::new();
        for (key, value) in call.as_object().ok_or("module call must be an object")? {
            if key != "source" {
                expressions.insert(key.clone(), expression(value)?);
            }
        }
        calls.insert(name.clone(), json!({"expressions":expressions,"module":module(&path.join(relative), &format!("{scope}module.{name}."), changes)?}));
    }
    let mut outputs = Map::new();
    for (name, output) in config["output"].as_object().into_iter().flatten() {
        outputs.insert(
            name.clone(),
            json!({"expression":expression(&output["value"])?}),
        );
    }
    Ok(json!({"resources":resources,"module_calls":calls,"outputs":outputs}))
}

fn network_configuration(value: &Value) -> Result<Value, String> {
    let blocks = value
        .as_array()
        .ok_or("fixture network_configuration must be an array")?;
    let mut generated = Vec::new();
    for block in blocks {
        let mut expressions = Map::new();
        for (key, value) in block
            .as_object()
            .ok_or("fixture network_configuration block must be an object")?
        {
            expressions.insert(key.clone(), expression(value)?);
        }
        generated.push(Value::Object(expressions));
    }
    Ok(Value::Array(generated))
}

fn expression(value: &Value) -> Result<Value, String> {
    let mut references = BTreeSet::new();
    collect(value, &mut references)?;
    Ok(match references.is_empty() {
        true => json!({"constant_value":value}),
        false => json!({"references":references}),
    })
}

fn collect(value: &Value, references: &mut BTreeSet<String>) -> Result<(), String> {
    match value {
        Value::String(text) => {
            let mut remaining = text.as_str();
            while let Some((_, tail)) = remaining.split_once("${") {
                let (reference, rest) = tail
                    .split_once('}')
                    .ok_or("unterminated fixture interpolation")?;
                if reference.is_empty()
                    || !reference
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "_.[]\"-".contains(c))
                {
                    return Err(format!(
                        "fixture generator supports traversal interpolations only: {reference}"
                    ));
                }
                references.insert(reference.to_owned());
                remaining = rest;
            }
        }
        Value::Array(values) => {
            for value in values {
                collect(value, references)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect(value, references)?;
            }
        }
        _ => {}
    }
    Ok(())
}

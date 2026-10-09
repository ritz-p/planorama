use serde_json::json;

#[test]
fn output_provenance_resolves_aliases_modules_and_multiple_sources_without_values() {
    let input = json!({"format_version":"1.2","resource_changes":[{"address":"terraform_data.a","type":"terraform_data"},{"address":"terraform_data.b","type":"terraform_data"}],"output_changes":{"direct":{"after":"TOP_SECRET"}},"configuration":{"root_module":{
    "locals":{"alias":{"references":["terraform_data.a.id"]}},
    "module_calls":{"child":{"module":{"outputs":{"result":{"expression":{"references":["var.input"]}}}},"expressions":{"input":{"references":["terraform_data.b.id"]}}}},
    "outputs":{
        "direct":{"expression":{"references":["terraform_data.a.id"]}},
        "alias":{"expression":{"references":["local.alias"]}},
        "module":{"expression":{"references":["module.child.result"]}},
        "multiple":{"expression":{"references":["terraform_data.a.id","terraform_data.b.id"]}},
        "literal":{"expression":{"constant_value":"TOP_SECRET"}},
        "missing":{"expression":{"references":["terraform_data.missing.id"]}}
    }}}});
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    for name in ["direct", "alias", "module", "multiple"] {
        assert!(
            raw.outputs
                .iter()
                .find(|o| o.name == name)
                .unwrap()
                .complete,
            "{name}"
        );
    }
    assert_eq!(
        raw.outputs
            .iter()
            .find(|o| o.name == "multiple")
            .unwrap()
            .sources
            .len(),
        2
    );
    for name in ["literal", "missing"] {
        let output = raw.outputs.iter().find(|o| o.name == name).unwrap();
        assert!(!output.complete);
        assert!(output.sources.is_empty());
    }
    assert!(!format!("{raw:?}").contains("TOP_SECRET"));
    assert!(
        crate::semantic::diagnostics::collect(&raw)
            .iter()
            .any(|d| d.address == "output.missing")
    );
}

#[test]
fn dynamic_or_ambiguous_outputs_do_not_choose_instances() {
    for reference in ["terraform_data.a[var.index].id", "terraform_data.a.id"] {
        let input = json!({"format_version":"1.2","resource_changes":[{"address":"terraform_data.a[0]","type":"terraform_data"},{"address":"terraform_data.a[1]","type":"terraform_data"}],"configuration":{"root_module":{"outputs":{"x":{"expression":{"references":[reference]}}}}}});
        let raw = crate::plan::parse(&input.to_string()).unwrap();
        assert!(!raw.outputs[0].complete);
        assert!(raw.outputs[0].sources.is_empty());
    }
}

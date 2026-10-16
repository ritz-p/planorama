use super::*;

#[test]
fn structural_module_matching_and_validation() {
    let parent = Address::parse("module.app").unwrap();
    for value in [
        "module.app",
        "module.app[0].module.db",
        r#"module.app["a,b.c:d"].module.db[2]"#,
    ] {
        assert!(parent.contains(&Address::parse(value).unwrap()));
    }
    assert!(!parent.contains(&Address::parse("module.application").unwrap()));
    let partial = Address::parse("module.app.module.db[2]").unwrap();
    assert!(partial.contains(&Address::parse("module.app[3].module.db[2]").unwrap()));
    assert!(!partial.contains(&Address::parse("module.app[3].module.db[1]").unwrap()));
    let key = Address::parse(r#"module.app["a,b.c:d"]"#).unwrap();
    assert_eq!(key.to_string(), r#"module.app["a,b.c:d"]"#);
    assert_eq!(selector(r#"module.app["a:b"]"#).unwrap().0, None);
    assert_eq!(
        selector(r#"prod:module.app["a:b"]"#)
            .unwrap()
            .0
            .unwrap()
            .as_str(),
        "prod"
    );
    for value in [
        "",
        "root",
        "module",
        "module.",
        "module.app.aws_instance.x",
        "module.app[-1]",
        "module.app[true]",
        "module.app[*]",
        "module.app[0].",
        "module.app[0][1]",
        "module.0bad",
        "module.app[\"bad]",
    ] {
        assert!(Address::parse(value).is_err(), "{value}");
    }
}

#[test]
fn inventory_distinguishes_empty_instances_from_configuration() {
    let json = serde_json::json!({
        "format_version":"1.2", "resource_changes":[],
        "planned_values":{"root_module":{"child_modules":[{"address":"module.empty[0]"}]}},
        "configuration":{"root_module":{"module_calls":{"unknown":{"module":{}}}}}
    });
    let plan = crate::plan::parse(&json.to_string()).unwrap();
    assert_eq!(
        plan.modules
            .0
            .get(&Address::parse("module.empty[0]").unwrap()),
        Some(&true)
    );
    assert_eq!(
        plan.modules
            .0
            .get(&Address::parse("module.unknown").unwrap()),
        Some(&false)
    );
    assert!(
        !plan
            .modules
            .0
            .contains_key(&Address::parse("module.unknown[0]").unwrap())
    );
}

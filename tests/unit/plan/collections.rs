use crate::{plan, semantic};
use serde_json::json;

#[test]
fn planned_values_in_child_modules_prove_ids_without_retaining_them() {
    for mixed in [false, true] {
        let ids = if mixed {
            json!(["TOP_SECRET_ID", "TOP_SECRET_EXTERNAL"])
        } else {
            json!(["TOP_SECRET_ID"])
        };
        let input = json!({"format_version":"1.2","planned_values":{"root_module":{"child_modules":[{"resources":[
            {"address":"module.app.aws_security_group.main","type":"aws_security_group","values":{"id":"TOP_SECRET_ID"}},
            {"address":"module.app.aws_instance.main","type":"aws_instance","values":{"vpc_security_group_ids":ids}}
        ]}]}},"configuration":{"root_module":{"module_calls":{"app":{"module":{"resources":[
            {"address":"aws_instance.main","expressions":{"vpc_security_group_ids":{"references":["aws_security_group.main.id"]}}}
        ]}}}}}});
        let raw = plan::parse(&input.to_string()).unwrap();
        let reference = raw
            .attributes
            .iter()
            .find(|r| r.attribute == "vpc_security_group_ids")
            .unwrap();
        assert_eq!(reference.collection_ids_complete, !mixed);
        assert!(!format!("{raw:?}").contains("TOP_SECRET"));
        let graph = semantic::transform(&raw);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.kind == crate::model::EdgeKind::Connection)
                .count(),
            usize::from(!mixed)
        );
    }
}

#[test]
fn cached_data_ids_are_used_but_explicit_after_null_does_not_fall_back() {
    let mut input = json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_instance.main","type":"aws_instance","change":{"after":{"vpc_security_group_ids":["sg-cached"]}}}
    ],"prior_state":{"values":{"root_module":{"resources":[{"address":"data.aws_security_group.shared","mode":"data","type":"aws_security_group","values":{"id":"sg-cached"}}]}}},
    "configuration":{"root_module":{"resources":[
        {"address":"data.aws_security_group.shared"},
        {"address":"aws_instance.main","expressions":{"vpc_security_group_ids":{"references":["data.aws_security_group.shared.id"]}}}
    ]}}});
    let complete = |input: &serde_json::Value| {
        plan::parse(&input.to_string())
            .unwrap()
            .attributes
            .iter()
            .find(|r| r.attribute == "vpc_security_group_ids")
            .unwrap()
            .collection_ids_complete
    };
    assert!(complete(&input));
    input["resource_changes"].as_array_mut().unwrap().push(json!({"address":"data.aws_security_group.shared","mode":"data","type":"aws_security_group","change":{"after":null}}));
    assert!(!complete(&input));
}

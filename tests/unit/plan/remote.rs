use super::*;

#[test]
fn module_instance_references_reach_nested_resources_in_the_original_scope() {
    for field in ["count_expression", "for_each_expression"] {
        let mut outer = serde_json::json!({"module":{"module_calls":{"inner":{"module":{"resources":[{"address":"test.a"},{"address":"test.b"}]}}}}});
        outer[field] =
            serde_json::json!({"references":["data.terraform_remote_state.root.outputs.count"]});
        outer["module"]["module_calls"]["inner"][field] =
            serde_json::json!({"references":["data.terraform_remote_state.local.outputs.keys"]});
        let addresses = [
            "module.outer[0].module.inner[\"blue\"].test.a",
            "module.outer[0].module.inner[\"blue\"].test.b",
        ];
        let changes: Vec<_> = addresses
            .iter()
            .map(|address| serde_json::json!({"address":address,"type":"test"}))
            .collect();
        let input = serde_json::json!({"format_version":"1.2","resource_changes":changes,"configuration":{"root_module":{"module_calls":{"outer":outer}}}});
        let raw = crate::plan::parse(&input.to_string()).unwrap();
        assert_eq!(raw.remote_references.len(), 4);
        for address in addresses {
            let remotes: BTreeSet<_> = raw
                .remote_references
                .iter()
                .filter(|r| r.consumer == address)
                .map(|r| r.remote.as_str())
                .collect();
            assert_eq!(
                remotes,
                BTreeSet::from([
                    "data.terraform_remote_state.root",
                    "module.outer[0].data.terraform_remote_state.local"
                ])
            );
        }
    }
}

#[test]
fn instance_expressions_are_collected_but_attribute_names_are_not_remote_roots() {
    for field in ["count_expression", "for_each_expression"] {
        let mut resource = serde_json::json!({"address":"test.app"});
        resource[field] = serde_json::json!({"references":["data.terraform_remote_state.network.outputs.subnets"]});
        resource["expressions"] = serde_json::json!({"input":{"references":["terraform_data.config.output.data.terraform_remote_state.name.outputs.value"]}});
        let input = serde_json::json!({"format_version":"1.2","resource_changes":[{"address":"test.app[0]","type":"test"}],"configuration":{"root_module":{"resources":[resource]}}});
        let raw = crate::plan::parse(&input.to_string()).unwrap();
        assert_eq!(raw.remote_references.len(), 1);
        assert_eq!(
            raw.remote_references[0].remote,
            "data.terraform_remote_state.network"
        );
        assert_eq!(raw.remote_references[0].consumer, "test.app[0]");
    }
}

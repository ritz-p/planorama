use serde_json::{Value, json};

pub(crate) fn known_associations(input: &mut Value) {
    fn configure(module: &mut Value) {
        for resource in module["resources"].as_array_mut().into_iter().flatten() {
            resource["provider_config_key"] = json!("aws");
        }
        for call in module["module_calls"]
            .as_object_mut()
            .into_iter()
            .flat_map(|calls| calls.values_mut())
        {
            configure(&mut call["module"]);
        }
    }
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    input["configuration"]["provider_config"] = json!({"aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}});
    configure(&mut input["configuration"]["root_module"]);
    for change in input["resource_changes"].as_array_mut().unwrap() {
        let address = change["address"].as_str().unwrap().to_owned();
        let node = raw.nodes.iter().position(|n| n.address == address).unwrap();
        if matches!(
            change["type"].as_str(),
            Some("aws_subnet" | "aws_route_table" | "aws_vpc")
        ) {
            change["change"]["after"] = json!({"id":address,"vpc_id":"fixture-vpc"});
        } else if change["type"] == "aws_route_table_association" {
            let mut after = json!({});
            for r in raw.attributes.iter().filter(|r| r.target == node) {
                if let [source] = r.sources.as_slice() {
                    after[&r.attribute] = json!(raw.nodes[*source].address);
                }
            }
            change["change"]["after"] = after;
        }
    }
    let parsed = crate::plan::parse(&input.to_string()).unwrap();
    for r in parsed
        .attributes
        .iter()
        .filter(|r| parsed.nodes[r.target].resource_type == "aws_route_table_association")
    {
        assert!(
            r.service_reference_verified,
            "{r:?} {:?}",
            parsed
                .nodes
                .iter()
                .map(|n| (&n.address, &n.provider_configuration))
                .collect::<Vec<_>>()
        );
    }
}

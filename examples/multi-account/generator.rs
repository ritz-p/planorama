use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const STATES: [&str; 3] = ["application-a", "application-b", "shared-services"];
pub const MAPPINGS: [&str; 3] = [
    "application-a:module.application.data.terraform_remote_state.shared=shared-services",
    "application-b:module.application.data.terraform_remote_state.shared=shared-services",
    "application-b:data.terraform_remote_state.peer=application-a",
];

pub fn generate(source: &str) -> Result<BTreeMap<String, String>, String> {
    let base: Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
    let mut result = BTreeMap::new();
    for state in STATES {
        let mut plan = base.clone();
        let account = if state == "shared-services" {
            "111111111111"
        } else {
            "222222222222"
        };
        let action = match state {
            "application-a" => "create",
            "application-b" => "update",
            _ => "no-op",
        };
        for change in plan["resource_changes"].as_array_mut().unwrap() {
            if change["mode"] != "data" {
                change["change"]["actions"] = json!([action]);
                if state == "shared-services" && change["type"] == "aws_route" {
                    change["change"]["actions"] = json!(["delete"]);
                }
            }
        }
        plan["configuration"]["provider_config"] = json!({
            "aws.primary": provider(account, Some("us-east-1")),
            "aws.west": provider(account, Some("us-west-2")),
            "aws.audit": provider("111111111111", Some("us-east-1")),
            "aws.legacy": provider(account, None),
            "aws.imported": {"full_name":"hashicorp/aws", "expressions":{"region":{"constant_value":"us-east-1"}}},
            "terraform": {"full_name":"terraform.io/builtin/terraform"}
        });
        add(
            &mut plan,
            "",
            "aws_cloudfront_distribution.cdn",
            "aws.primary",
            json!({}),
            json!({}),
            action,
        );
        module(&mut plan, "application")["outputs"]["role_arn"] =
            json!({"expression":{"references":["aws_iam_role.task.arn"]}});
        for (name, reference) in [
            ("endpoint", "module.application.dns_name"),
            ("ami", "module.shared.ami_id"),
            ("role", "module.application.role_arn"),
        ] {
            module(&mut plan, "")["outputs"][name] =
                json!({"expression":{"references":[reference]}});
            plan["output_changes"][name] = json!({"actions":[action]});
        }
        if state == "application-a" {
            for i in 0..5 {
                add(
                    &mut plan,
                    "network",
                    &format!("aws_vpc.partner_{i}"),
                    "aws.primary",
                    json!({"cidr_block":{"constant_value":format!("10.{}.0.0/16", 50+i)}}),
                    json!({}),
                    action,
                );
            }
            for i in 0..28 {
                add(
                    &mut plan,
                    "network",
                    &format!("aws_route_table.partner_{i:02}"),
                    "aws.primary",
                    json!({
                        "vpc_id":{"references":[format!("aws_vpc.partner_{}.id", i%5)]}
                    }),
                    json!({}),
                    action,
                );
                let instance = format!("aws_subnet.partner_integration_segment_{i:02}");
                add(
                    &mut plan,
                    "network",
                    &instance,
                    "aws.primary",
                    json!({
                        "vpc_id":{"references":[format!("aws_vpc.partner_{}.id", i%5)]},
                        "cidr_block":{"constant_value":format!("10.{}.{}.0/24", 50+i%5, i)}
                    }),
                    json!({}),
                    action,
                );
                add(
                    &mut plan,
                    "network",
                    &format!("aws_route_table_association.partner_{i:02}"),
                    "aws.primary",
                    json!({
                        "route_table_id":{"references":[format!("aws_route_table.partner_{i:02}.id")]},
                        "subnet_id":{"references":[format!("{instance}.id")]}
                    }),
                    json!({}),
                    action,
                );
            }
            for (name, binding) in [
                ("regional_archive", "aws.west"),
                ("central_audit", "aws.audit"),
                ("legacy_region", "aws.legacy"),
                ("imported_account", "aws.imported"),
            ] {
                add(
                    &mut plan,
                    "",
                    &format!("aws_s3_bucket.{name}"),
                    binding,
                    json!({"tags":{"references":["module.application.dns_name"]}}),
                    json!({}),
                    action,
                );
            }
        }
        if state != "shared-services" {
            add(
                &mut plan,
                "application",
                "data.terraform_remote_state.shared",
                "terraform",
                json!({"backend":{"constant_value":"local"}}),
                json!({}),
                "no-op",
            );
            for resource in module(&mut plan, "application")["resources"]
                .as_array_mut()
                .unwrap()
            {
                match resource["address"].as_str().unwrap() {
                    "aws_instance.workers_a" | "aws_instance.workers_b" => {
                        resource["expressions"]["ami"] =
                            json!({"references":["data.terraform_remote_state.shared.outputs.ami"]})
                    }
                    "aws_lb.app" | "aws_ecs_service.app" => {
                        resource["expressions"]["tags"]["references"] =
                            json!(["data.terraform_remote_state.shared.outputs.endpoint"])
                    }
                    "aws_ecs_task_definition.app" => {
                        resource["expressions"]["execution_role_arn"] = json!({"references":["data.terraform_remote_state.shared.outputs.role"]})
                    }
                    _ => {}
                }
            }
        }
        if state == "application-b" {
            add(
                &mut plan,
                "",
                "data.terraform_remote_state.peer",
                "terraform",
                json!({"backend":{"constant_value":"local"}}),
                json!({}),
                "no-op",
            );
            add(
                &mut plan,
                "",
                "aws_s3_bucket.peer_contract",
                "aws.primary",
                json!({"tags":{"references":["data.terraform_remote_state.peer.outputs.endpoint"]}}),
                json!({}),
                action,
            );
        }
        bind(module(&mut plan, ""));
        plan["resource_changes"]
            .as_array_mut()
            .unwrap()
            .sort_by(|a, b| a["address"].as_str().cmp(&b["address"].as_str()));
        result.insert(
            state.into(),
            serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())? + "\n",
        );
    }
    Ok(result)
}

fn provider(account: &str, region: Option<&str>) -> Value {
    let mut value = json!({"full_name":"hashicorp/aws","expressions":{"assume_role":[{"role_arn":{"constant_value":format!("arn:aws:iam::{account}:role/PlanoramaFixture")}}]}});
    if let Some(region) = region {
        value["expressions"]["region"] = json!({"constant_value":region});
    }
    value
}

fn module<'a>(plan: &'a mut Value, name: &str) -> &'a mut Value {
    let root = &mut plan["configuration"]["root_module"];
    if name.is_empty() {
        root
    } else {
        &mut root["module_calls"][name]["module"]
    }
}

fn add(
    plan: &mut Value,
    scope: &str,
    address: &str,
    binding: &str,
    expressions: Value,
    after: Value,
    action: &str,
) {
    let parts: Vec<_> = address.split('.').collect();
    let data = parts[0] == "data";
    let kind = parts[usize::from(data)];
    let mode = if data { "data" } else { "managed" };
    let qualified = if scope.is_empty() {
        address.to_owned()
    } else {
        format!("module.{scope}.{address}")
    };
    plan["resource_changes"].as_array_mut().unwrap().push(json!({"address":qualified,"type":kind,"mode":mode,"provider_name":if data {"terraform.io/builtin/terraform"} else {"registry.terraform.io/hashicorp/aws"},"change":{"actions":[action],"after":after}}));
    module(plan, scope)["resources"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":mode,"provider_config_key":binding,"expressions":expressions}));
}

fn bind(module: &mut Value) {
    for resource in module["resources"].as_array_mut().into_iter().flatten() {
        if resource.get("provider_config_key").is_none() {
            resource["provider_config_key"] = json!("aws.primary");
        }
    }
    for call in module["module_calls"]
        .as_object_mut()
        .into_iter()
        .flat_map(|m| m.values_mut())
    {
        bind(&mut call["module"]);
    }
}

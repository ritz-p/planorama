use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/global-accelerator-custom-plan.json");

fn complete(v: &Value, label: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .flat_map(|n| &n.references)
        .any(|r| r.label == label && r.complete)
}

#[test]
fn custom_owners_are_exact_and_do_not_reuse_standard_types() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (source, target, path, label) in [
        (0, 1, "accelerator_arn", "Listener accelerator"),
        (1, 2, "listener_arn", "Group listener"),
    ] {
        let mut pair = v.clone();
        pair["resource_changes"] =
            json!([v["resource_changes"][source], v["resource_changes"][target]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][source],
            v["configuration"]["root_module"]["resources"][target]
        ]);
        super::super::service_references::tests::check_service_fixture(
            &pair.to_string(),
            label,
            path,
        );
        pair["resource_changes"][0]["type"] = json!(if source == 0 {
            "aws_globalaccelerator_accelerator"
        } else {
            "aws_globalaccelerator_listener"
        });
        assert!(!complete(&pair, label));
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 9);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        2
    );
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_globalaccelerator_custom_routing_accelerator")
            .unwrap()
            .placement,
        crate::model::Placement::Spatial
    );
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
}

#[test]
fn custom_endpoints_require_complete_correlated_subnet_ids_and_regions() {
    for case in 0..12 {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            1=>v["resource_changes"][2]["change"]["after"]["endpoint_group_region"]=json!("us-west-2"),
            2=>v["resource_changes"][3]["type"]=json!("aws_instance"),
            3=>v["resource_changes"][3]["change"]["after_sensitive"]=json!({"id":true}),
            4=>v["resource_changes"][2]["change"]["after_unknown"]=json!({"endpoint_configuration":true}),
            5=>v["resource_changes"][2]["change"]["after"]["endpoint_configuration"].as_array_mut().unwrap().push(json!({"endpoint_id":"literal"})),
            6=>v["configuration"]["root_module"]["resources"][2]["expressions"]["endpoint_configuration"].as_array_mut().unwrap().swap(0,1),
            7=>v["resource_changes"][3]["provider_name"]=json!("example.com/foreign/aws"),
            8=>v["resource_changes"][3]["deposed"]=json!("old"),
            9=>v["resource_changes"][3]["change"]["actions"]=json!(["delete"]),
            10=>v["resource_changes"][3]["mode"]=json!("data"),
            11=>v["configuration"]["root_module"]["resources"][2]["expressions"]["endpoint_configuration"]=json!([]),
            _=>{}
        }
        assert_eq!(complete(&v, "Configured subnets"), case == 0, "case {case}");
    }
}

#[test]
fn attachments_use_arn_outputs_and_each_rows_region_without_cidr_or_principal_inference() {
    for case in 0..12 {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            1 => {
                v["resource_changes"][6]["change"]["after"]["resource"][1]["region"] =
                    json!("us-west-2")
            }
            2 => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"resource":[{}, {"region":true}, {}]})
            }
            3 => {
                v["resource_changes"][6]["change"]["after_sensitive"] =
                    json!({"resource":[{}, {"cidr_block":true}, {}]})
            }
            4 => {
                v["resource_changes"][6]["change"]["after"]["resource"][1]["cidr_block"] =
                    json!("192.0.2.0/24")
            }
            5 => {
                v["resource_changes"][7]["change"]["after"]["load_balancer_type"] = json!("gateway")
            }
            6 => v["resource_changes"][7]["change"]["after_sensitive"] = json!({"arn":true}),
            7 => v["configuration"]["root_module"]["resources"][6]["expressions"]["resource"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1),
            8 => v["resource_changes"][6]["change"]["after"]["resource"]
                .as_array_mut()
                .unwrap()
                .push(json!({"endpoint_id":"literal","region":"us-east-1"})),
            9 => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["resource"][1]["endpoint_id"] =
                    json!({"references":["aws_instance.main.id"]})
            }
            10 => v["resource_changes"][8]["change"]["actions"] = json!(["delete"]),
            11 => v["resource_changes"][8]["provider_name"] = json!("example.com/foreign/aws"),
            _ => {}
        }
        assert_eq!(complete(&v, "Shared endpoints"), case == 0, "case {case}");
    }
    let mut p = plan::parse(FIXTURE).unwrap();
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_instance")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    assert!(
        !semantic::transform(&p)
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Shared endpoints" && r.complete)
    );
}

#[test]
fn custom_logs_require_enabled_known_mode_and_outputs_stay_safe() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    assert!(complete(&v, "Flow log bucket"));
    v["resource_changes"][0]["change"]["after"]["attributes"][0]["flow_logs_enabled"] =
        json!(false);
    assert!(!complete(&v, "Flow log bucket"));
    v["resource_changes"][0]["change"]["after"]["attributes"][0]["flow_logs_enabled"] = json!(true);
    v["resource_changes"][0]["change"]["after_sensitive"] =
        json!({"attributes":[{"flow_logs_enabled":true}]});
    assert!(!complete(&v, "Flow log bucket"));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(output.contains("Configured subnets"));
        assert!(output.contains("Shared endpoints"));
        assert!(!output.contains("210987654321"));
    }
}

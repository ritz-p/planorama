use crate::{plan, semantic};
use serde_json::{Value, json};

fn verified(value: &Value, address: &str, path: &str) -> bool {
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let node = graph
        .nodes
        .iter()
        .find(|node| node.address == address)
        .unwrap();
    node.references.iter().any(|reference| {
        reference.attribute == path
            && reference.complete
            && !reference.dependency
            && !reference.targets.is_empty()
            && super::label(&node.resource_type, path) == Some(reference.label)
    })
}

#[test]
fn configured_links_require_current_typed_evidence() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/instance-launch-plan.json")).unwrap();
    for (address, path) in [
        ("aws_instance.sample0", "ami"),
        ("aws_instance.sample0", "iam_instance_profile"),
        ("aws_instance.sample0", "placement_group_id"),
        (
            "aws_instance.sample0",
            "capacity_reservation_specification.capacity_reservation_target.capacity_reservation_id",
        ),
        ("aws_ec2_instance_state.sample5", "instance_id"),
        ("aws_instance.sample0", "ebs_block_device.snapshot_id"),
    ] {
        assert!(verified(&fixture, address, path), "{address}/{path}");
        let mut masked = fixture.clone();
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_sensitive"] = json!(true);
        assert!(
            !verified(&masked, address, path),
            "{address}/{path} sensitive"
        );
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_sensitive"] = json!(false);
        entry["change"]["after_unknown"] = json!(true);
        assert!(!verified(&masked, address, path), "{address}/{path}");
    }
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["placement_group"] = json!("group-name");
    assert!(verified(
        &changed,
        "aws_instance.sample0",
        "placement_group_id"
    ));
    changed["configuration"]["root_module"]["resources"][0]["expressions"]["placement_group"] =
        json!({"constant_value":"group-name"});
    assert!(!verified(
        &changed,
        "aws_instance.sample0",
        "placement_group_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(&changed, "aws_instance.sample0", "ami"));
}

#[test]
fn refreshed_launch_template_uses_the_configured_selector() {
    let mut fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/instance-launch-plan.json")).unwrap();
    let template = json!({"id": "lt-123", "name": "app"});
    fixture["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "address": "aws_launch_template.app", "mode": "managed",
            "type": "aws_launch_template", "name": "app",
            "provider_name": "registry.terraform.io/hashicorp/aws",
            "change": {"actions": ["no-op"], "before": template, "after": template}
        }));
    fixture["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "address": "aws_launch_template.app", "mode": "managed",
            "type": "aws_launch_template", "name": "app", "provider_config_key": "aws",
            "expressions": {}
        }));
    fixture["resource_changes"][0]["change"]["after"]["launch_template"] = json!([template]);
    let change = &mut fixture["resource_changes"][0]["change"];
    change["before"] = change["after"].clone();
    change["actions"] = json!(["no-op"]);
    for (selected, other) in [("id", "name"), ("name", "id")] {
        let mut plan = fixture.clone();
        let path = format!("launch_template.{selected}");
        plan["configuration"]["root_module"]["resources"][0]["expressions"]["launch_template"] =
            json!([{(selected): {"references": [format!("aws_launch_template.app.{selected}")]}}]);
        assert!(verified(&plan, "aws_instance.sample0", &path), "{selected}");
        assert!(!verified(
            &plan,
            "aws_instance.sample0",
            &format!("launch_template.{other}")
        ));
        plan["configuration"]["root_module"]["resources"][0]["expressions"]["launch_template"][0]
            [other] = json!({"constant_value": null});
        assert!(
            verified(&plan, "aws_instance.sample0", &path),
            "explicit null"
        );
        plan["configuration"]["root_module"]["resources"][0]["expressions"]["launch_template"][0]
            [other] = json!({"constant_value": template[other]});
        assert!(
            !verified(&plan, "aws_instance.sample0", &path),
            "both selectors configured"
        );
    }
}

#[test]
fn device_sets_match_typed_outputs_when_the_plan_reorders_devices() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/instance-launch-plan.json")).unwrap();
    for (block, field, kind, output, ids) in [
        (
            "ebs_block_device",
            "snapshot_id",
            "aws_ebs_snapshot",
            "id",
            ["snap-a", "snap-b"],
        ),
        (
            "ebs_block_device",
            "kms_key_id",
            "aws_kms_key",
            "arn",
            [
                "arn:aws:kms:us-east-1:123456789012:key/a",
                "arn:aws:kms:us-east-1:123456789012:key/b",
            ],
        ),
        (
            "network_interface",
            "network_interface_id",
            "aws_network_interface",
            "id",
            ["eni-a", "eni-b"],
        ),
        (
            "secondary_network_interface",
            "secondary_subnet_id",
            "aws_ec2_secondary_subnet",
            "secondary_subnet_id",
            ["ss-a", "ss-b"],
        ),
    ] {
        let mut plan = fixture.clone();
        let mut config = Vec::new();
        let mut devices = Vec::new();
        for (index, id) in ids.iter().enumerate() {
            let name = format!("device{index}");
            let address = format!("{kind}.{name}");
            plan["resource_changes"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "address": address, "type": kind, "mode": "managed", "name": name,
                    "provider_name": "registry.terraform.io/hashicorp/aws",
                    "change": {"actions":["create"],"after":{(output):id}}
                }));
            plan["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "address": address, "type": kind, "mode":"managed", "name": name,
                    "provider_config_key":"aws", "expressions":{}
                }));
            let (slot, slot_value) = match block {
                "ebs_block_device" => ("device_name", json!(["/dev/sdf", "/dev/sdg"][index])),
                "network_interface" => ("device_index", json!(index)),
                _ => ("network_card_index", json!(index)),
            };
            config.push(json!({(slot): {"constant_value": slot_value}, (field): {"references": [format!("{address}.{output}")]}}));
            devices.push(json!({(slot): slot_value, (field): id}));
            if field == "kms_key_id" {
                devices[index]["encrypted"] = json!(true);
            }
        }
        devices.reverse();
        plan["configuration"]["root_module"]["resources"][0]["expressions"][block] = json!(config);
        plan["resource_changes"][0]["change"]["after"][block] = json!(devices);
        let path = format!("{block}.{field}");
        let has_link = |p: &Value| verified(p, "aws_instance.sample0", &path);
        assert!(has_link(&plan), "{path}");
        let mut missing = plan.clone();
        missing["resource_changes"][0]["change"]["after"][block][0][field] = json!(ids[0]);
        assert!(!has_link(&missing), "{path} missing endpoint");
        for mask in ["after_unknown", "after_sensitive"] {
            let mut masked = plan.clone();
            masked["resource_changes"][0]["change"][mask][block] = json!([{(field):true}, {}]);
            assert!(!has_link(&masked), "{path} {mask}");
        }
        let mut literal = plan.clone();
        literal["configuration"]["root_module"]["resources"][0]["expressions"][block][1][field] =
            json!({"constant_value":ids[1]});
        assert!(!has_link(&literal), "{path} literal");
        if field == "kms_key_id" {
            let mut shared = plan.clone();
            let configured =
                &mut shared["configuration"]["root_module"]["resources"][0]["expressions"][block];
            configured[1][field] = configured[0][field].clone();
            shared["resource_changes"][0]["change"]["after"][block][0][field] = json!(ids[0]);
            assert!(has_link(&shared), "same key on both devices");
            let mut ambiguous = plan.clone();
            ambiguous["resource_changes"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap()["change"]["after"][output] = json!(ids[0]);
            ambiguous["resource_changes"][0]["change"]["after"][block][0][field] = json!(ids[0]);
            assert!(
                !has_link(&ambiguous),
                "different resources with the same output"
            );

            plan["resource_changes"][0]["change"]["after"][block][0]["encrypted"] = json!(false);
            assert!(!has_link(&plan), "disabled device encryption");
            plan["configuration"]["root_module"]["resources"][0]["expressions"][block][1]
                .as_object_mut()
                .unwrap()
                .remove(field);
            plan["resource_changes"][0]["change"]["after"][block][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(has_link(&plan), "only the other device uses the key");
        }
    }
}

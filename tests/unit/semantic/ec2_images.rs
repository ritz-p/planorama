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
        serde_json::from_str(include_str!("../../fixtures/ec2-images-plan.json")).unwrap();
    for (address, path) in [
        ("aws_ami_copy.sample0", "source_ami_id"),
        ("aws_ami_copy.sample0", "kms_key_id"),
        ("aws_ami.sample1", "ebs_block_device.snapshot_id"),
        ("aws_ami_from_instance.sample4", "source_instance_id"),
        ("aws_ami_launch_permission.sample6", "image_id"),
        (
            "aws_snapshot_create_volume_permission.sample7",
            "snapshot_id",
        ),
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
    changed["resource_changes"][0]["change"]["after"]["source_ami_region"] = json!("us-west-2");
    assert!(!verified(&changed, "aws_ami_copy.sample0", "source_ami_id"));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["encrypted"] = json!(false);
    assert!(!verified(&changed, "aws_ami_copy.sample0", "kms_key_id"));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(&changed, "aws_ami_copy.sample0", "source_ami_id"));
}

#[test]
fn copying_a_shared_image_requires_the_source_account_grant() {
    let mut fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/ec2-images-plan.json")).unwrap();
    let configs = &mut fixture["configuration"]["provider_config"];
    configs["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Fixture"}}]);
    let mut dest = configs["aws"].clone();
    dest["alias"] = json!("dest");
    dest["expressions"]["region"] = json!({"constant_value":"eu-west-1"});
    dest["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
        json!("arn:aws:iam::222222222222:role/Fixture");
    configs["aws.dest"] = dest;
    fixture["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
        json!("aws.dest");
    let has_source = |f: &Value| {
        let plan = plan::evidence::tests::parse_fixture(&f.to_string()).unwrap();
        let graph = semantic::transform(&plan);
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_ami_copy.sample0")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Source image" && r.complete && !r.dependency)
    };
    assert!(has_source(&fixture));
    for field in ["group", "organization_arn", "organizational_unit_arn"] {
        let mut ambiguous = fixture.clone();
        ambiguous["resource_changes"][6]["change"]["after_unknown"][field] = json!(true);
        assert!(!has_source(&ambiguous));
        ambiguous["resource_changes"][6]["change"]["after_unknown"][field] = json!(false);
        ambiguous["resource_changes"][6]["change"]["after"][field] = json!("another-grantee");
        assert!(!has_source(&ambiguous));
    }

    fixture["resource_changes"][6]["change"]["after"]["account_id"] = json!("333333333333");
    assert!(!has_source(&fixture));
}

#[test]
fn a_matching_image_id_needs_a_single_current_explicit_source() {
    let plan: Value =
        serde_json::from_str(include_str!("../../fixtures/ec2-images-plan.json")).unwrap();
    for selector in [
        json!({"constant_value":"ami-fixture"}),
        json!({"references":["aws_ami.sample1.id","var.other"]}),
        json!({"references":["aws_ami.sample1.name"]}),
    ] {
        let mut changed = plan.clone();
        changed["configuration"]["root_module"]["resources"][0]["expressions"]["source_ami_id"] =
            selector;
        assert!(!verified(&changed, "aws_ami_copy.sample0", "source_ami_id"));
    }
    for mask in ["after_unknown", "after_sensitive"] {
        let mut changed = plan.clone();
        changed["resource_changes"][1]["change"][mask]["id"] = json!(true);
        assert!(!verified(&changed, "aws_ami_copy.sample0", "source_ami_id"));
    }
}

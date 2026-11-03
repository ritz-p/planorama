use crate::{plan, semantic};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/backup-vaults-plan.json")).unwrap()
}
fn complete(v: &Value, target: usize, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == v["resource_changes"][target]["address"].as_str().unwrap()
            && r.attribute == path
            && super::super::service_references::verified(&raw, r)
    })
}
#[test]
fn vault_relationships_require_explicit_current_matching_outputs_and_scope() {
    for (target, path, source, output) in [
        (3, "encryption_key_arn", 1, "arn"),
        (4, "backup_vault_name", 0, "name"),
        (5, "backup_vault_name", 0, "id"),
        (5, "sns_topic_arn", 2, "arn"),
        (6, "backup_vault_name", 0, "name"),
    ] {
        assert!(complete(&fixture(), target, path));
        for case in [
            "literal",
            "partial",
            "ambiguous",
            "unknown",
            "source-unknown",
            "mismatch",
            "delete",
            "deposed",
            "data",
            "wrong-type",
            "scope",
        ] {
            let mut v = fixture();
            match case {
                "literal"=>{v["configuration"]["root_module"]["resources"][target]["expressions"].as_object_mut().unwrap().remove(path);},
                "partial"=>v["configuration"]["root_module"]["resources"][target]["expressions"][path]["references"].as_array_mut().unwrap().push(json!("var.other")),
                "ambiguous"=>v["configuration"]["root_module"]["resources"][target]["expressions"][path]["references"].as_array_mut().unwrap().push(json!("aws_backup_logically_air_gapped_vault.main.arn")),
                "unknown"=>v["resource_changes"][target]["change"]["after_unknown"]=json!({path:true}),
                "source-unknown"=>v["resource_changes"][source]["change"]["after_unknown"]=json!({output:true}),
                "mismatch"=>v["resource_changes"][source]["change"]["after"][output]=json!("wrong"),
                "delete"=>v["resource_changes"][source]["change"]["actions"]=json!(["delete"]),
                "deposed"=>v["resource_changes"][source]["deposed"]=json!("old"),
                "data"=>v["resource_changes"][source]["mode"]=json!("data"),
                "wrong-type"=>v["resource_changes"][source]["type"]=json!("aws_s3_bucket"),
                _=>v["configuration"]["root_module"]["resources"][source]["expressions"]["region"]=json!({"constant_value":"us-west-2"}),
            }
            assert!(!complete(&v, target, path), "{target} {path} {case}");
        }
    }
}
#[test]
fn helpers_embed_without_losing_filtered_coverage_or_exposing_policies() {
    let v = fixture();
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(g.coverage.selected.resources, 7);
    let air = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_backup_logically_air_gapped_vault")
        .unwrap();
    assert_eq!(air.placement, crate::model::Placement::Spatial);
    assert!(air.member_owner.is_none());
    for node in g.nodes.iter().filter(|n| super::helper(&n.resource_type)) {
        assert!(crate::view::routes::owner(&g, node).is_some());
        let selected = crate::view::single(
            &g,
            &crate::view::Options {
                focus: Some(node.address.clone()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.nodes.len(), 1);
        assert!(crate::view::routes::owner(&selected, &selected.nodes[0]).is_none());
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            &v.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_POLICY"));
        assert!(out.contains("Vault configuration"));
    }
}
#[test]
fn computed_arn_corroborates_but_cannot_replace_configured_name() {
    for target in [4, 5, 6] {
        let mut v = fixture();
        v["resource_changes"][target]["change"]["after"]["backup_vault_arn"] =
            json!("arn:aws:backup:us-east-1:123456789012:backup-vault:other");
        assert!(!complete(&v, target, "backup_vault_name"));
        v = fixture();
        v["configuration"]["root_module"]["resources"][target]["expressions"] =
            json!({"backup_vault_arn":{"references":["aws_backup_vault.main.arn"]}});
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert!(
            g.nodes
                .iter()
                .find(|n| n.address == v["resource_changes"][target]["address"].as_str().unwrap())
                .unwrap()
                .member_owner
                .is_none()
        );
    }
}

#[test]
fn air_gapped_owner_is_a_distinct_vault_and_unknown_arn_is_not_provenance() {
    let mut v = fixture();
    for target in [4, 5, 6] {
        v["resource_changes"][target]["change"]["after"]["backup_vault_name"] = json!("airgap");
        v["resource_changes"][target]["change"]["after"]["backup_vault_arn"] =
            v["resource_changes"][3]["change"]["after"]["arn"].clone();
        v["configuration"]["root_module"]["resources"][target]["expressions"]["backup_vault_name"] =
            json!({"references":["aws_backup_logically_air_gapped_vault.main.name"]});
        assert!(complete(&v, target, "backup_vault_name"));
    }
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    for node in g.nodes.iter().filter(|n| super::helper(&n.resource_type)) {
        let owner = crate::view::routes::owner(&g, node).unwrap();
        assert_eq!(
            g.nodes[owner].resource_type,
            "aws_backup_logically_air_gapped_vault"
        );
    }
    v["resource_changes"][4]["change"]["after_unknown"] = json!({"backup_vault_arn":true});
    assert!(complete(&v, 4, "backup_vault_name"));
}

use crate::{plan, semantic};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/vpc-controls-plan.json")).unwrap()
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
fn vpc_controls_require_unique_current_matching_id_and_scope() {
    for (target, path, source) in [
        (3, "vpc_id", 0),
        (3, "dhcp_options_id", 2),
        (4, "subnet_id", 1),
        (5, "vpc_id", 0),
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
            "scope",
        ] {
            let mut v = fixture();
            match case {
                "literal" => {v["configuration"]["root_module"]["resources"][target]["expressions"].as_object_mut().unwrap().remove(path);},
                "partial" => v["configuration"]["root_module"]["resources"][target]["expressions"][path]["references"].as_array_mut().unwrap().push(json!("var.other")),
                "ambiguous" => v["configuration"]["root_module"]["resources"][target]["expressions"][path]["references"].as_array_mut().unwrap().push(json!(if source == 2 {"aws_vpc.main.id"} else {"aws_vpc_dhcp_options.main.id"})),
                "unknown" => v["resource_changes"][target]["change"]["after_unknown"]=json!({path:true}),
                "source-unknown" => v["resource_changes"][source]["change"]["after_unknown"]=json!({"id":true}),
                "mismatch" => v["resource_changes"][source]["change"]["after"]["id"]=json!("wrong"),
                "delete" => v["resource_changes"][source]["change"]["actions"]=json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"]=json!("old"),
                "data" => v["resource_changes"][source]["mode"]=json!("data"),
                _ => v["configuration"]["root_module"]["resources"][source]["expressions"]["region"]=json!({"constant_value":"us-west-2"}),
            }
            assert!(!complete(&v, target, path), "{target} {path} {case}");
        }
    }
}
#[test]
fn controls_embed_and_dhcp_association_preserves_shared_options() {
    let v = fixture();
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(g.coverage.selected.resources, 6);
    for kind in [
        "aws_vpc_block_public_access_exclusion",
        "aws_vpc_encryption_control",
    ] {
        let node = g.nodes.iter().find(|n| n.resource_type == kind).unwrap();
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
    assert!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_vpc_dhcp_options")
            .unwrap()
            .member_owner
            .is_none()
    );
    assert!(
        g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Association)
    );
    for case in ["both", "unknown-other", "mode", "default"] {
        let mut v = fixture();
        let (target, path) = match case {
            "both" => {
                v["resource_changes"][4]["change"]["after"]["vpc_id"] = json!("vpc-main");
                (4, "subnet_id")
            }
            "unknown-other" => {
                v["resource_changes"][4]["change"]["after_unknown"] = json!({"vpc_id":true});
                (4, "subnet_id")
            }
            "mode" => {
                v["resource_changes"][5]["change"]["after"]["mode"] = json!("invalid");
                (5, "vpc_id")
            }
            _ => {
                v["resource_changes"][3]["change"]["after"]["dhcp_options_id"] = json!("default");
                v["resource_changes"][2]["change"]["after"]["id"] = json!("default");
                (3, "dhcp_options_id")
            }
        };
        assert!(!complete(&v, target, path), "{case}");
    }
}

#[test]
fn exclusion_vpc_target_and_unknown_mode_do_not_use_computed_arn() {
    let mut v = fixture();
    v["resource_changes"][4]["change"]["after"] =
        json!({"vpc_id":"vpc-main","internet_gateway_exclusion_mode":"allow-bidirectional"});
    v["configuration"]["root_module"]["resources"][4]["expressions"] =
        json!({"vpc_id":{"references":["aws_vpc.main.id"]}});
    assert!(complete(&v, 4, "vpc_id"));
    v["resource_changes"][4]["change"]["after_unknown"] =
        json!({"internet_gateway_exclusion_mode":true});
    assert!(!complete(&v, 4, "vpc_id"));
    v["configuration"]["root_module"]["resources"][4]["expressions"] =
        json!({"resource_arn":{"references":["aws_vpc.main.arn"]}});
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_vpc_block_public_access_exclusion")
            .unwrap()
            .member_owner
            .is_none()
    );
}

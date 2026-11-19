use crate::{plan, semantic};
use serde_json::{Value, json};
fn proven(v: &Value, address: &str, path: &str) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    g.nodes
        .iter()
        .find(|n| n.address == address)
        .is_some_and(|n| {
            n.references.iter().any(|r| {
                r.attribute == path
                    && r.complete
                    && !r.dependency
                    && !r.targets.is_empty()
                    && super::label(&n.resource_type, path) == Some(r.label)
            })
        })
}

#[test]
fn sagemaker_workforce_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/sagemaker-workforce-plan.json")).unwrap();
    for (addr, path) in [
        (
            "aws_sagemaker_workforce.private",
            "cognito_config.user_pool",
        ),
        (
            "aws_sagemaker_workforce.private",
            "cognito_config.client_id",
        ),
        ("aws_sagemaker_workteam.team", "workforce_name"),
        (
            "aws_sagemaker_flow_definition.flow",
            "human_loop_config.workteam_arn",
        ),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][1]["change"]["after"]["user_pool_id"] = json!("other");
        assert!(
            !proven(
                &v,
                "aws_sagemaker_workforce.private",
                "cognito_config.client_id"
            ),
            "unexpected proof for aws_sagemaker_workforce.private cognito_config.client_id"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][2]["change"]["after_sensitive"] = json!({"workforce_name":true});
        assert!(
            !proven(&v, "aws_sagemaker_workteam.team", "workforce_name"),
            "unexpected proof for aws_sagemaker_workteam.team workforce_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][3]["deposed"] = json!("old");
        assert!(
            !proven(
                &v,
                "aws_sagemaker_flow_definition.flow",
                "human_loop_config.workteam_arn"
            ),
            "unexpected proof for aws_sagemaker_flow_definition.flow human_loop_config.workteam_arn"
        );
    }
}

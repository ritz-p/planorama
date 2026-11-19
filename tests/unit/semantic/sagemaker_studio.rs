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
fn sagemaker_studio_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/sagemaker-studio-plan.json")).unwrap();
    for (addr, path) in [
        ("aws_sagemaker_image.img", "role_arn"),
        ("aws_sagemaker_image_version.v", "image_name"),
        ("aws_sagemaker_user_profile.u", "domain_id"),
        ("aws_sagemaker_app.ui", "user_profile_name"),
        ("aws_sagemaker_app.ui", "resource_spec.sagemaker_image_arn"),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][4]["change"]["after"]["domain_id"] = json!("another-domain");
        assert!(
            !proven(&v, "aws_sagemaker_app.ui", "user_profile_name"),
            "unexpected proof for aws_sagemaker_app.ui user_profile_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][5]["change"]["after"]["space_name"] = json!("other");
        assert!(
            !proven(&v, "aws_sagemaker_app.ui", "user_profile_name"),
            "unexpected proof for aws_sagemaker_app.ui user_profile_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][1]["change"]["after_sensitive"] = json!({"arn":true});
        assert!(
            !proven(
                &v,
                "aws_sagemaker_app.ui",
                "resource_spec.sagemaker_image_arn"
            ),
            "unexpected proof for aws_sagemaker_app.ui resource_spec.sagemaker_image_arn"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][1]["deposed"] = json!("old");
        assert!(
            !proven(&v, "aws_sagemaker_image_version.v", "image_name"),
            "unexpected proof for aws_sagemaker_image_version.v image_name"
        );
    }
}

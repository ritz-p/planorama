use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

const GROUP: &str = "aws_quicksight_group";
const USER: &str = "aws_quicksight_user";
const MEMBERSHIP: &str = "aws_quicksight_group_membership";
const ROLE: &str = "aws_quicksight_role_membership";
const ROLE_PERMISSION: &str = "aws_quicksight_role_custom_permission";
const USER_PERMISSION: &str = "aws_quicksight_user_custom_permission";
const POLICY: &str = "aws_quicksight_iam_policy_assignment";
const KEY: &str = "aws_quicksight_key_registration";
const SUBSCRIPTION: &str = "aws_quicksight_account_subscription";

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        GROUP
            | USER
            | MEMBERSHIP
            | ROLE
            | ROLE_PERMISSION
            | USER_PERMISSION
            | POLICY
            | KEY
            | SUBSCRIPTION
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<super::Rule> {
    Some(match (kind, path) {
        (GROUP | USER | ROLE_PERMISSION, "namespace") => (
            "aws_quicksight_namespace",
            &["namespace"],
            "QuickSight namespace",
        ),
        (USER, "iam_arn") => ("aws_iam_role", &["arn"], "IAM identity"),
        (MEMBERSHIP, "group_name") | (ROLE, "member_name") | (POLICY, "identities.group") => {
            (GROUP, &["group_name"], "QuickSight group")
        }
        (MEMBERSHIP, "member_name")
        | (USER_PERMISSION, "user_name")
        | (POLICY, "identities.user") => (USER, &["user_name"], "QuickSight user"),
        (ROLE_PERMISSION | USER_PERMISSION, "custom_permissions_name") => (
            "aws_quicksight_custom_permissions",
            &["custom_permissions_name"],
            "Custom permissions",
        ),
        (POLICY, "policy_arn") => ("aws_iam_policy", &["arn"], "Assigned IAM policy"),
        (KEY, "key_registration.key_arn") => ("aws_kms_key", &["arn"], "Registered keys"),
        (SUBSCRIPTION, "directory_id") => (
            "aws_directory_service_directory",
            &["id"],
            "Subscription directory",
        ),
        (SUBSCRIPTION, "iam_identity_center_instance_arn") => (
            "aws_ssoadmin_instances",
            &["arns"],
            "Identity Center instance",
        ),
        _ => return None,
    })
}

pub(super) fn qualifies(
    nodes: &[TerraformEntity],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
    s: usize,
    path: &str,
) -> bool {
    let known = |i, k| super::super::service_evidence::known(e, i, k);
    let target = r.target;
    let endpoint = nodes[s].resource_type.as_str();
    match (nodes[target].resource_type.as_str(), r.attribute.as_str()) {
        (USER, "iam_arn") => {
            if known(target, "identity_type") != Some("IAM") {
                return false;
            }
            let session =
                super::super::nested_references::value(e[target].0, e[target].1, "session_name");
            match endpoint {
                "aws_iam_user" => session.is_some_and(|v| v.is_null() || v.as_str() == Some("")),
                "aws_iam_role" => session.is_some_and(|v| {
                    v.is_null()
                        || v.as_str() == Some("")
                        || v.as_str().is_some_and(|session| {
                            known(s, "name")
                                .zip(known(target, "user_name"))
                                .is_some_and(|(role, user)| user == format!("{role}/{session}"))
                        })
                }),
                _ => false,
            }
        }
        (MEMBERSHIP | ROLE | USER_PERMISSION | POLICY, _) if matches!(endpoint, USER | GROUP) => {
            known(target, "namespace").is_some_and(|ns| Some(ns) == known(s, "namespace"))
                && (!path.starts_with("identities.")
                    || super::super::nested_references::singleton(
                        e[target].0,
                        e[target].1,
                        "identities",
                    ))
        }
        (SUBSCRIPTION, "directory_id") => {
            known(target, "authentication_method") == Some("ACTIVE_DIRECTORY")
        }
        (KEY, _) => path.split('.').count() == 3,
        _ => true,
    }
}

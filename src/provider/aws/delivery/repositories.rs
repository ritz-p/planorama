use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_codecommit_approval_rule_template_association"
            | "aws_codecommit_repository"
            | "aws_codegurureviewer_repository_association"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_codecommit_approval_rule_template_association", "approval_rule_template_name") => {
            Rule::new(
                "Approval rule template",
                &[("aws_codecommit_approval_rule_template", &["name"])],
                false,
                &[],
            )
        }
        ("aws_codecommit_approval_rule_template_association", "repository_name") => Rule::new(
            "CodeCommit repository",
            &[("aws_codecommit_repository", &["repository_name"])],
            false,
            &[],
        ),
        ("aws_codecommit_repository", "kms_key_id") => Rule::new(
            "Repository encryption key",
            &[("aws_kms_key", &["arn", "id", "key_id"])],
            false,
            &[],
        ),
        ("aws_codegurureviewer_repository_association", "kms_key_details.kms_key_id") => Rule::new(
            "Reviewer encryption key",
            &[("aws_kms_key", &["arn", "id", "key_id"])],
            false,
            &[],
        ),
        ("aws_codegurureviewer_repository_association", "repository.codecommit.name") => Rule::new(
            "Reviewed CodeCommit repository",
            &[("aws_codecommit_repository", &["repository_name"])],
            false,
            &[],
        ),
        ("aws_codegurureviewer_repository_association", "repository.s3_bucket.bucket_name") => {
            Rule::new(
                "Repository source bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &[],
            )
        }
        ("aws_codegurureviewer_repository_association", "repository.bitbucket.connection_arn") => {
            Rule::new(
                "Bitbucket connection",
                &[
                    ("aws_codeconnections_connection", &["arn"]),
                    ("aws_codestarconnections_connection", &["arn"]),
                ],
                false,
                &[],
            )
        }
        (
            "aws_codegurureviewer_repository_association",
            "repository.github_enterprise_server.connection_arn",
        ) => Rule::new(
            "GitHub Enterprise connection",
            &[
                ("aws_codeconnections_connection", &["arn"]),
                ("aws_codestarconnections_connection", &["arn"]),
            ],
            false,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _: &str) -> bool {
    let Some(variant) = r
        .attribute
        .strip_prefix("repository.")
        .and_then(|path| path.split_once('.').map(|(variant, _)| variant))
    else {
        return r.attribute != "kms_key_details.kms_key_id"
            || e.text(r.target, "kms_key_details.0.encryption_option")
                == Some("CUSTOMER_MANAGED_CMK");
    };
    let available = [
        "codecommit",
        "s3_bucket",
        "bitbucket",
        "github_enterprise_server",
    ];
    available
        .into_iter()
        .filter(|other| *other != variant)
        .all(|other| e.absent(r.target, &format!("repository.0.{other}")))
        && match variant {
            "bitbucket" => e.text(source, "provider_type") == Some("Bitbucket"),
            "github_enterprise_server" => {
                e.text(source, "provider_type") == Some("GitHubEnterpriseServer")
            }
            _ => true,
        }
}

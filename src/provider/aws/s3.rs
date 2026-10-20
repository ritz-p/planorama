#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_s3_bucket_policy", "bucket") => Some("Policy bucket reference"),
        _ => None,
    }
}

pub(super) fn source_fields(
    resource: &str,
    attribute: &str,
    source: &str,
) -> &'static [&'static str] {
    match (resource, attribute, source) {
        ("aws_s3_bucket_policy", "bucket", "aws_s3_bucket") => &["id", "bucket"],
        _ => &[],
    }
}

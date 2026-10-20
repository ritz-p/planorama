#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudfront.rs"]
mod tests;

pub(super) fn label(attribute: &str) -> Option<&'static str> {
    match attribute {
        "origin.domain_name" => Some("Origin reference"),
        "origin.origin_access_control_id" => Some("Origin access control reference"),
        "origin.s3_origin_config.origin_access_identity" => {
            Some("Origin access identity reference")
        }
        "default_cache_behavior.cache_policy_id" | "ordered_cache_behavior.cache_policy_id" => {
            Some("Cache policy reference")
        }
        "default_cache_behavior.origin_request_policy_id"
        | "ordered_cache_behavior.origin_request_policy_id" => {
            Some("Origin request policy reference")
        }
        "default_cache_behavior.response_headers_policy_id"
        | "ordered_cache_behavior.response_headers_policy_id" => {
            Some("Response headers policy reference")
        }
        _ => None,
    }
}

pub(super) fn source_fields(attribute: &str, source: &str) -> &'static [&'static str] {
    match (attribute, source) {
        ("origin.domain_name", "aws_s3_bucket") => {
            &["bucket_regional_domain_name", "bucket_domain_name"]
        }
        ("origin.domain_name", "aws_lb") => &["dns_name"],
        ("origin.origin_access_control_id", "aws_cloudfront_origin_access_control") => &["id"],
        (
            "origin.s3_origin_config.origin_access_identity",
            "aws_cloudfront_origin_access_identity",
        ) => &["cloudfront_access_identity_path"],
        (
            "default_cache_behavior.cache_policy_id" | "ordered_cache_behavior.cache_policy_id",
            "aws_cloudfront_cache_policy",
        )
        | (
            "default_cache_behavior.origin_request_policy_id"
            | "ordered_cache_behavior.origin_request_policy_id",
            "aws_cloudfront_origin_request_policy",
        )
        | (
            "default_cache_behavior.response_headers_policy_id"
            | "ordered_cache_behavior.response_headers_policy_id",
            "aws_cloudfront_response_headers_policy",
        ) => &["id"],
        _ => &[],
    }
}

mod aws;

use crate::model::ResourceRole;

#[cfg(test)]
#[path = "../tests/unit/classification.rs"]
mod tests;

pub(crate) fn classify(resource_type: &str) -> ResourceRole {
    match resource_type.strip_prefix("aws_") {
        Some(_) => aws::classify(resource_type),
        None => ResourceRole::Unknown,
    }
}

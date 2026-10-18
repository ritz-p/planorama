use super::service_references::scalar;
use crate::model::TerraformEntity;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/glue.rs"]
mod tests;

pub(super) fn catalog_verified(
    consumer: &TerraformEntity,
    database: &TerraformEntity,
    value: &Value,
    unknown: &Value,
    target: &Value,
    target_unknown: &Value,
) -> bool {
    let region = |entity: &TerraformEntity| super::scope(entity).and_then(|scope| scope.region);
    let left = region(consumer);
    if left.is_none() || left != region(database) {
        return false;
    }
    let catalog = scalar(target, target_unknown, &["catalog_id"]);
    catalog.is_some() && catalog == scalar(value, unknown, &["catalog_id"])
}

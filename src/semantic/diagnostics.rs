//! Provider-owned inference explains its decisions through generic diagnostics.
pub fn collect(plan: &crate::model::TerraformPlan) -> Vec<crate::model::Diagnostic> {
    crate::provider::diagnostics(plan)
}

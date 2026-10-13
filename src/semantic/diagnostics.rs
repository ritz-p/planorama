pub fn collect(plan: &crate::model::TerraformPlan) -> Vec<crate::model::Diagnostic> {
    let mut diagnostics = crate::provider::diagnostics(plan);
    for node in &plan.nodes {
        if node
            .provider_configuration
            .as_ref()
            .is_some_and(|c| c.account_conflict)
        {
            diagnostics.push(crate::model::Diagnostic {
                address: node.address.clone(),
                attribute: "provider.account".into(),
                reason: crate::model::DiagnosticReason::ConflictingAwsAccount,
            });
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

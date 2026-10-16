pub fn collect(plan: &crate::model::TerraformPlan) -> Vec<crate::model::Diagnostic> {
    let mut diagnostics = collect_instances(plan);
    diagnostics.sort_by(|a, b| {
        (&a.address, &a.attribute, a.reason).cmp(&(&b.address, &b.attribute, b.reason))
    });
    diagnostics.dedup_by(|a, b| {
        a.address == b.address && a.attribute == b.attribute && a.reason == b.reason
    });
    diagnostics
}

pub fn collect_instances(plan: &crate::model::TerraformPlan) -> Vec<crate::model::Diagnostic> {
    let mut diagnostics = crate::provider::diagnostics(plan);
    for node in &plan.nodes {
        if node
            .provider_configuration
            .as_ref()
            .is_some_and(|c| c.account_conflict)
        {
            diagnostics.push(crate::model::Diagnostic {
                deposed_key: node.deposed_key.clone(),
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

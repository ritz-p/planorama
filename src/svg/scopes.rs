use crate::layout::Layout;
use std::fmt::Write;

pub(super) fn render(graph: &crate::model::Graph, layout: &Layout<'_>) -> String {
    let mut svg = String::new();
    for panel in &layout.scopes {
        let bounds = panel.bounds;
        let provider = super::escape(&panel.scope.provider);
        let account = panel.scope.account.as_ref().map(|a| a.as_str());
        if panel.account_panel {
            let account = account.unwrap();
            let member = graph
                .nodes
                .iter()
                .filter(|node| {
                    node.entity
                        .scope
                        .as_ref()
                        .and_then(|scope| scope.account.as_ref())
                        .map(|a| a.as_str())
                        == Some(account)
                })
                .flat_map(|node| &node.references)
                .filter(|reference| reference.kind == crate::model::ReferenceKind::DeploymentMember)
                .flat_map(|reference| &reference.targets)
                .next();
            if let Some(member) = member {
                let id = super::identity::account_scope(panel.scope.account.as_ref().unwrap());
                let prefix = member
                    .state
                    .as_ref()
                    .map(super::states::prefix)
                    .unwrap_or_default();
                let href = format!("{prefix}{}", super::identity::source(&member.id));
                let link_attribute = if member.state.is_some() {
                    "data-global-href"
                } else {
                    "href"
                };
                let title = super::escape(&format!(
                    "Organizations member Account: {}{}",
                    member
                        .state
                        .as_ref()
                        .map(|state| format!("{}:", state.as_str()))
                        .unwrap_or_default(),
                    member.id.address
                ));
                writeln!(svg, r##"<g id="{id}" data-viewer-target="true" tabindex="0" role="button" data-deployment-scope="true" data-scope-provider="{provider}" data-account="{account}" data-account-panel="true"><title>AWS Account deployment scope: {account}</title><rect x="{}" y="{}" width="{}" height="{}" rx="18" fill="#eef2ff" stroke="#64748b"/><a {link_attribute}="#{href}" data-deployment-member="true"><title>{title}</title><text x="{}" y="{}" font-size="16" font-weight="700" text-decoration="underline" fill="#334155">AWS Account: {account}</text></a></g>"##, bounds.origin.x, bounds.origin.y, bounds.width, bounds.height, bounds.origin.x + 16, bounds.origin.y + 25).unwrap();
                continue;
            }
            writeln!(svg, r##"<g data-deployment-scope="true" data-scope-provider="{provider}" data-account="{account}" data-account-panel="true"><rect x="{}" y="{}" width="{}" height="{}" rx="18" fill="#eef2ff" stroke="#64748b"/><text x="{}" y="{}" font-size="16" font-weight="700" fill="#334155">AWS Account: {account}</text></g>"##, bounds.origin.x, bounds.origin.y, bounds.width, bounds.height, bounds.origin.x + 16, bounds.origin.y + 25).unwrap();
            continue;
        }
        let account_attribute = account
            .map(|a| format!(" data-account=\"{a}\""))
            .unwrap_or_default();
        let region = panel.scope.region.as_ref().map(|r| r.0.as_str());
        let scope = region
            .map(|r| format!("data-region=\"{}\"", super::escape(r)))
            .unwrap_or_else(|| "data-global=\"true\"".into());
        let label = format!(
            "{provider} {}",
            region.map(super::escape).unwrap_or_else(|| "Global".into())
        );
        writeln!(svg, r##"<g data-deployment-scope="true" data-scope-provider="{provider}"{account_attribute} {scope}><rect x="{}" y="{}" width="{}" height="{}" rx="14" fill="#f8fafc" stroke="#94a3b8" stroke-dasharray="9 5"/><text x="{}" y="{}" font-size="15" font-weight="700" fill="#334155">{label}</text></g>"##, bounds.origin.x, bounds.origin.y, bounds.width, bounds.height, bounds.origin.x + 16, bounds.origin.y + 25).unwrap();
    }
    svg
}

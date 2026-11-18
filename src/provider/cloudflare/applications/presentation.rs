pub(super) fn present(plan: &crate::model::TerraformPlan, graph: &mut crate::model::Graph) {
    let durable_scripts: std::collections::BTreeSet<_> = plan
        .attributes
        .iter()
        .flat_map(|r| {
            r.durable_object_scripts.iter().map(|&source| {
                (
                    plan.nodes[r.target].address.as_str(),
                    plan.nodes[source].address.as_str(),
                )
            })
        })
        .collect();
    let roles: std::collections::BTreeMap<_, _> = plan
        .nodes
        .iter()
        .filter(|node| node.provider.is_cloudflare())
        .filter_map(|node| {
            let label = match node.resource_type.as_str() {
                "cloudflare_worker" | "cloudflare_workers_script" => "Service bindings",
                "cloudflare_pipeline_stream" => "Pipeline stream bindings",
                "cloudflare_ai_search_instance" => "AI Search instance bindings",
                "cloudflare_ai_search_namespace" => "AI Search namespace bindings",
                "cloudflare_workflow" => "Workflow bindings",
                "cloudflare_queue" => "Queue producer bindings",
                "cloudflare_hyperdrive_config" => "Hyperdrive bindings",
                "cloudflare_pages_project" => "Configured Pages CNAME",
                "cloudflare_zero_trust_tunnel_cloudflared" => "Configured Tunnel CNAME",
                "cloudflare_r2_bucket"
                | "cloudflare_workers_kv_namespace"
                | "cloudflare_d1_database" => "Storage bindings",
                _ => return None,
            };
            Some((crate::model::TerraformEntityId::from(node), label))
        })
        .collect();
    for node in &mut graph.nodes {
        if node.provider.is_cloudflare() && node.resource_type == "cloudflare_dns_record" {
            for reference in &mut node.references {
                if reference.attribute == "content" && reference.complete && !reference.dependency {
                    if let [target] = reference.targets.as_slice() {
                        if let Some(label) = roles.get(&target.id) {
                            reference.label = label;
                        }
                    }
                }
            }
        }
        if !node.provider.is_cloudflare()
            || !matches!(
                node.resource_type.as_str(),
                "cloudflare_worker_version"
                    | "cloudflare_workers_script"
                    | "cloudflare_pages_project"
            )
        {
            continue;
        }
        let mut split = Vec::new();
        for reference in &mut node.references {
            if reference.dependency
                || !matches!(
                    reference.attribute.as_str(),
                    "bindings" | "deployment_configs"
                )
                || !reference.complete
            {
                continue;
            }
            let mut groups = std::collections::BTreeMap::<_, Vec<_>>::new();
            for target in reference.targets.drain(..) {
                let label = if durable_scripts
                    .contains(&(node.address.as_str(), target.id.address.as_str()))
                {
                    "Durable Object scripts"
                } else {
                    roles.get(&target.id).copied().unwrap_or(reference.label)
                };
                groups.entry(label).or_default().push(target);
            }
            let mut groups = groups.into_iter();
            if let Some((label, targets)) = groups.next() {
                reference.label = label;
                reference.targets = targets;
                for (label, targets) in groups {
                    let mut role = reference.clone();
                    role.label = label;
                    role.targets = targets;
                    split.push(role);
                }
            }
        }
        node.references.extend(split);
    }
}

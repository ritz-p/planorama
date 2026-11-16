use crate::{plan, semantic};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/cloudflare-zero-trust-plan.json")).unwrap()
}
fn verified(v: &Value) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    p.attributes.iter().any(|r| {
        p.nodes[r.target].address == "cloudflare_dns_record.portal"
            && r.attribute == "content"
            && r.service_reference_verified
    })
}
#[test]
fn tunnel_dns_is_a_scoped_configured_reference_not_hostname_inference() {
    let base = fixture();
    assert!(verified(&base));
    let g = semantic::transform(&plan::parse(&base.to_string()).unwrap());
    let dns = g
        .nodes
        .iter()
        .find(|n| n.address == "cloudflare_dns_record.portal")
        .unwrap();
    assert!(
        dns.references
            .iter()
            .any(|r| r.label == "Configured Tunnel CNAME" && r.complete)
    );
    assert!(
        !dns.references
            .iter()
            .flat_map(|r| &r.targets)
            .any(|t| t.id.address.contains("access_application"))
    );
    for (path, value) in [
        (
            "/resource_changes/11/change/after/content",
            json!("40000000-0000-4000-8000-000000000004.cfargotunnel.com.evil.invalid"),
        ),
        ("/resource_changes/11/change/after/proxied", json!(false)),
        (
            "/resource_changes/11/change/after_unknown/content",
            json!(true),
        ),
        (
            "/resource_changes/11/change/after_sensitive/content",
            json!(true),
        ),
        (
            "/resource_changes/11/change/after_sensitive/proxied",
            json!(true),
        ),
        ("/resource_changes/11/mode", json!("data")),
        (
            "/resource_changes/5/change/after/account_id",
            json!("ffffffffffffffffffffffffffffffff"),
        ),
        ("/resource_changes/5/change/after_unknown/id", json!(true)),
        ("/resource_changes/5/change/after_sensitive/id", json!(true)),
        ("/resource_changes/5/change/actions", json!(["delete"])),
        (
            "/configuration/root_module/resources/11/expressions/content/references",
            json!(["cloudflare_zero_trust_tunnel_cloudflared.office.name"]),
        ),
        (
            "/configuration/root_module/resources/11/expressions/zone_id/references",
            json!([]),
        ),
    ] {
        let mut v = base.clone();
        let (parent, key) = path.rsplit_once('/').unwrap();
        v.pointer_mut(parent).unwrap()[key] = value;
        assert!(!verified(&v), "{path}");
    }
    let mut v = base.clone();
    v["configuration"]["root_module"]["resources"][11]["expressions"]["content"] =
        json!({"constant_value":base["resource_changes"][11]["change"]["after"]["content"]});
    assert!(!verified(&v));
    let mut v = base;
    let mut duplicate = v["resource_changes"][5].clone();
    duplicate["address"] = json!("cloudflare_zero_trust_tunnel_cloudflared.duplicate");
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(!verified(&v));
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["deposed"] = json!("old");
    assert!(verified(&v));
}

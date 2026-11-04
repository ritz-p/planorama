use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudfront_security.rs"]
mod tests;

const GROUP: &str = "aws_cloudfront_key_group";
const CONFIG: &str = "aws_cloudfront_field_level_encryption_config";
const PROFILE: &str = "aws_cloudfront_field_level_encryption_profile";
const KEY: &str = "aws_cloudfront_public_key";
const TRUST: &str = "aws_cloudfront_trust_store";
const CONTENT: &str = "content_type_profile_config.content_type_profiles.items.profile_id";
const QUERY: &str = "query_arg_profile_config.query_arg_profiles.items.profile_id";
const ENCRYPTION: &str = "encryption_entities.items.public_key_id";
const BUNDLE: &str = "ca_certificates_bundle_source.ca_certificates_bundle_s3_location";

pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, CONFIG | PROFILE)
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (GROUP, "items") => Some("Key group public keys"),
        (CONFIG, CONTENT | QUERY) => Some("Field encryption profiles"),
        (PROFILE, ENCRYPTION) => Some("Encryption public keys"),
        (TRUST, p) => match p.strip_prefix(BUNDLE)? {
            ".bucket" => Some("CA bundle bucket"),
            ".key" => Some("CA bundle object"),
            ".version" => Some("CA bundle object version"),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let normal = path
        .split('.')
        .filter(|p| p.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".");
    resource(kind) && normal != path && label(kind, &normal).is_some()
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

fn reference<'a>(
    refs: &'a [AttributeReference],
    target: usize,
    path: &str,
) -> Option<&'a AttributeReference> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == path);
    let r = found.next()?;
    found.next().is_none().then_some(r)
}

fn expected<'a>(
    nodes: &[TerraformEntity],
    evidence: &[(&'a Value, &'a Value)],
    r: &AttributeReference,
    kind: &str,
) -> Option<BTreeSet<&'a str>> {
    if !current(&nodes[r.target])
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.sources.is_empty()
    {
        return None;
    }
    let result = r
        .sources
        .iter()
        .map(|&s| {
            let attrs = r.source_attributes_by_instance.get(&s)?;
            (current(&nodes[s])
                && nodes[s].resource_type == kind
                && super::route53::configured(&nodes[r.target], &nodes[s])
                && attrs.len() == 1
                && attrs.contains("id"))
            .then(|| super::service_evidence::known(evidence, s, "id"))
            .flatten()
        })
        .collect::<Option<BTreeSet<_>>>()?;
    (result.len() == r.sources.len()).then_some(result)
}

type Leaf<'a> = (String, Vec<usize>, &'a str);
fn leaves<'a>(
    v: &'a Value,
    m: &Value,
    fields: &[&str],
    path: String,
    lengths: Vec<usize>,
    optional: bool,
    out: &mut Vec<Leaf<'a>>,
) -> Option<()> {
    if let Some(values) = v.as_array() {
        if fields.is_empty()
            || values.is_empty()
            || (!path.ends_with(".items") && values.len() != 1)
            || !matches!(m, Value::Null | Value::Bool(false) | Value::Array(_))
            || m.as_array().is_some_and(|m| m.len() != values.len())
        {
            return None;
        }
        for (i, value) in values.iter().enumerate() {
            let mut lengths = lengths.clone();
            lengths.push(values.len());
            leaves(
                value,
                &m[i],
                fields,
                format!("{path}.{i}"),
                lengths,
                optional,
                out,
            )?;
        }
    } else if let Some((field, rest)) = fields.split_first() {
        if !v.is_object() || !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        let path = if path.is_empty() {
            field.to_string()
        } else {
            format!("{path}.{field}")
        };
        leaves(&v[*field], &m[*field], rest, path, lengths, optional, out)?;
    } else {
        if !matches!(m, Value::Null | Value::Bool(false)) {
            return None;
        }
        if optional && v.is_null() {
            return Some(());
        }
        out.push((path, lengths, v.as_str().filter(|v| !v.is_empty())?));
    }
    Some(())
}

fn collection(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    let kind = if nodes[r.target].resource_type == CONFIG {
        PROFILE
    } else {
        KEY
    };
    if nodes[r.target].resource_type == GROUP {
        let (v, m) = evidence[r.target];
        return (super::streaming::values(v, m, "items", true)?
            == expected(nodes, evidence, r, kind)?)
        .then_some(());
    }
    if !current(&nodes[r.target]) || !r.instances_explicit || !r.issues.is_empty() {
        return None;
    }
    let (v, m) = evidence[r.target];
    let mut rows = Vec::new();
    leaves(
        v,
        m,
        &r.attribute.split('.').collect::<Vec<_>>(),
        String::new(),
        Vec::new(),
        r.attribute == CONTENT,
        &mut rows,
    )?;
    if rows.is_empty() {
        return None;
    }
    let mut sources = BTreeSet::new();
    for (path, lengths, actual) in rows {
        let row = reference(refs, r.target, &path)?;
        let depth = if r.attribute == ENCRYPTION { 2 } else { 3 };
        if lengths.len() != depth
            || row.indexed_block_lengths != lengths
            || expected(nodes, evidence, row, kind)? != [actual].into()
        {
            return None;
        }
        sources.extend(row.sources.iter().copied());
    }
    if sources != r.sources.iter().copied().collect() {
        return None;
    }
    let unique = sources
        .iter()
        .map(|&s| super::service_evidence::known(evidence, s, "id"))
        .collect::<Option<BTreeSet<_>>>()?;
    (unique.len() == sources.len()).then_some(())
}

fn location<'a>(
    evidence: &[(&'a Value, &'a Value)],
    target: usize,
) -> Option<(&'a Value, &'a Value)> {
    let (mut v, mut m) = evidence[target];
    for field in BUNDLE.split('.') {
        if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        let [value] = v[field].as_array()?.as_slice() else {
            return None;
        };
        m = match &m[field] {
            Value::Null | Value::Bool(false) => &Value::Null,
            Value::Array(a) if a.len() == 1 => &a[0],
            _ => return None,
        };
        v = value;
    }
    Some((v, m))
}

fn endpoint(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
    path: &str,
    kind: &str,
    fields: &[&str],
) -> Option<usize> {
    let r = reference(refs, target, path)?;
    let s = super::route53::endpoint(nodes, r, kind, fields)?;
    let actual = super::appsync::scalar(evidence, target, path)?;
    (super::route53::configured(&nodes[target], &nodes[s])
        && r.source_attributes
            .iter()
            .any(|f| super::service_evidence::known(evidence, s, f) == Some(actual)))
    .then_some(s)
}

fn bundle(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    let (v, m) = location(evidence, r.target)?;
    let region = super::service_evidence::known(&[(v, m)], 0, "region")?;
    let bucket_path = format!("{BUNDLE}.bucket");
    let bucket = endpoint(
        nodes,
        refs,
        evidence,
        r.target,
        &bucket_path,
        "aws_s3_bucket",
        &["bucket", "id"],
    )?;
    if super::scope::scope(&nodes[bucket])?
        .region
        .as_ref()?
        .0
        .as_str()
        != region
    {
        return None;
    }
    if r.attribute == bucket_path {
        return Some(());
    }
    let key_path = format!("{BUNDLE}.key");
    let key_ref = reference(refs, r.target, &key_path)?;
    let [object] = key_ref.sources.as_slice() else {
        return None;
    };
    let kind = nodes[*object].resource_type.as_str();
    if !matches!(kind, "aws_s3_object" | "aws_s3_bucket_object") {
        return None;
    }
    let object = endpoint(nodes, refs, evidence, r.target, &key_path, kind, &["key"])?;
    if endpoint(
        nodes,
        refs,
        evidence,
        object,
        "bucket",
        "aws_s3_bucket",
        &["bucket", "id"],
    )? != bucket
        || super::scope::scope(&nodes[object])?
            .region
            .as_ref()?
            .0
            .as_str()
            != region
    {
        return None;
    }
    let version_path = format!("{BUNDLE}.version");
    if v["version"].is_null()
        && matches!(m, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(&m["version"], Value::Null | Value::Bool(false))
    {
        return (r.attribute == key_path).then_some(());
    }
    (endpoint(
        nodes,
        refs,
        evidence,
        r.target,
        &version_path,
        kind,
        &["version_id"],
    )? == object)
        .then_some(())
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let verified: Vec<_> = refs
        .iter()
        .map(|r| {
            if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
                return r.service_reference_verified;
            }
            if reference(refs, r.target, &r.attribute).is_none() {
                return false;
            }
            if nodes[r.target].resource_type == TRUST {
                bundle(nodes, refs, evidence, r).is_some()
            } else {
                collection(nodes, refs, evidence, r).is_some()
            }
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(verified) {
        r.service_reference_verified = verified;
        if verified {
            r.complete = true;
        }
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    if !super::route53::bindings(raw, r) {
        return false;
    }
    if raw.nodes[r.target].resource_type != TRUST || r.attribute.ends_with(".bucket") {
        return true;
    }
    let Some(bucket) = reference(&raw.attributes, r.target, &format!("{BUNDLE}.bucket")) else {
        return false;
    };
    if !bucket.service_reference_verified || !super::route53::bindings(raw, bucket) {
        return false;
    }
    r.sources.iter().all(|&s| {
        reference(&raw.attributes, s, "bucket")
            .is_some_and(|owner| super::route53::bindings(raw, owner))
    })
}

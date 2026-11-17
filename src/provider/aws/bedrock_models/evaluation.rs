use super::{FLOW, GENERATE, RETRIEVE};
use crate::provider::aws::nested_references;
use serde_json::Value;
fn empty(v: &Value) -> bool {
    v.is_null() || v.as_array().is_some_and(Vec::is_empty)
}
fn choice(
    v: &Value,
    m: &Value,
    base: &str,
    selected: &str,
    alternatives: &[&str],
    max: usize,
) -> bool {
    nested_references::block(v, m, &format!("{base}.{selected}"), max)
        && alternatives
            .iter()
            .filter(|&&name| name != selected)
            .all(|name| {
                nested_references::value(v, m, &format!("{base}.{name}")).is_some_and(empty)
            })
}
pub(super) fn qualify(e: &[(&Value, &Value)], target: usize, path: &str) -> bool {
    let (v, m) = e[target];
    if !nested_references::singleton(v, m, "evaluation_config")
        || !nested_references::singleton(v, m, "inference_config")
    {
        return false;
    }
    let human = choice(
        v,
        m,
        "evaluation_config.0",
        "human",
        &["automated", "human"],
        1,
    );
    let automated = choice(
        v,
        m,
        "evaluation_config.0",
        "automated",
        &["automated", "human"],
        1,
    );
    let model = choice(
        v,
        m,
        "inference_config.0",
        "model",
        &["model", "rag_config"],
        2,
    );
    let rag = choice(
        v,
        m,
        "inference_config.0",
        "rag_config",
        &["model", "rag_config"],
        1,
    );
    if !(human || automated) || !(model || rag) {
        return false;
    }
    if path == FLOW {
        return human;
    }
    if matches!(path, RETRIEVE | GENERATE) {
        let selected = if path == RETRIEVE {
            "retrieve_config"
        } else {
            "retrieve_and_generate_config"
        };
        return rag
            && choice(
                v,
                m,
                "inference_config.0.rag_config.0",
                "knowledge_base_config",
                &["knowledge_base_config", "precomputed_rag_source_config"],
                1,
            )
            && choice(
                v,
                m,
                "inference_config.0.rag_config.0.knowledge_base_config.0",
                selected,
                &["retrieve_config", "retrieve_and_generate_config"],
                1,
            );
    }
    true
}

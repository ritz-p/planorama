use crate::plan;

#[test]
fn configured_and_dependency_references_keep_order_identity_and_diagnostics() {
    let raw = plan::parse(r#"{
      "format_version":"1.2",
      "resource_changes":[
        {"address":"terraform_data.source","type":"terraform_data"},
        {"address":"terraform_data.first","type":"terraform_data"},
        {"address":"terraform_data.second","type":"terraform_data"}
      ],
      "configuration":{"root_module":{"resources":[
        {"address":"terraform_data.source"},
        {"address":"terraform_data.first","depends_on":["terraform_data.source"],"expressions":{
          "config":{"references":["terraform_data.source.output","terraform_data.source.output"]},
          "literal":{"constant_value":"omitted"},
          "unresolved":{"references":["terraform_data.source.output","terraform_data.missing.output"]}
        }},
        {"address":"terraform_data.second","expressions":{"config":{"references":["terraform_data.first.output"]}}}
      ]}}
    }"#).unwrap();
    let graph = super::base_graph(&raw);
    let first = graph
        .nodes
        .iter()
        .find(|node| node.address == "terraform_data.first")
        .unwrap();
    assert_eq!(
        first
            .references
            .iter()
            .map(|r| (r.attribute.as_str(), r.dependency))
            .collect::<Vec<_>>(),
        [
            ("config", false),
            ("unresolved", false),
            ("depends_on", true)
        ]
    );
    assert!(first.references[0].complete);
    assert_eq!(first.references[0].targets.len(), 1);
    assert_eq!(
        first.references[0].targets[0].id.address,
        "terraform_data.source"
    );
    assert!(!first.references[1].complete);
    assert!(!first.references[1].issues.is_empty());
    assert!(
        graph
            .nodes
            .iter()
            .find(|node| node.address == "terraform_data.source")
            .unwrap()
            .references
            .is_empty()
    );
    let second = graph
        .nodes
        .iter()
        .find(|node| node.address == "terraform_data.second")
        .unwrap();
    assert_eq!(second.references.len(), 1);
    assert_eq!(second.references[0].attribute, "config");
    assert_eq!(
        second.references[0].targets[0].id.address,
        "terraform_data.first"
    );
    let mut empty = raw.clone();
    empty.attributes.clear();
    empty.graph_references.clear();
    assert!(
        super::base_graph(&empty)
            .nodes
            .iter()
            .all(|node| node.references.is_empty())
    );
}

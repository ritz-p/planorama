use crate::plan;

#[test]
fn attribute_provenance_excludes_depends_on_and_marks_partial_resolution() {
    let raw = plan::parse(
        r#"{
        "format_version":"1.2",
        "resource_changes":[{"address":"aws_vpc.v"},{"address":"aws_subnet.s"}],
        "configuration":{"root_module":{"resources":[
            {"address":"aws_vpc.v"},
            {"address":"aws_subnet.s","depends_on":["aws_vpc.v"],"expressions":{
                "vpc_id":{"references":["aws_vpc.v.id"]},
                "tags":{"references":["aws_vpc.v.id","var.unknown"]},
                "cidr_block":{"constant_value":"10.0.0.0/24"}
            }}
        ]}}
    }"#,
    )
    .unwrap();
    assert_eq!(raw.attributes.len(), 3);
    let vpc = raw
        .nodes
        .iter()
        .position(|n| n.address == "aws_vpc.v")
        .unwrap();
    let explicit = raw
        .attributes
        .iter()
        .find(|a| a.attribute == "vpc_id")
        .unwrap();
    assert_eq!(explicit.sources, [vpc]);
    assert!(explicit.complete);
    assert!(
        !raw.attributes
            .iter()
            .find(|a| a.attribute == "tags")
            .unwrap()
            .complete
    );
    assert!(
        !raw.attributes
            .iter()
            .find(|a| a.attribute == "cidr_block")
            .unwrap()
            .complete
    );
}

#[test]
fn module_input_references_are_resolved_without_copying_attribute_values() {
    let raw = plan::parse(include_str!("../../../examples/plan.json")).unwrap();
    let subnet = raw
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_subnet")
        .unwrap();
    let vpc = raw
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_vpc")
        .unwrap();
    let reference = raw
        .attributes
        .iter()
        .find(|a| a.target == subnet && a.attribute == "vpc_id")
        .unwrap();
    assert_eq!(reference.sources, [vpc]);
    assert!(reference.complete);
}

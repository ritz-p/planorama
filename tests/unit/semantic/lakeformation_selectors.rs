use crate::{plan, semantic};
use serde_json::{Value, json};
fn proven(v: &Value, address: &str, path: &str) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    g.nodes
        .iter()
        .find(|n| n.address == address)
        .is_some_and(|n| {
            n.references.iter().any(|r| {
                r.attribute == path
                    && r.complete
                    && !r.dependency
                    && !r.targets.is_empty()
                    && super::label(&n.resource_type, path) == Some(r.label)
            })
        })
}

#[test]
fn lakeformation_selectors_requires_scoped_provenance() {
    let original: Value = serde_json::from_str(include_str!(
        "../../fixtures/lakeformation-selectors-plan.json"
    ))
    .unwrap();
    for (addr, path) in [
        (
            "aws_lakeformation_data_cells_filter.sample1",
            "table_data.table_name",
        ),
        (
            "aws_lakeformation_lf_tag_expression.sample3",
            "expression.tag_key",
        ),
        ("aws_lakeformation_permissions.sample4", "table.name"),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][1]["change"]["after"]["table_data"][0]["table_catalog_id"] =
            json!("999999999999");
        assert!(
            !proven(
                &v,
                "aws_lakeformation_data_cells_filter.sample1",
                "table_data.table_name"
            ),
            "unexpected proof for aws_lakeformation_data_cells_filter.sample1 table_data.table_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][4]["change"]["after"]["table"][0]["wildcard"] = json!(true);
        assert!(
            !proven(&v, "aws_lakeformation_permissions.sample4", "table.name"),
            "unexpected proof for aws_lakeformation_permissions.sample4 table.name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][2]["change"]["after"]["catalog_id"] = json!("999999999999");
        assert!(
            !proven(
                &v,
                "aws_lakeformation_lf_tag_expression.sample3",
                "expression.tag_key"
            ),
            "unexpected proof for aws_lakeformation_lf_tag_expression.sample3 expression.tag_key"
        );
    }
}

use super::*;
#[test]
fn indexed_addresses_preserve_module_keys() {
    let address = "module.app[\"a.b]c\"].module.inner[0].aws_instance.web[\"x.y\"]";
    assert_eq!(
        static_address(address),
        "module.app.module.inner.aws_instance.web"
    );
    assert_eq!(module_of(address), "module.app[\"a.b]c\"].module.inner[0]");
}

#[test]
fn ordinary_count_and_each_names_are_not_iteration_metadata() {
    for reference in [
        "aws_instance.count.index",
        "module.each.key",
        "module.scope.aws_instance.count.index",
    ] {
        assert!(!dynamic_selection(reference, None), "{reference}");
        assert!(!meta_reference(reference), "{reference}");
    }
    for reference in [
        "each.value.subnet_id",
        "module.app[0].each.value.subnet_id",
        "module.app.each.value[\"subnet_id\"]",
    ] {
        assert!(!dynamic_selection(reference, None), "{reference}");
        assert!(meta_reference(reference), "{reference}");
    }
}

#[test]
fn only_address_indices_are_dynamic_instance_selections() {
    for (reference, address, expected) in [
        (
            "aws_instance.web.tags[var.key]",
            Some("aws_instance.web"),
            false,
        ),
        (
            "data.aws_instance.web.tags[var.key]",
            Some("data.aws_instance.web"),
            false,
        ),
        (
            "module.app[0].aws_instance.web.tags[var.key]",
            Some("module.app.aws_instance.web"),
            false,
        ),
        ("module.app.result[var.key]", None, false),
        ("module.app.result.items[var.key]", None, false),
        (
            "aws_instance.web[var.key].id",
            Some("aws_instance.web"),
            true,
        ),
        ("module.app[var.key].result", None, true),
        (
            "module.app[var.key].aws_instance.web.id",
            Some("module.app.aws_instance.web"),
            true,
        ),
    ] {
        assert_eq!(
            dynamic_selection(reference, address),
            expected,
            "{reference}"
        );
    }
}

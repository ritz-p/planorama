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

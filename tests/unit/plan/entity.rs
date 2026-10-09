use super::*;
use serde_json::json;

#[test]
fn explicit_mode_is_independent_of_action_and_address() {
    for (value, address, expected) in [
        ("managed", "aws_vpc.main", EntityMode::Managed),
        ("data", "data.aws_vpc.shared", EntityMode::Data),
        ("managed", "data.test.explicit", EntityMode::Managed),
        ("data", "test.explicit", EntityMode::Data),
    ] {
        let node = parse(
            &json!({"mode": value, "type": "aws_vpc"}),
            address,
            Action::Unchanged,
        );
        assert_eq!(node.mode, expected);
        assert_eq!(node.resource_type, "aws_vpc");
        assert_eq!(node.action, Action::Unchanged);
    }
}

#[test]
fn omitted_mode_uses_the_resource_address_after_module_keys() {
    for (address, expected) in [
        ("aws_vpc.main", EntityMode::Managed),
        ("data.aws_vpc.shared", EntityMode::Data),
        (
            "module.app[\"data.fake\"].aws_vpc.main",
            EntityMode::Managed,
        ),
        (
            "module.app[\"x.y\"].module.inner.data.aws_vpc.shared[0]",
            EntityMode::Data,
        ),
    ] {
        assert_eq!(parse(&json!({}), address, Action::Read).mode, expected);
    }
}

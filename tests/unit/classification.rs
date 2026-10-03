use super::*;

#[test]
fn dispatches_aws_and_preserves_unknown_provider_types() {
    assert_eq!(classify("aws_vpc"), ResourceRole::Container);
    for resource_type in [
        "azurerm_virtual_network",
        "terraform_data",
        "custom_aws_vpc",
        "vpc",
        "aws_unknown",
        "",
    ] {
        assert_eq!(classify(resource_type), ResourceRole::Unknown);
    }
}

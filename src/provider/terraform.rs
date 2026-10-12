use crate::icons::ResourceIcon;

pub(crate) fn icon_for(resource_type: &str) -> Option<ResourceIcon> {
    match resource_type {
        "terraform_data" | "terraform_remote_state" => Some(ResourceIcon::Terraform),
        _ => None,
    }
}

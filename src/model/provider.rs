#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProviderConfiguration {
    pub key: String,
    pub alias: Option<String>,
    pub region: Option<RegionScope>,
    pub account: Option<AwsAccountScope>,
    pub account_conflict: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AwsAccountScope(String);

impl AwsAccountScope {
    pub fn new(id: &str) -> Option<Self> {
        (id.len() == 12 && id.bytes().all(|b| b.is_ascii_digit())).then(|| Self(id.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegionScope(pub String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderIdentity {
    Explicit(String),
    InferredAws,
    InferredTerraform,
    Unknown,
}

impl ProviderIdentity {
    pub fn from_source(source: &str) -> Self {
        let source = source.trim().to_ascii_lowercase();
        let source = if source.split('/').count() == 2 {
            format!("registry.terraform.io/{source}")
        } else {
            source
        };
        Self::Explicit(source)
    }

    pub fn inferred(resource_type: &str) -> Self {
        if resource_type.starts_with("aws_") {
            Self::InferredAws
        } else if matches!(resource_type, "terraform_data" | "terraform_remote_state") {
            Self::InferredTerraform
        } else {
            Self::Unknown
        }
    }

    pub fn source(&self) -> Option<&str> {
        match self {
            Self::Explicit(source) => Some(source),
            Self::InferredAws => Some("registry.terraform.io/hashicorp/aws"),
            Self::InferredTerraform => Some("terraform.io/builtin/terraform"),
            Self::Unknown => None,
        }
    }

    pub fn is_aws(&self) -> bool {
        self.source() == Some("registry.terraform.io/hashicorp/aws")
    }

    pub fn is_terraform(&self) -> bool {
        self.source() == Some("terraform.io/builtin/terraform")
    }

    pub fn is_explicit(&self) -> bool {
        matches!(self, Self::Explicit(_))
    }
}

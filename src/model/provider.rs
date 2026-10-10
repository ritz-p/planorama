#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProviderConfiguration {
    pub key: String,
    pub alias: Option<String>,
    pub region: Option<RegionScope>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegionScope(pub String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderIdentity {
    Explicit(String),
    InferredAws,
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
        } else {
            Self::Unknown
        }
    }

    pub fn source(&self) -> Option<&str> {
        match self {
            Self::Explicit(source) => Some(source),
            Self::InferredAws => Some("registry.terraform.io/hashicorp/aws"),
            Self::Unknown => None,
        }
    }

    pub fn is_aws(&self) -> bool {
        self.source() == Some("registry.terraform.io/hashicorp/aws")
    }

    pub fn is_explicit(&self) -> bool {
        matches!(self, Self::Explicit(_))
    }
}

//! Versioned, platform-neutral types shared across NEXUS components.

pub const API_VERSION: u32 = 1;

/// A named capability advertised by an analysis module.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Capability(String);

impl Capability {
    pub fn new(value: impl Into<String>) -> Result<Self, ApiError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err(ApiError::InvalidIdentifier(value));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiError {
    InvalidIdentifier(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentifier(value) => write!(f, "invalid identifier: {value:?}"),
        }
    }
}
impl std::error::Error for ApiError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_capability_identifiers() {
        assert_eq!(Capability::new("pe.read").unwrap().as_str(), "pe.read");
        assert!(Capability::new("bad capability").is_err());
    }
}

//! In-process registry for compatible analysis modules.
use nexus_api::Capability;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleDescriptor {
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub capabilities: BTreeSet<Capability>,
}
impl ModuleDescriptor {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        api_version: u32,
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Result<Self, RegistryError> {
        let name = name.into();
        let version = version.into();
        if !valid_identifier(&name) {
            return Err(RegistryError::InvalidName(name));
        }
        if version.is_empty()
            || version.len() > 32
            || !version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
        {
            return Err(RegistryError::InvalidVersion(version));
        }
        Ok(Self {
            name,
            version,
            api_version,
            capabilities: capabilities.into_iter().collect(),
        })
    }
}
fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

#[derive(Debug, Eq, PartialEq)]
pub enum RegistryError {
    InvalidName(String),
    InvalidVersion(String),
    Duplicate(String),
    IncompatibleApi {
        module: String,
        found: u32,
        expected: u32,
    },
}
impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(n) => write!(f, "invalid module name: {n:?}"),
            Self::InvalidVersion(v) => write!(f, "invalid module version: {v:?}"),
            Self::Duplicate(n) => write!(f, "module already registered: {n}"),
            Self::IncompatibleApi {
                module,
                found,
                expected,
            } => write!(
                f,
                "module {module} requires API {found}; host provides {expected}"
            ),
        }
    }
}
impl std::error::Error for RegistryError {}

#[derive(Default)]
pub struct ModuleRegistry {
    modules: BTreeMap<String, ModuleDescriptor>,
}
impl ModuleRegistry {
    pub fn register(
        &mut self,
        descriptor: ModuleDescriptor,
        host_api: u32,
    ) -> Result<(), RegistryError> {
        if descriptor.api_version != host_api {
            return Err(RegistryError::IncompatibleApi {
                module: descriptor.name,
                found: descriptor.api_version,
                expected: host_api,
            });
        }
        if self.modules.contains_key(&descriptor.name) {
            return Err(RegistryError::Duplicate(descriptor.name));
        }
        self.modules.insert(descriptor.name.clone(), descriptor);
        Ok(())
    }
    pub fn get(&self, name: &str) -> Option<&ModuleDescriptor> {
        self.modules.get(name)
    }
    pub fn list(&self) -> impl Iterator<Item = &ModuleDescriptor> {
        self.modules.values()
    }
    pub fn len(&self) -> usize {
        self.modules.len()
    }
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_api::API_VERSION;
    fn module(name: &str, api: u32) -> ModuleDescriptor {
        ModuleDescriptor::new(name, "0.1.0", api, []).unwrap()
    }
    #[test]
    fn registers_lists_and_rejects_duplicates_and_incompatible_api() {
        let mut registry = ModuleRegistry::default();
        registry
            .register(module("formats.pe", API_VERSION), API_VERSION)
            .unwrap();
        assert_eq!(registry.get("formats.pe").unwrap().version, "0.1.0");
        assert_eq!(registry.len(), 1);
        assert!(matches!(
            registry.register(module("formats.pe", API_VERSION), API_VERSION),
            Err(RegistryError::Duplicate(_))
        ));
        assert!(matches!(
            registry.register(module("future", API_VERSION + 1), API_VERSION),
            Err(RegistryError::IncompatibleApi { .. })
        ));
    }
}

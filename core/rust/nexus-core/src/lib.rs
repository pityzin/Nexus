//! NEXUS application core: safe memory views, a versioned registry, and native services.
use nexus_api::{Capability, API_VERSION};
use nexus_binary_loader::{BinaryError, BinaryFile, BinaryLimits};
use nexus_memory::{MemoryError, ReadOnlyImage, ResourceLimits};
use nexus_registry::{ModuleDescriptor, ModuleRegistry, RegistryError};

pub use nexus_api;
pub use nexus_binary_loader;
pub use nexus_memory;
pub use nexus_registry;

#[derive(Debug)]
pub enum CoreError {
    Memory(MemoryError),
    Registry(RegistryError),
    Native(i32),
}
impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory(e) => e.fmt(f),
            Self::Registry(e) => e.fmt(f),
            Self::Native(code) => write!(f, "native operation failed with error code {code}"),
        }
    }
}
impl std::error::Error for CoreError {}
impl From<MemoryError> for CoreError {
    fn from(value: MemoryError) -> Self {
        Self::Memory(value)
    }
}
impl From<RegistryError> for CoreError {
    fn from(value: RegistryError) -> Self {
        Self::Registry(value)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CoreConfig {
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ConfigError(String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "configuration error: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

impl CoreConfig {
    /// Reads bounded resource configuration from `NEXUS_MAX_INPUT_BYTES`.
    pub fn from_env() -> Result<Self, ConfigError> {
        let value = match std::env::var("NEXUS_MAX_INPUT_BYTES") {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(ConfigError(
                    "NEXUS_MAX_INPUT_BYTES must contain valid Unicode digits".into(),
                ));
            }
        };
        Self::from_max_input_bytes(value.as_deref())
    }

    pub fn from_max_input_bytes(value: Option<&str>) -> Result<Self, ConfigError> {
        const MAX_CONFIGURED_INPUT_BYTES: usize = 2 * 1024 * 1024 * 1024;
        let max_input_bytes = match value {
            None => ResourceLimits::default().max_input_bytes,
            Some(raw) => raw.parse::<usize>().map_err(|_| {
                ConfigError("NEXUS_MAX_INPUT_BYTES must be a positive integer".into())
            })?,
        };
        if max_input_bytes == 0 || max_input_bytes > MAX_CONFIGURED_INPUT_BYTES {
            return Err(ConfigError(format!(
                "NEXUS_MAX_INPUT_BYTES must be between 1 and {MAX_CONFIGURED_INPUT_BYTES}"
            )));
        }
        Ok(Self {
            resource_limits: ResourceLimits { max_input_bytes },
        })
    }
}

pub fn default_registry() -> Result<ModuleRegistry, RegistryError> {
    let mut registry = ModuleRegistry::default();
    for (name, capability) in [
        ("formats.pe", "format.pe.inspect"),
        ("formats.elf", "format.elf.inspect"),
    ] {
        let capabilities = [Capability::new(capability).expect("static capability identifier")];
        registry.register(
            ModuleDescriptor::new(name, env!("CARGO_PKG_VERSION"), API_VERSION, capabilities)?,
            API_VERSION,
        )?;
    }
    Ok(registry)
}

/// Creates a bounded image view for callers; it never maps or executes the input.
pub fn inspect_input_header(input: &[u8], limits: ResourceLimits) -> Result<&[u8], CoreError> {
    let image = ReadOnlyImage::new(input, limits)?;
    Ok(image.read(0, image.len().min(16))?)
}

/// Reads and parses an input file under the caller's resource limit. The file is never executed.
pub fn load_binary(
    path: impl AsRef<std::path::Path>,
    limits: ResourceLimits,
) -> Result<nexus_binary_loader::BinaryMetadata, BinaryError> {
    let max_file_size =
        u64::try_from(limits.max_input_bytes).map_err(|_| BinaryError::InvalidField {
            context: "maximum file size",
            detail: "does not fit in u64",
        })?;
    let file = BinaryFile::open(path, BinaryLimits::new(max_file_size)?)?;
    file.parse()
}

/// Adds signed integers in native code, using NASM or the documented portable fallback.
pub fn native_add(left: i64, right: i64) -> Result<i64, CoreError> {
    let mut result = 0_i64;
    // SAFETY: output points to a valid, initialized i64 and the ABI specifies fixed-width integers.
    let status = unsafe { nexus_native_add_i64(left, right, &mut result) };
    if status == 0 {
        Ok(result)
    } else {
        Err(CoreError::Native(status))
    }
}

/// Computes a bounded checksum through the stable C ABI.
pub fn native_checksum(bytes: &[u8]) -> Result<u64, CoreError> {
    let mut result = 0_u64;
    // SAFETY: the slice supplies `len` readable bytes for this call; the C function retains no pointer.
    let status = unsafe { nexus_native_checksum(bytes.as_ptr(), bytes.len(), &mut result) };
    if status == 0 {
        Ok(result)
    } else {
        Err(CoreError::Native(status))
    }
}

pub fn native_abi_version() -> u32 {
    // SAFETY: the ABI version function takes no arguments and returns a fixed-width integer.
    unsafe { nexus_native_abi_version() }
}

unsafe extern "C" {
    fn nexus_native_abi_version() -> u32;
    fn nexus_native_add_i64(left: i64, right: i64, result: *mut i64) -> i32;
    fn nexus_native_checksum(data: *const u8, length: usize, result: *mut u64) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_modules_are_compatible_and_statically_declared() {
        let modules = default_registry().unwrap();
        assert_eq!(modules.len(), 2);
        assert!(modules.get("formats.pe").is_some());
        assert!(modules.get("formats.elf").is_some());
        assert!(modules.get("formats.macho").is_none());
    }

    #[test]
    fn configuration_defaults_and_rejects_invalid_or_unbounded_limits() {
        assert_eq!(
            CoreConfig::from_max_input_bytes(None)
                .unwrap()
                .resource_limits
                .max_input_bytes,
            512 * 1024 * 1024
        );
        assert_eq!(
            CoreConfig::from_max_input_bytes(Some("4096"))
                .unwrap()
                .resource_limits
                .max_input_bytes,
            4096
        );
        assert!(CoreConfig::from_max_input_bytes(Some("0")).is_err());
        assert!(CoreConfig::from_max_input_bytes(Some("unlimited")).is_err());
        assert!(CoreConfig::from_max_input_bytes(Some("2147483649")).is_err());
    }
}

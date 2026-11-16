//! Stable, safe Rust contracts for trusted in-process NEXUS plugins.
//!
//! This crate deliberately exposes a small metadata-only input and bounded output
//! writer. It does not expose binary buffers, engine objects, or native handles.

use std::fmt;

pub const PLUGIN_API_VERSION: u32 = 1;
pub const MAX_PLUGIN_ID_BYTES: usize = 64;
pub const MAX_CAPABILITIES: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ExtensionPoint {
    /// Export the normalized metadata fields currently produced by the PE/ELF loader.
    MetadataExport,
}

impl ExtensionPoint {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MetadataExport => "metadata_export",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginDescriptor {
    id: String,
    version: String,
    min_host_api: u32,
    max_host_api: u32,
    extension_point: ExtensionPoint,
    capabilities: Vec<String>,
}

impl PluginDescriptor {
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
        min_host_api: u32,
        max_host_api: u32,
        extension_point: ExtensionPoint,
        capabilities: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, PluginApiError> {
        let id = id.into();
        let version = version.into();
        if !valid_identifier(&id) {
            return Err(PluginApiError::InvalidIdentifier(id));
        }
        if !valid_semver(&version) {
            return Err(PluginApiError::InvalidVersion(version));
        }
        if min_host_api == 0 || min_host_api > max_host_api {
            return Err(PluginApiError::InvalidCompatibilityRange);
        }
        let mut capabilities = capabilities.into_iter().map(Into::into).collect::<Vec<_>>();
        if capabilities.len() > MAX_CAPABILITIES {
            return Err(PluginApiError::TooManyCapabilities);
        }
        if capabilities.iter().any(|value| !valid_identifier(value)) {
            return Err(PluginApiError::InvalidCapability);
        }
        capabilities.sort();
        if capabilities.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(PluginApiError::DuplicateCapability);
        }
        Ok(Self {
            id,
            version,
            min_host_api,
            max_host_api,
            extension_point,
            capabilities,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn min_host_api(&self) -> u32 {
        self.min_host_api
    }
    pub fn max_host_api(&self) -> u32 {
        self.max_host_api
    }
    pub fn extension_point(&self) -> ExtensionPoint {
        self.extension_point
    }
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PLUGIN_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
}

// Enforce a small, dependency-free SemVer subset (three numeric components and
// optional prerelease/build labels); reject leading zeroes in numeric components.
fn valid_semver(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 {
        return false;
    }
    let mut build_parts = value.split('+');
    let pre_and_core = build_parts.next().unwrap_or_default();
    let build = build_parts.next();
    if build_parts.next().is_some() || build.is_some_and(|label| !valid_semver_labels(label, false))
    {
        return false;
    }
    let mut prerelease_parts = pre_and_core.splitn(2, '-');
    let core = prerelease_parts.next().unwrap_or_default();
    let prerelease = prerelease_parts.next();
    if prerelease.is_some_and(|label| !valid_semver_labels(label, true)) {
        return false;
    }
    let mut parts = core.split('.');
    let numeric = (0..3).all(|_| {
        parts.next().is_some_and(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'))
        })
    });
    if !numeric || parts.next().is_some() {
        return false;
    }
    true
}

fn valid_semver_labels(value: &str, prerelease: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && (!prerelease
                    || !part.bytes().all(|byte| byte.is_ascii_digit())
                    || part == "0"
                    || !part.starts_with('0'))
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PluginInput<'a> {
    format: &'a str,
    architecture: &'a str,
    bitness: u8,
    file_size: u64,
    entry_point: u64,
    section_count: u32,
}

impl<'a> PluginInput<'a> {
    pub fn new(
        format: &'a str,
        architecture: &'a str,
        bitness: u8,
        file_size: u64,
        entry_point: u64,
        section_count: u32,
    ) -> Self {
        PluginInput {
            format,
            architecture,
            bitness,
            file_size,
            entry_point,
            section_count,
        }
    }

    pub fn format(&self) -> &str {
        self.format
    }
    pub fn architecture(&self) -> &str {
        self.architecture
    }
    pub fn bitness(&self) -> u8 {
        self.bitness
    }
    pub fn file_size(&self) -> u64 {
        self.file_size
    }
    pub fn entry_point(&self) -> u64 {
        self.entry_point
    }
    pub fn section_count(&self) -> u32 {
        self.section_count
    }

    /// Estimates the copied metadata size with checked arithmetic for host limits.
    pub fn encoded_size(&self) -> Option<usize> {
        self.format
            .len()
            .checked_add(self.architecture.len())?
            .checked_add(std::mem::size_of::<u8>())?
            .checked_add(std::mem::size_of::<u64>() * 2)?
            .checked_add(std::mem::size_of::<u32>())
    }
}

pub trait Plugin: Send + Sync {
    fn descriptor(&self) -> PluginDescriptor;

    /// Write the plugin result into the host's bounded buffer.
    fn execute(&self, input: PluginInput<'_>, output: &mut PluginOutput) -> Result<(), String>;
}

pub struct PluginOutput {
    text: String,
    max_bytes: usize,
}

impl PluginOutput {
    #[doc(hidden)]
    pub fn with_limit(max_bytes: usize) -> Self {
        Self {
            text: String::new(),
            max_bytes,
        }
    }

    pub fn write_str(&mut self, value: &str) -> Result<(), PluginOutputError> {
        let new_len = self
            .text
            .len()
            .checked_add(value.len())
            .ok_or(PluginOutputError::LimitExceeded)?;
        if new_len > self.max_bytes {
            return Err(PluginOutputError::LimitExceeded);
        }
        self.text
            .try_reserve(value.len())
            .map_err(|_| PluginOutputError::AllocationFailed)?;
        self.text.push_str(value);
        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginOutputError {
    LimitExceeded,
    AllocationFailed,
}

impl fmt::Display for PluginOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded => f.write_str("plugin output exceeds the configured limit"),
            Self::AllocationFailed => f.write_str("unable to reserve plugin output storage"),
        }
    }
}

impl std::error::Error for PluginOutputError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginApiError {
    InvalidIdentifier(String),
    InvalidVersion(String),
    InvalidCompatibilityRange,
    InvalidCapability,
    DuplicateCapability,
    TooManyCapabilities,
}

impl fmt::Display for PluginApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(value) => write!(f, "invalid plugin identifier: {value:?}"),
            Self::InvalidVersion(value) => write!(f, "invalid plugin version: {value:?}"),
            Self::InvalidCompatibilityRange => f.write_str("invalid host API compatibility range"),
            Self::InvalidCapability => f.write_str("invalid plugin capability identifier"),
            Self::DuplicateCapability => f.write_str("duplicate plugin capability"),
            Self::TooManyCapabilities => f.write_str("plugin declares too many capabilities"),
        }
    }
}

impl std::error::Error for PluginApiError {}

/// JSON string escaping for SDK examples that intentionally avoid a serialization dependency.
pub fn json_string(value: &str) -> String {
    let mut result = String::new();
    result.push('"');
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            character if character.is_control() => {
                result.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => result.push(character),
        }
    }
    result.push('"');
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_validation_checks_versions_ranges_and_capabilities() {
        assert!(PluginDescriptor::new(
            "valid.plugin",
            "1.2.3-alpha.1+build",
            1,
            2,
            ExtensionPoint::MetadataExport,
            ["export.metadata"]
        )
        .is_ok());
        for version in [
            "1",
            "01.2.3",
            "1.2.3.4",
            "1.2.x",
            "1.0.0-",
            "1.0.0+",
            "1.0.0-01",
            "1.0.0-x..y",
            "1.0.0+x+z",
            "",
        ] {
            assert!(PluginDescriptor::new(
                "valid",
                version,
                1,
                1,
                ExtensionPoint::MetadataExport,
                [] as [&str; 0]
            )
            .is_err());
        }
        assert!(PluginDescriptor::new(
            "bad id",
            "1.0.0",
            1,
            1,
            ExtensionPoint::MetadataExport,
            [] as [&str; 0]
        )
        .is_err());
        assert!(PluginDescriptor::new(
            "valid",
            "1.0.0",
            2,
            1,
            ExtensionPoint::MetadataExport,
            [] as [&str; 0]
        )
        .is_err());
        assert!(PluginDescriptor::new(
            "valid",
            "1.0.0",
            1,
            1,
            ExtensionPoint::MetadataExport,
            ["same", "same"]
        )
        .is_err());
    }

    #[test]
    fn output_writer_enforces_limit_and_json_is_escaped() {
        let mut output = PluginOutput::with_limit(3);
        output.write_str("abc").unwrap();
        assert_eq!(output.write_str("d"), Err(PluginOutputError::LimitExceeded));
        assert_eq!(json_string("x\n\""), "\"x\\n\\\"\"");
    }
}

//! Bounded host for trusted, statically linked Rust extensions.
//!
//! This is error containment, not a sandbox. Plugins run in-process and must be trusted.

use nexus_plugin_api::{
    ExtensionPoint, Plugin, PluginDescriptor, PluginInput, PluginOutput, PLUGIN_API_VERSION,
};
use std::{
    any::Any,
    collections::BTreeMap,
    panic::{catch_unwind, AssertUnwindSafe},
};

pub const DEFAULT_MAX_PLUGINS: usize = 64;
pub const DEFAULT_MAX_INPUT_BYTES: usize = 1024 * 1024;
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const HARD_MAX_PLUGINS: usize = 256;
const HARD_MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const HARD_MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_ERROR_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PluginLimits {
    max_plugins: usize,
    max_input_bytes: usize,
    max_output_bytes: usize,
}

impl PluginLimits {
    pub fn new(
        max_plugins: usize,
        max_input_bytes: usize,
        max_output_bytes: usize,
    ) -> Result<Self, HostError> {
        if max_plugins == 0
            || max_plugins > HARD_MAX_PLUGINS
            || max_input_bytes == 0
            || max_input_bytes > HARD_MAX_INPUT_BYTES
            || max_output_bytes == 0
            || max_output_bytes > HARD_MAX_OUTPUT_BYTES
        {
            return Err(HostError::InvalidLimits);
        }
        Ok(Self {
            max_plugins,
            max_input_bytes,
            max_output_bytes,
        })
    }
}

impl Default for PluginLimits {
    fn default() -> Self {
        Self {
            max_plugins: DEFAULT_MAX_PLUGINS,
            max_input_bytes: DEFAULT_MAX_INPUT_BYTES,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginStatus {
    pub descriptor: PluginDescriptor,
    pub enabled: bool,
}

struct Entry {
    plugin: Box<dyn Plugin>,
    descriptor: PluginDescriptor,
    enabled: bool,
}

pub struct PluginHost {
    host_api: u32,
    limits: PluginLimits,
    plugins: BTreeMap<String, Entry>,
}

impl PluginHost {
    pub fn new(host_api: u32, limits: PluginLimits) -> Result<Self, HostError> {
        if host_api == 0 {
            return Err(HostError::IncompatibleHostApi {
                plugin: "<host>".to_owned(),
                min: 1,
                max: u32::MAX,
                actual: host_api,
            });
        }
        Ok(Self {
            host_api,
            limits,
            plugins: BTreeMap::new(),
        })
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<(), HostError> {
        if self.plugins.len() >= self.limits.max_plugins {
            return Err(HostError::PluginLimitReached);
        }
        let descriptor = catch_unwind(AssertUnwindSafe(|| plugin.descriptor()))
            .map_err(|_| HostError::PluginPanicked("<descriptor>".to_owned()))?;
        if !(descriptor.min_host_api()..=descriptor.max_host_api()).contains(&self.host_api) {
            return Err(HostError::IncompatibleHostApi {
                plugin: descriptor.id().to_owned(),
                min: descriptor.min_host_api(),
                max: descriptor.max_host_api(),
                actual: self.host_api,
            });
        }
        if self.plugins.contains_key(descriptor.id()) {
            return Err(HostError::DuplicatePlugin(descriptor.id().to_owned()));
        }
        let id = descriptor.id().to_owned();
        self.plugins.insert(
            id,
            Entry {
                plugin,
                descriptor,
                enabled: true,
            },
        );
        Ok(())
    }

    pub fn enable(&mut self, id: &str) -> Result<(), HostError> {
        let entry = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| HostError::UnknownPlugin(id.to_owned()))?;
        entry.enabled = true;
        Ok(())
    }

    pub fn disable(&mut self, id: &str) -> Result<(), HostError> {
        let entry = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| HostError::UnknownPlugin(id.to_owned()))?;
        entry.enabled = false;
        Ok(())
    }

    pub fn list(&self) -> Vec<PluginStatus> {
        self.plugins
            .values()
            .map(|entry| PluginStatus {
                descriptor: entry.descriptor.clone(),
                enabled: entry.enabled,
            })
            .collect()
    }

    pub fn execute(&self, id: &str, input: PluginInput<'_>) -> Result<String, HostError> {
        let input_size = input.encoded_size().ok_or(HostError::InputTooLarge {
            actual: usize::MAX,
            limit: self.limits.max_input_bytes,
        })?;
        if input_size > self.limits.max_input_bytes {
            return Err(HostError::InputTooLarge {
                actual: input_size,
                limit: self.limits.max_input_bytes,
            });
        }
        let entry = self
            .plugins
            .get(id)
            .ok_or_else(|| HostError::UnknownPlugin(id.to_owned()))?;
        if !entry.enabled {
            return Err(HostError::PluginDisabled(id.to_owned()));
        }
        if entry.descriptor.extension_point() != ExtensionPoint::MetadataExport {
            return Err(HostError::UnsupportedExtensionPoint(id.to_owned()));
        }
        let mut output = PluginOutput::with_limit(self.limits.max_output_bytes);
        let result = catch_unwind(AssertUnwindSafe(|| {
            entry.plugin.execute(input, &mut output)
        }));
        match result {
            Ok(Ok(())) => Ok(output.as_str().to_owned()),
            Ok(Err(message)) => Err(HostError::PluginFailed {
                plugin: id.to_owned(),
                message: bounded_message(message),
            }),
            Err(payload) => {
                let detail = panic_message(payload.as_ref());
                Err(HostError::PluginPanicked(format!("{id}: {detail}")))
            }
        }
    }
}

fn bounded_message(value: String) -> String {
    let mut end = value.len().min(MAX_ERROR_BYTES);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("plugin panicked");
    bounded_message(detail.to_owned())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostError {
    InvalidLimits,
    PluginLimitReached,
    DuplicatePlugin(String),
    UnknownPlugin(String),
    PluginDisabled(String),
    IncompatibleHostApi {
        plugin: String,
        min: u32,
        max: u32,
        actual: u32,
    },
    InputTooLarge {
        actual: usize,
        limit: usize,
    },
    UnsupportedExtensionPoint(String),
    PluginFailed {
        plugin: String,
        message: String,
    },
    PluginPanicked(String),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLimits => f.write_str("plugin limits are outside the allowed range"),
            Self::PluginLimitReached => f.write_str("plugin registry limit reached"),
            Self::DuplicatePlugin(id) => write!(f, "plugin already registered: {id}"),
            Self::UnknownPlugin(id) => write!(f, "unknown plugin: {id}"),
            Self::PluginDisabled(id) => write!(f, "plugin is disabled: {id}"),
            Self::IncompatibleHostApi {
                plugin,
                min,
                max,
                actual,
            } => write!(
                f,
                "plugin {plugin} supports host API {min} through {max}; host provides {actual}"
            ),
            Self::InputTooLarge { actual, limit } => {
                write!(f, "plugin input size {actual} exceeds limit {limit}")
            }
            Self::UnsupportedExtensionPoint(id) => {
                write!(f, "unsupported extension point for plugin {id}")
            }
            Self::PluginFailed { plugin, message } => {
                write!(f, "plugin {plugin} failed: {message}")
            }
            Self::PluginPanicked(id) => write!(f, "plugin panicked: {id}"),
        }
    }
}

impl std::error::Error for HostError {}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new(PLUGIN_API_VERSION, PluginLimits::default())
            .expect("static default plugin limits are valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_plugin_api::{PluginApiError, PluginOutputError};

    struct TestPlugin {
        id: &'static str,
        behavior: Behavior,
    }

    enum Behavior {
        Echo,
        Panic,
        Fail,
        Overflow,
    }

    impl Plugin for TestPlugin {
        fn descriptor(&self) -> PluginDescriptor {
            PluginDescriptor::new(
                self.id,
                "1.0.0",
                PLUGIN_API_VERSION,
                PLUGIN_API_VERSION,
                ExtensionPoint::MetadataExport,
                ["test.echo"],
            )
            .unwrap()
        }

        fn execute(&self, input: PluginInput<'_>, output: &mut PluginOutput) -> Result<(), String> {
            match self.behavior {
                Behavior::Echo => output
                    .write_str(input.format())
                    .map_err(|error| error.to_string()),
                Behavior::Panic => panic!("controlled plugin panic"),
                Behavior::Fail => Err("controlled plugin failure".into()),
                Behavior::Overflow => output
                    .write_str("0123456789")
                    .map_err(|error| error.to_string()),
            }
        }
    }

    fn input(format: &str) -> PluginInput<'_> {
        PluginInput::new(format, "x86_64", 64, 128, 64, 2)
    }

    #[test]
    fn registers_checks_compatibility_and_rejects_duplicates() {
        let mut host = PluginHost::default();
        host.register(Box::new(TestPlugin {
            id: "a",
            behavior: Behavior::Echo,
        }))
        .unwrap();
        assert!(matches!(
            host.register(Box::new(TestPlugin {
                id: "a",
                behavior: Behavior::Echo,
            })),
            Err(HostError::DuplicatePlugin(_))
        ));
        assert!(matches!(
            host.register(Box::new(OldApiPlugin)),
            Err(HostError::IncompatibleHostApi { .. })
        ));
        assert_eq!(host.list().len(), 1);
    }

    struct OldApiPlugin;
    impl Plugin for OldApiPlugin {
        fn descriptor(&self) -> PluginDescriptor {
            PluginDescriptor::new(
                "old-api",
                "1.0.0",
                2,
                3,
                ExtensionPoint::MetadataExport,
                [] as [&str; 0],
            )
            .unwrap()
        }
        fn execute(&self, _: PluginInput<'_>, _: &mut PluginOutput) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn enable_disable_and_execution_are_explicit() {
        let mut host = PluginHost::default();
        host.register(Box::new(TestPlugin {
            id: "switchable",
            behavior: Behavior::Echo,
        }))
        .unwrap();
        assert_eq!(host.execute("switchable", input("elf")).unwrap(), "elf");
        host.disable("switchable").unwrap();
        assert!(matches!(
            host.execute("switchable", input("elf")),
            Err(HostError::PluginDisabled(_))
        ));
        host.enable("switchable").unwrap();
        assert_eq!(host.execute("switchable", input("pe")).unwrap(), "pe");
        assert!(matches!(
            host.disable("missing"),
            Err(HostError::UnknownPlugin(_))
        ));
    }

    #[test]
    fn output_limit_and_input_limit_are_enforced() {
        let limits = PluginLimits::new(4, 128, 4).unwrap();
        let mut host = PluginHost::new(PLUGIN_API_VERSION, limits).unwrap();
        host.register(Box::new(TestPlugin {
            id: "overflow",
            behavior: Behavior::Overflow,
        }))
        .unwrap();
        assert!(matches!(
            host.execute("overflow", input("pe")),
            Err(HostError::PluginFailed { message, .. }) if message.contains("output exceeds")
        ));
        assert!(matches!(
            host.execute("overflow", input(&"a".repeat(128))),
            Err(HostError::InputTooLarge { .. })
        ));
        assert!(PluginLimits::new(0, 1, 1).is_err());
    }

    #[test]
    fn plugin_panics_and_errors_do_not_unwind_through_host() {
        let mut host = PluginHost::default();
        host.register(Box::new(TestPlugin {
            id: "panic",
            behavior: Behavior::Panic,
        }))
        .unwrap();
        host.register(Box::new(TestPlugin {
            id: "failure",
            behavior: Behavior::Fail,
        }))
        .unwrap();
        assert!(matches!(
            host.execute("panic", input("elf")),
            Err(HostError::PluginPanicked(_))
        ));
        assert!(matches!(
            host.execute("failure", input("elf")),
            Err(HostError::PluginFailed { .. })
        ));
    }

    #[test]
    fn listing_is_stably_sorted_by_plugin_id() {
        let mut host = PluginHost::default();
        for id in ["zeta", "alpha"] {
            host.register(Box::new(TestPlugin {
                id,
                behavior: Behavior::Echo,
            }))
            .unwrap();
        }
        let ids = host
            .list()
            .into_iter()
            .map(|status| status.descriptor.id().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["alpha", "zeta"]);
    }

    #[test]
    fn plugin_failures_are_bounded_and_metadata_constructor_is_publicly_validated() {
        let mut host = PluginHost::default();
        host.register(Box::new(TestPlugin {
            id: "failure",
            behavior: Behavior::Fail,
        }))
        .unwrap();
        assert!(matches!(
            host.execute("failure", input("elf")),
            Err(HostError::PluginFailed { message, .. }) if message == "controlled plugin failure"
        ));
        let invalid: Result<PluginDescriptor, PluginApiError> = PluginDescriptor::new(
            "bad id",
            "1.0.0",
            1,
            1,
            ExtensionPoint::MetadataExport,
            [] as [&str; 0],
        );
        assert!(invalid.is_err());
        assert_eq!(
            PluginOutputError::LimitExceeded.to_string(),
            "plugin output exceeds the configured limit"
        );
    }

    #[test]
    fn plugin_count_limit_is_enforced_without_partial_registration() {
        let limits = PluginLimits::new(1, 1024, 1024).unwrap();
        let mut host = PluginHost::new(PLUGIN_API_VERSION, limits).unwrap();
        host.register(Box::new(TestPlugin {
            id: "first",
            behavior: Behavior::Echo,
        }))
        .unwrap();
        assert!(matches!(
            host.register(Box::new(TestPlugin {
                id: "second",
                behavior: Behavior::Echo,
            })),
            Err(HostError::PluginLimitReached)
        ));
        assert_eq!(host.list().len(), 1);
    }
}

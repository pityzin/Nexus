//! Example trusted metadata exporter built only against the public plugin SDK.

use nexus_plugin_api::{
    json_string, ExtensionPoint, Plugin, PluginDescriptor, PluginInput, PluginOutput,
    PLUGIN_API_VERSION,
};

#[derive(Default)]
pub struct MetadataSummaryPlugin;

impl Plugin for MetadataSummaryPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor::new(
            "metadata-summary",
            env!("CARGO_PKG_VERSION"),
            PLUGIN_API_VERSION,
            PLUGIN_API_VERSION,
            ExtensionPoint::MetadataExport,
            ["export.metadata-summary"],
        )
        .expect("example plugin descriptor is valid")
    }

    fn execute(&self, input: PluginInput<'_>, output: &mut PluginOutput) -> Result<(), String> {
        let json = format!(
            "{{\"schema_version\":1,\"plugin\":{},\"format\":{},\"architecture\":{},\"bitness\":{},\"file_size\":{},\"entry_point\":{},\"section_count\":{}}}",
            json_string("metadata-summary"),
            json_string(input.format()),
            json_string(input.architecture()),
            input.bitness(),
            input.file_size(),
            input.entry_point(),
            input.section_count(),
        );
        output.write_str(&json).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_exports_real_normalized_metadata_fields() {
        let plugin = MetadataSummaryPlugin;
        let mut output = PluginOutput::with_limit(1024);
        plugin
            .execute(
                PluginInput::new("elf", "x86_64", 64, 128, 4194304, 3),
                &mut output,
            )
            .unwrap();
        assert_eq!(output.as_str(), "{\"schema_version\":1,\"plugin\":\"metadata-summary\",\"format\":\"elf\",\"architecture\":\"x86_64\",\"bitness\":64,\"file_size\":128,\"entry_point\":4194304,\"section_count\":3}");
    }
}

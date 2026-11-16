# Migration notes

## 0.1.0 baseline to the Phase 09 workspace

The audited project baseline and packaged executable are version `0.1.0`; a verified v1.0.0 release was not present. This update does not change the workspace version, NIR schema v1, existing CLI commands, or native C ABI v1.

Existing PE/ELF inspection, profiling, module-listing and UI commands keep their prior inputs and output contracts. New commands are `nexus plugins` and `nexus export <file> --plugin metadata-summary`.

### Adding a Rust plugin

Plugins implement `nexus_plugin_api::Plugin`, declare a plugin SemVer and inclusive host API range, then register a boxed implementation with `nexus_plugin_host::PluginHost`. The current input is metadata-only. This API is source-level Rust compatibility, not a stable binary ABI, so compile plugins with the host's Rust toolchain and dependency graph. See [PLUGIN_SDK.md](PLUGIN_SDK.md).

There is no migration from a separately distributed v1.0.0 API, plugin directory, dynamic library format, saved plugin configuration, or analysis-pass interface because those were not implemented or released in the audited baseline.

## Compatibility policy

The plugin contract starts at API version 1. Additive methods should be avoided on public traits because they can break downstream implementations. Changes that alter the trait contract, input meaning, or output writer semantics require a plugin API version increment and a documented migration. A host accepts only plugin descriptors whose declared inclusive range contains the host API version. Plugin SemVer describes the plugin implementation, not the host API.

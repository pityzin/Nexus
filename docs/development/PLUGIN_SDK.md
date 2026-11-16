# NEXUS Rust Plugin SDK

## Scope

The workspace contains two plugin crates:

- `nexus-plugin-api`: versioned descriptor, metadata input, extension-point enum, plugin trait, and bounded output writer.
- `nexus-plugin-host`: compatibility checks, registration, enable/disable state, deterministic listing, input/output limits, and panic/error conversion.

The current extension point is `MetadataExport`. It receives normalized PE/ELF metadata only. Analysis-pass APIs are deferred until the corresponding decoder and analysis subsystems exist.

## Implement a plugin

The buildable example is `examples/plugins/metadata-summary`. It implements the SDK trait and emits JSON from the input fields:

```rust
use nexus_plugin_api::{
    ExtensionPoint, Plugin, PluginDescriptor, PluginInput, PluginOutput,
    PLUGIN_API_VERSION,
};

struct Summary;

impl Plugin for Summary {
    fn descriptor(&self) -> PluginDescriptor {
        PluginDescriptor::new(
            "metadata-summary", "1.0.0", PLUGIN_API_VERSION, PLUGIN_API_VERSION,
            ExtensionPoint::MetadataExport, ["export.metadata-summary"],
        ).expect("valid static descriptor")
    }

    fn execute(
        &self,
        input: PluginInput<'_>,
        output: &mut PluginOutput,
    ) -> Result<(), String> {
        output.write_str(input.format()).map_err(|error| error.to_string())
    }
}
```

Register it with `PluginHost::register(Box::new(Summary))`. Hosts can call `disable(id)`, `enable(id)`, `list()`, and `execute(id, input)`. Registration and runtime failures return `HostError` variants.

## Build and test

From a configured Windows Developer PowerShell or supported Rust environment:

```powershell
cargo test -p nexus-plugin-api -p nexus-plugin-host -p nexus-example-metadata-plugin
cargo run -p nexus-cli -- plugins
cargo run -p nexus-cli -- export .\sample.exe --plugin metadata-summary
```

The CLI example requires a PE or ELF file that the loader accepts. The plugin is statically linked; this workspace does not yet publish the SDK as a registry package or load separately compiled dynamic libraries.

## Compatibility and safety

Declare the lowest and highest host API versions supported. Plugin IDs and capabilities use ASCII letters, digits, `.`, `_`, or `-`; plugin versions use a SemVer-shaped syntax. The host enforces configurable hard-bounded metadata and output limits.

This SDK does not sandbox plugins. Only use trusted code. It does not promise binary compatibility between independently compiled Rust toolchains. No C ABI plugin bindings are provided because the current extension boundary is Rust-only and a C ABI would need a separate ownership, callback, and version contract.

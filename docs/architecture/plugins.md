# Plugin architecture

## Current status

The host exposes one implemented extension point: `metadata_export`. The CLI builds the `metadata-summary` example plugin statically and makes it available through `nexus plugins` and `nexus export <file> --plugin metadata-summary`. No instruction decoder, CFG, symbol processor, relocation processor, or cross-reference pass exists to extend. Those are not advertised as plugin hooks.

## API contract

The public Rust contract is in `nexus-plugin-api`:

- `PLUGIN_API_VERSION` identifies the plugin host contract (currently `1`).
- `PluginDescriptor` contains an identifier, plugin SemVer, inclusive minimum/maximum host API version, extension point, and sorted unique capability identifiers.
- `PluginInput` is a non-exhaustive, read-only metadata-only value constructed with `PluginInput::new`: format, architecture, bitness, file size, entry point, and section count.
- `Plugin::execute` receives that value and a `PluginOutput` writer with a host-defined byte limit.
- Plugins return a `Result`; the host bounds error text before exposing it.

The input contains no raw executable bytes, filesystem path, mutable NIR object, native handle, or callback to engine internals. Exporters define their own output schema; the included example emits schema version 1 JSON.

## Host lifecycle and limits

`nexus-plugin-host` owns registered plugin instances. Registration validates descriptor construction and host API compatibility before mutating the registry. Duplicate IDs are rejected. Plugins start enabled and can be enabled or disabled through the host API. Listing uses a `BTreeMap`, so plugin order is stable. Execution targets one enabled plugin by ID; plugin failures do not unwind across the host call boundary when compiled with panic unwinding.

Defaults cap the registry at 64 plugins, metadata input at 1 MiB, and output at 1 MiB. Hard limits prevent host configuration above 256 plugins or 16 MiB input/output. Output writes use fallible reservation and enforce the byte cap before appending.

## Trust model

These are **trusted in-process Rust extensions, not sandboxed plugins**. `catch_unwind` isolates ordinary Rust unwinding panics only. It cannot stop a plugin that loops forever, allocates outside the bounded output writer, spawns threads, invokes unsafe/native code, terminates the process, or uses `panic=abort`. The host has no per-plugin CPU budget, memory quota, unload hook, or security boundary. Do not load code from untrusted authors.

Plugins are registered by the host through Rust objects. There is no scanning of plugin directories, `dlopen`/`LoadLibrary`, subprocess protocol, C ABI plugin table, or WASM runtime. C ABI bindings are not justified for this first API because only Rust hosts are implemented and a C interface would require a separately versioned ownership and callback contract.

## Determinism and compatibility

The host checks the descriptor's inclusive supported API range against its API version. Plugin version strings use a dependency-free SemVer syntax check. The host does not promise stable Rust binary compatibility across compiler versions; plugins must be built with the host's Rust dependency graph/toolchain. The host invokes a selected plugin once, synchronously. Listing and registered IDs are deterministic, but plugin output determinism is the plugin author's responsibility.

API additions should be additive where possible. A breaking trait or input contract requires incrementing `PLUGIN_API_VERSION`; hosts must reject a plugin whose range excludes the current version. No plugin registry is persisted across application launches.

No Phase 09 plugin performance baseline or before/after comparison is claimed. The exporter handles a small fixed metadata record, and no optimization was made. The existing Phase 06 parser/FFI/allocation baselines are unchanged and do not measure plugin dispatch.

See the [SDK guide](../development/PLUGIN_SDK.md) for the example and build/test commands.

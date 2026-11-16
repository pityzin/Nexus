# Metadata summary example plugin

This statically linked trusted Rust plugin implements the `metadata_export` extension point using normalized format, architecture, bitness, file size, entry point, and section count fields. It emits deterministic JSON with schema version 1. It does not receive binary bytes or access engine internals.

Run it through the workspace CLI:

```powershell
cargo run -p nexus-cli -- plugins
cargo run -p nexus-cli -- export .\input.exe --plugin metadata-summary
```

It is an SDK example, not a dynamically loadable library or sandboxed extension.

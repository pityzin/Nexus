# ADR 0005: Trusted in-process Rust plugins

## Status

Accepted for the current metadata-export extension point.

## Context

The module registry contains descriptive capability metadata, but there was no executable extension API. The current engine parses PE/ELF metadata and produces NIR v1, while instruction decoding and advanced analysis passes are not implemented. A broad plugin API would imply engine stages that do not exist. The project also has no sandbox runtime or process protocol, and a native plugin can access all process resources.

## Decision

Add separate `nexus-plugin-api` and `nexus-plugin-host` crates. Expose one extension point, metadata export, through a versioned Rust trait. Plugins receive a small immutable metadata value, not file bytes, paths, internal engine objects, or native handles. The host validates plugin API compatibility, IDs, versions and capabilities; tracks enabled state; enforces plugin-count, input and output limits; catches Rust unwind panics; and lists plugins in stable identifier order.

The example is statically linked into the CLI. Do not load arbitrary dynamic libraries. Treat every plugin as trusted code; the in-process host is not a sandbox. No C ABI plugin interface is added because there is no C plugin host and no stable ownership/callback contract for this extension point.

## Consequences

- Rust hosts can implement and register metadata exporters against a small SDK.
- Plugin API breaking changes can be rejected through explicit inclusive host API version ranges.
- `nexus plugins` and `nexus export` exercise a real implementation.
- Only metadata exporters are supported. Decoder, analysis-pass, symbol, relocation and visualization extension points remain future work until their underlying systems exist.
- Input/output caps do not constrain plugin CPU time, arbitrary allocations, spawned threads, native code, process termination, or aborting panics. Untrusted plugins must not be loaded.
- Rust plugins are source/toolchain compatible APIs, not a promised stable binary ABI.

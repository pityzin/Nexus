# Architecture decision records

Record significant decisions here as `NNNN-short-title.md`. Each record should include context, decision, consequences, and status.

## Accepted decisions

- Rust is the orchestration and safe memory-view layer.
- Native boundaries use a versioned C ABI with fixed-width values and explicit error codes; C++ implementation details remain private.
- Assembly is a leaf routine behind the C++ implementation and has a portable fallback. NASM win64 is preferred when installed; MASM supports native Windows development without NASM.
- Phase 01 registers format capability names without claiming to implement parsers.
- [ADR 0002](0002-bounded-loader-and-nir.md): use a bounded owned file snapshot and NIR v1 instead of unsafe file mappings.
- [ADR 0003](0003-stateless-abi-and-borrowed-memory.md): keep ABI v1 stateless and defer custom allocation/worker infrastructure until measured workloads justify them.
- [ADR 0004](0004-loopback-ui.md): use an embedded local browser workbench while Tauri dependencies are unavailable offline; defer native desktop packaging.
- [ADR 0005](0005-trusted-rust-plugins.md): expose a narrow metadata-only Rust extension point and require plugins to be trusted in-process code.

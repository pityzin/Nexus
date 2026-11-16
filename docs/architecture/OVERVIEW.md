# Architecture overview

NEXUS separates orchestration, contracts, bounded input access, module metadata, and native code. The Phase 01 call path is:

```text
nexus CLI -> nexus-core -> nexus-binary-loader -> PE / ELF parsers -> NIR v1
                     \-> nexus-registry / bounded file reader
                     \-> versioned C ABI -> C and C++20 -> assembly or portable fallback
```

Rust is the orchestration layer: safe ownership and explicit result types keep ordinary core code free of unsafe operations. The API crate defines shared, versioned types. The memory crate only borrows caller-owned bytes and validates every range. The registry validates module descriptors and host API compatibility.

Native interoperability uses a C ABI because its fixed-width values and error codes remain usable across C, C++, and Rust compiler boundaries. C++ classes and standard library types never cross the ABI. Unsafe Rust is limited to the documented calls in `nexus-core`; the native functions retain no pointers. The current ABI is stateless and reentrant, with ownership rules and error codes recorded in [ABI v1](abi.md).

## Implemented and deferred

Implemented: CLI, versioned module metadata, safe read-only byte views, bounded file snapshots, PE32/PE32+, ELF32/ELF64 little/big-endian metadata parsing, NIR v1, an ABI version function, bounded checksum, integer addition, native/Rust parser tests, and C/C++ ABI consumer tests. Mach-O parsing, instruction decoding, symbols, relocations, CFG, and cross-reference/data-flow analysis are deferred. The module registry advertises only the implemented PE and ELF parsers.

The current memory model borrows caller-owned slices for temporary views and uses a bounded owned snapshot for file input. There is no custom allocator, arena, memory pool, or parallel analysis worker pool. Such components remain deferred until an analysis workload exists and measurements justify their cost.

Profiling is isolated in `nexus-profiling` and invoked only by explicit CLI profiling/benchmark commands. Current reports measure file snapshot and PE/ELF metadata parse wall time, native ABI call batches, and bounded allocation batches; they retain raw samples, calculate summary statistics, and attach host/build metadata. They do not add instrumentation to normal parsing paths. CPU sampling, allocator telemetry, and Phases 03/04 analysis-stage measurements are unavailable.

The optional Phase 07 interface is a static frontend embedded into the CLI and served from a loopback-only Rust bridge. It consumes PE/ELF metadata and NIR v1 results from the existing parser, with a strict upload cap and same-origin checks. It does not expose a shell or arbitrary filesystem paths. Tauri packaging is deferred; see [UI architecture](ui.md) and [ADR 0004](ADR/0004-loopback-ui.md).

Python, Zig, Haskell, and OCaml roles are reserved for later testing, native utilities, functional transformations, and intermediate representations. WebAssembly visualization is a future isolated interface. No placeholder executables or language stubs are shipped.

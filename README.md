<<<<<<< HEAD
# NEXUS

**Release status:** the audited source remains version `0.1.0`; the requested v1.0.0 and v2.0.0 milestones are not release-ready. Instruction decoding and Phase 04 advanced analysis are not implemented. See the [release-readiness audit](docs/RELEASE_READINESS.md) for verified scope, test results, limitations, and local artifact checksums.

**Modular Binary Analysis & Execution Engine** is a cross-platform framework for static executable inspection. Phase 01 provides the Rust workspace, stable C ABI, bounded memory abstraction, module registry, and CLI. Phase 02 adds a bounded read-only binary loader, PE32/PE32+ and ELF32/ELF64 parsers, NIR v1, and inspection commands. It never executes analyzed files.

## Current capabilities

- `nexus --version`, `nexus --help`, `nexus info`, `nexus modules`, and `nexus benchmark --help`.
- A 512 MiB default input limit and checked range access in `nexus-memory`.
- An environment-configurable input limit (`NEXUS_MAX_INPUT_BYTES`, capped at 2 GiB) and JSON-formatted CLI errors.
- Module name, version, capability, duplicate, and API compatibility validation.
- C ABI version query, checked byte checksum, and signed integer addition.
- An x86-64 leaf assembly routine: NASM win64 when available, MSVC MASM on Windows as a local fallback, otherwise a portable C++ implementation.
- A versioned, stateless C ABI with explicit borrowed-pointer ownership, null checks, output-on-success behavior, and concurrent-call semantics.
- C, C++, and Rust ABI tests, including boundary/error cases and concurrent native calls.
- Opt-in `profile` and `benchmark` CLI commands for the implemented PE/ELF metadata parser, with raw timing samples and statistical summaries.
- Explicit microbenchmarks for native C ABI addition throughput and bounded 4 KiB allocate/free pairs; allocator telemetry is not implemented.
- A loopback-only browser workbench for real PE/ELF metadata, sections, NIR v1, parser timing, and imported Phase 06 benchmark reports (`nexus ui`).
- Bounded read-only PE/ELF loading with checked endian-aware reads and a default 512 MiB / hard 2 GiB file limit.
- NIR v1 metadata and deterministic JSON for inspection, headers, and section output.
- CLI commands `inspect`, `headers`, `sections`, and `info <file>`.
- A versioned Rust plugin SDK and bounded in-process host for metadata export, with a compiled example exporter (`nexus plugins`, `nexus export <file> --plugin metadata-summary`). Plugins receive normalized metadata only, not raw file bytes or engine references.

The module registry advertises only the implemented PE and ELF parsers. Mach-O parsing is not implemented. The instruction engine and Phase 04 analysis layers (symbols, relocations, CFG, cross-references, data flow) are not implemented.

The Phase 09 plugin host currently supports only the metadata export extension point. Plugins are statically linked trusted Rust code; dynamic discovery and sandboxing are not implemented. See the [plugin architecture](docs/architecture/plugins.md), [SDK guide](docs/development/PLUGIN_SDK.md), and [migration notes](docs/development/MIGRATION.md).

Run `npm.cmd run dev --prefix apps/ui` from PowerShell to start the local workbench at **http://localhost:3001**. Port 3001 is fixed; if it is occupied, NEXUS reports the conflict and does not stop other processes or switch ports. The UI includes real PE/ELF metadata and the metadata-summary plugin export. Tauri packaging is deferred because its dependencies are unavailable in the offline Cargo registry; disassembly and advanced-analysis panels remain unavailable. See the [Phase 10 status report](docs/PHASE10_STATUS.md), [web interface guide](docs/development/WEB_INTERFACE.md), and [UI architecture](docs/architecture/ui.md).

## Architecture

Rust is the orchestration layer because its ownership and type system make resource boundaries and module contracts explicit without requiring unsafe code in the general core. The narrow, versioned C ABI provides a stable boundary to native components without exposing C++ object layout. C owns the ABI version entry point; C++20 implements validated native utilities; assembly is isolated behind that ABI. See [architecture overview](docs/architecture/OVERVIEW.md) and [module boundaries](docs/architecture/MODULES.md).

Python is reserved for test and automation tooling. Zig, Haskell, OCaml, and WebAssembly have no Phase 01 implementations. Their planned roles and current status are documented in the roadmap.

The loader, parser boundaries, and NIR contract are described in [binary-loader architecture](docs/architecture/binary-loader.md), [NIR v1](docs/specifications/nir.md), and [CLI usage](docs/development/CLI.md). Native ownership and error rules are in the [C ABI reference](docs/architecture/abi.md). Profiling scope and report formats are documented in [profiling architecture](docs/architecture/profiling.md) and [benchmark methodology](docs/benchmarks/README.md). Format support and validation rules are in [PE](docs/formats/pe.md) and [ELF](docs/formats/elf.md).

## Build and test

Requirements: Rust stable with `x86_64-pc-windows-msvc`, Visual Studio Build Tools with MSVC x64 and Windows SDK, and CMake 3.20+. NASM is optional; without it Windows uses the equivalent MASM leaf routine. Cargo's native build script locates CMake on PATH or in a standard Visual Studio installation.

In a Visual Studio Developer PowerShell or a shell with MSVC and CMake on PATH:

```powershell
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cmake --preset windows-x64
cmake --build --preset windows-x64-release
ctest --preset windows-x64
cargo run -p nexus-cli -- --version
cargo run -p nexus-cli -- info
cargo run -p nexus-cli -- modules
cargo bench --manifest-path benchmarks/rust/Cargo.toml --bench native_interop
```

The optional Criterion.rs suite remains dependent on its separate benchmark-only dependencies. A measured Windows x64 PE metadata baseline is committed under `benchmarks/baselines`; it covers only the existing parser and is not representative of instruction or CFG workloads. See [Windows setup](docs/development/WINDOWS.md), [build guide](docs/development/BUILDING.md), and [benchmark protocol](docs/benchmarks/README.md).

## Security model

Inputs are untrusted data. The loader checks a bounded read-only file snapshot, every table span, section names, and section byte ranges. It does not map images into process memory, resolve imports, use the network, or execute analyzed binaries. Review [SECURITY.md](SECURITY.md) before adding parsers or native interfaces.

## Roadmap

1. **Phase 01 — Foundation:** workspace, registry, native ABI, bounded memory, CLI, build and CI.
2. **Phase 02 — Binary loader and formats:** implemented PE32/PE32+ and ELF32/ELF64 parsing with NIR v1; Mach-O remains future work.
3. **Phase 03 — Instruction engine:** not implemented. Requires a real x86/x86-64 decoder and structured instruction model.
4. **Phase 04 — Advanced analysis:** not implemented. Depends on decoded instructions for CFG, instruction references, and data flow.
5. **Phase 05 — Systems engineering:** ABI contract tests are extended; Zig, custom allocation, and parallel analysis remain deferred.
6. **Phase 06 — Profiling:** in progress for the existing PE/ELF parser and file-read path. Broader subsystem profiling requires the missing Phase 03/04 analysis engine.
7. **Later:** ARM64, Haskell/OCaml components, full reporting, and isolated WebAssembly visualization.

See [CHANGELOG.md](CHANGELOG.md) for the current release notes.
=======
# Nexus
**NEXUS** is a high-performance binary analysis and reverse engineering platform featuring advanced disassembly, control flow analysis, cross-referencing, profiling, multi-architecture support, plugins, SDK, CLI, and a modern desktop interface.
>>>>>>> 83cc6c726fecaad17951621e1a150068b5cc6173

# Changelog

## 0.1.0 — Phase 01 foundation

- Added the Rust API, memory, registry, core, and CLI crates.
- Added a C ABI and C++20 native library with x86-64 assembly integration and portable fallback.
- Added native, Rust, FFI integration, and CLI tests plus Windows/Linux/macOS CI workflows.
- No PE, ELF, or Mach-O parsing or instruction decoding is included yet.

## Phase 02 — Binary loader and parsers

- Added a bounded read-only file loader, endian-aware checked reader, PE32/PE32+ parser, ELF32/ELF64 parser, and NIR v1.
- Added `inspect`, `headers`, `sections`, and `info <file>` CLI commands with deterministic JSON output.
- Added controlled parser fixtures and malformed-input regression/stress tests.
- Mach-O, symbols, relocations, and disassembly remain deferred.

## Phase 05 — Systems engineering (in progress)

- Documented the v1 C ABI ownership, error, and concurrency contract.
- Added pure-C ABI consumer coverage and expanded C++ tests for bounds, output preservation, wraparound, and concurrent calls.
- Expanded the Criterion suite to compare scalar and native ABI addition/checksum workloads; measurements are pending because the local Cargo cache is missing `anes` and registry access is unavailable.
- Kept borrowed memory views and bounded owned file snapshots; no custom allocator, Zig layer, or analysis worker pool was added without a concrete workload and measurements.
- Phase 03 instruction decoding and Phase 04 analysis prerequisites remain incomplete, so Phase 05 is not complete.

## Phase 06 — Profiling and performance engineering (in progress)

- Added opt-in profiling and repeated PE/ELF metadata parse measurements with raw samples and statistical summaries.
- Added a same-environment JSON baseline comparator that reports candidate regressions only with sufficient samples and non-overlapping confidence intervals.
- Recorded a measured 1,000-sample Windows x64 release baseline for PE metadata parsing; no hot-path optimization was justified by this single workload.
- CPU sampling, allocator telemetry, CFG, symbol, relocation, cross-reference, and parallel workload profiling remain unavailable; Phases 03/04 analysis stages are not implemented.

## Phase 07 — UI and visualization (in progress)

- Added a loopback-only local workbench for actual PE/ELF metadata parsing, section exploration, NIR v1 export, parser timing, and imported benchmark reports.
- Added paged/filterable section rendering, bounded browser uploads, same-origin validation, a strict CSP, and explicit unavailable states for unimplemented analysis views.
- Tauri is deferred because it is absent from the offline Cargo registry; this UI is a local browser application, not a native desktop package.

## Phase 08 — Release readiness audit (2026-11-16)

- Revalidated the Windows x64 Rust workspace, parser and CLI/UI integration suites, frontend tests, native CTest suite, and optimized CLI smoke commands; detailed results are in `docs/RELEASE_READINESS.md`.
- Prepared a checksummed Windows x64 portable archive for the existing `0.1.0` feature set.
- NEXUS v1.0.0 is **not ready**: the instruction engine and advanced analysis phases are absent, the UI is browser-based, Linux release behavior was not verified locally, and Criterion/dependency auditing could not run because required Cargo registry access was unavailable.

## Phase 09 — Extensibility (in progress)

- Added a versioned Rust plugin API and bounded host registry with compatibility ranges, capability metadata, deterministic listing, enable/disable controls, panic/error conversion, and output limits.
- Added a statically linked metadata summary exporter and CLI discovery/export commands. Plugins receive normalized metadata only.
- Dynamic plugin loading, sandboxing, instruction/analysis extension points, additional architectures, and a v1.1.0 release remain incomplete.

## Phase 10 — Ecosystem maturity (audit; v2.0.0 not released)

- Fixed the local browser workbench to require `127.0.0.1:3001`, expose its actual compiled metadata-summary plugin, and report port conflicts without stopping another process or selecting a fallback port.
- Removed the unimplemented Mach-O parser from the registered module list; only PE and ELF parsers are advertised.
- Revalidated the Windows workspace, CLI/UI bridge, frontend, and native C/C++ tests. See `docs/PHASE10_STATUS.md` for exact results and outstanding release blockers.
- This source remains version `0.1.0`. Phases 03–04, desktop packaging, dynamic/sandboxed plugins, validated non-x86 backends, and Linux release verification remain incomplete; this is not a v2.0.0 release.

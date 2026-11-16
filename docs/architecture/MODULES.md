# Module boundaries

| Component | Responsibility | Current status |
| --- | --- | --- |
| `nexus-api` | Shared API version and validated capability identifiers | Implemented |
| `nexus-memory` | Read-only byte views, checked ranges, resource limits | Implemented |
| `nexus-registry` | Module metadata, duplicate checks, API compatibility | Implemented |
| `nexus-plugin-api` | Versioned trusted Rust plugin contract and metadata-only input/output types | Implemented; API v1, metadata export only |
| `nexus-plugin-host` | Compatible plugin registration, enable/disable, deterministic listing, bounded output | Implemented for trusted statically linked plugins; no sandbox |
| `nexus-example-metadata-plugin` | JSON summary exporter for normalized PE/ELF metadata | Implemented and built into the CLI |
| `nexus-core` | Orchestration, default registry, narrow native calls | Implemented |
| `nexus-cli` | CLI inspection, profiling, benchmark commands, and loopback UI bridge | Implemented for the existing engine capabilities |
| `nexus-profiling` | Bounded opt-in wall-time samples and statistical summaries | Implemented for parser, native ABI, and bounded allocation benchmarks; no CPU sampling |
| `nexus-nir` | Format-neutral metadata schema and deterministic JSON | Implemented, schema v1 |
| `nexus-binary-loader` | Bounded read-only file snapshots, format detection, PE/ELF parsing | Implemented |
| C ABI / C++ | Versioned fixed-width ABI, validation, bounded checksum, integer helper | Implemented; stateless ABI v1 |
| C/C++ ABI tests | C and C++ consumer conformance, error limits, concurrent calls | Implemented; Windows x64 verified |
| PE32 / PE32+ | Headers, directories, sections, permissions | Implemented |
| ELF32 / ELF64 | Identification, header tables, sections, names, permissions | Implemented |
| Mach-O | File-format parsing | Deferred; not registered as an available parser |
| Instruction engine | x86/x86-64 decoding and instruction NIR | Deferred; Phase 03 prerequisite is missing |
| Symbols / relocations / CFG / xrefs / data flow | Advanced static analysis | Deferred; Phase 04 prerequisite is missing |
| Zig / Haskell / OCaml / WebAssembly | Independent utilities, transformations, IR, visualization | Deferred; no needed workload or local toolchain |

Modules register a bounded ASCII identifier, semantic version string, host API version, and capability set. Registration rejects duplicates and API mismatches. The analysis-module registry remains descriptive metadata; executable extension code uses the separate `nexus-plugin-host` API. Dynamic library loading and sandboxing are not implemented.

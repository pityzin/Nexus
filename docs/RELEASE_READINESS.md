# NEXUS Release Readiness Audit

Audit date: 2026-11-16  
Audited version: `0.1.0`  
Target requested: `v1.0.0`

## Decision

**NO-GO for NEXUS v1.0.0.** The checked-in source and executable are `0.1.0`, and required product capabilities from Phases 03 and 04 do not exist. Phase 07 is a browser workbench rather than a native desktop application. Changing the version label or describing these features as complete would be misleading. A tested Windows x64 portable package for the implemented `0.1.0` feature set is prepared locally; this is not a v1.0.0 release.

The workspace has no `.git` repository metadata, so a source commit identifier and a clean/dirty working-tree assertion cannot be recorded. No public release was published.

**Phase 09 update:** this audit predates the plugin extension changes. The workspace now contains a versioned metadata-export SDK, bounded trusted in-process host, and a compiled example exporter; see `docs/architecture/plugins.md`. This does not resolve the missing Phase 03/04 release blockers or change the project version.

The Phase 09 regression run on 2026-11-16 passed `cargo fmt --all -- --check`, strict Clippy, 39 Rust tests, 15 Python integration tests, 3 frontend tests, 2 native CTest tests, `cargo doc --workspace --no-deps`, and an optimized Windows x64 release build. The exact packaged CLI listed and executed the example exporter against a controlled ELF64 fixture. The updated ZIP contents and every SHA-256 entry were verified. No plugin performance comparison or additional architecture validation was performed.

## Phase validation matrix

| Phase | Actual implementation | Local validation | Release status |
| --- | --- | --- | --- |
| 01 Foundation | Rust workspace, CLI, module registry, bounded memory, versioned C ABI, C/C++20 and x64 assembly/fallback | Rust and native tests passed; Rust-to-native ABI integration passed | Implemented subset validated |
| 02 Binary core | Read-only bounded loader; PE32/PE32+, ELF32/ELF64 metadata and NIR v1 | Controlled parser fixtures and malformed input tests passed | Implemented subset validated; Mach-O unsupported |
| 03 Instruction engine | No decoder, instruction model, operand decoder or disassembly implementation | No tests exist for this absent subsystem | **Blocker** |
| 04 Advanced analysis | No instruction NIR transforms, CFG, symbols, relocations, xrefs or data-flow engine | No tests exist for these absent subsystems | **Blocker** |
| 05 Systems engineering | C ABI, native library, assembly leaf routine and portable fallback; no Zig integration, custom allocator or worker pool | FFI/native tests passed | Partial; not a v1-complete phase |
| 06 Profiling | Parser timing/statistics and limited ABI/allocation microbenchmarks; no CPU or allocator telemetry | Profiling tests passed; Criterion benchmark blocked by Cargo cache write denial | Partial; broad baselines unavailable |
| 07 UI | Loopback browser workbench for real PE/ELF metadata, sections, NIR metadata and imported benchmark reports | UI route tests and 3 frontend tests passed | Partial; no desktop package, disassembly, CFG, symbols, xrefs or live subsystem profiling |
| 08 Release hardening | Local Windows Release CLI package, checksum, audit record | Results below | **v1.0.0 not ready** |

## Validation results

Commands run from the repository root on Windows 10 x64 (build 19045), target `x86_64-pc-windows-msvc`, Rust stable `1.98.0`:

| Check | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all -- --check` | PASS | Workspace formatting check |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | No warnings |
| `cargo test --workspace` | PASS | 28 tests across unit, parser, CLI and FFI integration suites |
| `python -m unittest discover -s tools/python -v` | PASS | 14 CLI/UI/benchmark utility tests against the distribution executable |
| `npm.cmd test --prefix apps/ui` | PASS | 3 frontend data/validation tests |
| CTest in `build/windows-x64` | PASS | 2 native C and C++ tests; CTest was invoked from the installed Visual Studio CMake bundle |
| `cargo build --workspace --release` | PASS | Incremental optimized Windows MSVC build |
| Release CLI smoke commands | PASS | `--version`, `--help`, `info`, `modules`, `ui --help` exited successfully |
| Release PE identity | PASS | `dumpbin` reports x64 machine `8664`, Windows CUI subsystem |
| Release executable dependency check | PASS | Imports include `VCRUNTIME140.dll`; this runtime DLL is bundled in the portable directory |
| Criterion `native_interop` benchmark | BLOCKED | Cargo failed to unpack `aho-corasick v1.1.5` into the user Cargo registry with Windows access denied (OS error 5); no benchmark numbers were produced |
| `cargo audit` | BLOCKED | `cargo-audit` is not installed; registry access/cache writes are unavailable in this environment |
| Clean reproducible rebuild | NOT VERIFIED | Build succeeded incrementally; a clean build from a separate target directory was not established |
| Linux build/runtime | NOT RUN | No Linux host or WSL was assumed; CI has a Linux test matrix but no CI execution evidence is available locally |
| Visual UI inspection | NOT RUN | Automated HTTP/frontend tests passed; no visual browser inspection result is claimed |

The Rust test count is 28: API 1, binary parser integration 9, CLI/UI bridge 7, core 2, native FFI 1, memory 1, NIR 1, profiling 5, registry 1. No fuzz campaign or sanitizer run was performed. Existing deterministic malformed-input parser tests are regression tests, not exhaustive fuzzing.

## Security review

- The parsed file path uses read-only input handling and bounded sizes. PE and ELF parser tests cover truncated and inconsistent tables. The implementation does not execute analyzed files.
- The native ABI is small and stateless; C/C++ values use fixed-width C-compatible types. Rust unsafe usage is confined to FFI calls in `nexus-core`; no Rust panic-catching API is exposed across the ABI. Native tests cover null/bounds/error behavior and concurrent calls.
- The optional UI binds to IPv4 loopback, caps headers and uploads, validates Host/Origin, rejects transfer encoding and duplicate lengths, and applies a 30-second socket read/write timeout. It accepts selected bytes, not paths or commands. It is a synchronous single-connection server, so a local client can temporarily occupy the single handler up to the socket timeout; it is not intended as a network service.
- The frontend uses a restrictive CSP and text rendering for untrusted values. No Tauri permission model exists because the app is not Tauri-based.
- No Zig, Haskell, OCaml, WebAssembly, Mach-O parser, instruction decoder, or advanced analysis boundary exists in this source tree to audit.
- A dependency vulnerability database check was unavailable. Cargo workspace dependencies are predominantly local crates and the standard library; the separate Criterion benchmark dependency graph was not fully fetched or audited.
- No exhaustive fuzzing, Miri, ASan, or Windows Application Verifier run was made. These remain open verification items rather than implied passes.

## Build and run

Requirements verified here: Rust stable `1.98.0`, Cargo, `x86_64-pc-windows-msvc`, Python 3.14.6, Node 24.18.0, and Visual Studio 2026 Build Tools/MSVC 14.52.37025. CMake is installed inside Visual Studio but is not on this shell's PATH. NASM, Zig, GHC/Cabal, OCaml/Dune, and `cargo-audit` were unavailable; NASM has a MASM fallback on Windows. No system-wide PATH changes were made.

From a Visual Studio Developer PowerShell with CMake available to Cargo's build script:

```powershell
cargo build --workspace --release --locked --target x86_64-pc-windows-msvc
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python -m unittest discover -s tools/python -v
npm.cmd test --prefix apps/ui
```

The locally tested executable is `dist/nexus.exe`. Run `.dist\nexus.exe --help` or `.dist\nexus.exe ui`. `ui` starts a local browser workbench; it is not a native desktop application.

## Artifacts and checksums

The local portable archive is `dist/nexus-0.1.0-windows-x64-portable.zip`. It contains `nexus.exe`, `README.txt`, `LICENSE`, and the required `vcruntime140.dll`. `dist/SHA256SUMS.txt` contains current SHA-256 digests for the executable, runtime DLL and archive. After the Phase 09 plugin integration, the executable is 451,072 bytes; it remains version `0.1.0` and launched successfully during plugin smoke testing.

These artifacts are for the implemented `0.1.0` subset. They must not be renamed or advertised as `v1.0.0` artifacts.

## Release blockers

1. Implement and independently validate an instruction decoder before claiming Phase 03.
2. Implement and validate NIR instruction transforms, CFG, symbols, relocations, xrefs and data-flow before claiming Phase 04.
3. Choose and ship a supported desktop application architecture, or explicitly change the v1 product scope; the current UI is browser-only.
4. Make the release pipeline run tests and package/checksum artifacts. The current `release.yml` only builds the matrix binaries and does not upload release artifacts.
5. Establish clean Windows and Linux builds and runtime tests; local Linux was not available.
6. Restore access to the Cargo cache/registry, run the Criterion suite, and perform a dependency audit.
7. Run maintained fuzz targets and applicable sanitizers/Miri, and preserve any discovered crash inputs as regressions.
8. Add a repository commit/tag and verify version, source revision, package manifest and checksum provenance before any public release.

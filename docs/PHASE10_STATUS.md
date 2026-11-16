# Phase 10 ecosystem audit and status

## Release decision

NEXUS v2.0.0 is **not ready and has not been released**. The checked-in workspace and verified Windows artifact remain version `0.1.0`. This phase delivered a port-fixed browser workbench integration and removed one misleading capability declaration, but the roadmap prerequisites are not all implemented. The portable artifact in `dist/` is a verified `0.1.0` build, not a v2.0.0 release.

## Audit summary

The current implementation is a Windows-tested Rust workspace with a bounded, read-only PE32/PE32+ and ELF32/ELF64 metadata parser, NIR v1 metadata export, module registry, trusted statically linked metadata plugin, Rust-to-C ABI calls, C/C++ tests, and a local browser UI. Parsers do not execute inputs. The registry lists only PE and ELF parser modules.

The source tree does not contain an instruction decoder, instruction operands/disassembly, CFG, symbol or relocation parser, xref engine, or data-flow engine. Mach-O is unsupported. The plugin host is in-process and is not a sandbox; dynamic loading and the planned analysis extension points are absent. The UI is a browser workbench, not a desktop application. Zig, Haskell, OCaml and WebAssembly integrations are absent. These are release blockers against the v2.0.0 acceptance criteria, not completed features.

## Phase-by-phase evidence

| Roadmap area | Audited state | Local validation |
| --- | --- | --- |
| 01 Foundation | Rust workspace, CLI, C ABI, C/C++ library and x64 assembly/native fallback present | Rust tests, strict Clippy, formatting, CTest, and Rust-to-C/native integration passed on Windows x64 |
| 02 Binary core | Bounded read-only PE and ELF metadata parsing; NIR v1 | 9 parser integration/regression tests passed, including malformed input cases |
| 03 Instruction engine | Not present | Not applicable; release blocker |
| 04 Advanced analysis | CFG, symbols, relocations, xrefs, and data flow not present | Not applicable; release blocker |
| 05 Systems engineering | C ABI and native layer present; no Zig layer or custom allocator | ABI integration and native C/C++ tests passed; no claim of Zig integration |
| 06 Profiling | Wall-time parser/native/allocation benchmark commands and comparison tools present; not full CPU/allocator profiling | CLI benchmark/profile tests passed; no new Phase 10 comparative performance run was recorded |
| 07 UI | Rust-served local browser workbench for actual parser data | Frontend tests, HTTP smoke check on port 3001, and Python UI integration tests passed; no native desktop app |
| 08 Release engineering | Existing 0.1.0 portable Windows artifact refreshed | Release build, CLI smoke tests, runtime dependency bundle, archive, and SHA-256 manifest verified |
| 09 Extensibility | Versioned Rust plugin API/host and one compiled metadata exporter | Plugin API/host and exporter tests passed; no dynamic loading, sandbox, or plugin SDK ecosystem |
| 10 Ecosystem maturity | Port configuration and module capability accuracy addressed in this increment | Validation listed below; v2.0.0 acceptance remains unmet |

## Port 3001

The source UI binds only to `127.0.0.1:3001` and prints `http://localhost:3001/`. It does not choose a fallback port or terminate another process. The backend exposes parser status, module/plugin discovery, bounded parse input, and the real metadata-summary export. The documented Windows source startup command is:

```powershell
npm.cmd run dev --prefix apps/ui
```

The UI process was launched using this command. `GET http://localhost:3001/` returned HTTP 200, the page contained the plugin view, and API checks reported port 3001 and one compiled plugin. The process was then stopped and the port was confirmed free. A conflict regression test confirms a second server reports the conflict while the already-running server remains available.

## Validation performed on this Windows x64 host

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: passed.
- `cargo test --workspace --locked --offline`: passed, 41 Rust tests total across unit and integration targets.
- `python -m unittest discover -s tools/python -p 'test_*.py' -v`: passed, 17 tests against the release executable.
- `npm.cmd test --prefix apps/ui`: passed, 3 frontend tests.
- `node --check apps/ui/web/app.js` and `node --check apps/ui/web/data.js`: passed.
- CTest in `build/windows-x64`: passed, 2 native C/C++ tests.
- `cargo build --workspace --release --locked --offline`: passed for `x86_64-pc-windows-msvc`.
- The packaged CLI `--version`, `--help`, `info`, `modules`, and `plugins` commands returned exit code 0. The executable has an `MZ` signature and launched without missing-runtime errors on this host.
- The UI was reachable at `http://localhost:3001` and was shut down after the check.

The above is local Windows validation only. Linux build/runtime, desktop application packaging, exhaustive fuzzing, sanitizers/Miri, dependency advisory audit, full performance comparisons, and end-to-end UI visual interaction were not verified in this phase.

## Feature and platform support

| Area | Status |
| --- | --- |
| PE32/PE32+ and ELF32/ELF64 metadata | Implemented and locally tested |
| x86-64 native Windows host integration | Locally built/tested |
| Instruction-set backends (x86/x64/ARM/AArch64/RISC-V) | Not implemented as a decoder; architecture metadata alone is not backend support |
| Mach-O | Unsupported |
| Symbols, relocations, CFG, cross-references, data flow | Unsupported |
| Plugins | Trusted, statically linked Rust metadata-export plugins only; no sandbox |
| UI | Loopback browser interface at fixed port 3001; no native desktop package |
| Linux x64 | CI configuration may exist, but no local Phase 10 artifact/runtime verification; not claimed here |
| WebAssembly, Zig, Haskell, OCaml | Not integrated |

## Remaining blockers and technical debt

1. Implement and validate the instruction engine and advanced static analysis stages before claiming the original analysis platform scope.
2. Add Mach-O only if a real parser is implemented; it is deliberately absent from the module registry meanwhile.
3. Design plugin sandboxing or keep plugins restricted to trusted compiled extensions; current in-process plugins have host-process privileges.
4. Build a desktop app only after a supported and tested native desktop architecture is available.
5. Validate Linux release builds/runtimes and test the UI/browser workflow end to end on supported platforms.
6. Establish full fuzz, sanitizer, Miri, dependency audit, and reproducible comparative benchmark evidence.
7. Keep semver at the implemented version until release scope and acceptance criteria are met.

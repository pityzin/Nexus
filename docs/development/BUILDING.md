# Building NEXUS

## Prerequisites

- Rust stable and Cargo; on Windows install the `x86_64-pc-windows-msvc` target.
- CMake 3.20+ and a native compiler. Windows uses MSVC x64 and a Windows SDK; Linux/macOS use their normal C++20 toolchains.
- NASM is optional. CMake selects it for Windows x64 when present. With MSVC and no NASM, it uses the MASM equivalent; on other targets it uses the C++ fallback.
- Criterion.rs is fetched from crates.io when Cargo resolves the benchmark dependency.
- CMake 3.25+ is needed for the provided CMake presets; direct CMake builds and the Cargo native build script support CMake 3.20+.

## Windows PowerShell

Open **Developer PowerShell for Visual Studio** with the x64 tools installed. The Cargo build script can locate CMake from PATH or a standard Visual Studio Build Tools installation. Then run:

```powershell
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cmake --preset windows-x64
cmake --build --preset windows-x64-release
ctest --preset windows-x64
cargo run -p nexus-cli -- --help
cargo run -p nexus-cli -- ui
```

CTest builds and runs both a C-language ABI consumer and a C++ consumer, including concurrent-call tests. `cargo test --workspace` includes the Rust-to-native ABI integration test.

The Windows preset uses NMake from the Developer PowerShell environment. The optional `native` preset uses Ninja and requires Ninja to be installed.

## Linux and macOS

Install CMake and a C++20 compiler, then run `cmake -S . -B build/native`, `cmake --build build/native`, and `ctest --test-dir build/native --output-on-failure`. Cargo target options are provided by CI per platform; `.cargo/config.toml` defaults local Windows development to MSVC.

## Build behavior

`nexus-core/build.rs` configures CMake and builds `nexus_native`, then links that static library into Rust. Set `CMAKE` or `CMAKE_COMMAND` if CMake is not on PATH and is outside the normal Visual Studio installation location. CMake native tests are a separate CTest target and should be run explicitly. The standalone Criterion benchmark has its own manifest at `benchmarks/rust/Cargo.toml`; use `cargo bench --manifest-path benchmarks/rust/Cargo.toml --bench native_interop` when crates.io access is available.

Set `NEXUS_MAX_INPUT_BYTES` to a positive integer up to 2 GiB to change the default 512 MiB input budget. Invalid values cause `nexus info` to return a structured JSON error on stderr.

Parser tests generate minimal PE/ELF byte fixtures in memory; they do not launch the fixture data. Run `python -m unittest discover -s tools/python -v` with `NEXUS_BIN` set to the built CLI to exercise file-based commands and JSON output.

The local browser interface is a static frontend embedded in the CLI. It requires no npm install; `npm.cmd test --prefix apps/ui` runs its dependency-free utility tests. `nexus ui` starts the loopback bridge and prints the URL to open.

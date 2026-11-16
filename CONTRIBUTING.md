# Contributing to NEXUS

Read [the contributor build guide](docs/development/CONTRIBUTING.md) and [SECURITY.md](SECURITY.md) before opening a change.

- Keep Phase 01 interfaces small, versioned, documented, and testable.
- Treat every file as malformed or hostile input. Never execute analyzed files.
- Keep unsafe Rust at explicit FFI boundaries and document pointer ownership and lifetime assumptions.
- Preserve C ABI compatibility; expose fixed-width integer types and error codes, never C++ classes.
- Keep platform-specific code behind CMake or Rust target configuration.
- Add regression cases for parser or bounds bugs.
- Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and native CTest before submitting.
- Document benchmark environment and never present an unmeasured estimate as a result.

All project documentation and code comments intended for users should be in English.

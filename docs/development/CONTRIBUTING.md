# Development workflow

1. Read the root `CONTRIBUTING.md` and `SECURITY.md`.
2. Make the smallest change that preserves the public API and safe ownership boundaries.
3. Add focused unit, native, integration, and regression tests for changed behavior.
4. Run formatting, Clippy, Rust tests, native CMake tests, and the CLI commands listed in the build guide.
5. Record any platform-specific behavior and benchmark setup.

Do not add placeholder languages, pretend parsers, automatic execution, machine-specific paths, ignored errors, or undocumented unsafe blocks.

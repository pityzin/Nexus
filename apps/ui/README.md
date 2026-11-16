# NEXUS local workbench

This frontend is a dependency-free static browser application embedded into the `nexus` CLI at build time. It is served by `nexus ui` on the required loopback port 3001. Open `http://localhost:3001`. The browser selects local files explicitly and posts bytes to the Rust PE/ELF parser; no path, shell, or execution interface is exposed. Architecture labels in metadata identify parser machine fields only; NEXUS does not decode instructions for those architectures.

Implemented screens are Overview, Binary explorer, NIR metadata, Performance reports, and Plugin exports. The plugin screen discovers the statically compiled `metadata-summary` extension and runs it against the selected file through the Rust host. Disassembly, CFG, symbols, relocations, and cross-references remain unavailable because their engine stages do not exist in this repository.

To run:

```powershell
npm.cmd run dev --prefix apps/ui
```

This launches the Rust bridge and embedded frontend at `http://localhost:3001`. The server binds only to `127.0.0.1`. If port 3001 is occupied, startup exits with an error; it never selects another port or stops another process. Press Ctrl+C in the console to stop the server. UI uploads are capped at 128 MiB or the configured `NEXUS_MAX_INPUT_BYTES` value, whichever is lower.

The UI has no third-party packages or bundler build step; Cargo embeds the checked-in source assets into the executable. `npm.cmd run dev --prefix apps/ui` rebuilds/starts the Rust service. Run frontend utility tests with `npm.cmd test --prefix apps/ui`. The Rust bridge is covered by CLI unit tests and `tools/python/test_ui.py` black-box tests. Tauri packaging is deferred: Tauri is not available in the offline Cargo cache and is not vendored.

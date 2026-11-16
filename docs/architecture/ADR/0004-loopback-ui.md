# ADR 0004: Local browser workbench for Phase 07

- Status: Accepted for Phase 07; native wrapper deferred.
- Date: 2026-11-16.

## Context

The repository provides a Rust CLI and PE/ELF metadata APIs but no GUI framework. Tauri is preferred when its Rust dependencies and desktop tooling are available. The local Cargo registry is offline and does not contain Tauri, so adding it would make the application unbuildable in this environment. A WebView2 runtime is present, but the repository has no native WebView host bindings.

## Decision

Serve embedded static assets from a minimal Rust HTTP bridge bound to loopback and expose only a bounded binary-inspection endpoint plus capability status. The browser's file picker provides bytes explicitly. Do not expose filesystem paths, shell execution, or analyzed-binary execution. Keep the frontend and bridge separable so a later Tauri shell can use the same versioned UI API.

## Consequences

The interface is functional without frontend package dependencies and remains separate from the parser. It requires the user to open a local URL and does not satisfy native desktop packaging. Disassembly and advanced-analysis panels remain unavailable until the engine implements those features. Add a Tauri wrapper only when dependencies can be restored and the desktop package can be built and security-reviewed.

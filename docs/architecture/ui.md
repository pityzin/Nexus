# UI and desktop architecture

## Decision

The current UI is a dependency-free static workbench served by `nexus ui` from a Rust loopback HTTP server. The server binds only to `127.0.0.1:3001` and embeds the frontend assets at compile time. The user opens `http://localhost:3001` and selects files with the browser's explicit file picker. If port 3001 is unavailable, startup reports the conflict without selecting another port or terminating the listener.

Tauri was evaluated but is not available in the local offline Cargo registry. The WebView2 runtime is installed, but the Tauri crate/tooling cannot be built without fetching dependencies. Rather than add an unbuildable desktop shell, this phase delivers a working local interface and records native packaging as deferred. The loopback workbench is not represented as a native desktop executable.

## Components and communication

```text
Browser tab (apps/ui/web)
  ├── GET /, /app.js, /data.js, /styles.css
  ├── GET /api/v1/status
  └── POST /api/v1/inspect (selected file bytes)
         └── Rust UI bridge -> bounded PE/ELF parser -> NIR v1 JSON
```

The protocol is versioned (`protocol_version: 1`). Status responses report implemented and unavailable engine features. Inspect responses contain a parse duration and the exact NIR JSON serialized as a string so 64-bit values remain lossless in JavaScript. Errors have a stable `{error:{code,message}}` shape. The server does not accept filesystem paths or commands.

## Implemented views

- **Overview:** connection state, real engine capabilities, latest loaded metadata, parse duration, and recent imports held only in the current tab.
- **Binary explorer:** PE/ELF common metadata, format headers, section names, addresses, sizes and permissions. Section rows are paginated in groups of 50 and can be filtered.
- **NIR metadata:** exact NIR v1 JSON with copy and download actions.
- **Performance:** parser timing from the current import and validated imported Phase 06 benchmark JSON reports, with same-workload/environment comparison candidates.
- **Plugin exports:** discovered compiled plugin descriptors and an action that sends the selected file bytes to the Rust metadata-summary exporter, then allows JSON download.

Disassembly, CFG, symbols, imports/exports, relocations, xrefs, data flow, memory telemetry, settings, and graph export are shown as unavailable because the engine does not implement them. The UI does not manufacture data for these features.

## Security boundary

The server accepts only HTTP connections to its loopback-bound listener. It checks the `Host` and same-origin `Origin`, rejects transfer encoding and duplicate length headers, caps headers at 16 KiB, and caps the body to the configured input limit and 128 MiB. It parses bytes directly and never writes or executes the selected file. A strict CSP disallows inline scripts and cross-origin connections. Frontend values from filenames, parser strings, and reports are inserted with `textContent`; dynamic HTML is not used.

The page has no shell, arbitrary path, network, or persistent-session API. Imported benchmark reports are limited to 4 MiB each and 100 reports per tab. Browser upload can be cancelled; cancellation closes the connection and the server stops reading that request.

## Performance and limits

One synchronous local request is handled at a time. Static assets are embedded and section rendering is paginated. Input file content is buffered in the browser and server, so the UI upload cap is lower than the parser's 512 MiB default. No WebAssembly processing is used because the authoritative parser already runs in the Rust backend and no measurement justifies duplicating it.

This workbench requires manually opening the printed local URL and is not a signed installer or native Tauri application. Linux compilation may work through Cargo's existing cross-platform parser, but only Windows x64 is validated in this phase.

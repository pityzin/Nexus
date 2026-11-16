# NEXUS Web Interface

## Windows startup

From the repository root in PowerShell, run:

```powershell
npm.cmd run dev --prefix apps/ui
```

The script starts the Rust CLI's embedded frontend and backend on the required address **http://localhost:3001**. The listener binds to IPv4 loopback only. Keep the console open and press Ctrl+C to stop it. No separate frontend/backend ports or Node dependencies are used; Cargo embeds the checked-in frontend assets into the executable.

If port 3001 is in use, NEXUS exits with a clear error. It does not kill or inspect other processes, select a fallback port, or alter system port configuration. Close the process that owns port 3001 yourself and rerun the command.

## Real functionality

The browser talks to the Rust service through protocol v1. Supported workflows are real PE/ELF metadata parsing, headers, paged section exploration, NIR metadata export, parser timing, imported benchmark report viewing, and the built-in metadata-summary plugin. The plugin export posts selected bytes to the Rust parser and trusted plugin host. Files are never written or executed by the service.

Disassembly, CFG, symbols, relocations, cross-references, data flow, memory telemetry, settings, and native desktop packaging are not implemented. The navigation keeps unimplemented analysis views disabled; no simulated analysis is shown.

## Test

```powershell
npm.cmd test --prefix apps/ui
$env:NEXUS_BIN = (Resolve-Path 'dist\nexus.exe').Path
python -m unittest discover -s tools/python -v
```

The Python UI tests launch the backend, verify the port and protocol, exercise metadata/plugin endpoints, and verify that a second instance reports the port conflict without stopping the original server.

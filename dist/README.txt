NEXUS 0.1.0 - Windows x64 static binary workbench

NEXUS inspects supported PE and ELF files as data. It never executes analyzed files.

Run from PowerShell:
  .\nexus.exe --help
  .\nexus.exe --version
  .\nexus.exe info
  .\nexus.exe modules
  .\nexus.exe plugins
  .\nexus.exe inspect .\sample.exe
  .\nexus.exe sections .\sample.exe --format json
  .\nexus.exe export .\sample.exe --plugin metadata-summary
  .\nexus.exe ui

The browser workbench is served at http://localhost:3001. If that required port
is occupied, NEXUS reports the conflict and does not stop the other process or
select another port. To start from source: npm.cmd run dev --prefix apps/ui

The UI supports real PE/ELF parsing and a statically compiled metadata summary
plugin. Plugins execute in-process and are not sandboxed; only use trusted plugins.
This is a browser interface, not a native desktop application.

Disassembly, CFG, symbols, relocations, cross-references, and advanced static
analysis are unavailable. Supported architecture metadata is not instruction
backend support. This archive is version 0.1.0 and is not a v2.0.0 release.

The executable is x64 and uses the bundled vcruntime140.dll. Windows 10 system
Universal CRT API-set DLLs are required from the operating system.
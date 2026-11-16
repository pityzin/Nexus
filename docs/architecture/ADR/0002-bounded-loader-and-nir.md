# ADR 0002: Bounded file snapshots and NIR v1

- Status: Accepted
- Date: 2026-11-16

## Context

PE and ELF parsers need repeatable random access to untrusted file offsets. Phase 01 had safe borrowed byte views but no loader, parser, or NIR contract. Memory mapping would add unsafe lifetime and concurrent-truncation hazards; no mapping dependency is available in the supported offline build environment.

## Decision

Use a read-only `std::fs::File` snapshot in a size-bounded `Vec<u8>`, parse it through a checked endian-aware reader, and expose normalized metadata using a separate `nexus-nir` crate. Keep PE and ELF parsing in modules behind `nexus-binary-loader`.

## Consequences

The implementation makes one bounded input buffer and avoids parser copies, but does not provide memory-mapped I/O. The default cap is 512 MiB and the hard configurable ceiling is 2 GiB. Format-specific fields remain attached to the normalized record. Any future mapping implementation must preserve the same no-execution and bounds guarantees.

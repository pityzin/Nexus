# ADR 0003: Stateless native ABI and borrowed input memory

- Status: accepted
- Date: 2026-11-16

## Context

The current native layer exposes only integer addition and a bounded byte checksum. Neither operation needs persistent state, handles, callbacks, or native allocation. The Rust memory layer returns checked borrowed slices, and the file loader owns one bounded immutable snapshot.

## Decision

Keep ABI v1 stateless: callers own all buffers, native functions retain no pointers or allocate caller-visible memory, output values are written only on success, and concurrent calls are permitted. Keep input views borrowed and file ownership in `BinaryFile`. Do not add custom allocators, arenas, opaque handles, or a worker pool before a concrete analysis workload and benchmark justify them.

## Consequences

C and Rust consumers share fixed-width arguments and explicit status codes. Ownership does not cross the ABI, making cleanup unambiguous. C and C++ tests cover nulls, limits, output preservation, wraparound, and concurrent calls. Future stateful subsystems require a separately versioned handle API with explicit create/destroy operations and thread-safety rules.

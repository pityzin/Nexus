# Memory ownership and limits

NEXUS currently uses two explicit input models:

- `ReadOnlyImage<'a>` borrows an existing byte slice. Its lifetime is bounded by the caller, and range access checks addition and bounds before returning a view. It never allocates or mutates the input.
- `BinaryFile` owns one bounded snapshot of a read-only file. It checks the file length before reserving, uses fallible reservation, enforces the configured input ceiling, and exposes borrowed slices to parsers.

The normal file limit is 512 MiB and the configured hard ceiling is 2 GiB. Native checksum input has a separate 16 MiB ceiling. Native functions allocate no memory and retain no pointers.

No arena, bump allocator, pool, global allocator override, or allocation tracker is present. Current core operations either borrow input or make the single bounded file snapshot; an additional allocator would add ownership and failure modes without reducing measured allocation cost. When instruction and CFG workloads exist, benchmark their actual allocation profile before introducing analysis arenas or pools. Any such budget must be per analysis context, explicit, and fail with a resource-limit error rather than silently growing.

There is also no general worker pool yet. Independent analyses should first be measurable, then a bounded executor can be designed around cancellation and deterministic result ordering.

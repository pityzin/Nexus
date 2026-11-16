# Controlled parser fixtures

`modules/binary-loader/tests/parsers.rs` constructs minimal PE32, PE32+, ELF32 little-endian, ELF32 big-endian, and ELF64 little-endian inputs in memory using explicit field writers. These are synthetic metadata fixtures, not copied system executables and not intended to be launched. Malformed variants are made by changing offsets, signatures, counts, or truncating those fixtures. The fixture builders are the source of truth, so test inputs are reproducible without checked-in opaque executable files.

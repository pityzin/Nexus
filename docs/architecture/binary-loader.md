# Binary loader

`nexus-binary-loader` is a reusable Rust crate under `modules/binary-loader`. It opens paths read-only, checks the opened handle's metadata, reserves only within a configured size limit, reads a bounded snapshot, and passes borrowed slices to format parsers. The default maximum is 512 MiB; the absolute configured ceiling is 2 GiB. The CLI limit is set by `NEXUS_MAX_INPUT_BYTES`.

The loader currently uses one owned immutable byte buffer rather than a memory map. This avoids unsafe mapping lifetimes and races with a file that is truncated or modified while mapped. Parsing does not copy the input buffer. A future mapped implementation must prove the same bounds, lifetime, and mutation guarantees on Windows and other supported hosts before it can replace the safe path.

`BinaryReader` performs checked offset conversion, checked addition, range lookup, and explicit little/big-endian integer decoding. Format parsers validate table span arithmetic and hard-limit section/header counts before allocating metadata vectors. Section payload bytes are not decoded or executed.

Format detection accepts the `MZ` or ELF magic and dispatches to isolated PE and ELF modules. Unsupported magic, truncated records, invalid fields, overflow, oversized files, and I/O errors are returned as `BinaryError` values. PE/ELF section overlap is not used as a pointer or loaded into memory; valid in-file ranges are retained as metadata. Inconsistent overlap with format header tables is rejected where it would make parsed table interpretation ambiguous.

Both parsers return NIR v1. The loader is static-analysis only: it does not call a platform loader, map an executable image, resolve imports, contact a network, or launch the input.

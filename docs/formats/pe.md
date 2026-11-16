# PE parser (Phase 02)

The parser supports PE32 and PE32+ executable metadata from an immutable file snapshot. It validates the DOS `MZ` signature and 64-byte DOS header, `e_lfanew`, PE signature, 20-byte COFF header, optional-header class/size, data-directory table, section table, declared header size, COFF symbol/string table spans when present, and every nonempty section raw-data span.

## Exposed fields

- DOS header: `e_lfanew` (the remaining DOS stub is not interpreted).
- COFF: machine, section count, timestamp, symbol-table offset/count, optional-header size, and characteristics.
- Optional header: magic, entry-point RVA, image base, section/file alignment, image/header sizes, subsystem, DLL characteristics, directory count, and up to 16 data directories.
- Sections: name, virtual size/address, raw size/offset, characteristics, and normalized read/write/execute permissions.

Machine values commonly used for x86, x86-64, ARM, and ARM64 map to NIR architecture values; others remain numeric and map to `unknown`. The certificate directory is represented as a file offset; other directory values are RVAs.

## Limits and validation

At most 96 PE sections and 16 data directories are accepted. Required optional-header fields must fit the declared PE32/PE32+ header. The section table must fit within `SizeOfHeaders`, and `SizeOfHeaders` must fit in the file. Nonempty section data and a declared COFF symbol/string table must fit in the file. The parser does not allocate based on raw header counts without these caps and span checks.

Section alignment semantics, import/export contents, relocations, resources, certificates, symbols, and RVA-to-file mapping are not interpreted. Overlapping raw section ranges are preserved as metadata; they are never followed as pointers or executed.

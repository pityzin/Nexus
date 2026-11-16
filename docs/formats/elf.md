# ELF parser (Phase 02)

The parser reads ELF32 and ELF64 identification and headers with explicit little- or big-endian decoding. It extracts class, byte order, OS ABI, ABI version, file type, machine, version, entry point, flags, and program/section table offsets, entry sizes, and counts. Section names are resolved through a validated `SHT_STRTAB` when a section-name table is present.

## Sections

For each section, NIR retains the index, decoded name, type, flags, virtual address, virtual size, raw offset/size, and normalized permissions. `SHF_ALLOC` maps to NIR read, `SHF_WRITE` to write, and `SHF_EXECINSTR` to execute. `SHT_NOBITS` has no raw file bytes and is reported with raw size zero.

## Limits and validation

The ELF identification must be complete and use supported class, data encoding, and version. Class-specific header sizes and minimum program/section entry sizes are checked. Table counts are capped at 65,536; table multiplication/addition is checked; declared spans must be in-file and header tables cannot overlap each other or the ELF header. Non-`SHT_NOBITS` section contents must fit in the file and may not overlap header tables. Section-name indexes and NUL-terminated names are range-checked. Extended section/program counts and `SHN_XINDEX` are read from section header zero with bounds checks.

Program header entries are not decoded individually in this phase; their table metadata and byte span are validated. Dynamic linking, symbol tables, relocations, notes, compression, and load-segment mapping are not interpreted. No ELF image is mapped or executed.

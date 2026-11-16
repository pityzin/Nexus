use nexus_binary_loader::{
    parse_bytes, BinaryArchitecture, BinaryError, BinaryFile, BinaryFormat, BinaryLimits,
    Endianness, FormatHeaders,
};
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const PE: u32 = 0x80;
static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

fn put16(b: &mut [u8], at: usize, value: u16, e: Endianness) {
    b[at..at + 2].copy_from_slice(&if e == Endianness::Little {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    });
}
fn put32(b: &mut [u8], at: usize, value: u32, e: Endianness) {
    b[at..at + 4].copy_from_slice(&if e == Endianness::Little {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    });
}
fn put64(b: &mut [u8], at: usize, value: u64, e: Endianness) {
    b[at..at + 8].copy_from_slice(&if e == Endianness::Little {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    });
}

fn pe_fixture(is_64: bool) -> Vec<u8> {
    let optional_size = if is_64 { 240 } else { 224 };
    let optional = PE as usize + 4 + 20;
    let section = optional + optional_size;
    let mut b = vec![0_u8; 0x400];
    b[0..2].copy_from_slice(b"MZ");
    put32(&mut b, 0x3c, PE, Endianness::Little);
    b[PE as usize..PE as usize + 4].copy_from_slice(b"PE\0\0");
    let coff = PE as usize + 4;
    put16(
        &mut b,
        coff,
        if is_64 { 0x8664 } else { 0x014c },
        Endianness::Little,
    );
    put16(&mut b, coff + 2, 1, Endianness::Little);
    put32(&mut b, coff + 4, 0x12345678, Endianness::Little);
    put32(&mut b, coff + 8, 0, Endianness::Little);
    put32(&mut b, coff + 12, 0, Endianness::Little);
    put16(&mut b, coff + 16, optional_size as u16, Endianness::Little);
    put16(&mut b, coff + 18, 0x22, Endianness::Little);
    put16(
        &mut b,
        optional,
        if is_64 { 0x20b } else { 0x10b },
        Endianness::Little,
    );
    put32(&mut b, optional + 16, 0x1000, Endianness::Little);
    if is_64 {
        put64(&mut b, optional + 24, 0x140000000, Endianness::Little);
        put32(&mut b, optional + 108, 1, Endianness::Little);
        put32(&mut b, optional + 112, 0x3000, Endianness::Little);
        put32(&mut b, optional + 116, 40, Endianness::Little);
    } else {
        put32(&mut b, optional + 28, 0x400000, Endianness::Little);
        put32(&mut b, optional + 92, 1, Endianness::Little);
        put32(&mut b, optional + 96, 0x3000, Endianness::Little);
        put32(&mut b, optional + 100, 40, Endianness::Little);
    }
    put32(&mut b, optional + 32, 0x1000, Endianness::Little);
    put32(&mut b, optional + 36, 0x200, Endianness::Little);
    put32(&mut b, optional + 56, 0x2000, Endianness::Little);
    put32(&mut b, optional + 60, 0x200, Endianness::Little);
    put16(&mut b, optional + 68, 3, Endianness::Little);
    put16(&mut b, optional + 70, 0x140, Endianness::Little);
    b[section..section + 5].copy_from_slice(b".text");
    put32(&mut b, section + 8, 0x38, Endianness::Little);
    put32(&mut b, section + 12, 0x1000, Endianness::Little);
    put32(&mut b, section + 16, 0x200, Endianness::Little);
    put32(&mut b, section + 20, 0x200, Endianness::Little);
    put32(&mut b, section + 36, 0x60000020, Endianness::Little);
    b
}

fn elf_fixture(class: u8, endian: Endianness) -> Vec<u8> {
    let is_64 = class == 2;
    let eh = if is_64 { 64 } else { 52 };
    let ph = if is_64 { 56 } else { 32 };
    let sh = if is_64 { 64 } else { 40 };
    let phoff = eh;
    let shoff = eh + ph;
    let names = b"\0.text\0.shstrtab\0";
    let raw_text = (shoff + sh * 3 + 15) & !15;
    let raw_names = raw_text + 16;
    let size = raw_names + names.len();
    let mut b = vec![0_u8; size];
    b[..4].copy_from_slice(b"\x7fELF");
    b[4] = class;
    b[5] = if endian == Endianness::Little { 1 } else { 2 };
    b[6] = 1;
    b[7] = 3;
    b[8] = 1;
    put16(&mut b, 16, 2, endian);
    put16(&mut b, 18, if is_64 { 62 } else { 3 }, endian);
    put32(&mut b, 20, 1, endian);
    if is_64 {
        put64(&mut b, 24, 0x400123, endian);
        put64(&mut b, 32, phoff as u64, endian);
        put64(&mut b, 40, shoff as u64, endian);
        put32(&mut b, 48, 5, endian);
        put16(&mut b, 52, eh as u16, endian);
        put16(&mut b, 54, ph as u16, endian);
        put16(&mut b, 56, 1, endian);
        put16(&mut b, 58, sh as u16, endian);
        put16(&mut b, 60, 3, endian);
        put16(&mut b, 62, 2, endian);
    } else {
        put32(&mut b, 24, 0x8048123, endian);
        put32(&mut b, 28, phoff as u32, endian);
        put32(&mut b, 32, shoff as u32, endian);
        put32(&mut b, 36, 5, endian);
        put16(&mut b, 40, eh as u16, endian);
        put16(&mut b, 42, ph as u16, endian);
        put16(&mut b, 44, 1, endian);
        put16(&mut b, 46, sh as u16, endian);
        put16(&mut b, 48, 3, endian);
        put16(&mut b, 50, 2, endian);
    }
    let text = shoff + sh;
    let strtab = shoff + sh * 2;
    put32(&mut b, text, 1, endian);
    put32(&mut b, text + 4, 1, endian);
    if is_64 {
        put64(&mut b, text + 8, 6, endian);
        put64(&mut b, text + 16, 0x400000, endian);
        put64(&mut b, text + 24, raw_text as u64, endian);
        put64(&mut b, text + 32, 16, endian);
        put64(&mut b, text + 48, 16, endian);
    } else {
        put32(&mut b, text + 8, 6, endian);
        put32(&mut b, text + 12, 0x8048000, endian);
        put32(&mut b, text + 16, raw_text as u32, endian);
        put32(&mut b, text + 20, 16, endian);
        put32(&mut b, text + 32, 16, endian);
    }
    put32(&mut b, strtab, 7, endian);
    put32(&mut b, strtab + 4, 3, endian);
    if is_64 {
        put64(&mut b, strtab + 24, raw_names as u64, endian);
        put64(&mut b, strtab + 32, names.len() as u64, endian);
    } else {
        put32(&mut b, strtab + 16, raw_names as u32, endian);
        put32(&mut b, strtab + 20, names.len() as u32, endian);
    }
    b[raw_text..raw_text + 16].fill(0x90);
    b[raw_names..].copy_from_slice(names);
    b
}

#[test]
fn parses_pe32_and_pe32_plus_headers_sections_and_permissions() {
    for (is_64, bits, arch, magic, base) in [
        (false, 32, BinaryArchitecture::X86, 0x10b, 0x400000_u64),
        (true, 64, BinaryArchitecture::X86_64, 0x20b, 0x140000000),
    ] {
        let m = parse_bytes(&pe_fixture(is_64)).unwrap();
        assert_eq!(m.format, BinaryFormat::Pe);
        assert_eq!(m.bitness, bits);
        assert_eq!(m.architecture, arch);
        assert_eq!(m.entry_point, 0x1000);
        assert_eq!(m.sections[0].name, ".text");
        assert!(m.sections[0].permissions.execute);
        assert!(m.sections[0].permissions.read);
        assert!(!m.sections[0].permissions.write);
        let FormatHeaders::Pe(h) = m.headers else {
            panic!("expected PE metadata")
        };
        assert_eq!(h.optional_magic, magic);
        assert_eq!(h.image_base, base);
        assert_eq!(h.section_alignment, 0x1000);
        assert_eq!(h.file_alignment, 0x200);
        assert_eq!(h.subsystem, 3);
        assert_eq!(h.data_directories[0].name, "export");
    }
}

#[test]
fn parses_elf32_elf64_and_both_endiannesses() {
    for (class, endian, bits, arch) in [
        (1, Endianness::Little, 32, BinaryArchitecture::X86),
        (2, Endianness::Little, 64, BinaryArchitecture::X86_64),
        (1, Endianness::Big, 32, BinaryArchitecture::X86),
    ] {
        let m = parse_bytes(&elf_fixture(class, endian)).unwrap();
        assert_eq!(m.format, BinaryFormat::Elf);
        assert_eq!(m.bitness, bits);
        assert_eq!(m.endianness, endian);
        assert_eq!(m.architecture, arch);
        assert_eq!(m.sections.len(), 3);
        assert_eq!(m.sections[1].name, ".text");
        assert!(m.sections[1].permissions.execute);
        assert!(m.sections[1].permissions.read);
        let FormatHeaders::Elf(h) = m.headers else {
            panic!("expected ELF metadata")
        };
        assert_eq!(h.program_header_count, 1);
        assert_eq!(h.section_header_count, 3);
        assert_eq!(h.section_name_string_table_index, 2);
    }
}

#[test]
fn parses_elf_extended_counts_from_section_header_zero() {
    let mut b = elf_fixture(2, Endianness::Little);
    let shoff = 120;
    put16(&mut b, 56, 0xffff, Endianness::Little);
    put16(&mut b, 60, 0, Endianness::Little);
    put16(&mut b, 62, 0xffff, Endianness::Little);
    put32(&mut b, shoff + 40, 2, Endianness::Little); // sh_link: extended shstrndx
    put32(&mut b, shoff + 44, 1, Endianness::Little); // sh_info: extended phnum
    put64(&mut b, shoff + 32, 3, Endianness::Little); // sh_size: extended shnum
    let m = parse_bytes(&b).unwrap();
    let FormatHeaders::Elf(h) = m.headers else {
        panic!("expected ELF metadata")
    };
    assert_eq!(h.program_header_count, 1);
    assert_eq!(h.section_header_count, 3);
    assert_eq!(h.section_name_string_table_index, 2);
}

#[test]
fn loader_opens_read_only_snapshot_and_enforces_file_size_limit() {
    let path = temp_path("elf64");
    let bytes = elf_fixture(2, Endianness::Little);
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    f.write_all(&bytes).unwrap();
    drop(f);
    let loaded = BinaryFile::open(&path, BinaryLimits::default()).unwrap();
    assert_eq!(loaded.bytes(), bytes);
    assert_eq!(loaded.parse().unwrap().format, BinaryFormat::Elf);
    assert!(matches!(
        BinaryFile::open(&path, BinaryLimits::new(64).unwrap()),
        Err(BinaryError::FileTooLarge { .. })
    ));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn rejects_empty_signatures_and_every_truncated_pe_prefix() {
    assert!(matches!(parse_bytes(&[]), Err(BinaryError::EmptyInput)));
    assert!(matches!(
        parse_bytes(b"random"),
        Err(BinaryError::UnsupportedFormat)
    ));
    let pe = pe_fixture(true);
    for end in 0..pe.len() {
        assert!(
            parse_bytes(&pe[..end]).is_err(),
            "accepted truncated PE prefix of {end} bytes"
        );
    }
    let mut bad = pe_fixture(false);
    bad[PE as usize] = b'X';
    assert!(matches!(
        parse_bytes(&bad),
        Err(BinaryError::InvalidSignature { .. })
    ));
    let elf = elf_fixture(2, Endianness::Little);
    for end in 0..64 {
        assert!(
            parse_bytes(&elf[..end]).is_err(),
            "accepted truncated ELF prefix of {end} bytes"
        );
    }
}

#[test]
fn rejects_invalid_pe_offsets_counts_and_optional_headers() {
    let mut b = pe_fixture(true);
    put32(&mut b, 0x3c, u32::MAX, Endianness::Little);
    assert!(parse_bytes(&b).is_err());
    let mut b = pe_fixture(false);
    put16(&mut b, PE as usize + 4 + 2, 97, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::InvalidField {
            context: "COFF section count",
            ..
        })
    ));
    let mut b = pe_fixture(true);
    let optional = PE as usize + 4 + 20;
    put32(&mut b, optional + 108, 17, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::InvalidField {
            context: "data directory count",
            ..
        })
    ));
    let mut b = pe_fixture(true);
    let section = PE as usize + 4 + 20 + 240;
    put32(&mut b, section + 20, u32::MAX, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::Truncated {
            context: "PE section raw data",
            ..
        })
    ));
    let mut b = pe_fixture(true);
    put32(&mut b, section + 12, u32::MAX, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::InvalidField {
            context: "PE section virtual range",
            ..
        })
    ));
    let mut b = pe_fixture(true);
    put32(&mut b, section + 20, 0x100, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::InvalidField {
            context: "PE section raw offset",
            ..
        })
    ));
    let mut b = pe_fixture(false);
    put16(&mut b, PE as usize + 4 + 16, 8, Endianness::Little);
    assert!(parse_bytes(&b).is_err());
    let mut b = pe_fixture(true);
    put16(&mut b, PE as usize + 4 + 2, 96, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::Truncated {
            context: "PE section table",
            ..
        })
    ));
    let mut b = pe_fixture(false);
    put16(&mut b, PE as usize + 4 + 20, 0x999, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::InvalidField {
            context: "optional header magic",
            ..
        })
    ));
}

#[test]
fn rejects_inconsistent_elf_tables_and_names() {
    let mut b = elf_fixture(2, Endianness::Little);
    put64(&mut b, 40, u64::MAX, Endianness::Little);
    assert!(parse_bytes(&b).is_err());
    let mut b = elf_fixture(1, Endianness::Little);
    put16(&mut b, 48, 0xffff, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::Truncated {
            context: "ELF section header table",
            ..
        })
    ));
    let mut b = elf_fixture(2, Endianness::Little);
    put64(&mut b, 40, 64, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::InvalidField {
            context: "ELF header tables",
            ..
        })
    ));
    let mut b = elf_fixture(2, Endianness::Little);
    let shoff = 120;
    let strtab = shoff + 128;
    put64(&mut b, strtab + 24, u64::MAX - 8, Endianness::Little);
    assert!(parse_bytes(&b).is_err());
    let mut b = elf_fixture(1, Endianness::Little);
    b[5] = 3;
    assert!(parse_bytes(&b).is_err());
    let mut b = elf_fixture(2, Endianness::Little);
    put64(&mut b, 32, 65, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::InvalidField {
            context: "ELF header tables",
            ..
        })
    ));
    let mut b = elf_fixture(2, Endianness::Little);
    put64(&mut b, 120 + 64 + 16, u64::MAX - 4, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::InvalidField {
            context: "ELF section virtual range",
            ..
        })
    ));
    let mut b = elf_fixture(2, Endianness::Little);
    let string_header = 120 + 2 * 64;
    let string_offset = u64::from_le_bytes(
        b[string_header + 24..string_header + 32]
            .try_into()
            .unwrap(),
    ) as usize;
    b.resize(string_offset + 5001, b'a');
    put32(&mut b, 120 + 64, 17, Endianness::Little);
    b[string_offset + 5000] = 0;
    put64(&mut b, string_header + 32, 5001, Endianness::Little);
    assert!(matches!(
        parse_bytes(&b),
        Err(BinaryError::InvalidField {
            context: "section name",
            ..
        })
    ));
}

#[test]
fn deterministic_malformed_inputs_never_panic() {
    let mut state = 0x9e3779b97f4a7c15_u64;
    for len in 0..512 {
        let mut bytes = vec![0_u8; len];
        for byte in &mut bytes {
            state ^= state << 7;
            state ^= state >> 9;
            state ^= state << 8;
            *byte = state as u8;
        }
        let _ = parse_bytes(&bytes);
    }
}

#[test]
fn deterministic_format_seeded_fuzz_inputs_never_panic() {
    let mut state = 0xd1b54a32d192ed03_u64;
    for round in 0..2048 {
        let len = round % 512;
        let mut bytes = vec![0_u8; len];
        for byte in &mut bytes {
            state ^= state << 7;
            state ^= state >> 9;
            state ^= state << 8;
            *byte = state as u8;
        }
        if round & 1 == 0 && len >= 2 {
            bytes[..2].copy_from_slice(b"MZ");
            if len >= 64 {
                // Exercise PE dispatch with a randomized, untrusted e_lfanew.
                bytes[0x3c..0x40].copy_from_slice(&(state as u32).to_le_bytes());
            }
        } else if len >= 4 {
            bytes[..4].copy_from_slice(b"\x7fELF");
            if len >= 16 {
                bytes[4] = if round & 2 == 0 { 1 } else { 2 };
                bytes[5] = if round & 4 == 0 { 1 } else { 2 };
                bytes[6] = 1;
            }
        }
        let _ = parse_bytes(&bytes);
    }
}

fn temp_path(kind: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nexus-{kind}-{}-{}.bin",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

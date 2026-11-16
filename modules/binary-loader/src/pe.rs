use crate::{checked_add, checked_mul, BinaryError, BinaryReader};
use nexus_nir::{
    BinaryArchitecture, BinaryFormat, BinaryMetadata, BinarySection, DataDirectory,
    DataDirectoryAddressKind, Endianness, EntryPointKind, FormatHeaders, PeHeaders, Permissions,
};

const MAX_PE_SECTIONS: u16 = 96;
const MAX_DATA_DIRECTORIES: u32 = 16;
const DIRECTORY_NAMES: [&str; 16] = [
    "export",
    "import",
    "resource",
    "exception",
    "certificate",
    "base_relocation",
    "debug",
    "architecture",
    "global_pointer",
    "tls",
    "load_config",
    "bound_import",
    "iat",
    "delay_import",
    "clr",
    "reserved",
];

pub(crate) fn parse(bytes: &[u8]) -> Result<BinaryMetadata, BinaryError> {
    let r = BinaryReader::new(bytes);
    if bytes.len() < 2 {
        return Err(BinaryError::Truncated {
            context: "DOS signature",
            offset: 0,
            needed: 2,
            file_size: bytes.len() as u64,
        });
    }
    if r.slice(0, 2, "DOS signature")? != b"MZ" {
        return Err(BinaryError::InvalidSignature {
            format: "PE",
            offset: 0,
        });
    }
    if bytes.len() < 64 {
        return Err(BinaryError::Truncated {
            context: "DOS header",
            offset: 0,
            needed: 64,
            file_size: bytes.len() as u64,
        });
    }
    let pe_offset = r.u32(0x3c, Endianness::Little, "DOS e_lfanew")?;
    if pe_offset < 64 {
        return Err(BinaryError::InvalidField {
            context: "DOS e_lfanew",
            detail: "PE header overlaps the DOS header",
        });
    }
    let coff = u64::from(pe_offset)
        .checked_add(4)
        .ok_or(BinaryError::IntegerOverflow {
            context: "COFF offset",
        })?;
    if r.slice(u64::from(pe_offset), 4, "PE signature")? != b"PE\0\0" {
        return Err(BinaryError::InvalidSignature {
            format: "PE",
            offset: u64::from(pe_offset),
        });
    }
    r.slice(coff, 20, "COFF file header")?;
    let machine = r.u16(coff, Endianness::Little, "COFF machine")?;
    let section_count = r.u16(coff + 2, Endianness::Little, "COFF section count")?;
    if section_count > MAX_PE_SECTIONS {
        return Err(BinaryError::InvalidField {
            context: "COFF section count",
            detail: "exceeds the PE limit of 96 sections",
        });
    }
    let timestamp = r.u32(coff + 4, Endianness::Little, "COFF timestamp")?;
    let symbol_table_offset = r.u32(coff + 8, Endianness::Little, "COFF symbol table offset")?;
    let symbol_count = r.u32(coff + 12, Endianness::Little, "COFF symbol count")?;
    if symbol_count != 0 {
        if symbol_table_offset == 0 {
            return Err(BinaryError::InvalidField {
                context: "COFF symbol table offset",
                detail: "symbol entries are declared but the table offset is zero",
            });
        }
        let symbol_bytes = checked_mul(u64::from(symbol_count), 18, "COFF symbol table size")?;
        let string_table = checked_add(
            u64::from(symbol_table_offset),
            symbol_bytes,
            "COFF string table offset",
        )?;
        r.slice(
            u64::from(symbol_table_offset),
            symbol_bytes,
            "COFF symbol table",
        )?;
        let string_length = r.u32(string_table, Endianness::Little, "COFF string table size")?;
        if string_length < 4 {
            return Err(BinaryError::InvalidField {
                context: "COFF string table size",
                detail: "must include its four-byte length field",
            });
        }
        r.slice(string_table, u64::from(string_length), "COFF string table")?;
    }
    let optional_size = r.u16(coff + 16, Endianness::Little, "COFF optional header size")?;
    let characteristics = r.u16(coff + 18, Endianness::Little, "COFF characteristics")?;
    let optional = checked_add(coff, 20, "optional header offset")?;
    r.slice(optional, u64::from(optional_size), "optional header")?;
    if optional_size < 2 {
        return Err(BinaryError::InvalidField {
            context: "optional header size",
            detail: "must include the optional-header magic",
        });
    }
    let magic = r.u16(optional, Endianness::Little, "optional header magic")?;
    let (bitness, dir_count_offset, dir_offset, minimum_size, image_base_offset) = match magic {
        0x10b => (32, 92_u64, 96_u64, 96_u16, 28_u64),
        0x20b => (64, 108_u64, 112_u64, 112_u16, 24_u64),
        _ => {
            return Err(BinaryError::InvalidField {
                context: "optional header magic",
                detail: "expected PE32 (0x10b) or PE32+ (0x20b)",
            })
        }
    };
    if optional_size < minimum_size {
        return Err(BinaryError::Truncated {
            context: "PE optional header",
            offset: optional,
            needed: u64::from(minimum_size),
            file_size: bytes.len() as u64,
        });
    }
    let opt_at = |relative, context| checked_add(optional, relative, context);
    let image_base = if bitness == 32 {
        u64::from(r.u32(
            opt_at(image_base_offset, "image base offset")?,
            Endianness::Little,
            "PE32 image base",
        )?)
    } else {
        r.u64(
            opt_at(image_base_offset, "image base offset")?,
            Endianness::Little,
            "PE32+ image base",
        )?
    };
    let entry_point_rva = r.u32(
        opt_at(16, "entry point offset")?,
        Endianness::Little,
        "entry point RVA",
    )?;
    let section_alignment = r.u32(
        opt_at(32, "section alignment offset")?,
        Endianness::Little,
        "section alignment",
    )?;
    let file_alignment = r.u32(
        opt_at(36, "file alignment offset")?,
        Endianness::Little,
        "file alignment",
    )?;
    let image_size = r.u32(
        opt_at(56, "image size offset")?,
        Endianness::Little,
        "image size",
    )?;
    let headers_size = r.u32(
        opt_at(60, "headers size offset")?,
        Endianness::Little,
        "headers size",
    )?;
    let subsystem = r.u16(
        opt_at(68, "subsystem offset")?,
        Endianness::Little,
        "subsystem",
    )?;
    let dll_characteristics = r.u16(
        opt_at(70, "DLL characteristics offset")?,
        Endianness::Little,
        "DLL characteristics",
    )?;
    let declared_directory_count = r.u32(
        opt_at(dir_count_offset, "data directory count offset")?,
        Endianness::Little,
        "data directory count",
    )?;
    if declared_directory_count > MAX_DATA_DIRECTORIES {
        return Err(BinaryError::InvalidField {
            context: "data directory count",
            detail: "exceeds the supported PE directory count of 16",
        });
    }
    let available_directories =
        u64::from(optional_size)
            .checked_sub(dir_offset)
            .ok_or(BinaryError::InvalidField {
                context: "optional header size",
                detail: "ends before data directory table",
            })?
            / 8;
    if u64::from(declared_directory_count) > available_directories {
        return Err(BinaryError::Truncated {
            context: "PE data directory table",
            offset: optional + dir_offset,
            needed: u64::from(declared_directory_count) * 8,
            file_size: bytes.len() as u64,
        });
    }
    let mut directories = Vec::with_capacity(declared_directory_count as usize);
    for i in 0..declared_directory_count {
        let directory_table = checked_add(optional, dir_offset, "data directory table offset")?;
        let at = checked_add(
            directory_table,
            checked_mul(u64::from(i), 8, "data directory entry")?,
            "data directory entry",
        )?;
        let address = r.u32(at, Endianness::Little, "data directory address")?;
        let size = r.u32(at + 4, Endianness::Little, "data directory size")?;
        let address_kind = if i == 4 {
            DataDirectoryAddressKind::FileOffset
        } else {
            DataDirectoryAddressKind::Rva
        };
        if i != 4 && size != 0 && address.checked_add(size).is_none() {
            return Err(BinaryError::InvalidField {
                context: "PE data directory range",
                detail: "RVA plus size overflows 32 bits",
            });
        }
        if i == 4 && size != 0 && address == 0 {
            return Err(BinaryError::InvalidField {
                context: "PE certificate directory",
                detail: "nonempty certificate table has a zero file offset",
            });
        }
        if i == 4 && size != 0 {
            r.slice(
                u64::from(address),
                u64::from(size),
                "PE certificate directory",
            )?;
        }
        directories.push(DataDirectory {
            index: i,
            name: DIRECTORY_NAMES[i as usize].to_owned(),
            address,
            size,
            address_kind,
        });
    }
    let section_table = checked_add(optional, u64::from(optional_size), "section table offset")?;
    let table_size = checked_mul(u64::from(section_count), 40, "PE section table size")?;
    let table_end = checked_add(section_table, table_size, "PE section table end")?;
    r.slice(section_table, table_size, "PE section table")?;
    if u64::from(headers_size) > bytes.len() as u64 {
        return Err(BinaryError::InvalidField {
            context: "SizeOfHeaders",
            detail: "extends beyond end of file",
        });
    }
    if table_end > u64::from(headers_size) {
        return Err(BinaryError::InvalidField {
            context: "PE section table",
            detail: "extends beyond SizeOfHeaders",
        });
    }

    let mut sections = Vec::with_capacity(section_count as usize);
    for i in 0..section_count {
        let at = checked_add(section_table, u64::from(i) * 40, "PE section entry")?;
        let raw_name = r.slice(at, 8, "PE section name")?;
        let name_end = raw_name
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(raw_name.len());
        let name = String::from_utf8_lossy(&raw_name[..name_end]).into_owned();
        let virtual_size = r.u32(at + 8, Endianness::Little, "section virtual size")?;
        let virtual_address = r.u32(at + 12, Endianness::Little, "section virtual address")?;
        let raw_data_size = r.u32(at + 16, Endianness::Little, "section raw size")?;
        let raw_data_offset = r.u32(at + 20, Endianness::Little, "section raw offset")?;
        let flags = r.u32(at + 36, Endianness::Little, "section characteristics")?;
        let mapped_size = virtual_size.max(raw_data_size);
        if virtual_address.checked_add(mapped_size).is_none() {
            return Err(BinaryError::InvalidField {
                context: "PE section virtual range",
                detail: "virtual address plus mapped size overflows 32 bits",
            });
        }
        if raw_data_size != 0 {
            if raw_data_offset < headers_size {
                return Err(BinaryError::InvalidField {
                    context: "PE section raw offset",
                    detail: "section data overlaps the declared headers",
                });
            }
            r.slice(
                u64::from(raw_data_offset),
                u64::from(raw_data_size),
                "PE section raw data",
            )?;
        }
        sections.push(BinarySection {
            index: u32::from(i),
            name,
            virtual_size: u64::from(virtual_size),
            virtual_address: u64::from(virtual_address),
            raw_data_size: u64::from(raw_data_size),
            raw_data_offset: u64::from(raw_data_offset),
            flags: u64::from(flags),
            section_type: None,
            permissions: Permissions {
                read: flags & 0x4000_0000 != 0,
                write: flags & 0x8000_0000 != 0,
                execute: flags & 0x2000_0000 != 0,
            },
        });
    }
    let architecture = match machine {
        0x014c => BinaryArchitecture::X86,
        0x8664 => BinaryArchitecture::X86_64,
        0x01c0 | 0x01c4 => BinaryArchitecture::Arm,
        0xaa64 => BinaryArchitecture::Aarch64,
        other => BinaryArchitecture::Unknown(other),
    };
    if (machine == 0x014c && bitness != 32) || (matches!(machine, 0x8664 | 0xaa64) && bitness != 64)
    {
        return Err(BinaryError::InvalidField {
            context: "PE machine and optional header",
            detail: "machine architecture conflicts with PE32/PE32+ format",
        });
    }
    let pe = PeHeaders {
        pe_offset,
        machine,
        section_count,
        timestamp,
        symbol_table_offset,
        symbol_count,
        optional_header_size: optional_size,
        characteristics,
        optional_magic: magic,
        entry_point_rva,
        image_base,
        section_alignment,
        file_alignment,
        image_size,
        headers_size,
        subsystem,
        dll_characteristics,
        declared_directory_count,
        data_directories: directories,
    };
    Ok(BinaryMetadata {
        format: BinaryFormat::Pe,
        architecture,
        bitness,
        endianness: Endianness::Little,
        file_size: bytes.len() as u64,
        entry_point: u64::from(entry_point_rva),
        entry_point_kind: EntryPointKind::Rva,
        sections,
        headers: FormatHeaders::Pe(pe),
    })
}

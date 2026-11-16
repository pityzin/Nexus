use crate::{checked_add, checked_mul, BinaryError, BinaryReader};
use nexus_nir::{
    BinaryArchitecture, BinaryFormat, BinaryMetadata, BinarySection, ElfHeaders, Endianness,
    EntryPointKind, FormatHeaders, Permissions,
};

const MAX_ELF_HEADERS: u32 = 65_536;
const MAX_SECTION_NAME_BYTES: usize = 4096;
const MAX_TOTAL_SECTION_NAME_BYTES: usize = 1024 * 1024;
const PN_XNUM: u16 = 0xffff;
const SHN_XINDEX: u16 = 0xffff;
const SHT_NOBITS: u32 = 8;
const SHT_STRTAB: u32 = 3;
const SHT_NULL: u32 = 0;
const SHF_WRITE: u64 = 1;
const SHF_ALLOC: u64 = 2;
const SHF_EXECINSTR: u64 = 4;

#[derive(Clone, Copy)]
struct Layout {
    class: u8,
    endian: Endianness,
    eh_size: u16,
    ph_min: u16,
    sh_min: u16,
}

pub(crate) fn parse(bytes: &[u8]) -> Result<BinaryMetadata, BinaryError> {
    let r = BinaryReader::new(bytes);
    if bytes.len() < 16 {
        return Err(BinaryError::Truncated {
            context: "ELF identification",
            offset: 0,
            needed: 16,
            file_size: bytes.len() as u64,
        });
    }
    if r.slice(0, 4, "ELF magic")? != b"\x7fELF" {
        return Err(BinaryError::InvalidSignature {
            format: "ELF",
            offset: 0,
        });
    }
    let class = r.u8(4, "ELF class")?;
    let endian = match r.u8(5, "ELF data encoding")? {
        1 => Endianness::Little,
        2 => Endianness::Big,
        _ => {
            return Err(BinaryError::InvalidField {
                context: "ELF data encoding",
                detail: "must be little-endian (1) or big-endian (2)",
            })
        }
    };
    if r.u8(6, "ELF identification version")? != 1 {
        return Err(BinaryError::InvalidField {
            context: "ELF identification version",
            detail: "unsupported version",
        });
    }
    let layout = match class {
        1 => Layout {
            class: 32,
            endian,
            eh_size: 52,
            ph_min: 32,
            sh_min: 40,
        },
        2 => Layout {
            class: 64,
            endian,
            eh_size: 64,
            ph_min: 56,
            sh_min: 64,
        },
        _ => {
            return Err(BinaryError::InvalidField {
                context: "ELF class",
                detail: "must be 32-bit (1) or 64-bit (2)",
            })
        }
    };
    r.slice(0, u64::from(layout.eh_size), "ELF header")?;
    let os_abi = r.u8(7, "ELF OS ABI")?;
    let abi_version = r.u8(8, "ELF ABI version")?;
    let file_type = r.u16(16, endian, "ELF file type")?;
    let machine = r.u16(18, endian, "ELF machine")?;
    let version = r.u32(20, endian, "ELF version")?;
    if version != 1 {
        return Err(BinaryError::InvalidField {
            context: "ELF version",
            detail: "unsupported ELF header version",
        });
    }
    let (
        entry,
        phoff,
        shoff,
        flags,
        ehsize_at,
        phentsize_at,
        phnum_at,
        shentsize_at,
        shnum_at,
        shstrndx_at,
    ) = if layout.class == 32 {
        (
            u64::from(r.u32(24, endian, "ELF entry point")?),
            u64::from(r.u32(28, endian, "program header offset")?),
            u64::from(r.u32(32, endian, "section header offset")?),
            r.u32(36, endian, "ELF flags")?,
            40,
            42,
            44,
            46,
            48,
            50,
        )
    } else {
        (
            r.u64(24, endian, "ELF entry point")?,
            r.u64(32, endian, "program header offset")?,
            r.u64(40, endian, "section header offset")?,
            r.u32(48, endian, "ELF flags")?,
            52,
            54,
            56,
            58,
            60,
            62,
        )
    };
    let ehsize = r.u16(ehsize_at, endian, "ELF header size")?;
    if ehsize < layout.eh_size {
        return Err(BinaryError::InvalidField {
            context: "ELF header size",
            detail: "smaller than the class-specific ELF header",
        });
    }
    r.slice(0, u64::from(ehsize), "declared ELF header")?;
    let phentsize = r.u16(phentsize_at, endian, "program header entry size")?;
    let raw_phnum = r.u16(phnum_at, endian, "program header count")?;
    let shentsize = r.u16(shentsize_at, endian, "section header entry size")?;
    let raw_shnum = r.u16(shnum_at, endian, "section header count")?;
    let raw_shstrndx = r.u16(shstrndx_at, endian, "section name table index")?;

    if raw_phnum != 0 && phentsize < layout.ph_min {
        return Err(BinaryError::InvalidField {
            context: "program header entry size",
            detail: "smaller than the ELF class minimum",
        });
    }
    let phnum = if raw_phnum == PN_XNUM {
        extended_count(
            &r,
            layout,
            shoff,
            shentsize,
            28,
            44,
            "extended program header count",
        )?
    } else {
        u32::from(raw_phnum)
    };
    if phnum > MAX_ELF_HEADERS {
        return Err(BinaryError::InvalidField {
            context: "program header count",
            detail: "exceeds the supported limit of 65536",
        });
    }
    let ph_table = if phnum != 0 {
        if phoff < u64::from(ehsize) {
            return Err(BinaryError::InvalidField {
                context: "program header offset",
                detail: "program table overlaps the ELF header",
            });
        }
        let total = checked_mul(
            u64::from(phnum),
            u64::from(phentsize),
            "program header table size",
        )?;
        let end = checked_add(phoff, total, "program header table end")?;
        r.slice(phoff, total, "ELF program header table")?;
        Some((phoff, end))
    } else {
        None
    };

    if shoff == 0 && raw_shnum != 0 {
        return Err(BinaryError::InvalidField {
            context: "section header offset",
            detail: "is zero but section headers are declared",
        });
    }
    if shoff == 0 && (raw_shstrndx == SHN_XINDEX || raw_phnum == PN_XNUM) {
        return Err(BinaryError::InvalidField {
            context: "extended ELF counts",
            detail: "require section header zero",
        });
    }
    if shoff != 0 && shentsize < layout.sh_min {
        return Err(BinaryError::InvalidField {
            context: "section header entry size",
            detail: "smaller than the ELF class minimum",
        });
    }
    if shoff != 0 && shoff < u64::from(ehsize) {
        return Err(BinaryError::InvalidField {
            context: "section header offset",
            detail: "section table overlaps the ELF header",
        });
    }
    let section_zero = if shoff != 0 {
        Some(r.slice(shoff, u64::from(layout.sh_min), "ELF section header zero")?)
    } else {
        None
    };
    let section_count = if raw_shnum == 0 {
        match section_zero {
            Some(_) if layout.class == 32 => {
                r.u32(shoff + 20, endian, "extended section header count")?
            }
            Some(_) => u32::try_from(r.u64(shoff + 32, endian, "extended section header count")?)
                .map_err(|_| BinaryError::InvalidField {
                context: "extended section header count",
                detail: "exceeds the supported 32-bit count",
            })?,
            None => 0,
        }
    } else {
        u32::from(raw_shnum)
    };
    if section_count > MAX_ELF_HEADERS {
        return Err(BinaryError::InvalidField {
            context: "section header count",
            detail: "exceeds the supported limit of 65536",
        });
    }
    let sh_table = if section_count != 0 {
        let total = checked_mul(
            u64::from(section_count),
            u64::from(shentsize),
            "section header table size",
        )?;
        let end = checked_add(shoff, total, "section header table end")?;
        r.slice(shoff, total, "ELF section header table")?;
        Some((shoff, end))
    } else {
        None
    };
    if let (Some((p_start, p_end)), Some((s_start, s_end))) = (ph_table, sh_table) {
        if p_start < s_end && s_start < p_end {
            return Err(BinaryError::InvalidField {
                context: "ELF header tables",
                detail: "program and section tables overlap",
            });
        }
    }
    let shstrndx = if raw_shstrndx == SHN_XINDEX {
        section_zero
            .map(|z| u32_at(z, if layout.class == 32 { 24 } else { 40 }, endian))
            .transpose()?
            .unwrap_or(0)
    } else {
        u32::from(raw_shstrndx)
    };
    if shstrndx != 0 && shstrndx >= section_count {
        return Err(BinaryError::InvalidField {
            context: "section name table index",
            detail: "is outside the section table",
        });
    }

    let mut raw_sections = Vec::with_capacity(section_count as usize);
    for i in 0..section_count {
        let at = checked_add(
            shoff,
            checked_mul(
                u64::from(i),
                u64::from(shentsize),
                "ELF section header offset",
            )?,
            "ELF section header offset",
        )?;
        let name_offset = r.u32(at, endian, "section name offset")?;
        let section_type = r.u32(at + 4, endian, "section type")?;
        let (flags, address, offset, size) = if layout.class == 32 {
            (
                u64::from(r.u32(at + 8, endian, "section flags")?),
                u64::from(r.u32(at + 12, endian, "section address")?),
                u64::from(r.u32(at + 16, endian, "section offset")?),
                u64::from(r.u32(at + 20, endian, "section size")?),
            )
        } else {
            (
                r.u64(at + 8, endian, "section flags")?,
                r.u64(at + 16, endian, "section address")?,
                r.u64(at + 24, endian, "section offset")?,
                r.u64(at + 32, endian, "section size")?,
            )
        };
        if address.checked_add(size).is_none() {
            return Err(BinaryError::InvalidField {
                context: "ELF section virtual range",
                detail: "virtual address plus size overflows 64 bits",
            });
        }
        if !matches!(section_type, SHT_NULL | SHT_NOBITS) && size != 0 {
            let section_end = checked_add(offset, size, "ELF section end")?;
            r.slice(offset, size, "ELF section contents")?;
            if offset < u64::from(ehsize) {
                return Err(BinaryError::InvalidField {
                    context: "ELF section offset",
                    detail: "section contents overlap the ELF header",
                });
            }
            for (table_start, table_end) in [ph_table, sh_table].into_iter().flatten() {
                if offset < table_end && table_start < section_end {
                    return Err(BinaryError::InvalidField {
                        context: "ELF section offset",
                        detail: "section contents overlap an ELF header table",
                    });
                }
            }
        }
        raw_sections.push((name_offset, section_type, flags, address, offset, size));
    }
    let string_table = if shstrndx != 0 {
        let (_, kind, _, _, offset, size) = raw_sections[shstrndx as usize];
        if kind != SHT_STRTAB {
            return Err(BinaryError::InvalidField {
                context: "section name string table",
                detail: "section name table must have type SHT_STRTAB",
            });
        }
        Some(r.slice(offset, size, "ELF section name string table")?)
    } else {
        None
    };
    let mut sections = Vec::with_capacity(section_count as usize);
    let mut total_name_bytes = 0_usize;
    for (i, (name_offset, section_type, flags, address, offset, size)) in
        raw_sections.into_iter().enumerate()
    {
        let name = match string_table {
            Some(table) => read_name(table, name_offset)?,
            None if name_offset == 0 => String::new(),
            None => {
                return Err(BinaryError::InvalidField {
                    context: "section name",
                    detail: "name offset is nonzero but no string table is declared",
                })
            }
        };
        total_name_bytes =
            total_name_bytes
                .checked_add(name.len())
                .ok_or(BinaryError::IntegerOverflow {
                    context: "total section name bytes",
                })?;
        if total_name_bytes > MAX_TOTAL_SECTION_NAME_BYTES {
            return Err(BinaryError::InvalidField {
                context: "section names",
                detail: "total decoded name bytes exceed the 1 MiB limit",
            });
        }
        sections.push(BinarySection {
            index: i as u32,
            name,
            virtual_size: if section_type == SHT_NULL { 0 } else { size },
            virtual_address: address,
            raw_data_size: if matches!(section_type, SHT_NULL | SHT_NOBITS) {
                0
            } else {
                size
            },
            raw_data_offset: offset,
            flags,
            section_type: Some(section_type),
            permissions: Permissions {
                read: flags & SHF_ALLOC != 0,
                write: flags & SHF_WRITE != 0,
                execute: flags & SHF_EXECINSTR != 0,
            },
        });
    }
    let architecture = match machine {
        3 => BinaryArchitecture::X86,
        62 => BinaryArchitecture::X86_64,
        40 => BinaryArchitecture::Arm,
        183 => BinaryArchitecture::Aarch64,
        other => BinaryArchitecture::Unknown(other),
    };
    let headers = ElfHeaders {
        class_bits: layout.class,
        endianness: endian,
        os_abi,
        abi_version,
        file_type,
        machine,
        version,
        flags,
        header_size: ehsize,
        program_header_offset: phoff,
        program_header_entry_size: phentsize,
        program_header_count: phnum,
        section_header_offset: shoff,
        section_header_entry_size: shentsize,
        section_header_count: section_count,
        section_name_string_table_index: shstrndx,
    };
    Ok(BinaryMetadata {
        format: BinaryFormat::Elf,
        architecture,
        bitness: layout.class,
        endianness: endian,
        file_size: bytes.len() as u64,
        entry_point: entry,
        entry_point_kind: EntryPointKind::VirtualAddress,
        sections,
        headers: FormatHeaders::Elf(headers),
    })
}

fn extended_count(
    r: &BinaryReader<'_>,
    layout: Layout,
    shoff: u64,
    shentsize: u16,
    field32: u64,
    field64: u64,
    context: &'static str,
) -> Result<u32, BinaryError> {
    if shoff == 0 || shentsize < layout.sh_min {
        return Err(BinaryError::InvalidField {
            context,
            detail: "requires a valid section header zero",
        });
    }
    let at = checked_add(
        shoff,
        if layout.class == 32 { field32 } else { field64 },
        context,
    )?;
    r.u32(at, layout.endian, context)
}

fn u32_at(bytes: &[u8], offset: usize, endian: Endianness) -> Result<u32, BinaryError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(BinaryError::Truncated {
            context: "extended ELF index",
            offset: offset as u64,
            needed: 4,
            file_size: bytes.len() as u64,
        })?;
    Ok(match endian {
        Endianness::Little => u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]),
        Endianness::Big => u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]),
    })
}

fn read_name(table: &[u8], offset: u32) -> Result<String, BinaryError> {
    let start = usize::try_from(offset).map_err(|_| BinaryError::IntegerOverflow {
        context: "section name offset",
    })?;
    let tail = table.get(start..).ok_or(BinaryError::InvalidField {
        context: "section name offset",
        detail: "outside the string table",
    })?;
    let end = tail
        .iter()
        .position(|b| *b == 0)
        .ok_or(BinaryError::InvalidField {
            context: "section name",
            detail: "not NUL-terminated within the string table",
        })?;
    if end > MAX_SECTION_NAME_BYTES {
        return Err(BinaryError::InvalidField {
            context: "section name",
            detail: "decoded name exceeds the 4096-byte limit",
        });
    }
    Ok(String::from_utf8_lossy(&tail[..end]).into_owned())
}

//! NEXUS Intermediate Representation (NIR) v1: format-neutral executable metadata.

pub const NIR_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryFormat {
    Pe,
    Elf,
}
impl BinaryFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pe => "pe",
            Self::Elf => "elf",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryArchitecture {
    X86,
    X86_64,
    Arm,
    Aarch64,
    Unknown(u16),
}
impl BinaryArchitecture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X86_64 => "x86_64",
            Self::Arm => "arm",
            Self::Aarch64 => "aarch64",
            Self::Unknown(_) => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Endianness {
    Little,
    Big,
}
impl Endianness {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Little => "little",
            Self::Big => "big",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryPointKind {
    Rva,
    VirtualAddress,
}
impl EntryPointKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rva => "rva",
            Self::VirtualAddress => "virtual_address",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinarySection {
    pub index: u32,
    pub name: String,
    pub virtual_size: u64,
    pub virtual_address: u64,
    pub raw_data_size: u64,
    pub raw_data_offset: u64,
    pub flags: u64,
    pub section_type: Option<u32>,
    pub permissions: Permissions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataDirectoryAddressKind {
    Rva,
    FileOffset,
}
impl DataDirectoryAddressKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rva => "rva",
            Self::FileOffset => "file_offset",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataDirectory {
    pub index: u32,
    pub name: String,
    pub address: u32,
    pub size: u32,
    pub address_kind: DataDirectoryAddressKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeHeaders {
    pub pe_offset: u32,
    pub machine: u16,
    pub section_count: u16,
    pub timestamp: u32,
    pub symbol_table_offset: u32,
    pub symbol_count: u32,
    pub optional_header_size: u16,
    pub characteristics: u16,
    pub optional_magic: u16,
    pub entry_point_rva: u32,
    pub image_base: u64,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub image_size: u32,
    pub headers_size: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub declared_directory_count: u32,
    pub data_directories: Vec<DataDirectory>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElfHeaders {
    pub class_bits: u8,
    pub endianness: Endianness,
    pub os_abi: u8,
    pub abi_version: u8,
    pub file_type: u16,
    pub machine: u16,
    pub version: u32,
    pub flags: u32,
    pub header_size: u16,
    pub program_header_offset: u64,
    pub program_header_entry_size: u16,
    pub program_header_count: u32,
    pub section_header_offset: u64,
    pub section_header_entry_size: u16,
    pub section_header_count: u32,
    pub section_name_string_table_index: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatHeaders {
    Pe(PeHeaders),
    Elf(ElfHeaders),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryMetadata {
    pub format: BinaryFormat,
    pub architecture: BinaryArchitecture,
    pub bitness: u8,
    pub endianness: Endianness,
    pub file_size: u64,
    pub entry_point: u64,
    pub entry_point_kind: EntryPointKind,
    pub sections: Vec<BinarySection>,
    pub headers: FormatHeaders,
}

impl BinaryMetadata {
    pub fn inspect_json(&self) -> String {
        format!(
            "{{\"schema_version\":{NIR_SCHEMA_VERSION},\"format\":{},\"architecture\":{},\"bitness\":{},\"endianness\":{},\"file_size\":{},\"entry_point\":{},\"entry_point_kind\":{},\"headers\":{},\"sections\":{}}}",
            json_string(self.format.as_str()), json_string(self.architecture.as_str()), self.bitness,
            json_string(self.endianness.as_str()), self.file_size, self.entry_point,
            json_string(self.entry_point_kind.as_str()), self.headers_json(), self.sections_json()
        )
    }

    pub fn headers_json(&self) -> String {
        match &self.headers {
            FormatHeaders::Pe(h) => format!(
                "{{\"kind\":\"pe\",\"pe_offset\":{},\"machine\":{},\"section_count\":{},\"timestamp\":{},\"symbol_table_offset\":{},\"symbol_count\":{},\"optional_header_size\":{},\"characteristics\":{},\"optional_magic\":{},\"entry_point_rva\":{},\"image_base\":{},\"section_alignment\":{},\"file_alignment\":{},\"image_size\":{},\"headers_size\":{},\"subsystem\":{},\"dll_characteristics\":{},\"declared_directory_count\":{},\"data_directories\":[{}]}}",
                h.pe_offset, h.machine, h.section_count, h.timestamp, h.symbol_table_offset, h.symbol_count,
                h.optional_header_size, h.characteristics, h.optional_magic, h.entry_point_rva, h.image_base,
                h.section_alignment, h.file_alignment, h.image_size, h.headers_size, h.subsystem,
                h.dll_characteristics, h.declared_directory_count,
                h.data_directories.iter().map(|d| format!("{{\"index\":{},\"name\":{},\"address\":{},\"address_kind\":{},\"size\":{}}}", d.index, json_string(&d.name), d.address, json_string(d.address_kind.as_str()), d.size)).collect::<Vec<_>>().join(",")
            ),
            FormatHeaders::Elf(h) => format!(
                "{{\"kind\":\"elf\",\"class_bits\":{},\"endianness\":{},\"os_abi\":{},\"abi_version\":{},\"file_type\":{},\"machine\":{},\"version\":{},\"flags\":{},\"header_size\":{},\"program_header_offset\":{},\"program_header_entry_size\":{},\"program_header_count\":{},\"section_header_offset\":{},\"section_header_entry_size\":{},\"section_header_count\":{},\"section_name_string_table_index\":{}}}",
                h.class_bits, json_string(h.endianness.as_str()), h.os_abi, h.abi_version, h.file_type,
                h.machine, h.version, h.flags, h.header_size, h.program_header_offset,
                h.program_header_entry_size, h.program_header_count, h.section_header_offset,
                h.section_header_entry_size, h.section_header_count, h.section_name_string_table_index
            ),
        }
    }

    pub fn sections_json(&self) -> String {
        format!("[{}]", self.sections.iter().map(|s| format!(
            "{{\"index\":{},\"name\":{},\"virtual_size\":{},\"virtual_address\":{},\"raw_data_size\":{},\"raw_data_offset\":{},\"flags\":{},\"section_type\":{},\"permissions\":{{\"read\":{},\"write\":{},\"execute\":{}}}}}",
            s.index, json_string(&s.name), s.virtual_size, s.virtual_address, s.raw_data_size,
            s.raw_data_offset, s.flags, s.section_type.map_or("null".to_owned(), |v| v.to_string()),
            s.permissions.read, s.permissions.write, s.permissions.execute
        )).collect::<Vec<_>>().join(","))
    }
}

pub fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_escaping_is_deterministic() {
        assert_eq!(json_string("a\"\\\n"), r#""a\"\\\n""#);
    }
}

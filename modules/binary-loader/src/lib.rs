//! Safe, bounded, read-only PE and ELF metadata loading. No parser executes or maps input files.

mod elf;
mod pe;
mod reader;

pub use nexus_nir;
pub use nexus_nir::{
    BinaryArchitecture, BinaryFormat, BinaryMetadata, BinarySection, DataDirectory,
    DataDirectoryAddressKind, ElfHeaders, Endianness, EntryPointKind, FormatHeaders, PeHeaders,
    Permissions, NIR_SCHEMA_VERSION,
};
pub use reader::BinaryReader;

use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

pub const DEFAULT_MAX_FILE_SIZE: u64 = 512 * 1024 * 1024;
pub const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug)]
pub enum BinaryError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    FileTooLarge {
        size: u64,
        limit: u64,
    },
    EmptyInput,
    UnsupportedFormat,
    InvalidSignature {
        format: &'static str,
        offset: u64,
    },
    Truncated {
        context: &'static str,
        offset: u64,
        needed: u64,
        file_size: u64,
    },
    InvalidField {
        context: &'static str,
        detail: &'static str,
    },
    IntegerOverflow {
        context: &'static str,
    },
    AllocationFailed {
        size: u64,
    },
}

impl std::fmt::Display for BinaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::FileTooLarge { size, limit } => write!(f, "file is {size} bytes; configured limit is {limit} bytes"),
            Self::EmptyInput => f.write_str("input file is empty"),
            Self::UnsupportedFormat => f.write_str("unsupported binary format (expected PE or ELF signature)"),
            Self::InvalidSignature { format, offset } => write!(f, "invalid {format} signature at file offset 0x{offset:x}"),
            Self::Truncated { context, offset, needed, file_size } => write!(f, "truncated {context} at offset 0x{offset:x}: need {needed} bytes, file size is {file_size}"),
            Self::InvalidField { context, detail } => write!(f, "invalid {context}: {detail}"),
            Self::IntegerOverflow { context } => write!(f, "integer overflow while calculating {context}"),
            Self::AllocationFailed { size } => write!(f, "unable to reserve {size} bytes for input"),
        }
    }
}
impl std::error::Error for BinaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BinaryLimits {
    pub max_file_size: u64,
}
impl Default for BinaryLimits {
    fn default() -> Self {
        Self {
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        }
    }
}
impl BinaryLimits {
    pub fn new(max_file_size: u64) -> Result<Self, BinaryError> {
        if max_file_size == 0 || max_file_size > MAX_FILE_SIZE {
            return Err(BinaryError::InvalidField {
                context: "maximum file size",
                detail: "must be between 1 byte and 2 GiB",
            });
        }
        Ok(Self { max_file_size })
    }
}

/// An immutable, single-buffer snapshot of an input file.
pub struct BinaryFile {
    path: PathBuf,
    bytes: Arc<Vec<u8>>,
}
impl BinaryFile {
    pub fn open(path: impl AsRef<Path>, limits: BinaryLimits) -> Result<Self, BinaryError> {
        if limits.max_file_size == 0 || limits.max_file_size > MAX_FILE_SIZE {
            return Err(BinaryError::InvalidField {
                context: "maximum file size",
                detail: "must be between 1 byte and 2 GiB",
            });
        }
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path).map_err(|source| BinaryError::Io {
            path: path.clone(),
            source,
        })?;
        let metadata = file.metadata().map_err(|source| BinaryError::Io {
            path: path.clone(),
            source,
        })?;
        if !metadata.is_file() {
            return Err(BinaryError::InvalidField {
                context: "input path",
                detail: "must refer to a regular file",
            });
        }
        let expected = metadata.len();
        if expected > limits.max_file_size {
            return Err(BinaryError::FileTooLarge {
                size: expected,
                limit: limits.max_file_size,
            });
        }
        let capacity = usize::try_from(expected).map_err(|_| BinaryError::FileTooLarge {
            size: expected,
            limit: limits.max_file_size,
        })?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| BinaryError::AllocationFailed { size: expected })?;
        let read_limit =
            limits
                .max_file_size
                .checked_add(1)
                .ok_or(BinaryError::IntegerOverflow {
                    context: "read limit",
                })?;
        file.by_ref()
            .take(read_limit)
            .read_to_end(&mut bytes)
            .map_err(|source| BinaryError::Io {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > limits.max_file_size {
            return Err(BinaryError::FileTooLarge {
                size: bytes.len() as u64,
                limit: limits.max_file_size,
            });
        }
        if bytes.len() as u64 != expected {
            return Err(BinaryError::Io {
                path,
                source: io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "file size changed while reading",
                ),
            });
        }
        Ok(Self {
            path,
            bytes: Arc::new(bytes),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
    pub fn parse(&self) -> Result<BinaryMetadata, BinaryError> {
        parse_bytes_with_limit(self.bytes(), MAX_FILE_SIZE)
    }
}

pub fn parse_bytes(bytes: &[u8]) -> Result<BinaryMetadata, BinaryError> {
    parse_bytes_with_limit(bytes, DEFAULT_MAX_FILE_SIZE)
}

pub fn parse_bytes_with_limit(bytes: &[u8], limit: u64) -> Result<BinaryMetadata, BinaryError> {
    if limit == 0 || limit > MAX_FILE_SIZE {
        return Err(BinaryError::InvalidField {
            context: "maximum file size",
            detail: "must be between 1 byte and 2 GiB",
        });
    }
    if bytes.len() as u64 > limit {
        return Err(BinaryError::FileTooLarge {
            size: bytes.len() as u64,
            limit,
        });
    }
    if bytes.is_empty() {
        return Err(BinaryError::EmptyInput);
    }
    if bytes.starts_with(b"MZ") {
        pe::parse(bytes)
    } else if bytes.starts_with(b"\x7fELF") {
        elf::parse(bytes)
    } else {
        Err(BinaryError::UnsupportedFormat)
    }
}

pub(crate) fn checked_add(a: u64, b: u64, context: &'static str) -> Result<u64, BinaryError> {
    a.checked_add(b)
        .ok_or(BinaryError::IntegerOverflow { context })
}
pub(crate) fn checked_mul(a: u64, b: u64, context: &'static str) -> Result<u64, BinaryError> {
    a.checked_mul(b)
        .ok_or(BinaryError::IntegerOverflow { context })
}

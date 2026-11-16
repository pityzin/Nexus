use crate::BinaryError;
use nexus_nir::Endianness;

/// Bounds-checked reader over an already bounded, immutable byte slice.
#[derive(Clone, Copy)]
pub struct BinaryReader<'a> {
    bytes: &'a [u8],
}

impl<'a> BinaryReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn slice(
        &self,
        offset: u64,
        length: u64,
        context: &'static str,
    ) -> Result<&'a [u8], BinaryError> {
        let start =
            usize::try_from(offset).map_err(|_| BinaryError::IntegerOverflow { context })?;
        let size = usize::try_from(length).map_err(|_| BinaryError::IntegerOverflow { context })?;
        let end = start
            .checked_add(size)
            .ok_or(BinaryError::IntegerOverflow { context })?;
        self.bytes.get(start..end).ok_or(BinaryError::Truncated {
            context,
            offset,
            needed: length,
            file_size: self.bytes.len() as u64,
        })
    }

    pub fn u8(&self, offset: u64, context: &'static str) -> Result<u8, BinaryError> {
        Ok(self.slice(offset, 1, context)?[0])
    }
    pub fn u16(
        &self,
        offset: u64,
        endian: Endianness,
        context: &'static str,
    ) -> Result<u16, BinaryError> {
        let b = self.slice(offset, 2, context)?;
        Ok(match endian {
            Endianness::Little => u16::from_le_bytes([b[0], b[1]]),
            Endianness::Big => u16::from_be_bytes([b[0], b[1]]),
        })
    }
    pub fn u32(
        &self,
        offset: u64,
        endian: Endianness,
        context: &'static str,
    ) -> Result<u32, BinaryError> {
        let b = self.slice(offset, 4, context)?;
        Ok(match endian {
            Endianness::Little => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            Endianness::Big => u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        })
    }
    pub fn u64(
        &self,
        offset: u64,
        endian: Endianness,
        context: &'static str,
    ) -> Result<u64, BinaryError> {
        let b = self.slice(offset, 8, context)?;
        Ok(match endian {
            Endianness::Little => {
                u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
            }
            Endianness::Big => u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
        })
    }
}

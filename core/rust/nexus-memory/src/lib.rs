//! Bounded, read-only views over input bytes. This crate never maps or executes a file.
use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub struct ResourceLimits {
    pub max_input_bytes: usize,
}
impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 512 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MemoryError {
    InputTooLarge {
        actual: usize,
        limit: usize,
    },
    OutOfBounds {
        offset: usize,
        length: usize,
        total: usize,
    },
    IntegerOverflow,
}
impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputTooLarge { actual, limit } => {
                write!(f, "input is {actual} bytes; limit is {limit}")
            }
            Self::OutOfBounds {
                offset,
                length,
                total,
            } => write!(
                f,
                "range at {offset} of length {length} exceeds input size {total}"
            ),
            Self::IntegerOverflow => f.write_str("offset arithmetic overflow"),
        }
    }
}
impl std::error::Error for MemoryError {}

pub struct ReadOnlyImage<'a> {
    bytes: &'a [u8],
}
impl<'a> ReadOnlyImage<'a> {
    pub fn new(bytes: &'a [u8], limits: ResourceLimits) -> Result<Self, MemoryError> {
        if bytes.len() > limits.max_input_bytes {
            return Err(MemoryError::InputTooLarge {
                actual: bytes.len(),
                limit: limits.max_input_bytes,
            });
        }
        Ok(Self { bytes })
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub fn read(&self, offset: usize, length: usize) -> Result<&'a [u8], MemoryError> {
        let end = offset
            .checked_add(length)
            .ok_or(MemoryError::IntegerOverflow)?;
        self.bytes.get(offset..end).ok_or(MemoryError::OutOfBounds {
            offset,
            length,
            total: self.bytes.len(),
        })
    }
    pub fn range(&self, range: Range<usize>) -> Result<&'a [u8], MemoryError> {
        self.read(
            range.start,
            range
                .end
                .checked_sub(range.start)
                .ok_or(MemoryError::IntegerOverflow)?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_checks_and_enforces_resource_limit() {
        let image = ReadOnlyImage::new(&[1, 2, 3], ResourceLimits { max_input_bytes: 3 }).unwrap();
        assert_eq!(image.read(1, 2).unwrap(), &[2, 3]);
        assert!(matches!(
            image.read(usize::MAX, 2),
            Err(MemoryError::IntegerOverflow)
        ));
        assert!(matches!(
            image.read(2, 2),
            Err(MemoryError::OutOfBounds { .. })
        ));
        assert!(matches!(
            ReadOnlyImage::new(&[0; 4], ResourceLimits { max_input_bytes: 3 }),
            Err(MemoryError::InputTooLarge { .. })
        ));
    }
}

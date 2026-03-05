use crate::error::ParseError;

/// Ensures that the byte slice is long enough.
///
/// # Errors
///
/// This function will return an error if the byte slice is not long enough.
#[inline]
pub fn require_len(bytes: &[u8], min_len: usize) -> Result<(), ParseError> {
    if bytes.len() < min_len {
        return Err(ParseError::BufferTooSmall {
            expected: min_len,
            actual: bytes.len(),
        });
    }
    Ok(())
}

#[inline]
#[must_use]
pub fn be_u16(bytes: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([bytes[off], bytes[off + 1]])
}

#[inline]
#[must_use]
pub fn be_u32(bytes: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

#[inline]
#[must_use]
pub fn be_u64(bytes: &[u8], off: usize) -> u64 {
    u64::from_be_bytes([
        bytes[off],
        bytes[off + 1],
        bytes[off + 2],
        bytes[off + 3],
        bytes[off + 4],
        bytes[off + 5],
        bytes[off + 6],
        bytes[off + 7],
    ])
}

#[inline]
#[must_use]
pub fn le_u32(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

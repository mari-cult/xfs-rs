use crate::ParseError;
use crate::endian::{be_u16, be_u32, be_u64, require_len};

pub const XFS_IBT_MAGIC: u32 = 0x4941_4254;
pub const XFS_IBT_CRC_MAGIC: u32 = 0x4941_4233;
pub const XFS_FIBT_MAGIC: u32 = 0x4649_4254;
pub const XFS_FIBT_CRC_MAGIC: u32 = 0x4649_4233;
pub const XFS_BTREE_SBLOCK_CRC_OFF: usize = 52;
pub const XFS_BTREE_SBLOCK_CRC_LEN: usize = 56;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeBtreeKind {
    Inobt,
    Finobt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InodeBtreeRoot {
    pub kind: InodeBtreeKind,
    pub magic: u32,
    pub level: u16,
    pub numrecs: u16,
    pub leftsib: u32,
    pub rightsib: u32,
    pub blkno: u64,
    pub lsn: u64,
    pub uuid: [u8; 16],
    pub owner: u32,
    pub crc: u32,
}

impl InodeBtreeRoot {
    pub fn parse(bytes: &[u8], kind: InodeBtreeKind, crc_enabled: bool) -> Result<Self, ParseError> {
        if !crc_enabled {
            return Err(ParseError::UnsupportedVersion(4));
        }
        require_len(bytes, XFS_BTREE_SBLOCK_CRC_LEN)?;

        let magic = be_u32(bytes, 0);
        let expected = match kind {
            InodeBtreeKind::Inobt => XFS_IBT_CRC_MAGIC,
            InodeBtreeKind::Finobt => XFS_FIBT_CRC_MAGIC,
        };
        if magic != expected {
            return Err(ParseError::InvalidMagic {
                expected,
                actual: magic,
            });
        }

        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(&bytes[32..48]);

        Ok(Self {
            kind,
            magic,
            level: be_u16(bytes, 4),
            numrecs: be_u16(bytes, 6),
            leftsib: be_u32(bytes, 8),
            rightsib: be_u32(bytes, 12),
            blkno: be_u64(bytes, 16),
            lsn: be_u64(bytes, 24),
            uuid,
            owner: be_u32(bytes, 48),
            crc: u32::from_le_bytes([bytes[52], bytes[53], bytes[54], bytes[55]]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_be16(buf: &mut [u8], off: usize, value: u16) {
        buf[off..off + 2].copy_from_slice(&value.to_be_bytes());
    }
    fn put_be32(buf: &mut [u8], off: usize, value: u32) {
        buf[off..off + 4].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn parse_inobt_crc_header() {
        let mut raw = [0u8; XFS_BTREE_SBLOCK_CRC_LEN];
        put_be32(&mut raw, 0, XFS_IBT_CRC_MAGIC);
        put_be16(&mut raw, 4, 1);
        put_be16(&mut raw, 6, 3);
        put_be32(&mut raw, 8, 0xffff_ffff);
        put_be32(&mut raw, 12, 2);
        put_be32(&mut raw, 48, 0);
        let root = InodeBtreeRoot::parse(&raw, InodeBtreeKind::Inobt, true).expect("parse");
        assert_eq!(root.level, 1);
        assert_eq!(root.numrecs, 3);
        assert_eq!(root.rightsib, 2);
    }
}

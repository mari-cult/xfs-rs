use crate::endian::{be_u32, be_u64, require_len};
use crate::error::ParseError;

pub const XFS_AGF_MAGIC: u32 = 0x5841_4746;
pub const XFS_AGF_VERSION: u32 = 1;
pub const XFS_AGF_SIZE: usize = 224;
pub const XFS_AGF_CRC_OFF: usize = 216;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Agf {
    pub seqno: u32,
    pub length: u32,
    pub bno_root: u32,
    pub cnt_root: u32,
    pub rmap_root: u32,
    pub bno_level: u32,
    pub cnt_level: u32,
    pub rmap_level: u32,
    pub flfirst: u32,
    pub fllast: u32,
    pub flcount: u32,
    pub freeblks: u32,
    pub longest: u32,
    pub btreeblks: u32,
    pub uuid: [u8; 16],
    pub rmap_blocks: u32,
    pub refcount_blocks: u32,
    pub refcount_root: u32,
    pub refcount_level: u32,
    pub lsn: u64,
    pub crc: u32,
}

impl Agf {
    /// # Errors
    ///
    /// * [`ParseError`]
    pub fn parse(bytes: &[u8]) -> Result<Self, ParseError> {
        require_len(bytes, XFS_AGF_SIZE)?;

        let magic = be_u32(bytes, 0);
        if magic != XFS_AGF_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: XFS_AGF_MAGIC,
                actual: magic,
            });
        }

        let version = be_u32(bytes, 4);
        if version != XFS_AGF_VERSION {
            return Err(ParseError::UnsupportedVersion(version));
        }

        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(&bytes[64..80]);

        Ok(Self {
            seqno: be_u32(bytes, 8),
            length: be_u32(bytes, 12),
            bno_root: be_u32(bytes, 16),
            cnt_root: be_u32(bytes, 20),
            rmap_root: be_u32(bytes, 24),
            bno_level: be_u32(bytes, 28),
            cnt_level: be_u32(bytes, 32),
            rmap_level: be_u32(bytes, 36),
            flfirst: be_u32(bytes, 40),
            fllast: be_u32(bytes, 44),
            flcount: be_u32(bytes, 48),
            freeblks: be_u32(bytes, 52),
            longest: be_u32(bytes, 56),
            btreeblks: be_u32(bytes, 60),
            uuid,
            rmap_blocks: be_u32(bytes, 80),
            refcount_blocks: be_u32(bytes, 84),
            refcount_root: be_u32(bytes, 88),
            refcount_level: be_u32(bytes, 92),
            lsn: be_u64(bytes, 208),
            crc: be_u32(bytes, 216),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_be32(buf: &mut [u8], off: usize, value: u32) {
        buf[off..off + 4].copy_from_slice(&value.to_be_bytes());
    }
    fn put_be64(buf: &mut [u8], off: usize, value: u64) {
        buf[off..off + 8].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn parse_valid_agf() {
        let mut raw = [0u8; XFS_AGF_SIZE];
        put_be32(&mut raw, 0, XFS_AGF_MAGIC);
        put_be32(&mut raw, 4, XFS_AGF_VERSION);
        put_be32(&mut raw, 8, 3);
        put_be32(&mut raw, 12, 262_144);
        put_be32(&mut raw, 52, 1234);
        put_be64(&mut raw, 208, 0x55aa);
        put_be32(&mut raw, 216, 0xface_cafe);

        let agf = Agf::parse(&raw).expect("agf should parse");
        assert_eq!(agf.seqno, 3);
        assert_eq!(agf.length, 262_144);
        assert_eq!(agf.freeblks, 1234);
        assert_eq!(agf.lsn, 0x55aa);
        assert_eq!(agf.crc, 0xface_cafe);
    }

    #[test]
    fn rejects_bad_version() {
        let mut raw = [0u8; XFS_AGF_SIZE];
        put_be32(&mut raw, 0, XFS_AGF_MAGIC);
        put_be32(&mut raw, 4, 99);
        let err = Agf::parse(&raw).expect_err("version should fail");
        assert!(matches!(err, ParseError::UnsupportedVersion(99)));
    }
}

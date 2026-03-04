use crate::on_disk::superblock::Superblock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub block_size: u32,
    pub sector_size: u16,
    pub ag_blocks: u32,
    pub ag_count: u32,
}

impl Geometry {
    #[inline]
    pub fn from_superblock(sb: &Superblock) -> Self {
        Self {
            block_size: sb.block_size,
            sector_size: sb.sector_size,
            ag_blocks: sb.ag_blocks,
            ag_count: sb.ag_count,
        }
    }

    #[inline]
    pub fn fsb_to_bytes(self, fsb: u64) -> u64 {
        fsb.saturating_mul(self.block_size as u64)
    }

    #[inline]
    pub fn bytes_to_fsb(self, bytes: u64) -> u64 {
        bytes / self.block_size as u64
    }
}

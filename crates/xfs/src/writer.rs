use crate::crc::write_xfs_crc;
use crate::device::BlockDevice;
use crate::error::WriteError;
use crate::on_disk::agf::{Agf, XFS_AGF_CRC_OFF, XFS_AGF_SIZE};
use crate::on_disk::agfl::{Agfl, XFS_AGFL_CRC_OFF, XFS_AGFL_HEADER_SIZE};
use crate::on_disk::agi::{Agi, XFS_AGI_CRC_OFF, XFS_AGI_SIZE};
use crate::on_disk::inobt::{
    InodeBtreeKind, InodeBtreeRoot, XFS_BTREE_SBLOCK_CRC_OFF, XFS_FIBT_CRC_MAGIC, XFS_IBT_CRC_MAGIC,
};
use crate::on_disk::superblock::{
    Superblock, V5Fields, XFS_DSB_SIZE, XFS_SB_CRC_OFF, XFS_SB_VERSION_5,
};

#[derive(Debug, Clone, Copy)]
pub struct MkfsOptions {
    pub block_size: u32,
    pub sector_size: u16,
    pub ag_blocks: u32,
    pub total_blocks: u64,
    pub uuid: [u8; 16],
}

fn ilog2(mut n: u64) -> u8 {
    let mut res = 0;
    while n > 1 {
        n >>= 1;
        res += 1;
    }
    res
}

/// # Errors
///
/// * [`WriteError`]
#[allow(clippy::too_many_lines)]
pub fn mkfs<D: BlockDevice>(dev: &mut D, opts: &MkfsOptions) -> Result<(), WriteError> {
    let ag_count = opts.total_blocks.div_ceil(u64::from(opts.ag_blocks));
    #[allow(clippy::cast_possible_truncation)]
    let ag_count = ag_count as u32;

    let sb = Superblock {
        block_size: opts.block_size,
        dblocks: opts.total_blocks,
        rblocks: 0,
        rextents: 0,
        uuid: opts.uuid,
        logstart: 0, // No log for now
        rootino: 0,  // Will be set later
        rbmino: 0,
        rsumino: 0,
        rextsize: 0,
        ag_blocks: opts.ag_blocks,
        ag_count,
        rbm_blocks: 0,
        log_blocks: 0,
        version: XFS_SB_VERSION_5,
        sector_size: opts.sector_size,
        inode_size: 512,
        #[allow(clippy::cast_possible_truncation)]
        inode_per_block: (opts.block_size / 512) as u16,
        fname: [0u8; 12],
        block_log: ilog2(u64::from(opts.block_size)),
        sect_log: ilog2(u64::from(opts.sector_size)),
        inode_log: 9, // 2^9 = 512
        inprogress: false,
        icount: 0,
        ifree: 0,
        fdblocks: opts.total_blocks, // Roughly
        frextents: 0,
        uquotino: 0,
        gquotino: 0,
        qflags: 0,
        flags: 0,
        shared_vn: 0,
        inoalignmt: 0,
        unit: 0,
        width: 0,
        log_sector_size: opts.sector_size,
        log_sunit: 0,
        features2: 0,
        bad_features2: 0,
        v5: Some(V5Fields {
            features_compat: 0,
            features_ro_compat: 0,
            features_incompat: 0,
            features_log_incompat: 0,
            crc: 0,
            sparse_inode_align: 0,
            pquotino: 0,
            lsn: 0,
            meta_uuid: opts.uuid,
            metadirino: 0,
            rgcount: 0,
            rgextents: 0,
            rgblklog: 0,
            rtstart: 0,
            rtreserved: 0,
        }),
    };

    let mut sector = [0u8; 4096]; // Max sector size

    for agno in 0..ag_count {
        let ag_start = u64::from(agno) * u64::from(opts.ag_blocks) * u64::from(opts.block_size);

        // 1. Write Superblock
        sector.fill(0);
        sb.serialize(&mut sector[..XFS_DSB_SIZE])?;
        write_xfs_crc(&mut sector[..opts.sector_size as usize], XFS_SB_CRC_OFF);
        dev.write_at(ag_start, &sector[..opts.sector_size as usize])?;

        // 2. Write AGF
        let agf = Agf {
            seqno: agno,
            length: opts.ag_blocks,
            bno_root: 4,
            cnt_root: 5,
            rmap_root: 0,
            bno_level: 1,
            cnt_level: 1,
            rmap_level: 0,
            flfirst: 0,
            fllast: 0,
            flcount: 0,
            freeblks: opts.ag_blocks - 10,
            longest: opts.ag_blocks - 10,
            btreeblks: 0,
            uuid: opts.uuid,
            rmap_blocks: 0,
            refcount_blocks: 0,
            refcount_root: 0,
            refcount_level: 0,
            lsn: 0,
            crc: 0,
        };
        sector.fill(0);
        agf.serialize(&mut sector[..XFS_AGF_SIZE])?;
        write_xfs_crc(&mut sector[..opts.sector_size as usize], XFS_AGF_CRC_OFF);
        dev.write_at(
            ag_start + u64::from(opts.sector_size),
            &sector[..opts.sector_size as usize],
        )?;

        // 3. Write AGI
        let agi = Agi {
            seqno: agno,
            length: opts.ag_blocks,
            count: 0,
            root: 6,
            level: 1,
            freecount: 0,
            newino: 0xffff_ffff,
            dirino: 0xffff_ffff,
            unlinked: [0xffff_ffff; 64],
            uuid: opts.uuid,
            crc: 0,
            lsn: 0,
            free_root: 7,
            free_level: 1,
            iblocks: 0,
            fblocks: 0,
        };
        sector.fill(0);
        agi.serialize(&mut sector[..XFS_AGI_SIZE])?;
        write_xfs_crc(&mut sector[..opts.sector_size as usize], XFS_AGI_CRC_OFF);
        dev.write_at(
            ag_start + 2 * u64::from(opts.sector_size),
            &sector[..opts.sector_size as usize],
        )?;

        // 4. Write AGFL
        let agfl = Agfl {
            magicnum: 0,
            seqno: agno,
            uuid: opts.uuid,
            lsn: 0,
            crc: 0,
            entries_total: 0,
        };
        sector.fill(0);
        agfl.serialize(&mut sector[..XFS_AGFL_HEADER_SIZE], true)?;
        write_xfs_crc(&mut sector[..opts.sector_size as usize], XFS_AGFL_CRC_OFF);
        dev.write_at(
            ag_start + 3 * u64::from(opts.sector_size),
            &sector[..opts.sector_size as usize],
        )?;

        // 5. Write B-tree roots (bnobt, cntbt, inobt, finobt)
        let root_blocks: [(u32, u32); 4] = [
            (4, 0x4142_3342 /* BNO3 */),
            (5, 0x434e_3342 /* CNT3 */),
            (6, XFS_IBT_CRC_MAGIC),
            (7, XFS_FIBT_CRC_MAGIC),
        ];

        for (blk, magic) in root_blocks {
            let root = InodeBtreeRoot {
                kind: InodeBtreeKind::Inobt, // Dummy kind
                magic,
                level: 1,
                numrecs: 0,
                leftsib: 0xffff_ffff,
                rightsib: 0xffff_ffff,
                blkno: u64::from(agno) * u64::from(opts.ag_blocks) + u64::from(blk),
                lsn: 0,
                uuid: opts.uuid,
                owner: agno,
                crc: 0,
            };
            sector.fill(0);
            root.serialize(&mut sector[..XFS_DSB_SIZE])?;
            write_xfs_crc(
                &mut sector[..opts.block_size as usize],
                XFS_BTREE_SBLOCK_CRC_OFF,
            );
            dev.write_at(
                ag_start + u64::from(blk) * u64::from(opts.block_size),
                &sector[..opts.block_size as usize],
            )?;
        }
    }

    Ok(())
}

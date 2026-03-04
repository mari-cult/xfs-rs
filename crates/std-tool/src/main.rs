fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), libxfs_rs::ReadError> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/Users/theo/src/xfsprogs-dev/test.xfs".to_string());

    let mut dev = libxfs_rs::StdFileDevice::open(path).map_err(libxfs_rs::ReadError::from)?;
    let sb = libxfs_rs::read_superblock(&mut dev)?;
    let agf = libxfs_rs::read_agf(&mut dev, &sb, 0)?;
    let agi = libxfs_rs::read_agi(&mut dev, &sb, 0)?;
    let agfl = libxfs_rs::read_agfl(&mut dev, &sb, 0)?;
    let mut scratch = vec![0u8; sb.block_size as usize];
    let inobt = libxfs_rs::read_inobt_root(&mut dev, &sb, &agi, 0, &mut scratch)?;
    let finobt = if sb.has_ro_compat_feature(libxfs_rs::on_disk::superblock::XFS_SB_FEAT_RO_COMPAT_FINOBT)
    {
        Some(libxfs_rs::read_finobt_root(&mut dev, &sb, &agi, 0, &mut scratch)?)
    } else {
        None
    };

    let incompat = sb.v5.map(|v| v.features_incompat).unwrap_or(0);
    let ro_compat = sb.v5.map(|v| v.features_ro_compat).unwrap_or(0);
    println!(
        "SB version={} block_size={} ag_count={} inode_size={} ro_compat=0x{:08x} incompat=0x{:08x}",
        sb.version, sb.block_size, sb.ag_count, sb.inode_size, ro_compat, incompat
    );
    println!(
        "AGF seqno={} length={} freeblks={} longest={}",
        agf.seqno, agf.length, agf.freeblks, agf.longest
    );
    println!(
        "AGI seqno={} length={} count={} freecount={} root={} free_root={} iblocks={} fblocks={}",
        agi.seqno,
        agi.length,
        agi.count,
        agi.freecount,
        agi.root,
        agi.free_root,
        agi.iblocks,
        agi.fblocks
    );
    println!(
        "AGFL seqno={} entries_total={} lsn={}",
        agfl.seqno, agfl.entries_total, agfl.lsn
    );
    println!(
        "INOBT level={} numrecs={} owner={} blkno={}",
        inobt.level, inobt.numrecs, inobt.owner, inobt.blkno
    );
    if let Some(finobt) = finobt {
        println!(
            "FINOBT level={} numrecs={} owner={} blkno={}",
            finobt.level, finobt.numrecs, finobt.owner, finobt.blkno
        );
    }

    Ok(())
}

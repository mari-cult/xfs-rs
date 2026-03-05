use clap::Parser as _;
use rootcause::{Report, prelude::ResultExt};

#[derive(clap::Parser)]
struct Args {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    Info {
        #[clap(short, long)]
        file: std::path::PathBuf,
    },
    Mkfs {
        #[clap(short, long)]
        file: std::path::PathBuf,
        #[clap(short, long)]
        size: u64,
    },
}

fn main() -> Result<(), Report> {
    let args = Args::parse();

    match args.subcommand {
        Subcommand::Mkfs { file, size } => {
            let mut dev = xfs::device::StdFileDevice::create(&file)
                .attach(format!("Failed to create file: {}", file.display()))?;
            let opts = xfs::MkfsOptions {
                block_size: 4096,
                sector_size: 512,
                ag_blocks: 262_144, // 1GB AGs by default
                total_blocks: size / 4096,
                uuid: *uuid::Uuid::new_v4().as_bytes(),
            };
            xfs::mkfs(&mut dev, &opts).attach(format!("Failed to mkfs: {}", file.display()))?;
        }
        Subcommand::Info { file } => {
            let mut dev = xfs::device::StdFileDevice::open(&file)
                .attach(format!("Failed to open file: {}", file.display()))?;
            let sb = xfs::reader::read_superblock(&mut dev)
                .attach(format!("Failed to read superblock: {}", file.display()))?;
            let agf = xfs::reader::read_agf(&mut dev, &sb, 0)
                .attach(format!("Failed to read AGF: {}", file.display()))?;
            let agi = xfs::reader::read_agi(&mut dev, &sb, 0)
                .attach(format!("Failed to read AGI: {}", file.display()))?;
            let agfl = xfs::reader::read_agfl(&mut dev, &sb, 0)
                .attach(format!("Failed to read AGFL: {}", file.display()))?;
            let mut scratch = vec![0u8; sb.block_size as usize];
            let inobt = xfs::reader::read_inobt_root(&mut dev, &sb, &agi, 0, &mut scratch)
                .attach(format!("Failed to read inobt root: {}", file.display()))?;
            let finobt = if sb
                .has_ro_compat_feature(xfs::on_disk::superblock::XFS_SB_FEAT_RO_COMPAT_FINOBT)
            {
                Some(
                    xfs::reader::read_finobt_root(&mut dev, &sb, &agi, 0, &mut scratch)
                        .attach(format!("Failed to read finobt root: {}", file.display()))?,
                )
            } else {
                None
            };

            let incompat = sb.v5.map_or(0, |v| v.features_incompat);
            let ro_compat = sb.v5.map_or(0, |v| v.features_ro_compat);
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
        }
    }

    Ok(())
}

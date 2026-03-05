use clap::Parser as _;
use rootcause::{Report, prelude::ResultExt};

#[cfg(feature = "fuse")]
mod fuse;

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
        #[clap(short, long, value_parser = parse_size)]
        size: u64,
    },
    #[cfg(feature = "fuse")]
    Mount {
        #[clap(short, long)]
        file: std::path::PathBuf,
        #[clap(short, long)]
        mountpoint: std::path::PathBuf,
    },
}

fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let mut num_end = 0;
    while num_end < s.len() && s.as_bytes()[num_end].is_ascii_digit() {
        num_end += 1;
    }

    if num_end == 0 {
        return Err("size must start with a number".to_string());
    }

    let num: u64 = s[..num_end]
        .parse()
        .map_err(|e| format!("failed to parse number: {e}"))?;
    let unit = s[num_end..].trim().to_uppercase();

    let multiplier = match unit.as_str() {
        "" | "B" => 1,
        "K" | "KB" | "KIB" => 1024,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        "T" | "TB" | "TIB" => 1024 * 1024 * 1024 * 1024,
        _ => return Err(format!("unknown unit: {unit}")),
    };

    num.checked_mul(multiplier)
        .ok_or("size overflow".to_string())
}

#[allow(clippy::too_many_lines)]
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
        #[cfg(feature = "fuse")]
        Subcommand::Mount { file, mountpoint } => {
            let mut dev = xfs::device::StdFileDevice::open(&file)
                .attach(format!("Failed to open file: {}", file.display()))?;
            let sb = xfs::reader::read_superblock(&mut dev)
                .attach(format!("Failed to read superblock: {}", file.display()))?;
            let fs = fuse::XfsFuse {
                dev: std::sync::Arc::new(std::sync::Mutex::new(dev)),
                sb,
            };

            let mut fuse_config = fuser::Config::default();
            fuse_config.acl = fuser::SessionACL::All;
            fuse_config
                .mount_options
                .push(fuser::MountOption::AutoUnmount);
            fuse_config
                .mount_options
                .push(fuser::MountOption::DefaultPermissions);
            let mount_handle = fuser::spawn_mount2(fs, mountpoint.clone(), &fuse_config)
                .attach("Failed to mount filesystem")?;

            let stop_signal_received =
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let r = stop_signal_received.clone();

            ctrlc::set_handler(move || {
                r.store(true, std::sync::atomic::Ordering::SeqCst);
            })
            .attach("Failed to set Ctrl+C handler")?;

            println!(
                "Mounting at {}... (Ctrl+C to unmount)",
                mountpoint.display()
            );

            // 2. Main thread blocks here
            while !stop_signal_received.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            // 3. Explicitly drop the handle or let it fall out of scope here.
            // On macOS, this triggers the programmatic C-level unmount.
            println!("Unmounting...");
            drop(mount_handle);
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
                    xfs::reader::read_finobt_root(&mut dev, &sb, &agi, 0, &mut scratch).attach(
                        format!("Failed to read finobt root: {path}", path = file.display()),
                    )?,
                )
            } else {
                None
            };

            let incompat = sb.v5.map_or(0, |v| v.features_incompat);
            let ro_compat = sb.v5.map_or(0, |v| v.features_ro_compat);
            println!(
                "SB version={} block_size={} ag_count={} ag_blocks={} inode_size={} ro_compat=0x{:08x} incompat=0x{:08x}",
                sb.version,
                sb.block_size,
                sb.ag_count,
                sb.ag_blocks,
                sb.inode_size,
                ro_compat,
                incompat
            );
            println!(
                "LOGS block_log={} inopblog={} agblklog={}",
                sb.block_log, sb.inopblog, sb.agblklog
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

            let rootino = sb.rootino;
            let parsed_root_inode = xfs::reader::read_inode(&mut dev, &sb, rootino, &mut scratch)
                .attach(format!("Failed to read root inode ({rootino})"))?;
            println!("Root Inode: {parsed_root_inode:#?}");
        }
    }

    Ok(())
}

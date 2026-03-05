use fuser::{
    Errno, FileAttr, FileHandle, FileType, Filesystem, Generation, INodeNo, LockOwner, OpenFlags,
    ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use std::ffi::OsStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};
use xfs::device::BlockDevice;
use xfs::on_disk::inode::Inode;
use xfs::on_disk::superblock::Superblock;

const TTL: Duration = Duration::from_secs(1); // 1 second

pub struct XfsFuse<D> {
    pub dev: Arc<Mutex<D>>,
    pub sb: Superblock,
}

impl<D> XfsFuse<D> {
    fn map_ino(&self, ino: INodeNo) -> u64 {
        if ino.0 == 1 { self.sb.rootino } else { ino.0 }
    }
}

fn inode_to_attr(ino: &Inode) -> FileAttr {
    let kind = match ino.mode & 0xf000 {
        0x4000 => FileType::Directory,
        0xa000 => FileType::Symlink,
        0x2000 => FileType::CharDevice,
        0x6000 => FileType::BlockDevice,
        0x1000 => FileType::NamedPipe,
        0xc000 => FileType::Socket,
        _ => FileType::RegularFile,
    };

    #[allow(clippy::cast_sign_loss)]
    FileAttr {
        ino: INodeNo(ino.ino),
        size: ino.size as u64,
        blocks: ino.nblocks,
        atime: UNIX_EPOCH + Duration::from_secs(ino.atime.0 as u64),
        mtime: UNIX_EPOCH + Duration::from_secs(ino.mtime.0 as u64),
        ctime: UNIX_EPOCH + Duration::from_secs(ino.ctime.0 as u64),
        crtime: UNIX_EPOCH + Duration::from_secs(ino.crtime.0 as u64),
        kind,
        perm: ino.mode & 0x0fff,
        nlink: ino.nlink,
        uid: ino.uid,
        gid: ino.gid,
        rdev: 0,
        flags: 0,
        blksize: 512,
    }
}

impl<D: BlockDevice + Send + Sync + 'static> Filesystem for XfsFuse<D> {
    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let real_parent = self.map_ino(parent);
        let mut dev = self.dev.lock().unwrap();
        let Some(name_str) = name.to_str() else {
            reply.error(Errno::from_i32(libc::EINVAL));
            return;
        };

        match xfs::reader::list_dir_entries(&mut *dev, &self.sb, real_parent) {
            Ok(entries) => {
                if let Some(entry) = entries.iter().find(|e| e.name == name_str) {
                    let mut scratch = [0u8; 512];
                    match xfs::reader::read_inode(&mut *dev, &self.sb, entry.ino, &mut scratch) {
                        Ok(inode) => {
                            let mut attr = inode_to_attr(&inode);
                            if entry.ino == self.sb.rootino {
                                attr.ino = INodeNo(1);
                            }
                            reply.entry(&TTL, &attr, Generation(0));
                        }
                        Err(_) => reply.error(Errno::from_i32(libc::EIO)),
                    }
                } else {
                    reply.error(Errno::from_i32(libc::ENOENT));
                }
            }
            Err(_) => reply.error(Errno::from_i32(libc::EIO)),
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let real_ino = self.map_ino(ino);
        let mut dev = self.dev.lock().unwrap();
        let mut scratch = [0u8; 512];
        match xfs::reader::read_inode(&mut *dev, &self.sb, real_ino, &mut scratch) {
            Ok(inode) => {
                let mut attr = inode_to_attr(&inode);
                // We must use the inode number that the kernel expects.
                // If it's the root, use 1.
                if real_ino == self.sb.rootino {
                    attr.ino = INodeNo(1);
                }
                reply.attr(&TTL, &attr);
            }
            Err(_) => {
                reply.error(Errno::from_i32(libc::ENOENT));
            }
        }
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        let real_ino = self.map_ino(ino);
        let mut dev = self.dev.lock().unwrap();

        match xfs::reader::list_dir_entries(&mut *dev, &self.sb, real_ino) {
            Ok(entries) => {
                for (i, entry) in entries
                    .into_iter()
                    .enumerate()
                    .skip(usize::try_from(offset).unwrap())
                {
                    let mut ftype = match entry.ftype {
                        2 => FileType::Directory,
                        3 => FileType::Symlink,
                        4 => FileType::CharDevice,
                        5 => FileType::BlockDevice,
                        6 => FileType::NamedPipe,
                        7 => FileType::Socket,
                        _ => FileType::RegularFile,
                    };

                    let mut fuse_ino = entry.ino;
                    if fuse_ino == self.sb.rootino {
                        fuse_ino = 1;
                        ftype = FileType::Directory;
                    }

                    if reply.add(INodeNo(fuse_ino), (i + 1) as u64, ftype, entry.name) {
                        break;
                    }
                }
                reply.ok();
            }
            Err(_) => reply.error(Errno::from_i32(libc::EIO)),
        }
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        let real_ino = self.map_ino(ino);
        let mut dev = self.dev.lock().unwrap();

        match xfs::reader::read_file_data(&mut *dev, &self.sb, real_ino, offset, size) {
            Ok(data) => reply.data(&data),
            Err(_) => reply.error(Errno::from_i32(libc::EIO)),
        }
    }

    fn readlink(&self, _req: &Request, ino: INodeNo, reply: ReplyData) {
        let real_ino = self.map_ino(ino);
        let mut dev = self.dev.lock().unwrap();

        // Symlinks are usually small, 4KB should be enough
        match xfs::reader::read_file_data(&mut *dev, &self.sb, real_ino, 0, 4096) {
            Ok(data) => reply.data(&data),
            Err(_) => reply.error(Errno::from_i32(libc::EIO)),
        }
    }
}

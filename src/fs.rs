use crate::{
    inode::{KubeFSINodes, KubeFSInode, KubeFSLevel},
    KubeClient,
};
use fuse::{FileAttr, FileType, Filesystem, ReplyAttr, ReplyDirectory, ReplyEntry, Request};
use libc::ENOENT;
use std::ffi::OsStr;
use time::Timespec;

pub struct KubeFS {
    inodes: KubeFSINodes,
}

impl KubeFS {
    pub fn new(client: KubeClient) -> Self {
        KubeFS {
            inodes: KubeFSINodes::new(Box::new(client)),
        }
    }
}

#[derive(Debug, Clone)]
struct FSNamespace {
    name: String,
    attrs: FileAttr,
}

const TTL: Timespec = Timespec { sec: 1, nsec: 0 }; // 1 second

const CREATE_TIME: Timespec = Timespec {
    sec: 1381237736,
    nsec: 0,
};

impl Filesystem for KubeFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if let Some(name) = name.to_str() {
            let inode = self.inodes.lookup_inode_by_parent_and_name(&parent, name);
            if let Some(inode) = inode {
                reply.entry(
                    &TTL,
                    &create_file_attr(&inode),
                    0,
                )
            } else {
                reply.error(ENOENT)
            }
        } else {
            reply.error(ENOENT)
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let inode = self.inodes.get_inode(&ino);

        match inode {
            Some(inode) => {
                reply.attr(
                    &TTL,
                    &create_file_attr(&inode),
                )
            }
            None => reply.error(ENOENT),
        }
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, _size: u32, reply: ReplyData) {

        let data = self.inodes.get_file_contents(&ino);

        match data {
            Ok(data) => reply.data(&data.as_bytes()[offset as usize..]),
            Err(_) => reply.error(ENOENT)
        };
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let res = self.inodes.fetch_child_nodes_for_node(&ino);

        match res {
            Ok(_) => {
                let child_inodes = self.inodes.find_inode_by_parent(&ino);
                for (i, inode) in child_inodes.iter().enumerate().skip(offset as usize) {
                    reply.add(
                        inode.ino,
                        (i + 1) as i64,
                        match inode.level {
                            KubeFSLevel::File => FileType::RegularFile,
                            _ => FileType::Directory,
                        },
                        &inode.name,
                    );
                }
                reply.ok();
            }
            Err(_) => reply.error(ENOENT),
        };
    }
}

fn create_file_attr(inode: &KubeFSInode) -> FileAttr {
    FileAttr {
        ino: inode.ino,
        size: 0,
        blocks: 0,
        atime: CREATE_TIME,
        mtime: CREATE_TIME,
        ctime: CREATE_TIME,
        crtime: CREATE_TIME,
        kind: match inode.level {
            KubeFSLevel::File => FileType::RegularFile,
            _ => FileType::Directory,
        },
        perm: match inode.level {
            KubeFSLevel::File => 0o644,
            _ => 0o755,
        },
        nlink: 2,
        uid: 501,
        gid: 20,
        rdev: 0,
        flags: 0,
    }
}
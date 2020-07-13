use crate::{
    inode::{KubeFSINodes, KubeFSInode, KubeFSLevel},
    KubeClient,
};
use fuse::{FileAttr, FileType, Filesystem, ReplyAttr, ReplyDirectory, ReplyEntry, Request};
use libc::ENOENT;
use std::ffi::OsStr;
use time::Timespec;

const MAX_SUPPORTED_NAMESPACES: u64 = 10000;

pub struct KubeFS {
    namespaces: Vec<FSNamespace>,
    inodes: KubeFSINodes,
}

impl KubeFS {
    pub fn new(client: KubeClient) -> Self {
        KubeFS {
            namespaces: vec![],
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
        println!(" lookup parent is {} and name is {:?}", parent, name);

        if let Some(name) = name.to_str() {
            let inode = self.inodes.lookup_inode_by_parent_and_name(&parent, name);
            if let Some(inode) = inode {
                reply.entry(
                    &TTL,
                    &FileAttr {
                        ino: inode.ino,
                        size: 0,
                        blocks: 0,
                        atime: CREATE_TIME,
                        mtime: CREATE_TIME,
                        ctime: CREATE_TIME,
                        crtime: CREATE_TIME,
                        kind: match inode.level {
                            KubeFSLevel::file => FileType::RegularFile,
                            _ => FileType::Directory,
                        },
                        perm: 0o755,
                        nlink: 2,
                        uid: 501,
                        gid: 20,
                        rdev: 0,
                        flags: 0,
                    },
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
        println!(" getattr Ino is {}", ino);

        let inode = self.inodes.get_inode(&ino);

        println!(" getattr Inode is {:?}", inode);
        match inode {
            Some(inode) => {
                let level = inode.level;
                reply.attr(
                    &TTL,
                    &FileAttr {
                        ino: ino,
                        size: 0,
                        blocks: 0,
                        atime: CREATE_TIME,
                        mtime: CREATE_TIME,
                        ctime: CREATE_TIME,
                        crtime: CREATE_TIME,
                        kind: match level {
                            KubeFSLevel::file => FileType::RegularFile,
                            _ => FileType::Directory,
                        },
                        perm: 0o755,
                        nlink: 2,
                        uid: 501,
                        gid: 20,
                        rdev: 0,
                        flags: 0,
                    },
                )
            }
            None => reply.error(ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _offset: i64,
        mut reply: ReplyDirectory,
    ) {
        println!(" readdir Ino is {}", ino);

        let res = self.inodes.fetch_child_nodes_for_node(&ino);

        match res {
            Ok(_) => {
                let child_inodes = self.inodes.find_inode_by_parent(&ino);
                println!("CHild nodes for {} are {:?}", ino, child_inodes);
                for (i, inode) in child_inodes.iter().enumerate() {
                    reply.add(
                        inode.ino,
                        (i + 1) as i64,
                        match inode.level {
                            KubeFSLevel::file => FileType::RegularFile,
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

const ROOT_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: CREATE_TIME,
    mtime: CREATE_TIME,
    ctime: CREATE_TIME,
    crtime: CREATE_TIME,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

use crate::{
    inode::{KubeFSINodes, KubeFSInode, KubeFSLevel},
    KubeClient,
};
use fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyWrite, Request,
};
use libc::ENOENT;
use std::ffi::OsStr;
use time::Timespec;
use users::{get_current_uid, get_current_gid};

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

        println!("Lookup called with parent = {} and name = {:?}", parent, name);

        if let Some(name) = name.to_str() {
            let mut inode = self.inodes.lookup_inode_by_parent_and_name(&parent, name);

            if inode.is_none() {
                let res = self.inodes.fetch_child_nodes_for_node(&parent);
                if res.is_err() {
                    reply.error(ENOENT);
                    return
                }
                inode = self.inodes.lookup_inode_by_parent_and_name(&parent, name);
            }

            if let Some(inode) = inode {
                reply.entry(&TTL, &create_file_attr(&inode), 0)
            } else {
                reply.error(ENOENT)
            }
        } else {
            reply.error(ENOENT)
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {

        println!("getattr called with ino = {}", ino);

        let inode = self.inodes.get_inode(&ino);

        match inode {
            Some(inode) => reply.attr(&TTL, &create_file_attr(&inode)),
            None => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        reply: ReplyData,
    ) {
        println!("read called with ino = {}", ino);

        let data = self.inodes.get_file_contents(&ino);

        match data {
            Ok(data) => reply.data(&data.as_bytes()[offset as usize..]),
            Err(_) => reply.error(ENOENT),
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
        println!("readdir called with ino = {}", ino);
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

    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, reply: ReplyEntry) {
        if let Some(name) = name.to_str() {
            let res = self.inodes.create_object(name, &parent, &[]);

            match res {
                Ok(()) => {
                    let inode = KubeFSInode {
                        ino: 10000000,
                        level: KubeFSLevel::Namespace,
                        name: name.to_string(),
                        parent: Some(parent),
                    };

                    reply.entry(&TTL, &create_file_attr(&inode), 0);
                }
                Err(_) => {
                    reply.error(ENOENT);
                }
            };
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        if let Some(name) = name.to_str() {
            let res = self.inodes.delete_object(name, &parent);

            match res {
                Ok(()) => reply.ok(),
                Err(_) => reply.error(ENOENT),
            };
        }
    }

    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        fh: u64,
        _offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        println!(
            "Write called with ino = {}, data = {:?}, fh = {}",
            ino,
            data.to_ascii_lowercase(),
            fh
        );
        reply.written(data.len() as u32);
    }

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _flags: u32,
        reply: ReplyCreate,
    ) {
        println!("Create called with parent = {}, name = {:?}", parent, name);
        reply.created(
            &TTL,
            &FileAttr {
                ino: 10000,
                size: 10000,
                blocks: 0,
                atime: CREATE_TIME,
                mtime: CREATE_TIME,
                ctime: CREATE_TIME,
                crtime: CREATE_TIME,
                kind: FileType::RegularFile,
                perm: 0o644,
                nlink: 2,
                uid: 501,
                gid: 20,
                rdev: 0,
                flags: 0,
            },
            0,
            1,
            0o644,
        );
    }
}

fn create_file_attr(inode: &KubeFSInode) -> FileAttr {
    FileAttr {
        ino: inode.ino,
        size: 10000,
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
        uid: get_current_uid(),
        gid: get_current_gid(),
        rdev: 0,
        flags: 0,
    }
}

use crate::{
    inode::{KubeFSINodes, KubeFSInode, KubeFSLevel},
    KubeClient,
};
use fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyWrite, Request,
};
use libc::ENOENT;
use log::{info, error};
use std::{collections::HashMap, ffi::OsStr};
use time::Timespec;
use users::{get_current_gid, get_current_uid};

pub struct KubeFS {
    inodes: KubeFSINodes,
    swap_files: HashMap<String, SwapFile>,
}

const SWAP_FILE_START_INO: u64 = 1000000;

struct SwapFile {
    name: String,
    ino: u64,
}

impl KubeFS {
    pub fn new(client: KubeClient) -> Self {
        KubeFS {
            inodes: KubeFSINodes::new(Box::new(client)),
            swap_files: HashMap::new(),
        }
    }

    pub fn create_empty_swap_file(&mut self, name: &str) {
        let name = String::from(name);
        self.swap_files.insert(
            name.clone(),
            SwapFile {
                name: name.clone(),
                ino: SWAP_FILE_START_INO + (self.swap_files.len() as u64),
            },
        );
    }

    pub fn create_swap_file_attr(&self, name: &str) -> FileAttr {
        let name = name.to_string();
        let swap_file = self.swap_files.get(&name).unwrap();

        let inode = KubeFSInode {
            ino: swap_file.ino,
            parent: None,
            name: swap_file.name.clone(),
            level: KubeFSLevel::File,
        };

        self.create_file_attr(&inode)
    }

    fn create_file_attr(&self, inode: &KubeFSInode) -> FileAttr {
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
        info!(
            "Lookup called with parent = {} and name = {:?}",
            parent, name
        );

        if let Some(name) = name.to_str() {
            // If swap file then return
            if name.contains("swp") {
                if self.swap_files.get(name).is_some() {
                    reply.entry(&TTL, &self.create_swap_file_attr(&name), 0);
                    return;
                }
            }

            let mut inode = self.inodes.lookup_inode_by_parent_and_name(&parent, name);

            if inode.is_none() {
                let res = self.inodes.fetch_child_nodes_for_node(&parent);
                if res.is_err() {
                    reply.error(ENOENT);
                    return;
                }
                inode = self.inodes.lookup_inode_by_parent_and_name(&parent, name);
            }

            if let Some(inode) = inode {
                reply.entry(&TTL, &self.create_file_attr(&inode), 0)
            } else {
                reply.error(ENOENT)
            }
        } else {
            reply.error(ENOENT)
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        info!("getattr called with ino = {}", ino);

        let inode = self.inodes.get_inode(&ino);

        match inode {
            Some(inode) => reply.attr(&TTL, &self.create_file_attr(&inode)),
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
        info!("read called with ino = {}", ino);

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
        info!("readdir called with ino = {}", ino);
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

                    reply.entry(&TTL, &self.create_file_attr(&inode), 0);
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
        info!(
            "Write called with ino = {}, data = {:?}, fh = {}",
            ino,
            data.to_ascii_lowercase(),
            fh
        );

        let d = std::str::from_utf8(data);

        if let Ok(data) = d {
            // Find ino in nodes
            // Write to K8s
            match self.inodes.update_object(&ino, data) {
                Ok(_) => info!("write - update completed for ino {}", ino),
                Err(e) => error!("Error updating ino {}", e)
            };
        }

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
        info!("Create called with parent = {}, name = {:?}", parent, name);

        if let Some(name) = name.to_str() {
            // If swap then add to swap files
            if name.contains("swp") {
                self.create_empty_swap_file(name);

                reply.created(&TTL, &self.create_swap_file_attr(&name), 0, 1, 0o644);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        info!("Unlink called with parent = {}, name = {:?}", parent, name);
        if let Some(name) = name.to_str() {
            // If swap then remove swap file
            if name.contains("swp") {
                self.swap_files.remove(name);
                reply.ok();
            }
        } else {
            reply.error(ENOENT);
        }
    }
}

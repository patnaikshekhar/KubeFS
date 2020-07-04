use crate::{inode::K8sInteractions, KubeClient};
use fuse::{FileAttr, FileType, Filesystem, ReplyAttr, ReplyDirectory, ReplyEntry, Request};
use libc::ENOENT;
use std::ffi::OsStr;
use time::Timespec;

const MAX_SUPPORTED_NAMESPACES: u64 = 10000;

pub struct KubeFS {
    client: Box<dyn K8sInteractions>,
    namespaces: Vec<FSNamespace>,
}

impl KubeFS {
    pub fn new(client: KubeClient) -> Self {
        KubeFS {
            client: Box::new(client),
            namespaces: vec![],
        }
    }

    fn populate_namespaces(&mut self) -> anyhow::Result<()> {
        let ns = self.client.get_namespaces()?;

        let mut index = 1;

        self.namespaces = vec![];

        for namespace in ns {
            index += 1;

            self.namespaces.push(FSNamespace {
                name: namespace,
                attrs: FileAttr {
                    ino: index + 2,
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
                },
            });
        }

        Ok(())
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

        let mut entry: Option<FileAttr> = None;

        if parent == 1 {
            for namespace in self.namespaces.to_owned() {
                if namespace.name.eq(&name.to_str().unwrap_or_default()) {
                    entry = Some(namespace.attrs);
                    break;
                }
            }
        } else if parent > MAX_SUPPORTED_NAMESPACES {
            entry = Some(FileAttr {
                ino: (parent * MAX_SUPPORTED_NAMESPACES) + 1,
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
            })
        }

        match entry {
            Some(e) => reply.entry(&TTL, &e, 0),
            None => reply.error(ENOENT),
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        println!(" getattr Ino is {}", ino);

        match ino {
            1 => reply.attr(&TTL, &ROOT_DIR_ATTR),
            _ => reply.attr(
                &TTL,
                &FileAttr {
                    ino: ino,
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
                },
            ),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        println!(" readdir Ino is {}", ino);

        if ino == 1 {
            match self.populate_namespaces() {
                Ok(()) => {
                    let entries = self
                        .namespaces
                        .to_owned()
                        .into_iter()
                        .map(|ns| (ns.attrs.ino, FileType::Directory, ns.name));

                    for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
                        reply.add(entry.0, (i + 2) as i64, entry.1, entry.2);
                    }
                    reply.ok()
                }
                Err(_) => reply.error(ENOENT),
            }
        } else {
            let entries = vec![
                (1, FileType::Directory, "deployments"),
                (2, FileType::Directory, "services"),
                (3, FileType::Directory, "statefulsets"),
                (4, FileType::Directory, "configmaps"),
                (5, FileType::Directory, "secrets"),
                (6, FileType::Directory, "pods"),
            ];

            for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
                reply.add(
                    entry.0,
                    (ino * MAX_SUPPORTED_NAMESPACES + i as u64) as i64,
                    entry.1,
                    entry.2,
                );
            }
            reply.ok();
        }
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

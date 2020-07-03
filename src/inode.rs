#[derive(Debug, Clone, Copy)]
pub enum KubeFSLevel {
    root,
    namespace,
    object,
    file,
}

const KUBEFS_OBJECTS: [&str; 6] = [
    "deployments",
    "services",
    "pods",
    "statefulsets",
    "configmaps",
    "secrets",
];

#[derive(Debug, Clone)]
pub struct KubeFSInode {
    ino: u64,
    parent: Option<u64>,
    name: String,
    level: KubeFSLevel,
}

pub struct KubeFSINodes {
    inodes: Vec<KubeFSInode>,
}

impl KubeFSINodes {
    pub fn new() -> Self {
        KubeFSINodes {
            inodes: vec![KubeFSInode {
                ino: 1,
                parent: None,
                name: String::from("Root"),
                level: KubeFSLevel::root,
            }],
        }
    }

    pub fn fetch_child_nodes_for_node(&self, inode: KubeFSInode) {
        match inode.level {
            Root => {}
            Namespace => {}
            Object => {}
            File => {}
        }
    }

    pub fn find_inode_by_parent(&self, parent: u64) -> Vec<KubeFSInode> {
        self.inodes
            .iter()
            .filter(|inode| inode.parent == Some(parent))
            .cloned()
            .collect()
    }
    // pub fn lookup_inode_by_parent_and_name(&self, parent: u64, name: u64) -> KubeFSInode {}
}

// fn calculate_level_by_ino(ino: u64) -> KubeFSLevel {
//   let r = ino % MAX_SUPPORTED_NAMESPACES;
//   match (ino, r) {
//       (1, _) => KubeFSLevel::Root,
//       (_, 0..=5) => KubeFSLevel::Namespace,
//       (_, _) => KubeFSLevel::Object,
//   }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_inode_by_parent_root() {
        let mut inodes = KubeFSINodes::new();

        inodes.inodes.push(KubeFSInode {
            ino: 2,
            parent: Some(1),
            name: String::from("default"),
            level: KubeFSLevel::namespace,
        });

        inodes.inodes.push(KubeFSInode {
            ino: 3,
            parent: Some(1),
            name: String::from("dev"),
            level: KubeFSLevel::namespace,
        });

        inodes.inodes.push(KubeFSInode {
            ino: 4,
            parent: Some(2),
            name: String::from("prod"),
            level: KubeFSLevel::namespace,
        });

        let child_inodes = inodes.find_inode_by_parent(1);

        assert_eq!(child_inodes.len(), 2);
    }
}

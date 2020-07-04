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

pub trait K8sInteractions {
    fn get_namespaces(&mut self) -> Result<Vec<String>, anyhow::Error>;
}

pub struct KubeFSINodes {
    inodes: Vec<KubeFSInode>,
    client: Box<dyn K8sInteractions>,
}

impl KubeFSINodes {
    pub fn new(client: Box<dyn K8sInteractions>) -> Self {
        KubeFSINodes {
            inodes: vec![KubeFSInode {
                ino: 1,
                parent: None,
                name: String::from("Root"),
                level: KubeFSLevel::root,
            }],
            client: client,
        }
    }

    pub fn fetch_child_nodes_for_node(&mut self, inode: &KubeFSInode) {
        match inode.level {
            root => {}
            namespace => {}
            object => {}
            file => {}
        }
    }

    pub fn find_inode_by_parent(&self, parent: u64) -> Vec<KubeFSInode> {
        self.inodes
            .iter()
            .filter(|inode| inode.parent == Some(parent))
            .cloned()
            .collect()
    }

    pub fn lookup_inode_by_parent_and_name(&self, parent: u64, name: &str) -> Option<KubeFSInode> {
        self.inodes
            .iter()
            .filter(|inode| inode.parent == Some(parent) && inode.name == name)
            .cloned()
            .nth(0)
    }
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
        let mut inodes = KubeFSINodes::new(Box::new(MockClient {}));

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

    #[test]
    fn test_find_inode_by_parent_when_parent_does_not_exist() {
        let inodes = KubeFSINodes::new(Box::new(MockClient {}));

        let child_inodes = inodes.find_inode_by_parent(2);

        assert_eq!(child_inodes.len(), 0);
    }

    #[test]
    fn test_lookup_inode_by_parent_and_name() {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient {}));

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

        let inode = inodes.lookup_inode_by_parent_and_name(1, "dev");

        assert_ne!(true, inode.is_none());

        if let Some(n) = inode {
            assert_eq!(n.ino, 3);
        }
    }

    #[test]
    fn test_lookup_inode_by_parent_when_no_node_exists() {
        let inodes = KubeFSINodes::new(Box::new(MockClient {}));
        let inode = inodes.lookup_inode_by_parent_and_name(1, "dev");

        assert_eq!(true, inode.is_none());
    }

    #[test]
    fn test_fetch_child_nodes_for_node_when_root() {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient {}));

        let root_node = inodes.inodes[0].clone();

        inodes.fetch_child_nodes_for_node(&root_node);

        assert_eq!(inodes.inodes.len(), 4);
    }

    struct MockClient;

    impl K8sInteractions for MockClient {
        fn get_namespaces(&mut self) -> Result<Vec<String>, anyhow::Error> {
            return Ok(vec![
                String::from("default"),
                String::from("dev"),
                String::from("prod"),
            ]);
        }
    }
}

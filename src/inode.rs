use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display},
};

#[derive(Debug, Clone, Copy)]
pub enum KubeFSLevel {
    root,
    namespace,
    object,
    file,
}

const MAX_SUPPORTED_NAMESPACES: u64 = 10000;

const KUBEFS_OBJECTS: [&str; 6] = [
    "deployments",
    "services",
    "pods",
    "statefulsets",
    "configmaps",
    "secrets",
];

#[derive(Debug)]
enum KubeFSInodeError {
    MissingInode,
}

impl Error for KubeFSInodeError {}

impl Display for KubeFSInodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Missing Inode")
    }
}

#[derive(Debug, Clone)]
pub struct KubeFSInode {
    pub ino: u64,
    pub parent: Option<u64>,
    pub name: String,
    pub level: KubeFSLevel,
}

pub trait K8sInteractions {
    fn get_namespaces(&mut self) -> Result<Vec<String>, anyhow::Error>;
    fn get_objects(
        &mut self,
        namespace: &str,
        object_name: &str,
    ) -> Result<Vec<String>, anyhow::Error>;
}

pub struct KubeFSINodes {
    pub inodes: HashMap<u64, KubeFSInode>,
    client: Box<dyn K8sInteractions>,
}

impl KubeFSINodes {
    pub fn new(client: Box<dyn K8sInteractions>) -> Self {
        let mut inodes = HashMap::new();
        inodes.insert(
            1,
            KubeFSInode {
                ino: 1,
                parent: None,
                name: String::from("Root"),
                level: KubeFSLevel::root,
            },
        );

        KubeFSINodes {
            inodes: inodes,
            client: client,
        }
    }

    pub fn get_inode(&self, ino: &u64) -> Option<&KubeFSInode> {
        self.inodes.get(ino)
    }

    pub fn fetch_child_nodes_for_node(&mut self, inode: &KubeFSInode) -> Result<(), anyhow::Error> {
        match inode.level {
            KubeFSLevel::root => {
                // Delete all namespace nodes
                self.delete_by_parent_ino(&inode.ino);
                // Fetch all namespaces
                let namespaces = self.client.get_namespaces()?;

                // Add Namespace inodes
                for (i, ns) in namespaces.iter().enumerate() {
                    self.inodes.insert(
                        (i + 2) as u64,
                        KubeFSInode {
                            ino: (i + 2) as u64,
                            name: ns.clone(),
                            parent: Some(inode.ino),
                            level: KubeFSLevel::namespace,
                        },
                    );
                }
            }
            KubeFSLevel::namespace => {
                self.delete_by_parent_ino(&inode.ino);

                for (i, o) in KUBEFS_OBJECTS.iter().enumerate() {
                    self.inodes.insert(
                        MAX_SUPPORTED_NAMESPACES + (i as u64),
                        KubeFSInode {
                            ino: MAX_SUPPORTED_NAMESPACES + (i as u64),
                            name: o.to_string(),
                            parent: Some(inode.ino),
                            level: KubeFSLevel::object,
                        },
                    );
                }
            }
            KubeFSLevel::object => {
                self.delete_by_parent_ino(&inode.ino);

                let parent_ino = inode.parent.ok_or(KubeFSInodeError::MissingInode)?;
                let namespace_inode = self
                    .inodes
                    .get(&parent_ino)
                    .ok_or(KubeFSInodeError::MissingInode)?;
                let namespace_name = &namespace_inode.name;
                let object_name = &inode.name;

                let objects = self.client.get_objects(namespace_name, object_name)?;

                for (i, o) in objects.iter().enumerate() {
                    self.inodes.insert(
                        MAX_SUPPORTED_NAMESPACES + (KUBEFS_OBJECTS.len() + i) as u64,
                        KubeFSInode {
                            ino: MAX_SUPPORTED_NAMESPACES + (KUBEFS_OBJECTS.len() + i) as u64,
                            name: o.clone(),
                            parent: Some(inode.ino),
                            level: KubeFSLevel::file,
                        },
                    );
                }
            }
            KubeFSLevel::file => {}
        }

        Ok(())
    }

    pub fn find_inode_by_parent(&self, parent: &u64) -> Vec<KubeFSInode> {
        self.inodes
            .values()
            .filter(|inode| inode.parent == Some(*parent))
            .cloned()
            .collect()
    }

    pub fn lookup_inode_by_parent_and_name(&self, parent: &u64, name: &str) -> Option<KubeFSInode> {
        self.inodes
            .values()
            .filter(|inode| inode.parent == Some(*parent) && inode.name == name)
            .cloned()
            .nth(0)
    }

    fn delete_by_parent_ino(&mut self, parent: &u64) {
        self.inodes.retain(|_, inode| inode.parent != Some(*parent))
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

        inodes.inodes.insert(
            2,
            KubeFSInode {
                ino: 2,
                parent: Some(1),
                name: String::from("default"),
                level: KubeFSLevel::namespace,
            },
        );

        inodes.inodes.insert(
            3,
            KubeFSInode {
                ino: 3,
                parent: Some(1),
                name: String::from("dev"),
                level: KubeFSLevel::namespace,
            },
        );

        inodes.inodes.insert(
            4,
            KubeFSInode {
                ino: 4,
                parent: Some(2),
                name: String::from("prod"),
                level: KubeFSLevel::namespace,
            },
        );

        let child_inodes = inodes.find_inode_by_parent(&1);

        assert_eq!(child_inodes.len(), 2);
    }

    #[test]
    fn test_find_inode_by_parent_when_parent_does_not_exist() {
        let inodes = KubeFSINodes::new(Box::new(MockClient {}));

        let child_inodes = inodes.find_inode_by_parent(&2);

        assert_eq!(child_inodes.len(), 0);
    }

    #[test]
    fn test_lookup_inode_by_parent_and_name() {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient {}));

        inodes.inodes.insert(
            2,
            KubeFSInode {
                ino: 2,
                parent: Some(1),
                name: String::from("default"),
                level: KubeFSLevel::namespace,
            },
        );

        inodes.inodes.insert(
            3,
            KubeFSInode {
                ino: 3,
                parent: Some(1),
                name: String::from("dev"),
                level: KubeFSLevel::namespace,
            },
        );

        let inode = inodes.lookup_inode_by_parent_and_name(&1, "dev");

        assert_ne!(true, inode.is_none());

        if let Some(n) = inode {
            assert_eq!(n.ino, 3);
        }
    }

    #[test]
    fn test_lookup_inode_by_parent_when_no_node_exists() {
        let inodes = KubeFSINodes::new(Box::new(MockClient {}));
        let inode = inodes.lookup_inode_by_parent_and_name(&1, "dev");

        assert_eq!(true, inode.is_none());
    }

    #[test]
    fn test_delete_by_parent_ino() {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient {}));
        inodes.inodes.insert(
            2,
            KubeFSInode {
                ino: 2,
                parent: Some(1),
                name: String::from("default"),
                level: KubeFSLevel::namespace,
            },
        );

        inodes.inodes.insert(
            3,
            KubeFSInode {
                ino: 3,
                parent: Some(1),
                name: String::from("dev"),
                level: KubeFSLevel::namespace,
            },
        );

        inodes.inodes.insert(
            4,
            KubeFSInode {
                ino: 4,
                parent: None,
                name: String::from("dev"),
                level: KubeFSLevel::namespace,
            },
        );

        assert_eq!(inodes.inodes.len(), 4);

        inodes.delete_by_parent_ino(&1);
        assert_eq!(inodes.inodes.len(), 2);
    }

    #[test]
    fn test_fetch_child_nodes_for_node_when_root() -> Result<(), anyhow::Error> {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient {}));

        let root_node = inodes.inodes[&1].clone();

        inodes.fetch_child_nodes_for_node(&root_node)?;
        assert_eq!(inodes.inodes.len(), 4);
        println!("{:?}", inodes.inodes);
        assert_eq!(inodes.inodes.get(&2).unwrap().name, "default");

        Ok(())
    }

    #[test]
    fn test_fetch_child_nodes_for_node_when_namespace() -> Result<(), anyhow::Error> {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient {}));

        let root_node = inodes.inodes[&1].clone();

        inodes.fetch_child_nodes_for_node(&root_node)?;

        let default_namespace_node = inodes.inodes[&2].clone();

        inodes.fetch_child_nodes_for_node(&default_namespace_node)?;

        assert_eq!(inodes.inodes.len(), 4 + KUBEFS_OBJECTS.len());
        assert_eq!(
            inodes.inodes.get(&MAX_SUPPORTED_NAMESPACES).unwrap().name,
            KUBEFS_OBJECTS[0]
        );

        Ok(())
    }

    #[test]
    fn test_fetch_child_nodes_for_node_when_object() -> Result<(), anyhow::Error> {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient {}));

        let root_node = inodes.inodes[&1].clone();

        inodes.fetch_child_nodes_for_node(&root_node)?;

        let default_namespace_node = inodes.inodes[&2].clone();

        inodes.fetch_child_nodes_for_node(&default_namespace_node)?;

        let deployments_node = inodes.inodes[&MAX_SUPPORTED_NAMESPACES].clone();
        inodes.fetch_child_nodes_for_node(&deployments_node)?;

        // println!("Deployments index {:?}", MAX_SUPPORTED_NAMESPACES);
        // println!("{:?}", inodes.inodes);

        assert_eq!(inodes.inodes.len(), 7 + KUBEFS_OBJECTS.len());
        assert_eq!(
            inodes
                .inodes
                .get(&(MAX_SUPPORTED_NAMESPACES + KUBEFS_OBJECTS.len() as u64))
                .unwrap()
                .name,
            "deploy-1"
        );

        Ok(())
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

        fn get_objects(
            &mut self,
            namespace: &str,
            object_name: &str,
        ) -> Result<Vec<String>, anyhow::Error> {
            if namespace == "default" && object_name == "deployments" {
                Ok(vec![
                    String::from("deploy-1"),
                    String::from("deploy-2"),
                    String::from("deploy-3"),
                ])
            } else {
                Ok(vec![])
            }
        }
    }
}

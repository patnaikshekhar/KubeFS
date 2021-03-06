use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display},
};

#[derive(Debug, Clone, Copy)]
pub enum KubeFSLevel {
    Root,
    Namespace,
    Object,
    File,
}

const MAX_SUPPORTED_NAMESPACES: u64 = 10000;

const KUBEFS_OBJECTS: [&str; 7] = [
    "deployments",
    "services",
    "pods",
    "statefulsets",
    "configmaps",
    "secrets",
    "serviceaccounts",
];

#[derive(Debug)]
pub enum KubeFSInodeError {
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
    fn update_object(
        &mut self,
        name: &str,
        namespace: &str,
        object_name: &str,
        data: &str,
    ) -> Result<(), anyhow::Error>;
    fn get_object_data_as_yaml(
        &mut self,
        name: &str,
        namespace: &str,
        object_name: &str,
    ) -> anyhow::Result<String>;
    fn create_namespace(&mut self, name: &str) -> anyhow::Result<()>;
    fn remove_namespace(&mut self, name: &str) -> anyhow::Result<()>;
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
                level: KubeFSLevel::Root,
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

    pub fn fetch_child_nodes_for_node(&mut self, ino: &u64) -> anyhow::Result<()> {
        let inode = self
            .inodes
            .get(ino)
            .ok_or(KubeFSInodeError::MissingInode)?
            .clone();

        match inode.level {
            KubeFSLevel::Root => {
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
                            level: KubeFSLevel::Namespace,
                        },
                    );
                }
            }
            KubeFSLevel::Namespace => {
                self.delete_by_parent_ino(&inode.ino);

                for (i, o) in KUBEFS_OBJECTS.iter().enumerate() {
                    self.inodes.insert(
                        MAX_SUPPORTED_NAMESPACES + (i as u64),
                        KubeFSInode {
                            ino: MAX_SUPPORTED_NAMESPACES + (i as u64),
                            name: o.to_string(),
                            parent: Some(inode.ino),
                            level: KubeFSLevel::Object,
                        },
                    );
                }
            }
            KubeFSLevel::Object => {
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
                            level: KubeFSLevel::File,
                        },
                    );
                }
            }
            KubeFSLevel::File => {}
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

    pub fn get_file_contents(&mut self, ino: &u64) -> anyhow::Result<String> {
        let inode = self
            .get_inode(&ino)
            .ok_or(KubeFSInodeError::MissingInode)?
            .clone();

        match inode.level {
            KubeFSLevel::File => {
                let object = self
                    .get_inode(&inode.parent.ok_or(KubeFSInodeError::MissingInode)?)
                    .ok_or(KubeFSInodeError::MissingInode)?
                    .clone();

                let namespace = self
                    .get_inode(&object.parent.ok_or(KubeFSInodeError::MissingInode)?)
                    .ok_or(KubeFSInodeError::MissingInode)?
                    .clone();

                let data = self.client.get_object_data_as_yaml(
                    &inode.name,
                    &namespace.name,
                    &object.name,
                )?;

                Ok(data)
            }
            _ => Ok(String::new()),
        }
    }

    pub fn create_object(
        &mut self,
        name: &str,
        parent_ino: &u64,
        _data: &[u8],
    ) -> anyhow::Result<()> {
        let inode = self
            .get_inode(&parent_ino)
            .ok_or(KubeFSInodeError::MissingInode)?
            .clone();

        match inode.level {
            KubeFSLevel::Root => {
                self.client.create_namespace(name)?;
            }
            _ => {}
        };

        Ok(())
    }

    pub fn update_object(&mut self, ino: &u64, data: &str) -> anyhow::Result<()> {
        let inode = self
            .get_inode(&ino)
            .ok_or(KubeFSInodeError::MissingInode)?
            .clone();

        match inode.level {
            KubeFSLevel::File => {
                let object = self
                    .get_inode(&inode.parent.ok_or(KubeFSInodeError::MissingInode)?)
                    .ok_or(KubeFSInodeError::MissingInode)?
                    .clone();

                let namespace = self
                    .get_inode(&object.parent.ok_or(KubeFSInodeError::MissingInode)?)
                    .ok_or(KubeFSInodeError::MissingInode)?
                    .clone();

                self.client
                    .update_object(&inode.name, &namespace.name, &object.name, data)?;
            }
            _ => {}
        }

        Ok(())
    }

    pub fn delete_object(&mut self, name: &str, parent_ino: &u64) -> anyhow::Result<()> {
        let inode = self
            .get_inode(&parent_ino)
            .ok_or(KubeFSInodeError::MissingInode)?
            .clone();

        match inode.level {
            KubeFSLevel::Root => {
                self.client.remove_namespace(name)?;
            }
            _ => {}
        };

        Ok(())
    }

    fn delete_by_parent_ino(&mut self, parent: &u64) {
        self.inodes.retain(|_, inode| inode.parent != Some(*parent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_inode_by_parent_root() {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient::new()));

        inodes.inodes.insert(
            2,
            KubeFSInode {
                ino: 2,
                parent: Some(1),
                name: String::from("default"),
                level: KubeFSLevel::Namespace,
            },
        );

        inodes.inodes.insert(
            3,
            KubeFSInode {
                ino: 3,
                parent: Some(1),
                name: String::from("dev"),
                level: KubeFSLevel::Namespace,
            },
        );

        inodes.inodes.insert(
            4,
            KubeFSInode {
                ino: 4,
                parent: Some(2),
                name: String::from("prod"),
                level: KubeFSLevel::Namespace,
            },
        );

        let child_inodes = inodes.find_inode_by_parent(&1);

        assert_eq!(child_inodes.len(), 2);
    }

    #[test]
    fn test_find_inode_by_parent_when_parent_does_not_exist() {
        let inodes = KubeFSINodes::new(Box::new(MockClient::new()));

        let child_inodes = inodes.find_inode_by_parent(&2);

        assert_eq!(child_inodes.len(), 0);
    }

    #[test]
    fn test_lookup_inode_by_parent_and_name() {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient::new()));

        inodes.inodes.insert(
            2,
            KubeFSInode {
                ino: 2,
                parent: Some(1),
                name: String::from("default"),
                level: KubeFSLevel::Namespace,
            },
        );

        inodes.inodes.insert(
            3,
            KubeFSInode {
                ino: 3,
                parent: Some(1),
                name: String::from("dev"),
                level: KubeFSLevel::Namespace,
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
        let inodes = KubeFSINodes::new(Box::new(MockClient::new()));
        let inode = inodes.lookup_inode_by_parent_and_name(&1, "dev");

        assert_eq!(true, inode.is_none());
    }

    #[test]
    fn test_delete_by_parent_ino() {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient::new()));
        inodes.inodes.insert(
            2,
            KubeFSInode {
                ino: 2,
                parent: Some(1),
                name: String::from("default"),
                level: KubeFSLevel::Namespace,
            },
        );

        inodes.inodes.insert(
            3,
            KubeFSInode {
                ino: 3,
                parent: Some(1),
                name: String::from("dev"),
                level: KubeFSLevel::Namespace,
            },
        );

        inodes.inodes.insert(
            4,
            KubeFSInode {
                ino: 4,
                parent: None,
                name: String::from("dev"),
                level: KubeFSLevel::Namespace,
            },
        );

        assert_eq!(inodes.inodes.len(), 4);

        inodes.delete_by_parent_ino(&1);
        assert_eq!(inodes.inodes.len(), 2);
    }

    #[test]
    fn test_fetch_child_nodes_for_node_when_root() -> Result<(), anyhow::Error> {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient::new()));

        let root_node = inodes.inodes[&1].clone();

        inodes.fetch_child_nodes_for_node(&root_node.ino)?;
        assert_eq!(inodes.inodes.len(), 4);
        println!("{:?}", inodes.inodes);
        assert_eq!(inodes.inodes.get(&2).unwrap().name, "default");

        Ok(())
    }

    #[test]
    fn test_fetch_child_nodes_for_node_when_namespace() -> Result<(), anyhow::Error> {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient::new()));

        let root_node = inodes.inodes[&1].clone();

        inodes.fetch_child_nodes_for_node(&root_node.ino)?;

        let default_namespace_node = inodes.inodes[&2].clone();

        inodes.fetch_child_nodes_for_node(&default_namespace_node.ino)?;

        assert_eq!(inodes.inodes.len(), 4 + KUBEFS_OBJECTS.len());
        assert_eq!(
            inodes.inodes.get(&MAX_SUPPORTED_NAMESPACES).unwrap().name,
            KUBEFS_OBJECTS[0]
        );

        Ok(())
    }

    #[test]
    fn test_fetch_child_nodes_for_node_when_object() -> Result<(), anyhow::Error> {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient::new()));

        let root_node = inodes.inodes[&1].clone();

        inodes.fetch_child_nodes_for_node(&root_node.ino)?;

        let default_namespace_node = inodes.inodes[&2].clone();

        inodes.fetch_child_nodes_for_node(&default_namespace_node.ino)?;

        let deployments_node = inodes.inodes[&MAX_SUPPORTED_NAMESPACES].clone();
        inodes.fetch_child_nodes_for_node(&deployments_node.ino)?;

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

    #[test]
    fn test_get_yaml_for_file() -> Result<(), anyhow::Error> {
        let mut inodes = KubeFSINodes::new(Box::new(MockClient::new()));

        let root_node = inodes.inodes[&1].clone();

        inodes.fetch_child_nodes_for_node(&root_node.ino)?;

        let default_namespace_node = inodes.inodes[&2].clone();

        inodes.fetch_child_nodes_for_node(&default_namespace_node.ino)?;

        let deployments_node = inodes.inodes[&MAX_SUPPORTED_NAMESPACES].clone();
        inodes.fetch_child_nodes_for_node(&deployments_node.ino)?;

        let deploy_1_node = inodes
            .inodes
            .get(&(MAX_SUPPORTED_NAMESPACES + KUBEFS_OBJECTS.len() as u64))
            .ok_or(KubeFSInodeError::MissingInode)?
            .clone();

        let contents = inodes.get_file_contents(&deploy_1_node.ino)?;

        assert_eq!(contents, "Data");

        Ok(())
    }

    #[test]
    fn test_create_object_creates_namespace() -> Result<(), anyhow::Error> {
        let client = MockClient::new();
        let mut inodes = KubeFSINodes::new(Box::new(client));

        inodes.create_object("test", &1, &Vec::new())?;

        Ok(())
    }

    struct MockClient {}

    impl MockClient {
        pub fn new() -> Self {
            MockClient {}
        }
    }

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

        fn get_object_data_as_yaml(
            &mut self,
            name: &str,
            namespace: &str,
            object_name: &str,
        ) -> anyhow::Result<String> {
            if name == "deploy-1" && namespace == "default" && object_name == "deployments" {
                Ok(String::from("Data"))
            } else {
                Ok(String::new())
            }
        }

        fn create_namespace(&mut self, _name: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn update_object(
            &mut self,
            name: &str,
            namespace: &str,
            object_name: &str,
            data: &str,
        ) -> Result<(), anyhow::Error> {
            Ok(())
        }

        fn remove_namespace(&mut self, _name: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }
}

use crate::inode::K8sInteractions;
use k8s_openapi::api::apps::v1::{Deployment, StatefulSet};
use k8s_openapi::api::core::v1::{Namespace, Pod, Service};

use k8s_openapi::Resource;
use serde::de::DeserializeOwned;
use std::clone::Clone;

use kube::{
    api::{ListParams, Meta},
    Api, Client,
};

use tokio::runtime::Runtime;

pub struct KubeClient {
    client: Client,
    runtime: Runtime,
}

impl KubeClient {
    pub fn new() -> Self {
        let mut runtime = Runtime::new().unwrap();

        KubeClient {
            client: runtime.block_on(Client::try_default()).unwrap(),
            runtime: runtime,
        }
    }

    // async fn get_objects(&mut self, namespace: &str, objects: Api<Resource>) -> anyhow::Result<Vec<String>> {
    //     // let objects: Api<T> = Api::namespaced(self.client.clone(), namespace);

    //     let lp = ListParams::default();

    //     let objectList = objects.list(lp).await?;

    //     Ok(objectList.iter().map(|o| { Meta::name(o) }))
    // }
}

struct KubeObject<T: Resource + Clone + DeserializeOwned + Meta> {
    x: T,
    client: Client,
}

impl<T: Resource + Clone + DeserializeOwned + Meta> KubeObject<T> {
    async fn get_objects(&self, namespace: &str) -> Result<Vec<String>, anyhow::Error> {
        let objects: Api<T> = Api::<T>::namespaced(self.client.clone(), namespace);

        let lp = ListParams::default();

        let objectList = objects.list(&lp).await?;

        Ok(objectList.iter().map(|o| Meta::name(o)).collect())
    }
}

impl K8sInteractions for KubeClient {
    fn get_namespaces(&mut self) -> Result<Vec<String>, anyhow::Error> {
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        let lp = ListParams::default();

        let ns = self.runtime.block_on(namespaces.list(&lp))?;
        let mut res: Vec<String> = vec![];

        for n in ns {
            let name = Meta::name(&n);
            res.push(name);
        }

        Ok(res)
    }

    fn get_objects(
        &mut self,
        namespace: &str,
        object_name: &str,
    ) -> Result<Vec<String>, anyhow::Error> {
        let lp = ListParams::default();
        // objectList.iter().map(|o| { Meta::name(o) })

        match object_name {
            "deployments" => {
                let objects = Api::<Deployment>::namespaced(self.client.clone(), namespace);
                let objectList = self.runtime.block_on(objects.list(&lp))?;
            }
            "services" => {
                let objects = Api::<Service>::namespaced(self.client.clone(), namespace);
                let objectList = self.runtime.block_on(objects.list(&lp))?;
            }
            _ => {}
        };

        Ok(vec![])
    }
}

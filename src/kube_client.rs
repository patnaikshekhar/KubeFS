use crate::inode::K8sInteractions;
use k8s_openapi::{
    api::{
        apps::v1::{Deployment, StatefulSet},
        core::v1::{ConfigMap, Namespace, Pod, Secret, Service, ServiceAccount},
    },
    Resource,
};

use serde::{de::DeserializeOwned, ser::Serialize};
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

    fn get_object_names<T: Resource + Clone + DeserializeOwned + Meta>(
        &mut self,
        namespace: &str,
    ) -> Result<Vec<String>, anyhow::Error> {
        let objects: Api<T> = Api::<T>::namespaced(self.client.clone(), namespace);

        let lp = ListParams::default();

        let object_list = self.runtime.block_on(objects.list(&lp))?;

        Ok(object_list.iter().map(|o| Meta::name(o)).collect())
    }

    fn get_object<T: Resource + Clone + DeserializeOwned + Meta + Serialize>(
        &mut self,
        name: &str,
        namespace: &str,
    ) -> Result<String, anyhow::Error> {
        let objects: Api<T> = Api::<T>::namespaced(self.client.clone(), namespace);

        let o = self.runtime.block_on(objects.get(name))?;

        Ok(serde_yaml::to_string(&o)?)
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
        let res = match object_name {
            "deployments" => self.get_object_names::<Deployment>(namespace)?,
            "pods" => self.get_object_names::<Pod>(namespace)?,
            "services" => self.get_object_names::<Service>(namespace)?,
            "statefulsets" => self.get_object_names::<StatefulSet>(namespace)?,
            "configmaps" => self.get_object_names::<ConfigMap>(namespace)?,
            "secrets" => self.get_object_names::<Secret>(namespace)?,
            "serviceaccounts" => self.get_object_names::<ServiceAccount>(namespace)?,
            _ => vec![],
        };

        Ok(res)
    }

    fn get_object_data_as_yaml(
        &mut self,
        name: &str,
        namespace: &str,
        object_name: &str,
    ) -> anyhow::Result<String> {
        let data = match object_name {
            "deployments" => self.get_object::<Deployment>(name, namespace)?,
            "pods" => self.get_object::<Pod>(name, namespace)?,
            "services" => self.get_object::<Service>(name, namespace)?,
            "statefulsets" => self.get_object::<StatefulSet>(name, namespace)?,
            "configmaps" => self.get_object::<ConfigMap>(name, namespace)?,
            "secrets" => self.get_object::<Secret>(name, namespace)?,
            "serviceaccounts" => self.get_object::<ServiceAccount>(name, namespace)?,
            _ => String::new(),
        };

        Ok(data)
    }
}

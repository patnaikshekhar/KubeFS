use async_std::task::block_on;
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{ListParams, Meta},
    Api, Client,
};

pub struct KubeClient {
    client: Client,
}

impl KubeClient {
    pub fn new() -> Self {
        KubeClient {
            client: block_on(Client::try_default()).unwrap(),
        }
    }

    pub fn get_namespaces(self) -> Result<Vec<String>, anyhow::Error> {
        let namespaces: Api<Namespace> = Api::all(self.client);
        let lp = ListParams::default();

        println!("Here 1");
        let ns = block_on(namespaces.list(&lp))?;
        println!("Here 2");
        let mut res: Vec<String> = vec![];

        for n in ns {
            let name = Meta::name(&n);
            res.push(name);
        }

        Ok(res)
    }
}

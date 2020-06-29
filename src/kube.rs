use k8s_openapi::api::core::v1::Namespace;
use kube::{api::Api, Client};

pub async fn GetNamespaces() -> Result<(), dyn Error>{
    let client = Client::try_default().await?;
    let namespaces: Api<Namespace> = Api::all(client); 
    let ns = namespaces.list(&ListParams{});
}
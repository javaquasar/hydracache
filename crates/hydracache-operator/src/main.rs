use std::error::Error;

use hydracache_operator::controller::{run, Ctx};
use hydracache_operator::crd::HydraCacheCluster;
use kube::CustomResourceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    hydracache_operator::install_default_rustls_provider();

    if std::env::args().nth(1).as_deref() == Some("--print-crd-json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&HydraCacheCluster::crd())?
        );
    } else {
        let client = kube::Client::try_default().await?;
        let identity = std::env::var("HYDRACACHE_OPERATOR_IDENTITY")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "hydracache-operator".to_owned());
        let namespace = std::env::var("HYDRACACHE_OPERATOR_NAMESPACE").ok();
        run(Ctx::new(client, identity, namespace)).await;
    }
    Ok(())
}

use std::error::Error;

use hydracache_operator::crd::HydraCacheCluster;
use kube::CustomResourceExt;

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().nth(1).as_deref() == Some("--print-crd-json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&HydraCacheCluster::crd())?
        );
    } else {
        println!("hydracache-operator scaffold ready");
    }
    Ok(())
}

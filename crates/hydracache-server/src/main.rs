use std::error::Error;

use hydracache_server::{ServerConfig, ServerRuntime};

fn main() -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::from_env()?;
    let runtime = ServerRuntime::new(config)?.start();
    println!("{}", serde_json_like_health(runtime.health().status));
    Ok(())
}

fn serde_json_like_health(status: &str) -> String {
    format!(r#"{{"status":"{status}"}}"#)
}

use std::error::Error;

use hydracache_server::{AdminHttpSurface, ServerConfig, ServerRuntime};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::from_env()?;
    let admin_enabled = config.admin_api.enabled;
    let admin_addr = config.admin_api.listen_addr;
    let runtime = ServerRuntime::new(config)?.start();
    println!("{}", serde_json_like_health(runtime.health().status));
    if admin_enabled {
        let listener = TcpListener::bind(admin_addr).await?;
        axum::serve(listener, AdminHttpSurface::new(runtime).routes()).await?;
    } else {
        std::future::pending::<()>().await;
    }
    Ok(())
}

fn serde_json_like_health(status: &str) -> String {
    format!(r#"{{"status":"{status}"}}"#)
}

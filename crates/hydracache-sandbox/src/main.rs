use hydracache_sandbox::{build_sandbox, startup_messages, SandboxConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = SandboxConfig::from_env_and_args(std::env::args())?;
    let bind = config.bind;
    let messages = startup_messages(&config);
    let sandbox = build_sandbox(config).await?;

    for message in messages {
        println!("{message}");
    }

    sandbox.serve(bind).await?;
    Ok(())
}

use hydracache_sandbox::{build_sandbox, SandboxConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = SandboxConfig::from_env_and_args(std::env::args())?;
    let bind = config.bind;
    let profile = config.profile.label();
    let backend = config.backend.label();
    let sandbox = build_sandbox(config).await?;

    println!("HydraCache sandbox listening on http://{bind}");
    println!("Profile: {profile}");
    println!("Backend: {backend}");
    println!("Swagger UI: http://{bind}/swagger-ui");
    println!("Actuator health: http://{bind}/actuator/hydracache/health");

    sandbox.serve(bind).await?;
    Ok(())
}

use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::{self, EnvFilter};

use codebase_memory_rlm_rs::server::CbmRlmServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let server = CbmRlmServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
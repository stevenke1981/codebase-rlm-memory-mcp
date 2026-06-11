use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::{self, EnvFilter};

use codebase_rlm_memory_mcp::hooks;
use codebase_rlm_memory_mcp::server::CbrlmServer;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 {
        match args[1].as_str() {
            "hook-session-start" => std::process::exit(hooks::hook_session_start()),
            "hook-augment" => std::process::exit(hooks::hook_augment()),
            "--help" | "-h" | "help" => {
                eprintln!(
                    "cbrlm — codebase-rlm-memory-mcp (CBRLM)\n\
                     \n\
                     Usage:\n\
                       cbrlm                    Start MCP stdio server\n\
                       cbrlm hook-session-start Print SessionStart reminder (stdout)\n\
                       cbrlm hook-augment       PreToolUse graph augmenter (stdin JSON)\n"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }

    run_mcp_server()
}

#[tokio::main]
async fn run_mcp_server() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let server = CbrlmServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

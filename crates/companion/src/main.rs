mod ipc;
mod llm;

use anyhow::Context;
use clap::Parser;
use tracing::info;

use crate::llm::OllamaClient;

#[derive(Debug, Parser)]
#[command(name = "AgesBeyondCompanion")]
#[command(about = "LLM companion process for Civilization IV: Ages Beyond")]
struct Args {
    #[arg(long)]
    pipe: String,

    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    #[arg(long, default_value = "llama3.1")]
    model: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ages_beyond_companion=info,info".into()),
        )
        .init();

    let args = Args::parse();
    let llm = OllamaClient::new(args.ollama_url, args.model)
        .context("failed to initialize Ollama client")?;

    info!(pipe = %args.pipe, "starting Ages Beyond companion");
    ipc::run_server(&args.pipe, llm).await
}

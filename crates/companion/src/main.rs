mod chronicle;
mod ipc;
mod llm;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use tracing::info;

use crate::chronicle::ChronicleWriter;
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

    #[arg(long)]
    chronicle: Option<PathBuf>,
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

    let chronicle = args.chronicle.map(ChronicleWriter::new);

    info!(pipe = %args.pipe, "starting Ages Beyond companion");
    ipc::run_server(&args.pipe, llm, chronicle).await
}

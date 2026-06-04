#[cfg(windows)]
use ages_beyond_protocol::{CompanionRequest, CompanionResponse};
#[cfg(windows)]
use anyhow::Context;
#[cfg(windows)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(windows)]
use tracing::{error, info};

use crate::llm::LlmClient;

#[cfg(windows)]
pub async fn run_server<L>(pipe_name: &str, llm: L) -> anyhow::Result<()>
where
    L: LlmClient,
{
    use tokio::net::windows::named_pipe::ServerOptions;

    let pipe = ServerOptions::new()
        .first_pipe_instance(true)
        .create(pipe_name)
        .with_context(|| format!("failed to create named pipe {pipe_name}"))?;

    info!(pipe = %pipe_name, "waiting for DLL connection");
    pipe.connect()
        .await
        .with_context(|| format!("failed while waiting for pipe client {pipe_name}"))?;
    info!("DLL connected");

    handle_connection(pipe, llm).await
}

#[cfg(not(windows))]
pub async fn run_server<L>(_pipe_name: &str, _llm: L) -> anyhow::Result<()>
where
    L: LlmClient,
{
    anyhow::bail!("Windows named pipes are only available on Windows builds")
}

#[cfg(windows)]
async fn handle_connection<S, L>(stream: S, llm: L) -> anyhow::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    L: LlmClient,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines
        .next_line()
        .await
        .context("failed to read pipe line")?
    {
        let response = match serde_json::from_str::<CompanionRequest>(&line) {
            Ok(request) => {
                let id = request.id.clone();
                match llm.respond(&request.body).await {
                    Ok(text) => CompanionResponse::ok(id, text),
                    Err(err) => {
                        error!(request_id = %id, error = %err, "failed to handle request");
                        CompanionResponse::error(id, err.to_string())
                    }
                }
            }
            Err(err) => {
                error!(error = %err, "invalid request JSON");
                CompanionResponse::error("invalid-request", err.to_string())
            }
        };

        let mut output = serde_json::to_vec(&response).context("failed to serialize response")?;
        output.push(b'\n');
        writer
            .write_all(&output)
            .await
            .context("failed to write pipe response")?;
        writer
            .flush()
            .await
            .context("failed to flush pipe response")?;
    }

    info!("DLL disconnected");
    Ok(())
}

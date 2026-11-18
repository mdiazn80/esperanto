use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use esperanto_bench::{manifest, report, run};
use esperanto_inference::{OpenAiCompatibleProvider, OpenAiConfig};

/// Measure transcription/translation quality and latency of a model served
/// through an OpenAI-compatible API (e.g. llama-server).
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Test set manifest (JSON Lines).
    #[arg(long)]
    manifest: PathBuf,
    /// API base URL, including the version prefix.
    #[arg(long, default_value = "http://127.0.0.1:8080/v1")]
    base_url: String,
    /// Model name sent in requests.
    #[arg(long, default_value = "voxtral-mini-3b")]
    model: String,
    /// Bearer token for remote gateways. Read from the environment so it
    /// doesn't end up in shell history.
    #[arg(long, env = "ESPERANTO_API_TOKEN", hide_env_values = true)]
    api_token: Option<String>,
    /// Use only the first N clips.
    #[arg(long)]
    limit: Option<usize>,
    /// Skip the transcription + translation requests.
    #[arg(long)]
    no_translate: bool,
    /// Ask the model to detect the spoken language instead of telling it
    /// (the app always tells it).
    #[arg(long)]
    auto_source: bool,
    /// Don't send a JSON schema (`response_format`) with requests.
    #[arg(long)]
    no_structured_output: bool,
    /// Skip the warm-up request.
    #[arg(long)]
    no_warmup: bool,
    /// Request timeout in seconds.
    #[arg(long, default_value_t = 120)]
    timeout: u64,
    /// PID of the model server: its peak memory (physical footprint, from
    /// macOS `vmmap`) is added to the report.
    #[arg(long)]
    server_pid: Option<u32>,
    /// Folder for results.jsonl, summary.json and summary.md.
    #[arg(long, default_value = "bench-results")]
    out: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut items = manifest::load(&cli.manifest)?;
    if let Some(limit) = cli.limit {
        items.truncate(limit);
    }

    let mut config = OpenAiConfig::new(&cli.base_url, &cli.model);
    config.api_token = cli.api_token.clone();
    config.timeout = Duration::from_secs(cli.timeout);
    config.structured_output = !cli.no_structured_output;
    let provider = OpenAiCompatibleProvider::new(config)?;

    let options = run::RunOptions {
        translate: !cli.no_translate,
        fixed_source: !cli.auto_source,
        warmup: !cli.no_warmup,
    };
    let records = run::run(&provider, &items, &options).await?;
    let rows = report::summarize(&records);
    let title = format!(
        "{} · {} clips · source {}",
        cli.model,
        items.len(),
        if cli.auto_source { "auto" } else { "fixed" }
    );
    let mut markdown = report::to_markdown(&title, &rows);
    if let Some(pid) = cli.server_pid {
        match peak_footprint(pid) {
            Some(peak) => markdown.push_str(&format!(
                "\nPeak memory of the model server (physical footprint, pid {pid}): {peak}\n"
            )),
            None => eprintln!("could not read the memory of pid {pid}"),
        }
    }

    std::fs::create_dir_all(&cli.out)
        .with_context(|| format!("cannot create {}", cli.out.display()))?;
    let jsonl: String = records
        .iter()
        .map(|r| serde_json::to_string(r).map(|line| line + "\n"))
        .collect::<Result<_, _>>()?;
    std::fs::write(cli.out.join("results.jsonl"), jsonl)?;
    std::fs::write(
        cli.out.join("summary.json"),
        serde_json::to_string_pretty(&rows)?,
    )?;
    std::fs::write(cli.out.join("summary.md"), &markdown)?;

    println!("{markdown}");
    eprintln!("Results written to {}", cli.out.display());
    Ok(())
}

/// "Physical footprint (peak)" of a process, as reported by `vmmap` (macOS).
/// On Apple Silicon this includes the GPU memory of the process.
fn peak_footprint(pid: u32) -> Option<String> {
    let output = std::process::Command::new("vmmap")
        .args(["-summary", &pid.to_string()])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Physical footprint (peak):"))
        .map(|value| value.trim().to_owned())
}

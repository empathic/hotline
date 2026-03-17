use std::path::Path;

use clap::{Parser, ValueEnum};

#[derive(Clone, ValueEnum)]
enum Backend {
    Github,
    Linear,
}

#[derive(Parser)]
#[command(about = "File a bug report")]
struct Cli {
    /// Backend to file the issue to
    backend: Backend,

    /// Short summary of the bug
    title: String,

    /// Detailed description
    #[arg(short, long)]
    description: Option<String>,

    /// Inline a file as a code block in the description (repeatable, must be UTF-8)
    #[arg(short, long)]
    file: Vec<String>,

    /// Upload a file as an attachment (repeatable, binary OK, Linear only)
    #[arg(short, long)]
    attachment: Vec<String>,

    /// Proxy URL (or set HOTLINE_PROXY_URL)
    #[arg(long, env = "HOTLINE_PROXY_URL")]
    proxy_url: String,

    /// Bearer token for proxy auth (or set HOTLINE_PROXY_TOKEN)
    #[arg(long, env = "HOTLINE_PROXY_TOKEN")]
    proxy_token: Option<String>,
}

fn system_info_text() -> String {
    format!(
        "## System Info\n\n| Field | Value |\n|-------|-------|\n| OS | {} |\n| Arch | {} |",
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

fn read_file(path_str: &str) -> anyhow::Result<(String, Vec<u8>)> {
    let path = Path::new(path_str);
    let data = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment")
        .to_string();
    Ok((filename, data))
}

fn read_file_text(path_str: &str) -> anyhow::Result<(String, String)> {
    let (filename, data) = read_file(path_str)?;
    let content = String::from_utf8(data)
        .map_err(|_| anyhow::anyhow!("file is not valid UTF-8: {}", filename))?;
    Ok((filename, content))
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if !cli.attachment.is_empty() && matches!(cli.backend, Backend::Github) {
        anyhow::bail!("--attachment is only supported with the linear backend");
    }

    let system_info = system_info_text();

    let url = match cli.backend {
        Backend::Github => {
            let mut issue = hotln::github(&cli.proxy_url).title(&cli.title);
            if let Some(token) = &cli.proxy_token {
                issue = issue.with_token(token);
            }
            if let Some(desc) = &cli.description {
                issue = issue.text(desc);
            }
            for path_str in &cli.file {
                let (filename, content) = read_file_text(path_str)?;
                issue = issue.file(&filename, &content);
            }
            issue.text(&system_info).create()?
        }
        Backend::Linear => {
            let mut issue = hotln::linear(&cli.proxy_url).title(&cli.title);
            if let Some(token) = &cli.proxy_token {
                issue = issue.with_token(token);
            }
            if let Some(desc) = &cli.description {
                issue = issue.text(desc);
            }
            for path_str in &cli.file {
                let (filename, content) = read_file_text(path_str)?;
                issue = issue.file(&filename, &content);
            }
            for path_str in &cli.attachment {
                let (filename, data) = read_file(path_str)?;
                issue = issue.attachment(&filename, &data);
            }
            issue.text(&system_info).create()?
        }
    };

    println!("{}", url);
    Ok(())
}

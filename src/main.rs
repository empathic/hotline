use std::path::Path;

use clap::Parser;

#[derive(Parser)]
#[command(about = "File a bug report to Linear")]
struct Cli {
    /// Short summary of the bug
    title: String,

    /// Detailed description
    #[arg(short, long)]
    description: Option<String>,

    /// Attach a file (repeatable)
    #[arg(short, long = "attach")]
    attach: Vec<String>,

    /// Linear API key (or set HOTLINE_API_KEY)
    #[arg(long, env = "HOTLINE_API_KEY")]
    api_key: Option<String>,

    /// Proxy URL to use instead of calling Linear directly (or set HOTLINE_PROXY_URL)
    #[arg(long, env = "HOTLINE_PROXY_URL")]
    proxy_url: Option<String>,

    /// Bearer token for proxy authentication (or set HOTLINE_PROXY_TOKEN)
    #[arg(long, env = "HOTLINE_PROXY_TOKEN")]
    proxy_token: Option<String>,

    /// Linear team ID (required for direct mode, or set HOTLINE_TEAM_ID)
    #[arg(long, env = "HOTLINE_TEAM_ID")]
    team_id: Option<String>,

    /// Linear project ID (required for direct mode, or set HOTLINE_PROJECT_ID)
    #[arg(long, env = "HOTLINE_PROJECT_ID")]
    project_id: Option<String>,
}

fn mime_for_file(path: &Path, data: &[u8]) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "json" => "application/json",
        "pdf" => "application/pdf",
        _ if std::str::from_utf8(data).is_ok() => "text/plain",
        _ => "application/octet-stream",
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let system_info = [
        ("OS", std::env::consts::OS),
        ("Arch", std::env::consts::ARCH),
    ];

    let mut attachments = Vec::new();
    for path_str in &cli.attach {
        let path = Path::new(path_str);
        let data = std::fs::read(path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment")
            .to_string();
        attachments.push(hotln::Attachment {
            content_type: mime_for_file(path, &data).to_string(),
            filename,
            data,
        });
    }

    let url = match (cli.proxy_url, cli.api_key) {
        (Some(url), _) => {
            let mut client = hotln::proxy(&url);
            if let Some(token) = cli.proxy_token {
                client = client.with_token(&token);
            }
            client.create_issue(
                &cli.title,
                cli.description.as_deref(),
                &system_info,
                &attachments,
            )?
        }
        (None, Some(api_key)) => {
            let team_id = cli
                .team_id
                .ok_or_else(|| anyhow::anyhow!("--team-id is required for direct mode"))?;
            let project_id = cli
                .project_id
                .ok_or_else(|| anyhow::anyhow!("--project-id is required for direct mode"))?;
            hotln::direct(&api_key, &team_id, &project_id).create_issue(
                &cli.title,
                cli.description.as_deref(),
                &system_info,
                &attachments,
            )?
        }
        (None, None) => anyhow::bail!(
            "Provide either --proxy-url / HOTLINE_PROXY_URL or --api-key / HOTLINE_API_KEY"
        ),
    };

    println!("{}", url);
    Ok(())
}

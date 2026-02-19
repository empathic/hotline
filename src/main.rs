use clap::Parser;

#[derive(Parser)]
#[command(about = "File a bug report to Linear")]
struct Cli {
    /// Short summary of the bug
    title: String,

    /// Detailed description
    #[arg(short, long)]
    description: Option<String>,

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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let system_info = [
        ("OS", std::env::consts::OS),
        ("Arch", std::env::consts::ARCH),
    ];

    let url = match (cli.proxy_url, cli.api_key) {
        (Some(url), _) => {
            let mut client = hotln::proxy(&url);
            if let Some(token) = cli.proxy_token {
                client = client.with_token(&token);
            }
            client.create_issue(&cli.title, cli.description.as_deref(), &system_info)?
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
            )?
        }
        (None, None) => anyhow::bail!(
            "Provide either --proxy-url / HOTLINE_PROXY_URL or --api-key / HOTLINE_API_KEY"
        ),
    };

    println!("{}", url);
    Ok(())
}

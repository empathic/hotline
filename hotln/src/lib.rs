//! File bug reports to issue trackers from your application.
//!
//! Supports [Linear](https://linear.app) and [GitHub Issues](https://github.com).
//! Reports are sent through a proxy server that holds API credentials.
//!
//! ```no_run
//! // GitHub
//! hotln::github("https://worker.example.com")
//!     .with_token("secret")
//!     .title("crash on startup")
//!     .text("Details here.")
//!     .file("log.txt", "log contents")
//!     .create()?;
//!
//! // Linear
//! hotln::linear("https://worker.example.com")
//!     .with_token("secret")
//!     .title("crash on startup")
//!     .text("Details.")
//!     .attachment("crash.log", b"log data")
//!     .create()?;
//! # Ok::<(), hotln::Error>(())
//! ```

pub use ureq;

mod github;
mod linear;

pub use github::Issue as GitHubIssue;
pub use linear::Issue as LinearIssue;

/// Create a GitHub issue builder that posts through a proxy.
pub fn github(proxy_url: &str) -> GitHubIssue {
    GitHubIssue::new(proxy_url)
}

/// Create a Linear issue builder that posts through a proxy.
pub fn linear(proxy_url: &str) -> LinearIssue {
    LinearIssue::new(proxy_url)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(Box<ureq::Error>),
    #[error("Failed to parse response: {0}")]
    Parse(String),
    #[error("Proxy returned error {status}: {body}")]
    Proxy { status: u16, body: String },
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        Error::Http(Box::new(e))
    }
}

pub(crate) fn inline_file(filename: &str, content: &str) -> String {
    let ext = filename.rsplit('.').next().unwrap_or("");
    format!("**{filename}**\n```{ext}\n{content}\n```")
}

pub(crate) fn mime_for_ext(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "txt" | "log" => "text/plain",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_file() {
        let result = inline_file("config.toml", "key = \"value\"");
        assert_eq!(result, "**config.toml**\n```toml\nkey = \"value\"\n```");
    }
}

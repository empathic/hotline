//! File bug reports to [Linear](https://linear.app) from your application.
//!
//! Two modes: call the Linear API directly, or go through a proxy that holds
//! the API key (recommended for open source / distributed binaries).
//!
//! ```no_run
//! // Via proxy (with optional bearer token)
//! hotln::proxy("https://your-worker.example.com")
//!     .with_token("secret-token")
//!     .create_issue("crash on startup", Some("details..."), &[("OS", "macos")])?;
//!
//! // Direct
//! hotln::direct("lin_api_...", "team-id", "project-id")
//!     .create_issue("crash on startup", Some("details..."), &[("OS", "macos")])?;
//! # Ok::<(), hotln::Error>(())
//! ```

use tracing::{debug, info};

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] ureq::Error),
    #[error("Linear API error: {0}")]
    Api(String),
    #[error("Failed to parse response: {0}")]
    Parse(String),
    #[error("Proxy returned error {status}: {body}")]
    Proxy { status: u16, body: String },
}

/// A client that calls Linear's GraphQL API directly with an API key.
pub struct DirectClient {
    api_key: String,
    team_id: String,
    project_id: String,
}

/// A client that posts bug reports to a proxy URL.
pub struct ProxyClient {
    url: String,
    token: Option<String>,
}

/// Create a client that calls Linear's GraphQL API directly.
pub fn direct(api_key: &str, team_id: &str, project_id: &str) -> DirectClient {
    DirectClient {
        api_key: api_key.to_string(),
        team_id: team_id.to_string(),
        project_id: project_id.to_string(),
    }
}

/// Create a client that posts bug reports to a proxy URL.
pub fn proxy(url: &str) -> ProxyClient {
    ProxyClient {
        url: url.to_string(),
        token: None,
    }
}

impl DirectClient {
    /// Create a bug report issue on Linear. Returns the URL of the created issue.
    pub fn create_issue(
        &self,
        title: &str,
        description: Option<&str>,
        system_info: &[(&str, &str)],
    ) -> Result<String, Error> {
        let description = format_description(description, system_info);

        let query = r#"mutation IssueCreate($input: IssueCreateInput!) {
            issueCreate(input: $input) {
                success
                issue {
                    id
                    identifier
                    url
                }
            }
        }"#;

        let body = serde_json::json!({
            "query": query,
            "variables": {
                "input": {
                    "teamId": self.team_id,
                    "projectId": self.project_id,
                    "title": title,
                    "description": description,
                }
            }
        });

        let resp = graphql_request(LINEAR_API_URL, &self.api_key, &body)?;

        let issue = &resp["data"]["issueCreate"]["issue"];
        let url = issue["url"]
            .as_str()
            .ok_or_else(|| Error::Parse("Linear response missing issue url".into()))?
            .to_string();
        let identifier = issue["identifier"].as_str().unwrap_or("unknown");

        info!(identifier, url = %url, "Created Linear issue");
        Ok(url)
    }
}

impl ProxyClient {
    /// Set a bearer token for proxy authentication.
    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    /// Create a bug report issue via the proxy. Returns the URL of the created issue.
    pub fn create_issue(
        &self,
        title: &str,
        description: Option<&str>,
        system_info: &[(&str, &str)],
    ) -> Result<String, Error> {
        let description = format_description(description, system_info);

        let payload = serde_json::json!({
            "title": title,
            "description": description,
        });
        let body = payload.to_string();

        let mut req = ureq::post(&self.url).set("Content-Type", "application/json");
        if let Some(token) = &self.token {
            req = req.set("Authorization", &format!("Bearer {}", token));
        }

        let resp_str = match req.send_string(&body) {
            Ok(resp) => resp
                .into_string()
                .map_err(|e| Error::Parse(e.to_string()))?,
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                return Err(Error::Proxy { status: code, body });
            }
            Err(e) => return Err(e.into()),
        };

        let resp: serde_json::Value =
            serde_json::from_str(&resp_str).map_err(|e| Error::Parse(e.to_string()))?;

        let url = resp["url"]
            .as_str()
            .ok_or_else(|| Error::Parse("proxy response missing url".into()))?
            .to_string();

        info!(url = %url, "Created Linear issue via proxy");
        Ok(url)
    }
}

fn format_description(description: Option<&str>, system_info: &[(&str, &str)]) -> String {
    let mut body = String::new();

    if let Some(desc) = description {
        body.push_str(desc);
        body.push_str("\n\n");
    }

    if !system_info.is_empty() {
        body.push_str("## System Info\n\n");
        body.push_str("| Field | Value |\n|-------|-------|\n");
        for (key, value) in system_info {
            body.push_str(&format!("| {} | {} |\n", key, value));
        }
    }

    body.trim_end().to_string()
}

fn graphql_request(
    url: &str,
    api_key: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let resp_str = match ureq::post(url)
        .set("Authorization", api_key)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
    {
        Ok(resp) => resp
            .into_string()
            .map_err(|e| Error::Parse(e.to_string()))?,
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            return Err(Error::Api(format!(
                "Linear API returned {}: {}",
                code, body
            )));
        }
        Err(e) => return Err(e.into()),
    };

    let resp_json: serde_json::Value =
        serde_json::from_str(&resp_str).map_err(|e| Error::Parse(e.to_string()))?;

    if let Some(errors) = resp_json.get("errors") {
        return Err(Error::Api(format!("Linear API error: {}", errors)));
    }

    debug!("Linear API response: {}", resp_json);
    Ok(resp_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphql_request_success() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/graphql")
            .match_header("Content-Type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "data": {
                        "issueCreate": {
                            "success": true,
                            "issue": {
                                "id": "abc-123",
                                "identifier": "EMP-42",
                                "url": "https://linear.app/empathic/issue/EMP-42"
                            }
                        }
                    }
                })
                .to_string(),
            )
            .create();

        let body = serde_json::json!({"query": "test"});
        let resp =
            graphql_request(&format!("{}/graphql", server.url()), "test-key", &body).unwrap();

        assert_eq!(
            resp["data"]["issueCreate"]["issue"]["url"],
            "https://linear.app/empathic/issue/EMP-42"
        );
        mock.assert();
    }

    #[test]
    fn test_graphql_request_error() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/graphql")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "errors": [{"message": "Invalid input"}]
                })
                .to_string(),
            )
            .create();

        let body = serde_json::json!({"query": "test"});
        let result = graphql_request(&format!("{}/graphql", server.url()), "test-key", &body);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Linear API error"));
        mock.assert();
    }

    #[test]
    fn test_proxy_create_issue() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/")
            .match_header("Content-Type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "url": "https://linear.app/empathic/issue/EMP-99"
                })
                .to_string(),
            )
            .create();

        let client = proxy(&server.url());
        let url = client
            .create_issue("Bug Report: test", Some("desc"), &[])
            .unwrap();

        assert_eq!(url, "https://linear.app/empathic/issue/EMP-99");
        mock.assert();
    }

    #[test]
    fn test_proxy_with_token() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/")
            .match_header("Authorization", "Bearer my-secret-token")
            .match_header("Content-Type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "url": "https://linear.app/empathic/issue/EMP-100"
                })
                .to_string(),
            )
            .create();

        let client = proxy(&server.url()).with_token("my-secret-token");
        let url = client
            .create_issue("Bug Report: auth test", Some("desc"), &[])
            .unwrap();

        assert_eq!(url, "https://linear.app/empathic/issue/EMP-100");
        mock.assert();
    }

    #[test]
    fn test_proxy_error() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/")
            .with_status(429)
            .with_body("rate limited")
            .create();

        let client = proxy(&server.url());
        let result = client.create_issue("Bug Report: test", Some("desc"), &[]);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxy { status, body } => {
                assert_eq!(status, 429);
                assert_eq!(body, "rate limited");
            }
            other => panic!("expected Proxy error, got: {}", other),
        }
        mock.assert();
    }
}

//! File bug reports to [Linear](https://linear.app) from your application.
//!
//! Two modes: call the Linear API directly, or go through a proxy that holds
//! the API key (recommended for open source / distributed binaries).
//!
//! ```no_run
//! // Via proxy (with optional bearer token)
//! hotln::proxy("https://your-worker.example.com")
//!     .with_token("secret-token")
//!     .create_issue("crash on startup", Some("details..."), &[("OS", "macos")], &[])?;
//!
//! // Direct
//! hotln::direct("lin_api_...", "team-id", "project-id")
//!     .create_issue("crash on startup", Some("details..."), &[("OS", "macos")], &[])?;
//! # Ok::<(), hotln::Error>(())
//! ```

pub use ureq;

use base64::prelude::*;
use tracing::{debug, info};

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

/// A file to attach to an issue.
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

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
    #[error("Upload failed: {0}")]
    Upload(String),
}

/// A client that calls Linear's GraphQL API directly with an API key.
pub struct DirectClient {
    api_key: String,
    team_id: String,
    project_id: String,
    api_url: String,
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
        api_url: LINEAR_API_URL.to_string(),
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
        attachments: &[Attachment],
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

        let resp = graphql_request(&self.api_url, &self.api_key, &body)?;

        let issue = &resp["data"]["issueCreate"]["issue"];
        let url = issue["url"]
            .as_str()
            .ok_or_else(|| Error::Parse("Linear response missing issue url".into()))?
            .to_string();
        let issue_id = issue["id"]
            .as_str()
            .ok_or_else(|| Error::Parse("Linear response missing issue id".into()))?;
        let identifier = issue["identifier"].as_str().unwrap_or("unknown");

        info!(identifier, url = %url, "Created Linear issue");

        for attachment in attachments {
            self.upload_attachment(issue_id, attachment)?;
        }

        Ok(url)
    }

    fn upload_attachment(&self, issue_id: &str, attachment: &Attachment) -> Result<(), Error> {
        // Step 1: Get presigned upload URL
        let (upload_url, asset_url, headers) = self.file_upload(attachment)?;

        // Step 2: PUT file bytes to presigned URL
        upload_file_bytes(
            &upload_url,
            &headers,
            &attachment.content_type,
            &attachment.data,
        )?;

        // Step 3: Link attachment to issue
        self.attach_to_issue(issue_id, &asset_url, &attachment.filename)?;

        info!(filename = %attachment.filename, "Attached file to issue");
        Ok(())
    }

    fn file_upload(
        &self,
        attachment: &Attachment,
    ) -> Result<(String, String, Vec<(String, String)>), Error> {
        let query = r#"mutation FileUpload($contentType: String!, $filename: String!, $size: Int!) {
            fileUpload(contentType: $contentType, filename: $filename, size: $size) {
                uploadFile {
                    uploadUrl
                    assetUrl
                    headers {
                        key
                        value
                    }
                }
            }
        }"#;

        let body = serde_json::json!({
            "query": query,
            "variables": {
                "contentType": attachment.content_type,
                "filename": attachment.filename,
                "size": attachment.data.len(),
            }
        });

        let resp = graphql_request(&self.api_url, &self.api_key, &body)?;
        let upload = &resp["data"]["fileUpload"]["uploadFile"];

        let upload_url = upload["uploadUrl"]
            .as_str()
            .ok_or_else(|| Error::Upload("missing uploadUrl".into()))?
            .to_string();
        let asset_url = upload["assetUrl"]
            .as_str()
            .ok_or_else(|| Error::Upload("missing assetUrl".into()))?
            .to_string();

        let headers = upload["headers"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|h| {
                Some((
                    h["key"].as_str()?.to_string(),
                    h["value"].as_str()?.to_string(),
                ))
            })
            .collect();

        Ok((upload_url, asset_url, headers))
    }

    fn attach_to_issue(&self, issue_id: &str, asset_url: &str, title: &str) -> Result<(), Error> {
        let query = r#"mutation AttachmentCreate($issueId: String!, $url: String!, $title: String!) {
            attachmentCreate(input: { issueId: $issueId, url: $url, title: $title }) {
                success
            }
        }"#;

        let body = serde_json::json!({
            "query": query,
            "variables": {
                "issueId": issue_id,
                "url": asset_url,
                "title": title,
            }
        });

        graphql_request(&self.api_url, &self.api_key, &body)?;
        Ok(())
    }
}

fn upload_file_bytes(
    upload_url: &str,
    headers: &[(String, String)],
    content_type: &str,
    data: &[u8],
) -> Result<(), Error> {
    let mut req = ureq::put(upload_url)
        .set("Content-Type", content_type)
        .set("Content-Length", &data.len().to_string());
    for (key, value) in headers {
        req = req.set(key, value);
    }
    match req.send_bytes(data) {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            Err(Error::Upload(format!("PUT returned {}: {}", code, body)))
        }
        Err(e) => Err(Error::Upload(e.to_string())),
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
        attachments: &[Attachment],
    ) -> Result<String, Error> {
        let description = format_description(description, system_info);

        let encoded_attachments: Vec<serde_json::Value> = attachments
            .iter()
            .map(|a| match std::str::from_utf8(&a.data) {
                Ok(text) => serde_json::json!({
                    "filename": a.filename,
                    "contentType": a.content_type,
                    "data": text,
                    "encoding": "text",
                }),
                Err(_) => serde_json::json!({
                    "filename": a.filename,
                    "contentType": a.content_type,
                    "data": BASE64_STANDARD.encode(&a.data),
                    "encoding": "base64",
                }),
            })
            .collect();

        let payload = serde_json::json!({
            "title": title,
            "description": description,
            "attachments": encoded_attachments,
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
            .create_issue("Bug Report: test", Some("desc"), &[], &[])
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
            .create_issue("Bug Report: auth test", Some("desc"), &[], &[])
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
        let result = client.create_issue("Bug Report: test", Some("desc"), &[], &[]);
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

    #[test]
    fn test_direct_create_issue_with_attachments() {
        let mut server = mockito::Server::new();

        // 1. issueCreate
        let issue_mock = server
            .mock("POST", "/graphql")
            .match_body(mockito::Matcher::Regex("IssueCreate".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "data": {
                        "issueCreate": {
                            "success": true,
                            "issue": {
                                "id": "issue-uuid",
                                "identifier": "EMP-50",
                                "url": "https://linear.app/empathic/issue/EMP-50"
                            }
                        }
                    }
                })
                .to_string(),
            )
            .create();

        // 2. fileUpload
        let file_upload_mock = server
            .mock("POST", "/graphql")
            .match_body(mockito::Matcher::Regex("FileUpload".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "data": {
                        "fileUpload": {
                            "uploadFile": {
                                "uploadUrl": format!("{}/upload", server.url()),
                                "assetUrl": "https://uploads.linear.app/asset-123",
                                "headers": [
                                    {"key": "X-Custom", "value": "header-val"}
                                ]
                            }
                        }
                    }
                })
                .to_string(),
            )
            .create();

        // 3. PUT upload
        let put_mock = server
            .mock("PUT", "/upload")
            .match_header("X-Custom", "header-val")
            .match_body("file contents")
            .with_status(200)
            .create();

        // 4. attachmentCreate
        let attach_mock = server
            .mock("POST", "/graphql")
            .match_body(mockito::Matcher::Regex("AttachmentCreate".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "data": { "attachmentCreate": { "success": true } }
                })
                .to_string(),
            )
            .create();

        let client = DirectClient {
            api_key: "test-key".into(),
            team_id: "team-1".into(),
            project_id: "proj-1".into(),
            api_url: format!("{}/graphql", server.url()),
        };

        let attachments = [Attachment {
            filename: "test.log".into(),
            content_type: "text/plain".into(),
            data: b"file contents".to_vec(),
        }];

        let url = client
            .create_issue("crash", Some("details"), &[], &attachments)
            .unwrap();

        assert_eq!(url, "https://linear.app/empathic/issue/EMP-50");
        issue_mock.assert();
        file_upload_mock.assert();
        put_mock.assert();
        attach_mock.assert();
    }

    #[test]
    fn test_proxy_create_issue_with_attachments() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/")
            .match_header("Content-Type", "application/json")
            .match_body(mockito::Matcher::PartialJsonString(
                serde_json::json!({
                    "attachments": [{
                        "filename": "crash.log",
                        "contentType": "text/plain",
                        "data": "log data",
                        "encoding": "text",
                    }]
                })
                .to_string(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "url": "https://linear.app/empathic/issue/EMP-51"
                })
                .to_string(),
            )
            .create();

        let client = proxy(&server.url());
        let attachments = [Attachment {
            filename: "crash.log".into(),
            content_type: "text/plain".into(),
            data: b"log data".to_vec(),
        }];

        let url = client
            .create_issue("crash", Some("details"), &[], &attachments)
            .unwrap();

        assert_eq!(url, "https://linear.app/empathic/issue/EMP-51");
        mock.assert();
    }
}

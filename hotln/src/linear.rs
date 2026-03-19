use base64::prelude::*;
use tracing::info;

use crate::{Error, inline_file, mime_for_ext};

pub struct Issue {
    url: String,
    token: Option<String>,
    title: String,
    description: String,
    attachments: Vec<(String, Vec<u8>)>,
}

impl Issue {
    pub(crate) fn new(proxy_url: &str) -> Self {
        Self {
            url: proxy_url.to_string(),
            token: None,
            title: "Untitled".to_string(),
            description: String::new(),
            attachments: Vec::new(),
        }
    }

    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        if !self.description.is_empty() {
            self.description.push_str("\n\n");
        }
        self.description.push_str(text);
        self
    }

    pub fn file(mut self, filename: &str, content: &str) -> Self {
        if !self.description.is_empty() {
            self.description.push_str("\n\n");
        }
        self.description.push_str(&inline_file(filename, content));
        self
    }

    pub fn attachment(mut self, filename: &str, data: &[u8]) -> Self {
        self.attachments.push((filename.to_string(), data.to_vec()));
        self
    }

    /// Consume the builder and create the issue. Returns the issue URL.
    pub fn create(self) -> Result<String, Error> {
        let encoded_attachments: Vec<serde_json::Value> = self
            .attachments
            .iter()
            .map(|(filename, data)| {
                let content_type = mime_for_ext(filename);
                match std::str::from_utf8(data) {
                    Ok(text) => serde_json::json!({
                        "filename": filename,
                        "contentType": content_type,
                        "data": text,
                        "encoding": "text",
                    }),
                    Err(_) => serde_json::json!({
                        "filename": filename,
                        "contentType": content_type,
                        "data": BASE64_STANDARD.encode(data),
                        "encoding": "base64",
                    }),
                }
            })
            .collect();

        let payload = serde_json::json!({
            "title": self.title,
            "description": self.description,
            "attachments": encoded_attachments,
        });

        let mut req =
            ureq::post(&format!("{}/linear", self.url)).set("Content-Type", "application/json");
        if let Some(token) = &self.token {
            req = req.set("Authorization", &format!("Bearer {}", token));
        }

        let resp_str = match req.send_string(&payload.to_string()) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_issue() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/linear")
            .match_header("Content-Type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "url": "https://linear.app/test-org/issue/TEST-99"
                })
                .to_string(),
            )
            .create();

        let url = Issue::new(&server.url())
            .title("Bug Report: test")
            .text("desc")
            .create()
            .unwrap();

        assert_eq!(url, "https://linear.app/test-org/issue/TEST-99");
        mock.assert();
    }

    #[test]
    fn test_with_token() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/linear")
            .match_header("Authorization", "Bearer my-secret-token")
            .match_header("Content-Type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "url": "https://linear.app/test-org/issue/TEST-100"
                })
                .to_string(),
            )
            .create();

        let url = Issue::new(&server.url())
            .with_token("my-secret-token")
            .title("Bug Report: auth test")
            .text("desc")
            .create()
            .unwrap();

        assert_eq!(url, "https://linear.app/test-org/issue/TEST-100");
        mock.assert();
    }

    #[test]
    fn test_proxy_error() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/linear")
            .with_status(429)
            .with_body("rate limited")
            .create();

        let result = Issue::new(&server.url())
            .title("Bug Report: test")
            .text("desc")
            .create();
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
    fn test_with_attachments() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/linear")
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
                    "url": "https://linear.app/test-org/issue/TEST-51"
                })
                .to_string(),
            )
            .create();

        let url = Issue::new(&server.url())
            .title("crash")
            .text("details")
            .attachment("crash.log", b"log data")
            .create()
            .unwrap();

        assert_eq!(url, "https://linear.app/test-org/issue/TEST-51");
        mock.assert();
    }

    #[test]
    fn test_binary_attachment_base64() {
        let mut server = mockito::Server::new();
        let binary_data: &[u8] = &[0xff, 0xd8, 0xff, 0xe0]; // not valid UTF-8
        let mock = server
            .mock("POST", "/linear")
            .match_body(mockito::Matcher::PartialJsonString(
                serde_json::json!({
                    "attachments": [{
                        "filename": "image.png",
                        "contentType": "image/png",
                        "encoding": "base64",
                    }]
                })
                .to_string(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "url": "https://linear.app/test-org/issue/TEST-52"
                })
                .to_string(),
            )
            .create();

        let url = Issue::new(&server.url())
            .title("binary test")
            .attachment("image.png", binary_data)
            .create()
            .unwrap();

        assert_eq!(url, "https://linear.app/test-org/issue/TEST-52");
        mock.assert();
    }
}

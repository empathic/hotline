use crate::{Error, inline_file};

pub struct Issue {
    url: String,
    token: Option<String>,
    title: String,
    description: String,
}

impl Issue {
    pub(crate) fn new(proxy_url: &str) -> Self {
        Self {
            url: proxy_url.to_string(),
            token: None,
            title: "Untitled".to_string(),
            description: String::new(),
        }
    }

    pub fn with_token(&mut self, token: &str) -> &mut Self {
        self.token = Some(token.to_string());
        self
    }

    pub fn title(&mut self, title: &str) -> &mut Self {
        self.title = title.to_string();
        self
    }

    pub fn text(&mut self, text: &str) -> &mut Self {
        if !self.description.is_empty() {
            self.description.push_str("\n\n");
        }
        self.description.push_str(text);
        self
    }

    pub fn file(&mut self, filename: &str, content: &str) -> &mut Self {
        if !self.description.is_empty() {
            self.description.push_str("\n\n");
        }
        self.description.push_str(&inline_file(filename, content));
        self
    }

    /// Create the issue. Returns the issue URL.
    pub fn create(&self) -> Result<String, Error> {
        let payload = serde_json::json!({
            "title": self.title,
            "description": self.description,
        });

        let mut req =
            ureq::post(&format!("{}/github", self.url)).set("Content-Type", "application/json");
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
            .mock("POST", "/github")
            .match_header("Content-Type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "url": "https://github.com/owner/repo/issues/1"
                })
                .to_string(),
            )
            .create();

        let url = Issue::new(&server.url())
            .title("crash on startup")
            .text("Something went wrong")
            .create()
            .unwrap();

        assert_eq!(url, "https://github.com/owner/repo/issues/1");
        mock.assert();
    }

    #[test]
    fn test_create_issue_with_file() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/github")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "url": "https://github.com/owner/repo/issues/2"
                })
                .to_string(),
            )
            .create();

        let url = Issue::new(&server.url())
            .title("config issue")
            .text("Bad config detected")
            .file("config.toml", "key = \"value\"")
            .text("Please investigate")
            .create()
            .unwrap();

        assert_eq!(url, "https://github.com/owner/repo/issues/2");
        mock.assert();
    }

    #[test]
    fn test_with_token() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/github")
            .match_header("Authorization", "Bearer my-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "url": "https://github.com/owner/repo/issues/3"
                })
                .to_string(),
            )
            .create();

        let url = Issue::new(&server.url())
            .with_token("my-token")
            .title("auth test")
            .text("details")
            .create()
            .unwrap();

        assert_eq!(url, "https://github.com/owner/repo/issues/3");
        mock.assert();
    }

    #[test]
    fn test_proxy_error() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/github")
            .with_status(429)
            .with_body("rate limited")
            .create();

        let result = Issue::new(&server.url())
            .title("test")
            .text("desc")
            .create();

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

use serde::de::DeserializeOwned;
use std::time::Duration;

use crate::error::{AuditError, Result};

const API_ROOT: &str = "https://api.github.com";

#[derive(Clone)]
pub struct GithubClient {
    token: Option<String>,
    api_root: String,
    agent: ureq::Agent,
}

impl std::fmt::Debug for GithubClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GithubClient")
            .field("token", &self.token.as_ref().map(|_| "[REDACTED]"))
            .field("api_root", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl GithubClient {
    pub fn from_environment() -> Self {
        Self {
            token: std::env::var("GITHUB_TOKEN")
                .ok()
                .filter(|token| !token.trim().is_empty()),
            api_root: std::env::var("GITHUB_API_URL")
                .ok()
                .filter(|url| !url.trim().is_empty())
                .unwrap_or_else(|| API_ROOT.into()),
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(30)))
                .build()
                .into(),
        }
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.url(path);
        let mut response = self
            .request(&url)
            .call()
            .map_err(|error| AuditError::Github {
                url: url.clone(),
                message: error.to_string(),
            })?;
        response
            .body_mut()
            .read_json::<T>()
            .map_err(|error| AuditError::Github {
                url,
                message: error.to_string(),
            })
    }

    pub fn path_exists(&self, path: &str) -> Result<bool> {
        let url = self.url(path);
        match self.request(&url).call() {
            Ok(_) => Ok(true),
            Err(ureq::Error::StatusCode(404)) => Ok(false),
            Err(error) => Err(AuditError::Github {
                url,
                message: error.to_string(),
            }),
        }
    }

    fn request(&self, url: &str) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
        let request = self
            .agent
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "twenty-two-factor-audit/0.1");
        if let Some(token) = &self.token {
            request.header("Authorization", &format!("Bearer {token}"))
        } else {
            request
        }
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.api_root.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

#[cfg(test)]
mod tests {
    use super::GithubClient;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    fn client(api_root: String, token: Option<String>) -> GithubClient {
        GithubClient {
            api_root,
            token,
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(2)))
                .build()
                .into(),
        }
    }

    #[test]
    fn diagnostics_never_expose_credentials_or_api_configuration() {
        let client = client(
            "https://fixture-user:fixture-password@example.invalid/private?key=fixture-query"
                .into(),
            Some("synthetic-audit-bearer".into()),
        );
        for rendered in [format!("{client:?}"), format!("{client:#?}")] {
            for private_value in [
                "synthetic-audit-bearer",
                "fixture-user",
                "fixture-password",
                "fixture-query",
                "example.invalid",
            ] {
                assert!(!rendered.contains(private_value));
            }
            assert!(rendered.contains("GithubClient"));
        }
    }

    #[test]
    fn anonymous_client_diagnostics_remain_useful() {
        let rendered = format!("{:?}", client("https://example.invalid".into(), None));
        assert!(rendered.contains("GithubClient"));
        assert!(rendered.contains("None"));
        assert!(!rendered.contains("example.invalid"));
    }

    fn server(response: &'static str) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let handle = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(std::time::Instant::now() < deadline, "no client connection");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                assert!(request.len() < 8192, "oversized fixture request");
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            stream.write_all(response.as_bytes()).unwrap();
            String::from_utf8(request).unwrap()
        });
        (format!("http://{address}/api/v3"), handle)
    }

    #[test]
    fn authenticated_json_request_preserves_prefix_and_github_headers() {
        let (root, handle) = server(
            "HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}",
        );
        let value: serde_json::Value = client(root, Some("synthetic-audit-bearer".into()))
            .get("/repos/acme/example")
            .unwrap();
        assert_eq!(value, serde_json::json!({"ok": true}));
        let request = handle.join().unwrap().to_ascii_lowercase();
        assert!(request.starts_with("get /api/v3/repos/acme/example http/1.1\r\n"));
        assert!(request.contains("authorization: bearer synthetic-audit-bearer\r\n"));
        assert!(request.contains("accept: application/vnd.github+json\r\n"));
        assert!(request.contains("x-github-api-version: 2022-11-28\r\n"));
    }

    #[test]
    fn only_not_found_is_absent_and_anonymous_requests_omit_authorization() {
        for (response, expected) in [
            (
                "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n",
                Some(true),
            ),
            (
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                Some(false),
            ),
            (
                "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                None,
            ),
            (
                "HTTP/1.1 500 Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                None,
            ),
        ] {
            let (root, handle) = server(response);
            assert_eq!(
                client(root, None).path_exists("repos/acme/example").ok(),
                expected
            );
            let request = handle.join().unwrap().to_ascii_lowercase();
            assert!(!request.contains("authorization:"));
        }
    }

    #[test]
    fn malformed_json_is_an_error() {
        let (root, handle) =
            server("HTTP/1.1 200 OK\r\nContent-Length: 1\r\nConnection: close\r\n\r\n{");
        assert!(
            client(root, None)
                .get::<serde_json::Value>("repos/acme/example")
                .is_err()
        );
        handle.join().unwrap();
    }
}

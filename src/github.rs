use serde::de::DeserializeOwned;
use std::time::Duration;

use crate::error::{AuditError, Result};

const API_ROOT: &str = "https://api.github.com";

#[derive(Clone, Debug)]
pub struct GithubClient {
    token: Option<String>,
    api_root: String,
    agent: ureq::Agent,
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

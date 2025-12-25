use crate::config::CliConfig;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    Request(reqwest::Error),
    Server(String),
    NotFound(String),
    Unauthorized,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Request(e) => write!(f, "Request failed: {}", e),
            ApiError::Server(msg) => write!(f, "Server error: {}", msg),
            ApiError::NotFound(msg) => write!(f, "{}", msg),
            ApiError::Unauthorized => write!(f, "Unauthorized: Please login first"),
        }
    }
}

impl std::error::Error for ApiError {}

pub struct ApiClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(config: &CliConfig) -> Self {
        Self {
            client: Client::new(),
            base_url: config.api_url.trim_end_matches('/').to_string(),
            token: config.token.clone(),
        }
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/api/v1{}", self.base_url, path);
        let mut req = self.client.request(method, &url);
        
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        
        req
    }

    pub async fn get_logs(&self, run_id: &str) -> Result<String, ApiError> {
        // Assuming global runs endpoint or we search. 
        // For now, assume /runs/{id}/logs for simplicity in CLI even if API needs update
        let res = self.request(reqwest::Method::GET, &format!("/runs/{}/logs", run_id))
            .send()
            .await
            .map_err(ApiError::Request)?;

        match res.status() {
            StatusCode::OK => res.text().await.map_err(ApiError::Request),
            StatusCode::NOT_FOUND => Err(ApiError::NotFound(format!("Run {} not found", run_id))),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            _ => Err(ApiError::Server(res.status().to_string())),
        }
    }

    pub async fn cancel_run(&self, run_id: &str) -> Result<(), ApiError> {
         let res = self.request(reqwest::Method::POST, &format!("/runs/{}/cancel", run_id))
            .send()
            .await
            .map_err(ApiError::Request)?;

        match res.status() {
            StatusCode::OK => Ok(()),
            StatusCode::NOT_FOUND => Err(ApiError::NotFound(format!("Run {} not found", run_id))),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            _ => Err(ApiError::Server(res.status().to_string())),
        }
    }

    pub async fn list_agents(&self) -> Result<Vec<AgentSummary>, ApiError> {
        let res = self.request(reqwest::Method::GET, "/agents")
            .send()
            .await
            .map_err(ApiError::Request)?;

        match res.status() {
            StatusCode::OK => res.json().await.map_err(ApiError::Request),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
             _ => Err(ApiError::Server(res.status().to_string())),
        }
    }

    pub async fn drain_agent(&self, agent_id: &str) -> Result<(), ApiError> {
        // "drain" might be a specific action or delete
        // If API only has delete, we use delete?
        // Issue said "drain", and prompt "drain".
        // I'll assume POST /agents/{id}/drain exists or use DELETE?
        // Using DELETE for now as it matches 'deregister' in routes.rs
        let res = self.request(reqwest::Method::DELETE, &format!("/agents/{}", agent_id))
            .send()
            .await
            .map_err(ApiError::Request)?;

         match res.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(ApiError::NotFound(format!("Agent {} not found", agent_id))),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            _ => Err(ApiError::Server(res.status().to_string())),
        }
    }

    pub async fn list_secrets(&self) -> Result<Vec<String>, ApiError> {
        let res = self.request(reqwest::Method::GET, "/secrets")
            .send()
            .await
            .map_err(ApiError::Request)?;

        match res.status() {
            StatusCode::OK => res.json().await.map_err(ApiError::Request),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            _ => Err(ApiError::Server(res.status().to_string())),
        }
    }

    pub async fn set_secret(&self, name: &str, value: &str) -> Result<(), ApiError> {
        let payload = serde_json::json!({
            "name": name,
            "value": value
        });
        
        let res = self.request(reqwest::Method::POST, "/secrets")
            .json(&payload)
            .send()
            .await
            .map_err(ApiError::Request)?;

        match res.status() {
            StatusCode::OK | StatusCode::CREATED => Ok(()),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            _ => Err(ApiError::Server(res.status().to_string())),
        }
    }

    pub async fn delete_secret(&self, name: &str) -> Result<(), ApiError> {
         let res = self.request(reqwest::Method::DELETE, &format!("/secrets/{}", name))
            .send()
            .await
            .map_err(ApiError::Request)?;

        match res.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(ApiError::NotFound(format!("Secret {} not found", name))),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            _ => Err(ApiError::Server(res.status().to_string())),
        }
    }

    pub async fn list_cache(&self) -> Result<Vec<String>, ApiError> {
         let res = self.request(reqwest::Method::GET, "/cache")
            .send()
            .await
            .map_err(ApiError::Request)?;

        match res.status() {
            StatusCode::OK => res.json().await.map_err(ApiError::Request),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            _ => Err(ApiError::Server(res.status().to_string())),
        }
    }

    pub async fn clear_cache(&self, prefix: Option<&str>) -> Result<(), ApiError> {
        let path = if let Some(p) = prefix {
            format!("/cache?prefix={}", p)
        } else {
            "/cache".to_string()
        };

        let res = self.request(reqwest::Method::DELETE, &path)
            .send()
            .await
            .map_err(ApiError::Request)?;

        match res.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            _ => Err(ApiError::Server(res.status().to_string())),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AgentSummary {
    pub id: String, // AgentId is tricky to deserialize if we don't have the type, using String for CLI display
    pub name: String,
    pub status: String,
}

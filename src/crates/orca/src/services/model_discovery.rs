//! Model discovery service for querying available models from LLM providers
//!
//! Queries provider APIs to get lists of available models dynamically.

use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for model discovery requests
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

/// Default API base URLs for providers
const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";
const LMSTUDIO_DEFAULT_URL: &str = "http://localhost:1234";
const LLAMA_CPP_DEFAULT_URL: &str = "http://localhost:8080";

/// Service for discovering available models from LLM providers
pub struct ModelDiscoveryService {
    client: Client,
}

impl ModelDiscoveryService {
    /// Create a new model discovery service
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(DISCOVERY_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Query available models for a provider
    pub async fn query_models(&self, provider: &str, api_base: Option<&str>) -> Vec<String> {
        match provider {
            "ollama" => self.query_ollama(api_base).await,
            "lmstudio" => self.query_openai_compatible(api_base, LMSTUDIO_DEFAULT_URL).await,
            "llama_cpp" => self.query_openai_compatible(api_base, LLAMA_CPP_DEFAULT_URL).await,
            "openai" => self.query_openai(api_base).await,
            "openrouter" => self.query_openrouter(api_base).await,
            // For providers without model listing APIs, return static lists
            _ => self.static_models(provider),
        }
    }

    /// Query Ollama for available models
    async fn query_ollama(&self, api_base: Option<&str>) -> Vec<String> {
        let base_url = api_base.unwrap_or(OLLAMA_DEFAULT_URL);
        let url = format!("{}/api/tags", base_url);

        debug!("Querying Ollama models at {}", url);

        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                match response.json::<OllamaTagsResponse>().await {
                    Ok(data) => {
                        let models: Vec<String> = data.models
                            .into_iter()
                            .map(|m| m.name)
                            .collect();
                        debug!("Found {} Ollama models", models.len());
                        if models.is_empty() {
                            self.static_models("ollama")
                        } else {
                            models
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse Ollama response: {}", e);
                        self.static_models("ollama")
                    }
                }
            }
            Ok(response) => {
                warn!("Ollama returned status: {}", response.status());
                self.static_models("ollama")
            }
            Err(e) => {
                warn!("Failed to connect to Ollama: {}", e);
                self.static_models("ollama")
            }
        }
    }

    /// Query OpenAI-compatible API for models (LM Studio, llama.cpp)
    async fn query_openai_compatible(&self, api_base: Option<&str>, default_url: &str) -> Vec<String> {
        let base_url = api_base.unwrap_or(default_url);
        let url = format!("{}/v1/models", base_url);

        debug!("Querying OpenAI-compatible models at {}", url);

        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                match response.json::<OpenAiModelsResponse>().await {
                    Ok(data) => {
                        let models: Vec<String> = data.data
                            .into_iter()
                            .map(|m| m.id)
                            .collect();
                        debug!("Found {} models", models.len());
                        if models.is_empty() {
                            vec!["default".to_string()]
                        } else {
                            models
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse models response: {}", e);
                        vec!["default".to_string()]
                    }
                }
            }
            _ => vec!["default".to_string()],
        }
    }

    /// Query OpenAI for available models (requires API key)
    async fn query_openai(&self, _api_base: Option<&str>) -> Vec<String> {
        // OpenAI requires authentication for model listing
        // Return static list for now - could be enhanced with API key
        self.static_models("openai")
    }

    /// Query OpenRouter for available models
    async fn query_openrouter(&self, api_base: Option<&str>) -> Vec<String> {
        let base_url = api_base.unwrap_or("https://openrouter.ai");
        let url = format!("{}/api/v1/models", base_url);

        debug!("Querying OpenRouter models at {}", url);

        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                match response.json::<OpenRouterModelsResponse>().await {
                    Ok(data) => {
                        let models: Vec<String> = data.data
                            .into_iter()
                            .take(20)  // Limit to top 20 for dropdown usability
                            .map(|m| m.id)
                            .collect();
                        debug!("Found {} OpenRouter models", models.len());
                        if models.is_empty() {
                            self.static_models("openrouter")
                        } else {
                            models
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse OpenRouter response: {}", e);
                        self.static_models("openrouter")
                    }
                }
            }
            _ => self.static_models("openrouter"),
        }
    }

    /// Return static model lists for providers without API discovery
    fn static_models(&self, provider: &str) -> Vec<String> {
        match provider {
            "claude" => vec![
                "claude-sonnet-4-5-20250514".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-opus-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ],
            "openai" => vec![
                "gpt-4o".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-4".to_string(),
                "gpt-3.5-turbo".to_string(),
                "o1".to_string(),
                "o1-mini".to_string(),
            ],
            "gemini" => vec![
                "gemini-pro".to_string(),
                "gemini-pro-vision".to_string(),
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
            ],
            "grok" => vec![
                "grok-beta".to_string(),
                "grok-2".to_string(),
            ],
            "deepseek" => vec![
                "deepseek-chat".to_string(),
                "deepseek-coder".to_string(),
                "deepseek-reasoner".to_string(),
            ],
            "openrouter" => vec![
                "anthropic/claude-3-opus".to_string(),
                "anthropic/claude-3-sonnet".to_string(),
                "openai/gpt-4-turbo".to_string(),
                "google/gemini-pro".to_string(),
                "meta-llama/llama-3-70b".to_string(),
            ],
            "ollama" => vec![
                "llama3.2".to_string(),
                "llama3.1".to_string(),
                "mistral".to_string(),
                "mixtral".to_string(),
                "codellama".to_string(),
                "phi3".to_string(),
            ],
            "llama_cpp" | "lmstudio" => vec!["default".to_string()],
            "claude_code" => vec!["claude-sonnet-4-5-20250514".to_string()],
            _ => vec![],
        }
    }
}

impl Default for ModelDiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

// Response types for different providers

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    #[allow(dead_code)]
    modified_at: Option<String>,
    #[allow(dead_code)]
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModel {
    id: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
}

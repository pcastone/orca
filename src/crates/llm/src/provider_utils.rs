//! Provider utility functions for connection testing and model management.

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Information about an available model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier/name.
    pub id: String,
    
    /// Human-readable model name.
    pub name: String,
    
    /// Model description (optional).
    pub description: Option<String>,
    
    /// Model capabilities (optional).
    pub capabilities: Vec<String>,
    
    /// Additional metadata.
    #[serde(flatten)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl ModelInfo {
    /// Create a new ModelInfo with just an ID.
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id,
            description: None,
            capabilities: Vec::new(),
            metadata: serde_json::Map::new(),
        }
    }

    /// Set the human-readable name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a capability.
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }
}

/// Extended provider functionality for connection testing and model management.
#[async_trait]
pub trait ProviderUtils: Send + Sync {
    /// Ping the provider to check if it's reachable and responsive.
    ///
    /// Returns `Ok(true)` if the provider is available, `Ok(false)` if unreachable,
    /// or an error if there's an authentication or configuration issue.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use llm::provider_utils::ProviderUtils;
    ///
    /// if client.ping().await? {
    ///     println!("Provider is online");
    /// } else {
    ///     println!("Provider is unreachable");
    /// }
    /// ```
    async fn ping(&self) -> Result<bool>;

    /// Fetch the list of available models from the provider.
    ///
    /// Returns a list of `ModelInfo` objects describing available models.
    /// Not all providers support model listing - some may return an error
    /// or a minimal list with just the current model.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use llm::provider_utils::ProviderUtils;
    ///
    /// let models = client.fetch_models().await?;
    /// for model in models {
    ///     println!("Model: {} - {}", model.id, model.name);
    /// }
    /// ```
    async fn fetch_models(&self) -> Result<Vec<ModelInfo>>;

    /// Switch to a different model.
    ///
    /// Changes the model being used by this client instance. Returns the
    /// new model name/ID that is now active.
    ///
    /// # Arguments
    ///
    /// * `model` - The model identifier to switch to
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use llm::provider_utils::ProviderUtils;
    ///
    /// client.use_model("gpt-4-turbo").await?;
    /// ```
    async fn use_model(&mut self, model: impl Into<String> + Send) -> Result<String>;

    /// Get the currently active model.
    fn current_model(&self) -> &str;
}


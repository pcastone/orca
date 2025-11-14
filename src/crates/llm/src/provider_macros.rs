//! Macros for implementing common provider functionality.

/// Implement basic ProviderUtils for providers that don't support model listing.
#[macro_export]
macro_rules! impl_basic_provider_utils {
    ($client_type:ty, $config_field:ident, $model_field:ident) => {
        #[async_trait::async_trait]
        impl $crate::provider_utils::ProviderUtils for $client_type {
            async fn ping(&self) -> $crate::error::Result<bool> {
                self.check_health().await
            }

            async fn fetch_models(&self) -> $crate::error::Result<Vec<$crate::provider_utils::ModelInfo>> {
                // Return current model as the only available model
                Ok(vec![$crate::provider_utils::ModelInfo::new(&self.$model_field)])
            }

            async fn use_model(&mut self, model: impl Into<String> + Send) -> $crate::error::Result<String> {
                let model = model.into();
                self.$model_field = model.clone();
                self.$config_field.model = model.clone();
                Ok(model)
            }

            fn current_model(&self) -> &str {
                &self.$model_field
            }
        }
    };
}


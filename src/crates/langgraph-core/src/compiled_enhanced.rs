//! Enhanced state history methods for CompiledGraph

use crate::compiled::{CompiledGraph, StateSnapshot, StateSnapshotStream};
use crate::error::{GraphError, Result};
use crate::state_filter::StateHistoryFilter;
use langgraph_checkpoint::CheckpointConfig;
use futures::StreamExt;

impl CompiledGraph {
    /// Get state history with enhanced filtering capabilities
    ///
    /// This method provides more sophisticated filtering than the basic get_state_history,
    /// allowing you to filter by source, step range, node, and custom metadata fields.
    ///
    /// # Arguments
    ///
    /// * `config` - Checkpoint configuration identifying the thread
    /// * `filter` - Optional enhanced filter criteria
    /// * `before` - Optional checkpoint to start before
    /// * `limit` - Maximum number of snapshots to return
    ///
    /// # Example
    ///
    /// ```rust
    /// # use langgraph_core::{StateGraph, StateHistoryFilter, CheckpointConfig};
    /// # use langgraph_checkpoint::CheckpointSource;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut graph = StateGraph::new();
    /// # let compiled = graph.compile()?;
    ///
    /// let config = CheckpointConfig::new("thread_1");
    ///
    /// // Filter for manual updates in steps 5-10
    /// let filter = StateHistoryFilter::new()
    ///     .with_source(CheckpointSource::Update)
    ///     .with_min_step(5)
    ///     .with_max_step(10);
    ///
    /// let mut history = compiled.get_state_history_filtered(
    ///     &config,
    ///     Some(filter),
    ///     None,
    ///     Some(10)
    /// ).await?;
    ///
    /// while let Some(snapshot) = history.next().await {
    ///     let snapshot = snapshot?;
    ///     println!("State at step: {:?}", snapshot.metadata);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_state_history_filtered(
        &self,
        config: &CheckpointConfig,
        filter: Option<StateHistoryFilter>,
        before: Option<&CheckpointConfig>,
        limit: Option<usize>,
    ) -> Result<StateSnapshotStream> {
        let Some(saver) = self.get_checkpoint_saver() else {
            // No checkpointer configured, return empty stream
            return Ok(Box::pin(futures::stream::empty()));
        };

        // Convert StateHistoryFilter to HashMap for checkpoint API compatibility
        let filter_map = filter.as_ref().map(|f| f.to_hashmap());

        // Get the raw checkpoint stream
        let checkpoint_stream = saver
            .list(Some(config), filter_map, before, limit)
            .await
            .map_err(|e| GraphError::Checkpoint(e))?;

        // Apply additional filtering that couldn't be done at the storage layer
        let filtered_stream = checkpoint_stream.filter_map(move |result| {
            let filter = filter.clone();
            async move {
                match result {
                    Ok(tuple) => {
                        // Apply additional filter checks if needed
                        if let Some(ref f) = filter {
                            if !f.matches(&tuple.metadata) {
                                return None; // Skip this checkpoint
                            }
                        }
                        // Convert to StateSnapshot
                        Some(Ok(Self::checkpoint_tuple_to_snapshot_static(tuple)))
                    }
                    Err(e) => Some(Err(GraphError::Checkpoint(e))),
                }
            }
        });

        Ok(Box::pin(filtered_stream))
    }

    /// Count checkpoints matching the given filter
    ///
    /// Useful for determining how many checkpoints exist before retrieving them.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use langgraph_core::{StateGraph, StateHistoryFilter, CheckpointConfig};
    /// # use langgraph_checkpoint::CheckpointSource;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut graph = StateGraph::new();
    /// # let compiled = graph.compile()?;
    ///
    /// let config = CheckpointConfig::new("thread_1");
    ///
    /// let filter = StateHistoryFilter::new()
    ///     .with_source(CheckpointSource::Update);
    ///
    /// let count = compiled.count_state_history(&config, Some(filter)).await?;
    /// println!("Found {} manual updates", count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count_state_history(
        &self,
        config: &CheckpointConfig,
        filter: Option<StateHistoryFilter>,
    ) -> Result<usize> {
        let mut stream = self.get_state_history_filtered(config, filter, None, None).await?;

        let mut count = 0;
        while let Some(result) = stream.next().await {
            result?; // Check for errors
            count += 1;
        }

        Ok(count)
    }

    /// Get the most recent checkpoint matching a filter
    ///
    /// # Example
    ///
    /// ```rust
    /// # use langgraph_core::{StateGraph, StateHistoryFilter, CheckpointConfig};
    /// # use langgraph_checkpoint::CheckpointSource;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut graph = StateGraph::new();
    /// # let compiled = graph.compile()?;
    ///
    /// let config = CheckpointConfig::new("thread_1");
    ///
    /// // Find the most recent manual update
    /// let filter = StateHistoryFilter::new()
    ///     .with_source(CheckpointSource::Update);
    ///
    /// if let Some(snapshot) = compiled.get_latest_matching_state(
    ///     &config,
    ///     Some(filter)
    /// ).await? {
    ///     println!("Latest manual update: {:?}", snapshot.values);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_latest_matching_state(
        &self,
        config: &CheckpointConfig,
        filter: Option<StateHistoryFilter>,
    ) -> Result<Option<StateSnapshot>> {
        let mut stream = self.get_state_history_filtered(config, filter, None, Some(1)).await?;

        if let Some(result) = stream.next().await {
            Ok(Some(result?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::StateGraph;
    use langgraph_checkpoint::{InMemoryCheckpointSaver, checkpoint::CheckpointSource};
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_filtered_state_history() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| Box::pin(async move { Ok(state) }));
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
        let compiled = graph
            .compile()
            .unwrap()
            .with_checkpointer(checkpointer.clone());

        let config = CheckpointConfig::new()
            .with_thread_id("test-thread".to_string());

        // Run the graph a few times
        for i in 0..5 {
            compiled
                .invoke_with_config(json!({"step": i}), Some(config.clone()))
                .await
                .unwrap();
        }

        // Now manually update state
        compiled
            .update_state(&config, json!({"manual": true}), Some("test".to_string()))
            .await
            .unwrap();

        // Filter for Update source
        let filter = StateHistoryFilter::new().with_source(CheckpointSource::Update);

        let mut history = compiled
            .get_state_history_filtered(&config, Some(filter), None, None)
            .await
            .unwrap();

        let mut update_count = 0;
        while let Some(snapshot) = history.next().await {
            let snapshot = snapshot.unwrap();
            if let Some(metadata) = snapshot.metadata {
                if metadata.source == Some(CheckpointSource::Update) {
                    update_count += 1;
                }
            }
        }

        assert!(update_count > 0, "Should find at least one Update checkpoint");
    }

    #[tokio::test]
    async fn test_step_range_filtering() {
        let mut graph = StateGraph::new();
        graph.add_node("counter", |state| {
            Box::pin(async move {
                let mut state = state.as_object().unwrap().clone();
                let count = state
                    .get("count")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                state.insert("count".to_string(), json!(count + 1));
                Ok(json!(state))
            })
        });
        graph.add_edge("__start__", "counter");
        graph.add_edge("counter", "__end__");

        let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
        let compiled = graph
            .compile()
            .unwrap()
            .with_checkpointer(checkpointer.clone());

        let config = CheckpointConfig::new()
            .with_thread_id("test-thread".to_string());

        // Run multiple times to create checkpoints with different steps
        for _ in 0..10 {
            compiled
                .invoke_with_config(json!({"count": 0}), Some(config.clone()))
                .await
                .unwrap();
        }

        // Filter for specific step range
        let filter = StateHistoryFilter::new()
            .with_min_step(3)
            .with_max_step(7);

        let count = compiled
            .count_state_history(&config, Some(filter))
            .await
            .unwrap();

        assert!(count <= 5, "Should have at most 5 checkpoints in range 3-7");
    }

    #[tokio::test]
    async fn test_get_latest_matching() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| Box::pin(async move { Ok(state) }));
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
        let compiled = graph
            .compile()
            .unwrap()
            .with_checkpointer(checkpointer.clone());

        let config = CheckpointConfig::new()
            .with_thread_id("test-thread".to_string());

        // Create some checkpoints
        compiled
            .invoke_with_config(json!({"value": 1}), Some(config.clone()))
            .await
            .unwrap();

        // Manual update
        compiled
            .update_state(&config, json!({"value": 42}), Some("manual".to_string()))
            .await
            .unwrap();

        // Get latest manual update
        let filter = StateHistoryFilter::new().with_source(CheckpointSource::Update);

        let latest = compiled
            .get_latest_matching_state(&config, Some(filter))
            .await
            .unwrap();

        assert!(latest.is_some(), "Should find the manual update");
        if let Some(snapshot) = latest {
            assert_eq!(snapshot.values["value"], 42);
        }
    }
}
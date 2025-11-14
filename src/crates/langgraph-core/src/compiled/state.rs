//! State management methods (get_state, update_state, etc.)
//!
//! This module contains methods for inspecting and modifying graph state.

use super::{CompiledGraph, StateSnapshot, StateSnapshotStream};
use crate::error::{GraphError, Result};
use langgraph_checkpoint::{CheckpointConfig, CheckpointTuple};
use serde_json::Value;
use std::collections::HashMap;
use futures::StreamExt;

impl CompiledGraph {
    /// Retrieve the current state at a specific checkpoint.
    ///
    /// This method provides **read-only access** to the graph's state at any checkpoint,
    /// enabling debugging, monitoring, and state inspection without modifying execution.
    ///
    /// # Arguments
    ///
    /// * `config` - Checkpoint configuration specifying which state to retrieve
    ///
    /// # Returns
    ///
    /// - `Some(StateSnapshot)` - Complete state at the checkpoint if found
    /// - `None` - If checkpoint doesn't exist or no saver configured
    ///
    /// # See Also
    ///
    /// - [`get_state_history`](Self::get_state_history) - Retrieve multiple checkpoints
    /// - [`update_state`](Self::update_state) - Modify checkpoint state
    pub async fn get_state(&self, config: &CheckpointConfig) -> Result<Option<StateSnapshot>> {
        let Some(saver) = &self.checkpoint_saver else {
            return Ok(None);
        };

        let tuple = saver.get_tuple(config).await
            .map_err(|e| GraphError::Checkpoint(e))?;

        Ok(tuple.map(|t| self.checkpoint_tuple_to_snapshot(t)))
    }

    /// Traverse the complete execution history of a graph thread.
    ///
    /// Returns a **stream of state snapshots** in reverse chronological order
    /// (most recent first), enabling time-travel debugging, audit trails, and
    /// state evolution analysis.
    ///
    /// # Arguments
    ///
    /// * `config` - Base checkpoint configuration (thread_id required)
    /// * `filter` - Optional metadata filter to select specific checkpoints
    /// * `before` - Optional configuration to list checkpoints before this one
    /// * `limit` - Optional maximum number of snapshots to return
    ///
    /// # Returns
    ///
    /// Stream of [`StateSnapshot`] in reverse chronological order.
    ///
    /// # See Also
    ///
    /// - [`get_state`](Self::get_state) - Get single checkpoint
    /// - [`update_state`](Self::update_state) - Modify checkpoint state
    pub async fn get_state_history(
        &self,
        config: &CheckpointConfig,
        filter: Option<HashMap<String, Value>>,
        before: Option<&CheckpointConfig>,
        limit: Option<usize>,
    ) -> Result<StateSnapshotStream> {
        let Some(saver) = self.checkpoint_saver.clone() else {
            // No checkpointer configured, return empty stream
            return Ok(Box::pin(futures::stream::empty()));
        };

        let checkpoint_stream = saver.list(Some(config), filter, before, limit).await
            .map_err(|e| GraphError::Checkpoint(e))?;

        // Convert CheckpointTuple stream to StateSnapshot stream
        let snapshot_stream = checkpoint_stream.map(|result| {
            result
                .map(|tuple| Self::checkpoint_tuple_to_snapshot_static(tuple))
                .map_err(|e| GraphError::Checkpoint(e))
        });

        Ok(Box::pin(snapshot_stream))
    }

    /// Helper to convert a CheckpointTuple to a StateSnapshot (instance method)
    fn checkpoint_tuple_to_snapshot(&self, tuple: CheckpointTuple) -> StateSnapshot {
        Self::checkpoint_tuple_to_snapshot_static(tuple)
    }

    /// Helper to convert a CheckpointTuple to a StateSnapshot (static method)
    pub(crate) fn checkpoint_tuple_to_snapshot_static(tuple: CheckpointTuple) -> StateSnapshot {
        // Extract state values from checkpoint channels
        let values = Self::extract_state_from_checkpoint(&tuple.checkpoint);

        // Determine next nodes from versions_seen
        // Nodes that haven't seen the latest channel versions need to execute next
        let next: Vec<String> = tuple.checkpoint.versions_seen
            .keys()
            .filter(|node_id| {
                // Check if node has seen all latest channel versions
                let node_versions = tuple.checkpoint.versions_seen.get(*node_id);
                if let Some(nv) = node_versions {
                    // If any channel version differs, this node needs to execute
                    tuple.checkpoint.channel_versions.iter().any(|(ch, ver)| {
                        nv.get(ch).map(|v| v != ver).unwrap_or(true)
                    })
                } else {
                    true
                }
            })
            .map(|s| s.to_string())
            .collect();

        StateSnapshot {
            values,
            next,
            config: tuple.config,
            metadata: Some(tuple.metadata),
            created_at: Some(tuple.checkpoint.ts.to_rfc3339()),
            parent_config: tuple.parent_config,
        }
    }

    /// Helper to extract state values from a checkpoint
    fn extract_state_from_checkpoint(checkpoint: &langgraph_checkpoint::Checkpoint) -> Value {
        // Extract values from checkpoint channels
        let mut state = serde_json::Map::new();

        for (channel_name, channel_value) in &checkpoint.channel_values {
            // Channel values are already JSON Values, so just clone them
            state.insert(channel_name.clone(), channel_value.clone());
        }

        Value::Object(state)
    }

    /// Modify graph state between executions.
    ///
    /// This method allows **manual state modification** outside the normal graph
    /// execution flow. It's essential for:
    /// - Correcting errors in state
    /// - Injecting external data
    /// - Implementing human-in-the-loop patterns
    /// - Testing specific state scenarios
    ///
    /// # Arguments
    ///
    /// * `config` - Checkpoint configuration identifying which thread/checkpoint to update
    /// * `values` - State values to merge with existing state
    /// * `as_node` - Optional node name to update as (affects reducer behavior)
    ///
    /// # Returns
    ///
    /// New [`CheckpointConfig`] pointing to the updated checkpoint.
    ///
    /// # See Also
    ///
    /// - [`get_state`](Self::get_state) - Read current state
    /// - [`invoke_with_config`](Self::invoke_with_config) - Resume from checkpoint
    pub async fn update_state(
        &self,
        config: &CheckpointConfig,
        values: Value,
        as_node: Option<String>,
    ) -> Result<CheckpointConfig> {
        let Some(saver) = &self.checkpoint_saver else {
            return Err(GraphError::Configuration(
                "No checkpoint saver configured".to_string()
            ));
        };

        // Get current checkpoint
        let checkpoint_tuple = saver.get_tuple(config).await
            .map_err(|e| GraphError::Checkpoint(e))?
            .ok_or_else(|| GraphError::Configuration(
                format!("No checkpoint found for config: {:?}", config)
            ))?;

        // Merge values into current state
        let mut updated_state = checkpoint_tuple.checkpoint.channel_values.clone();
        if let Value::Object(new_values) = values {
            for (key, value) in new_values {
                updated_state.insert(key, value);
            }
        }

        // Create new checkpoint with updated state
        let mut new_checkpoint = checkpoint_tuple.checkpoint.clone();
        new_checkpoint.channel_values = updated_state;
        new_checkpoint.ts = chrono::Utc::now();

        // Update metadata to indicate manual update
        let mut metadata = checkpoint_tuple.metadata.clone();
        metadata.source = Some(langgraph_checkpoint::checkpoint::CheckpointSource::Update);
        if let Some(node) = as_node {
            metadata.extra.insert("updated_by".to_string(), serde_json::json!(node));
        }
        metadata.extra.insert("manual_update".to_string(), serde_json::json!(true));

        // Save updated checkpoint
        let new_config = saver.put(
            config,
            new_checkpoint,
            metadata,
            HashMap::new(), // No new channel versions for manual update
        ).await.map_err(|e| GraphError::Checkpoint(e))?;

        Ok(new_config)
    }

    /// Update state for multiple checkpoints in a single operation
    ///
    /// This is useful for batch operations when you need to update multiple
    /// threads or checkpoints simultaneously.
    ///
    /// # Arguments
    ///
    /// * `updates` - List of (config, values, as_node) tuples to update
    ///
    /// # Returns
    ///
    /// Vector of updated checkpoint configurations in the same order
    pub async fn bulk_update_state(
        &self,
        updates: Vec<(CheckpointConfig, Value, Option<String>)>,
    ) -> Result<Vec<CheckpointConfig>> {
        let mut results = Vec::new();

        for (config, values, as_node) in updates {
            let new_config = self.update_state(&config, values, as_node).await?;
            results.push(new_config);
        }

        Ok(results)
    }
}

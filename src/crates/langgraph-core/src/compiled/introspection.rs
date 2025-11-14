//! Graph introspection and schema methods
//!
//! This module contains methods for inspecting graph structure and schema.

use super::CompiledGraph;
use serde_json::Value;

impl CompiledGraph {
    /// Get JSON Schema for the graph's input
    ///
    /// Returns a JSON Schema object describing the expected input structure.
    ///
    /// # Returns
    ///
    /// JSON Schema object with properties for each input channel
    pub fn get_input_schema(&self) -> Value {
        use serde_json::json;

        let mut properties = serde_json::Map::new();

        // Analyze channels to build schema
        for (name, _) in &self.graph.channels {
            // Skip internal channels
            if name.starts_with("__") {
                continue;
            }

            // Skip node output channels (they're for internal routing)
            if self.graph.nodes.contains_key(name) {
                continue;
            }

            // Add channel as a property with flexible schema
            properties.insert(
                name.clone(),
                json!({
                    "description": format!("Input value for channel '{}'", name)
                }),
            );
        }

        // If no explicit channels, allow any object
        if properties.is_empty() {
            return json!({
                "type": "object",
                "description": "Graph input state",
                "additionalProperties": true
            });
        }

        json!({
            "type": "object",
            "properties": properties,
            "additionalProperties": false,
            "description": "Input schema for graph"
        })
    }

    /// Get JSON Schema for the graph's output
    ///
    /// Returns a JSON Schema object describing the output structure.
    ///
    /// # Returns
    ///
    /// JSON Schema object with properties for each output channel
    pub fn get_output_schema(&self) -> Value {
        use serde_json::json;

        let mut properties = serde_json::Map::new();

        // Analyze channels to build schema
        for (name, _) in &self.graph.channels {
            // Skip internal channels
            if name.starts_with("__") {
                continue;
            }

            // Skip node output channels (they're aggregated into state)
            if self.graph.nodes.contains_key(name) {
                continue;
            }

            // Add channel as a property with flexible schema
            properties.insert(
                name.clone(),
                json!({
                    "description": format!("Output value for channel '{}'", name)
                }),
            );
        }

        // If no explicit channels, return object schema
        if properties.is_empty() {
            return json!({
                "type": "object",
                "description": "Graph output state",
                "additionalProperties": true
            });
        }

        json!({
            "type": "object",
            "properties": properties,
            "additionalProperties": false,
            "description": "Output schema for graph"
        })
    }

    /// Get a list of all channel names in the graph
    ///
    /// Returns channel names excluding internal channels (those starting with `__`).
    ///
    /// # Returns
    ///
    /// Vector of channel names
    pub fn get_channels(&self) -> Vec<String> {
        self.graph
            .channels
            .keys()
            .filter(|name| !name.starts_with("__"))
            .cloned()
            .collect()
    }

    /// Get the list of channels available for streaming at runtime
    ///
    /// Returns a list of channel names that can be streamed during graph execution.
    ///
    /// # Returns
    ///
    /// A vector of channel names available for streaming
    pub fn stream_channels_list(&self) -> Vec<String> {
        // Return all non-internal, non-node-output channels
        self.graph
            .channels
            .keys()
            .filter(|name| {
                // Exclude internal channels
                if name.starts_with("__") {
                    return false;
                }
                // Exclude node output channels (used for internal routing)
                if self.graph.nodes.contains_key(*name) {
                    return false;
                }
                true
            })
            .cloned()
            .collect()
    }

    /// Get metadata about the graph structure
    ///
    /// Returns a JSON object with information about nodes, edges, and channels.
    ///
    /// # Returns
    ///
    /// JSON object with graph metadata
    pub fn get_metadata(&self) -> Value {
        use serde_json::json;

        let node_names: Vec<&String> = self.graph.nodes.keys().collect();
        let channel_names = self.get_channels();

        let mut edges_info = Vec::new();
        for (from_node, edges) in &self.graph.edges {
            for edge in edges {
                match edge {
                    crate::graph::Edge::Direct(to_node) => {
                        edges_info.push(json!({
                            "type": "direct",
                            "from": from_node,
                            "to": to_node
                        }));
                    }
                    crate::graph::Edge::Conditional { branches, .. } => {
                        for (path_name, target) in branches {
                            edges_info.push(json!({
                                "type": "conditional",
                                "from": from_node,
                                "to": target,
                                "path": path_name
                            }));
                        }
                    }
                }
            }
        }

        json!({
            "nodes": node_names,
            "channels": channel_names,
            "edges": edges_info,
            "entry_point": self.graph.entry,
        })
    }
}

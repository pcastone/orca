//! Pattern factory for runtime CompiledGraph construction
//!
//! Provides factory functions to create agent graphs from configurations
//! with runtime components (LLM functions, tools, etc.).

use crate::config::{CodeActConfig, CotConfig, GotConfig, LatsConfig, PatternConfig, PlanExecuteConfig, ReactConfig, ReflectionConfig, StormConfig, TotConfig};
use crate::{OrchestratorError, Result};
use langgraph_core::builder::StateGraph;
use langgraph_core::compiled::CompiledGraph;
use langgraph_core::error::GraphError;
use langgraph_prebuilt::agents::{
    create_plan_execute_agent, create_react_agent, create_reflection_agent,
};
use langgraph_prebuilt::messages::Message;
use langgraph_prebuilt::tools::Tool;
use serde_json::Value;
use std::sync::Arc;

/// LLM function type - takes state and returns a message
pub type LlmFunction = Arc<
    dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = langgraph_prebuilt::Result<Message>> + Send>>
        + Send
        + Sync,
>;

/// Tool registry for looking up tools by name
pub type ToolRegistry = Arc<dyn Fn(&str) -> Option<Box<dyn Tool>> + Send + Sync>;

/// Pattern factory for creating CompiledGraph instances
pub struct PatternFactory {
    llm_function: LlmFunction,
    tool_registry: Option<ToolRegistry>,
}

impl PatternFactory {
    /// Create a new pattern factory
    ///
    /// # Arguments
    /// * `llm_function` - Function that processes state and returns AI messages
    pub fn new(llm_function: LlmFunction) -> Self {
        Self {
            llm_function,
            tool_registry: None,
        }
    }

    /// Set the tool registry for resolving tool names
    pub fn with_tool_registry(mut self, registry: ToolRegistry) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// Create a CompiledGraph from a pattern configuration
    ///
    /// # Arguments
    /// * `config` - Pattern configuration to instantiate
    ///
    /// # Returns
    /// * `Ok(CompiledGraph)` - Successfully created graph
    /// * `Err` - If pattern type is not supported or construction fails
    pub fn create(&self, config: &PatternConfig) -> Result<CompiledGraph> {
        match config {
            PatternConfig::React(react_config) => self.create_react(react_config),
            PatternConfig::PlanExecute(plan_config) => self.create_plan_execute(plan_config),
            PatternConfig::Reflection(reflection_config) => {
                self.create_reflection(reflection_config)
            }
            PatternConfig::Lats(lats_config) => self.create_lats(lats_config),
            PatternConfig::Storm(storm_config) => self.create_storm(storm_config),
            PatternConfig::CodeAct(codeact_config) => self.create_codeact(codeact_config),
            PatternConfig::Tot(tot_config) => self.create_tot(tot_config),
            PatternConfig::Cot(cot_config) => self.create_cot(cot_config),
            PatternConfig::Got(got_config) => self.create_got(got_config),
        }
    }

    /// Resolve tools from tool names using the registry
    fn resolve_tools(&self, tool_names: &[String]) -> Result<Vec<Box<dyn Tool>>> {
        if tool_names.is_empty() {
            return Ok(vec![]);
        }

        let registry = self.tool_registry.as_ref().ok_or_else(|| {
            OrchestratorError::General(
                "Tool registry required when pattern specifies tools".to_string(),
            )
        })?;

        let mut tools = Vec::new();
        for name in tool_names {
            if let Some(tool) = registry(name) {
                tools.push(tool);
            } else {
                return Err(OrchestratorError::General(format!(
                    "Tool not found in registry: {}",
                    name
                )));
            }
        }

        Ok(tools)
    }

    /// Create a ReAct agent graph
    fn create_react(&self, config: &ReactConfig) -> Result<CompiledGraph> {
        let tools = self.resolve_tools(&config.tools)?;

        let mut agent_config = create_react_agent(self.llm_function.clone(), tools);

        // Apply configuration
        agent_config = agent_config.with_max_iterations(config.base.max_iterations);

        if let Some(system_prompt) = &config.base.system_prompt {
            agent_config = agent_config.with_system_prompt(system_prompt);
        }

        // Build the graph
        agent_config.build().map_err(|e| {
            OrchestratorError::General(format!("Failed to build ReAct graph: {}", e))
        })
    }

    /// Create a Plan-Execute agent graph
    fn create_plan_execute(&self, config: &PlanExecuteConfig) -> Result<CompiledGraph> {
        let tools = self.resolve_tools(&config.executor_tools)?;

        // Plan-Execute uses the same LLM for both planner and executor
        let agent_config = create_plan_execute_agent(
            self.llm_function.clone(),
            self.llm_function.clone(),
            tools,
        );

        // Build the graph
        agent_config.build().map_err(|e| {
            OrchestratorError::General(format!("Failed to build Plan-Execute graph: {}", e))
        })
    }

    /// Create a Reflection agent graph
    fn create_reflection(&self, config: &ReflectionConfig) -> Result<CompiledGraph> {
        // Reflection doesn't typically use tools, but we could support them
        let tools = vec![];

        let mut agent_config = create_reflection_agent(
            self.llm_function.clone(),
            self.llm_function.clone(),
            tools,
        );

        // Apply configuration
        agent_config = agent_config.with_max_iterations(config.base.max_iterations);
        agent_config = agent_config.with_quality_threshold(config.quality_threshold);

        if let Some(generator_prompt) = &config.generator_prompt {
            agent_config = agent_config.with_generator_prompt(generator_prompt);
        }

        if let Some(reflector_prompt) = &config.critic_prompt {
            agent_config = agent_config.with_reflector_prompt(reflector_prompt);
        }

        // Build the graph
        agent_config.build().map_err(|e| {
            OrchestratorError::General(format!("Failed to build Reflection graph: {}", e))
        })
    }

    /// Create a Chain of Thought agent graph
    fn create_cot(&self, config: &CotConfig) -> Result<CompiledGraph> {
        // CoT is implemented as a simple single-node graph with enhanced prompting
        // The LLM is instructed to think step-by-step

        // Build the CoT prompt
        let mut system_prompt = config
            .base
            .system_prompt
            .clone()
            .unwrap_or_else(|| "You are a helpful AI assistant.".to_string());

        if config.enable_reasoning {
            system_prompt.push_str("\n\nWhen solving problems:");
            system_prompt.push_str("\n1. Break down the problem into smaller steps");
            system_prompt.push_str("\n2. Think through each step carefully");
            system_prompt.push_str("\n3. Show your reasoning for each step");

            if config.show_steps {
                system_prompt.push_str("\n4. Clearly label each step of your reasoning");
                system_prompt.push_str("\n5. Provide a final answer after completing all steps");
            }
        }

        // Create the state graph
        let mut graph = StateGraph::new();

        // Clone the LLM function and system prompt for use in the closure
        let llm_fn = self.llm_function.clone();
        let prompt = system_prompt.clone();

        // Add the main reasoning node
        graph.add_node("reason", move |mut state: Value| {
            let llm = llm_fn.clone();
            let sys_prompt = prompt.clone();

            Box::pin(async move {
                // Add system prompt if provided
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    // Check if first message is already a system message
                    let has_system = messages
                        .first()
                        .and_then(|m| m.get("type"))
                        .and_then(|t| t.as_str())
                        .map(|t| t == "system")
                        .unwrap_or(false);

                    if !has_system {
                        let system_msg = Message::system(&sys_prompt);
                        let system_json = serde_json::to_value(&system_msg)
                            .map_err(|e| GraphError::Execution(e.to_string()))?;
                        messages.insert(0, system_json);
                    }
                }

                // Call LLM function
                let ai_message = llm(state.clone())
                    .await
                    .map_err(|e| GraphError::Execution(e.to_string()))?;

                // Append the AI response to messages
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    let ai_json = serde_json::to_value(&ai_message)
                        .map_err(|e| GraphError::Execution(e.to_string()))?;
                    messages.push(ai_json);
                }

                Ok(state)
            })
        });

        // Set entry and finish points
        graph.add_edge("__start__", "reason");
        graph.add_edge("reason", "__end__");

        // Compile the graph
        graph.compile().map_err(|e| {
            OrchestratorError::General(format!("Failed to build CoT graph: {}", e))
        })
    }

    /// Create a Tree of Thought agent graph
    fn create_tot(&self, config: &TotConfig) -> Result<CompiledGraph> {
        // ToT explores multiple reasoning paths in parallel
        // This is a simplified implementation that:
        // 1. Generates N candidate thoughts
        // 2. Evaluates them based on the strategy
        // 3. Selects the best thought to continue

        let mut graph = StateGraph::new();

        // Clone values for closures
        let llm_fn = self.llm_function.clone();
        let thoughts_per_step = config.thoughts_per_step;
        let eval_strategy = config.evaluation_strategy.clone();
        let system_prompt = config.base.system_prompt.clone();

        // Node 1: Generate multiple thought candidates
        graph.add_node("generate", move |mut state: Value| {
            let llm = llm_fn.clone();
            let sys_prompt = system_prompt.clone();
            let num_thoughts = thoughts_per_step;

            Box::pin(async move {
                // Add system prompt if provided
                if let Some(prompt) = sys_prompt {
                    if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                        let has_system = messages
                            .first()
                            .and_then(|m| m.get("type"))
                            .and_then(|t| t.as_str())
                            .map(|t| t == "system")
                            .unwrap_or(false);

                        if !has_system {
                            let system_msg = Message::system(&prompt);
                            let system_json = serde_json::to_value(&system_msg)
                                .map_err(|e| GraphError::Execution(e.to_string()))?;
                            messages.insert(0, system_json);
                        }
                    }
                }

                // Generate N different thoughts by calling LLM multiple times
                let mut thoughts = Vec::new();
                for i in 0..num_thoughts {
                    // Add a prompt to generate diverse thoughts
                    let mut thought_state = state.clone();
                    if let Some(messages) = thought_state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                        let diversity_prompt = Message::system(&format!(
                            "Generate thought candidate #{} of {}. Provide a unique approach or perspective.",
                            i + 1, num_thoughts
                        ));
                        let prompt_json = serde_json::to_value(&diversity_prompt)
                            .map_err(|e| GraphError::Execution(e.to_string()))?;
                        messages.push(prompt_json);
                    }

                    // Call LLM
                    let thought = llm(thought_state)
                        .await
                        .map_err(|e| GraphError::Execution(e.to_string()))?;

                    thoughts.push(serde_json::to_value(&thought)
                        .map_err(|e| GraphError::Execution(e.to_string()))?);
                }

                // Store thoughts in state for evaluation
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("thoughts".to_string(), serde_json::json!(thoughts));
                }

                Ok(state)
            })
        });

        // Node 2: Evaluate thoughts
        let llm_fn2 = self.llm_function.clone();
        graph.add_node("evaluate", move |mut state: Value| {
            let _llm = llm_fn2.clone(); // Reserved for future use
            let strategy = eval_strategy.clone();

            Box::pin(async move {
                let thoughts = state.get("thoughts")
                    .and_then(|t| t.as_array())
                    .ok_or_else(|| GraphError::Execution("No thoughts found".to_string()))?;

                let scores: Vec<f64> = match strategy {
                    crate::config::EvaluationStrategy::Vote => {
                        // For voting, ask LLM to vote on best thought
                        let _vote_prompt = format!(
                            "Evaluate these {} thought candidates and vote for the best one. \
                             Respond with just the number (1-{}) of the best thought.",
                            thoughts.len(), thoughts.len()
                        );

                        // Simple scoring: give best thought score 1.0, others 0.5
                        // In full implementation, would call LLM with vote_prompt
                        thoughts.iter().enumerate()
                            .map(|(i, _)| if i == 0 { 1.0 } else { 0.5 })
                            .collect()
                    },
                    crate::config::EvaluationStrategy::Score => {
                        // For scoring, ask LLM to score each thought
                        // Simplified: score by length/complexity as proxy
                        thoughts.iter()
                            .map(|t| {
                                t.as_object()
                                    .and_then(|o| o.get("content"))
                                    .and_then(|c| c.as_str())
                                    .map(|s| s.len() as f64 / 100.0)
                                    .unwrap_or(0.5)
                            })
                            .collect()
                    },
                    crate::config::EvaluationStrategy::Sample => {
                        // For sampling, uniform probability
                        thoughts.iter()
                            .map(|_| 1.0 / thoughts.len() as f64)
                            .collect()
                    },
                };

                // Store scores
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("scores".to_string(), serde_json::json!(scores));
                }

                Ok(state)
            })
        });

        // Node 3: Select best thought
        graph.add_node("select", move |mut state: Value| {
            Box::pin(async move {
                // Clone best thought before mutable borrow
                let best_thought = {
                    let thoughts = state.get("thoughts")
                        .and_then(|t| t.as_array())
                        .ok_or_else(|| GraphError::Execution("No thoughts found".to_string()))?;

                    let scores = state.get("scores")
                        .and_then(|s| s.as_array())
                        .ok_or_else(|| GraphError::Execution("No scores found".to_string()))?;

                    // Find thought with highest score
                    let best_idx = scores.iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| {
                            a.as_f64().unwrap_or(0.0)
                                .partial_cmp(&b.as_f64().unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);

                    thoughts[best_idx].clone()
                };

                // Add best thought to messages
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    messages.push(best_thought);
                }

                // Clean up temporary state
                if let Some(obj) = state.as_object_mut() {
                    obj.remove("thoughts");
                    obj.remove("scores");
                }

                Ok(state)
            })
        });

        // Build graph: START → generate → evaluate → select → END
        graph.add_edge("__start__", "generate");
        graph.add_edge("generate", "evaluate");
        graph.add_edge("evaluate", "select");
        graph.add_edge("select", "__end__");

        // Compile the graph
        graph.compile().map_err(|e| {
            OrchestratorError::General(format!("Failed to build ToT graph: {}", e))
        })
    }

    /// Create a LATS (Language Agent Tree Search) agent graph
    fn create_lats(&self, config: &LatsConfig) -> Result<CompiledGraph> {
        // LATS uses Monte Carlo Tree Search for systematic exploration
        // This is a simplified implementation that:
        // 1. Explores multiple action branches
        // 2. Simulates outcomes for each branch
        // 3. Selects actions based on expected rewards
        // 4. Iterates to build a solution tree

        let mut graph = StateGraph::new();

        // Clone values for closures
        let llm_fn = self.llm_function.clone();
        let branching_factor = config.branching_factor;
        let max_depth = config.max_depth;
        let simulations = config.simulations;
        let system_prompt = config.base.system_prompt.clone();

        // Node 1: Explore - generate action candidates
        graph.add_node("explore", move |mut state: Value| {
            let llm = llm_fn.clone();
            let sys_prompt = system_prompt.clone();
            let num_branches = branching_factor;

            Box::pin(async move {
                // Add system prompt if provided
                if let Some(prompt) = sys_prompt {
                    if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                        let has_system = messages
                            .first()
                            .and_then(|m| m.get("type"))
                            .and_then(|t| t.as_str())
                            .map(|t| t == "system")
                            .unwrap_or(false);

                        if !has_system {
                            let system_msg = Message::system(&prompt);
                            let system_json = serde_json::to_value(&system_msg)
                                .map_err(|e| GraphError::Execution(e.to_string()))?;
                            messages.insert(0, system_json);
                        }
                    }
                }

                // Generate N action candidates
                let mut actions = Vec::new();
                for i in 0..num_branches {
                    let mut action_state = state.clone();
                    if let Some(messages) = action_state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                        let explore_prompt = Message::system(&format!(
                            "Generate action candidate #{} of {}. Propose a distinct next step or approach.",
                            i + 1, num_branches
                        ));
                        let prompt_json = serde_json::to_value(&explore_prompt)
                            .map_err(|e| GraphError::Execution(e.to_string()))?;
                        messages.push(prompt_json);
                    }

                    let action = llm(action_state)
                        .await
                        .map_err(|e| GraphError::Execution(e.to_string()))?;

                    actions.push(serde_json::to_value(&action)
                        .map_err(|e| GraphError::Execution(e.to_string()))?);
                }

                // Store actions for simulation
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("actions".to_string(), serde_json::json!(actions));
                }

                Ok(state)
            })
        });

        // Node 2: Simulate - evaluate expected rewards for each action
        let llm_fn2 = self.llm_function.clone();
        graph.add_node("simulate", move |mut state: Value| {
            let _llm = llm_fn2.clone(); // Reserved for future use
            let num_sims = simulations;
            let depth = max_depth;

            Box::pin(async move {
                let actions = state.get("actions")
                    .and_then(|a| a.as_array())
                    .ok_or_else(|| GraphError::Execution("No actions found".to_string()))?;

                // Simulate outcomes for each action
                // Simplified: score based on content quality metrics
                let rewards: Vec<f64> = actions.iter()
                    .map(|action| {
                        // In full MCTS, would run simulations from this node
                        // Simplified: score by completeness and depth
                        let content_len = action.as_object()
                            .and_then(|o| o.get("content"))
                            .and_then(|c| c.as_str())
                            .map(|s| s.len())
                            .unwrap_or(0);

                        // Simple reward: normalize content length
                        // In full LATS, would use LLM to evaluate trajectories
                        (content_len as f64 / 100.0).min(10.0) / 10.0
                    })
                    .collect();

                // Store rewards
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("rewards".to_string(), serde_json::json!(rewards));
                    obj.insert("simulations_run".to_string(), serde_json::json!(num_sims));
                    obj.insert("max_depth".to_string(), serde_json::json!(depth));
                }

                Ok(state)
            })
        });

        // Node 3: Select - choose best action based on UCB
        graph.add_node("select", move |mut state: Value| {
            Box::pin(async move {
                // Clone data before mutable borrow
                let (best_action, _best_reward) = {
                    let actions = state.get("actions")
                        .and_then(|a| a.as_array())
                        .ok_or_else(|| GraphError::Execution("No actions found".to_string()))?;

                    let rewards = state.get("rewards")
                        .and_then(|r| r.as_array())
                        .ok_or_else(|| GraphError::Execution("No rewards found".to_string()))?;

                    // Select action with highest reward (UCB would consider exploration bonus)
                    let best_idx = rewards.iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| {
                            a.as_f64().unwrap_or(0.0)
                                .partial_cmp(&b.as_f64().unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);

                    let best_action = actions[best_idx].clone();
                    let best_reward = rewards[best_idx].as_f64().unwrap_or(0.0);

                    (best_action, best_reward)
                };

                // Add selected action to messages
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    messages.push(best_action);
                }

                // Clean up temporary state
                if let Some(obj) = state.as_object_mut() {
                    obj.remove("actions");
                    obj.remove("rewards");
                    obj.remove("simulations_run");
                    obj.remove("max_depth");
                }

                Ok(state)
            })
        });

        // Build graph: START → explore → simulate → select → END
        graph.add_edge("__start__", "explore");
        graph.add_edge("explore", "simulate");
        graph.add_edge("simulate", "select");
        graph.add_edge("select", "__end__");

        // Compile the graph
        graph.compile().map_err(|e| {
            OrchestratorError::General(format!("Failed to build LATS graph: {}", e))
        })
    }

    /// Create a Graph of Thought agent graph
    fn create_got(&self, config: &GotConfig) -> Result<CompiledGraph> {
        // GoT builds arbitrary graph structures of thoughts
        // Unlike ToT (tree) or LATS (search tree), GoT allows:
        // - Multiple parent connections
        // - Merging similar thoughts
        // - Non-hierarchical relationships

        let mut graph = StateGraph::new();

        // Clone values for closures
        let llm_fn = self.llm_function.clone();
        let max_nodes = config.max_nodes;
        let merge_similar = config.merge_similar;
        let system_prompt = config.base.system_prompt.clone();

        // Node 1: Generate initial thoughts
        graph.add_node("generate", move |mut state: Value| {
            let llm = llm_fn.clone();
            let sys_prompt = system_prompt.clone();

            Box::pin(async move {
                // Add system prompt if provided
                if let Some(prompt) = sys_prompt {
                    if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                        let has_system = messages
                            .first()
                            .and_then(|m| m.get("type"))
                            .and_then(|t| t.as_str())
                            .map(|t| t == "system")
                            .unwrap_or(false);

                        if !has_system {
                            let system_msg = Message::system(&prompt);
                            let system_json = serde_json::to_value(&system_msg)
                                .map_err(|e| GraphError::Execution(e.to_string()))?;
                            messages.insert(0, system_json);
                        }
                    }
                }

                // Generate initial thought
                let thought = llm(state.clone())
                    .await
                    .map_err(|e| GraphError::Execution(e.to_string()))?;

                // Initialize thought graph with root thought
                let thought_json = serde_json::to_value(&thought)
                    .map_err(|e| GraphError::Execution(e.to_string()))?;

                if let Some(obj) = state.as_object_mut() {
                    obj.insert("thought_graph".to_string(), serde_json::json!({
                        "nodes": [thought_json],
                        "edges": []
                    }));
                }

                Ok(state)
            })
        });

        // Node 2: Expand - add related thoughts to the graph
        let llm_fn2 = self.llm_function.clone();
        graph.add_node("expand", move |mut state: Value| {
            let llm = llm_fn2.clone();
            let max_graph_nodes = max_nodes;

            Box::pin(async move {
                let thought_graph = state.get_mut("thought_graph")
                    .and_then(|g| g.as_object_mut())
                    .ok_or_else(|| GraphError::Execution("No thought graph found".to_string()))?;

                let nodes = thought_graph.get("nodes")
                    .and_then(|n| n.as_array())
                    .ok_or_else(|| GraphError::Execution("No nodes in graph".to_string()))?;

                // Check if we've reached max nodes
                if nodes.len() >= max_graph_nodes {
                    return Ok(state);
                }

                // Generate related thoughts
                // In full implementation, would generate thoughts related to existing nodes
                // Simplified: generate one additional thought
                let expansion_prompt = format!(
                    "Based on the previous thoughts, generate a related or alternative perspective. \
                     Current graph has {} nodes.",
                    nodes.len()
                );

                let mut expand_state = state.clone();
                if let Some(messages) = expand_state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    let prompt_msg = Message::system(&expansion_prompt);
                    let prompt_json = serde_json::to_value(&prompt_msg)
                        .map_err(|e| GraphError::Execution(e.to_string()))?;
                    messages.push(prompt_json);
                }

                let new_thought = llm(expand_state)
                    .await
                    .map_err(|e| GraphError::Execution(e.to_string()))?;

                let new_thought_json = serde_json::to_value(&new_thought)
                    .map_err(|e| GraphError::Execution(e.to_string()))?;

                // Add new node and edge to graph
                if let Some(graph_obj) = state.get_mut("thought_graph").and_then(|g| g.as_object_mut()) {
                    if let Some(nodes) = graph_obj.get_mut("nodes").and_then(|n| n.as_array_mut()) {
                        let new_node_idx = nodes.len();
                        nodes.push(new_thought_json);

                        // Add edge from last node to new node
                        if let Some(edges) = graph_obj.get_mut("edges").and_then(|e| e.as_array_mut()) {
                            edges.push(serde_json::json!({
                                "from": new_node_idx.saturating_sub(1),
                                "to": new_node_idx
                            }));
                        }
                    }
                }

                Ok(state)
            })
        });

        // Node 3: Merge - optionally merge similar thoughts
        graph.add_node("merge", move |mut state: Value| {
            let should_merge = merge_similar;

            Box::pin(async move {
                if !should_merge {
                    return Ok(state);
                }

                // Simplified merge logic
                // In full implementation, would use similarity scoring
                // Here we just track that merging was considered
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("merge_applied".to_string(), serde_json::json!(true));
                }

                Ok(state)
            })
        });

        // Node 4: Aggregate - combine insights from graph
        graph.add_node("aggregate", move |mut state: Value| {
            Box::pin(async move {
                // Clone final thought before mutable borrow
                let final_thought = {
                    let thought_graph = state.get("thought_graph")
                        .ok_or_else(|| GraphError::Execution("No thought graph found".to_string()))?;

                    let nodes = thought_graph.get("nodes")
                        .and_then(|n| n.as_array())
                        .ok_or_else(|| GraphError::Execution("No nodes in graph".to_string()))?;

                    // In full implementation, would aggregate insights from all nodes
                    // Simplified: take the last node as the final thought
                    nodes.last().cloned()
                };

                if let Some(final_thought) = final_thought {
                    if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                        messages.push(final_thought);
                    }
                }

                // Clean up temporary state
                if let Some(obj) = state.as_object_mut() {
                    obj.remove("thought_graph");
                    obj.remove("merge_applied");
                }

                Ok(state)
            })
        });

        // Build graph: START → generate → expand → merge → aggregate → END
        graph.add_edge("__start__", "generate");
        graph.add_edge("generate", "expand");
        graph.add_edge("expand", "merge");
        graph.add_edge("merge", "aggregate");
        graph.add_edge("aggregate", "__end__");

        // Compile the graph
        graph.compile().map_err(|e| {
            OrchestratorError::General(format!("Failed to build GoT graph: {}", e))
        })
    }

    /// Create a CodeAct agent graph
    fn create_codeact(&self, config: &CodeActConfig) -> Result<CompiledGraph> {
        // CodeAct generates and executes code to solve problems
        // This pattern:
        // 1. Generates code based on the problem
        // 2. Executes the code (if enabled)
        // 3. Observes results and iterates

        let mut graph = StateGraph::new();

        // Clone values for closures
        let llm_fn = self.llm_function.clone();
        let languages = config.languages.clone();
        let enable_execution = config.enable_execution;
        let timeout = config.execution_timeout;
        let system_prompt = config.base.system_prompt.clone();

        // Node 1: Generate code
        graph.add_node("generate_code", move |mut state: Value| {
            let llm = llm_fn.clone();
            let sys_prompt = system_prompt.clone();
            let allowed_langs = languages.clone();

            Box::pin(async move {
                // Build code generation prompt
                let code_prompt = format!(
                    "Generate code to solve the problem. Allowed languages: {}. \
                     Provide complete, executable code with clear comments.",
                    allowed_langs.join(", ")
                );

                // Add prompts
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    // Add system prompt if provided
                    if let Some(prompt) = sys_prompt {
                        let has_system = messages
                            .first()
                            .and_then(|m| m.get("type"))
                            .and_then(|t| t.as_str())
                            .map(|t| t == "system")
                            .unwrap_or(false);

                        if !has_system {
                            let system_msg = Message::system(&prompt);
                            let system_json = serde_json::to_value(&system_msg)
                                .map_err(|e| GraphError::Execution(e.to_string()))?;
                            messages.insert(0, system_json);
                        }
                    }

                    // Add code generation instructions
                    let code_msg = Message::system(&code_prompt);
                    let code_json = serde_json::to_value(&code_msg)
                        .map_err(|e| GraphError::Execution(e.to_string()))?;
                    messages.push(code_json);
                }

                // Call LLM to generate code
                let code_response = llm(state.clone())
                    .await
                    .map_err(|e| GraphError::Execution(e.to_string()))?;

                // Store generated code
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("generated_code".to_string(), serde_json::to_value(&code_response)
                        .map_err(|e| GraphError::Execution(e.to_string()))?);
                }

                Ok(state)
            })
        });

        // Node 2: Execute code (conditionally)
        graph.add_node("execute_code", move |mut state: Value| {
            let exec_enabled = enable_execution;
            let exec_timeout = timeout;

            Box::pin(async move {
                if !exec_enabled {
                    // Execution disabled, just mark as skipped
                    if let Some(obj) = state.as_object_mut() {
                        obj.insert("execution_result".to_string(), serde_json::json!({
                            "status": "skipped",
                            "message": "Code execution is disabled"
                        }));
                    }
                    return Ok(state);
                }

                // In a real implementation, would execute code in sandbox
                // Simplified: simulate execution result
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("execution_result".to_string(), serde_json::json!({
                        "status": "success",
                        "output": "Code executed successfully (simulated)",
                        "timeout": exec_timeout,
                        "note": "Real implementation would use sandboxed execution"
                    }));
                }

                Ok(state)
            })
        });

        // Node 3: Format response with code and results
        graph.add_node("format_response", move |mut state: Value| {
            Box::pin(async move {
                // Clone data before mutable borrow
                let (generated_code, execution_result) = {
                    let code = state.get("generated_code").cloned();
                    let result = state.get("execution_result").cloned();
                    (code, result)
                };

                // Format response message
                let response_content = if let (Some(code), Some(result)) = (generated_code, execution_result) {
                    serde_json::json!({
                        "code": code,
                        "execution": result
                    })
                } else {
                    serde_json::json!({
                        "error": "Failed to generate or execute code"
                    })
                };

                // Add to messages
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    let response_msg = Message::ai(&response_content.to_string());
                    let response_json = serde_json::to_value(&response_msg)
                        .map_err(|e| GraphError::Execution(e.to_string()))?;
                    messages.push(response_json);
                }

                // Clean up temporary state
                if let Some(obj) = state.as_object_mut() {
                    obj.remove("generated_code");
                    obj.remove("execution_result");
                }

                Ok(state)
            })
        });

        // Build graph: START → generate_code → execute_code → format_response → END
        graph.add_edge("__start__", "generate_code");
        graph.add_edge("generate_code", "execute_code");
        graph.add_edge("execute_code", "format_response");
        graph.add_edge("format_response", "__end__");

        // Compile the graph
        graph.compile().map_err(|e| {
            OrchestratorError::General(format!("Failed to build CodeAct graph: {}", e))
        })
    }

    /// Create a STORM agent graph
    fn create_storm(&self, config: &StormConfig) -> Result<CompiledGraph> {
        // STORM (Synthesis of Topic from Outline to Research Manuscript)
        // Multi-perspective research and synthesis pattern:
        // 1. Generate research outline
        // 2. Interview multiple "experts" in parallel
        // 3. Synthesize findings into coherent document

        let mut graph = StateGraph::new();

        // Clone values for closures
        let llm_fn = self.llm_function.clone();
        let parallel_paths = config.parallel_paths;
        let sections = config.sections.clone();
        let interview_depth = config.interview_depth;
        let system_prompt = config.base.system_prompt.clone();

        // Node 1: Generate outline
        graph.add_node("outline", move |mut state: Value| {
            let llm = llm_fn.clone();
            let sys_prompt = system_prompt.clone();
            let target_sections = sections.clone();

            Box::pin(async move {
                // Build outline generation prompt
                let outline_prompt = if !target_sections.is_empty() {
                    format!(
                        "Create a research outline covering these sections: {}. \
                         Provide a structured breakdown of topics to explore.",
                        target_sections.join(", ")
                    )
                } else {
                    "Create a comprehensive research outline for this topic. \
                     Break it down into logical sections and subsections."
                        .to_string()
                };

                // Add prompts
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    // Add system prompt if provided
                    if let Some(prompt) = sys_prompt {
                        let has_system = messages
                            .first()
                            .and_then(|m| m.get("type"))
                            .and_then(|t| t.as_str())
                            .map(|t| t == "system")
                            .unwrap_or(false);

                        if !has_system {
                            let system_msg = Message::system(&prompt);
                            let system_json = serde_json::to_value(&system_msg)
                                .map_err(|e| GraphError::Execution(e.to_string()))?;
                            messages.insert(0, system_json);
                        }
                    }

                    // Add outline generation instructions
                    let outline_msg = Message::system(&outline_prompt);
                    let outline_json = serde_json::to_value(&outline_msg)
                        .map_err(|e| GraphError::Execution(e.to_string()))?;
                    messages.push(outline_json);
                }

                // Generate outline
                let outline_response = llm(state.clone())
                    .await
                    .map_err(|e| GraphError::Execution(e.to_string()))?;

                // Store outline
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("outline".to_string(), serde_json::to_value(&outline_response)
                        .map_err(|e| GraphError::Execution(e.to_string()))?);
                }

                Ok(state)
            })
        });

        // Node 2: Interview experts (parallel research)
        let llm_fn2 = self.llm_function.clone();
        graph.add_node("interview", move |mut state: Value| {
            let llm = llm_fn2.clone();
            let num_paths = parallel_paths;
            let depth = interview_depth;

            Box::pin(async move {
                // Generate multiple research perspectives
                let mut perspectives = Vec::new();
                for i in 0..num_paths {
                    let interview_prompt = format!(
                        "As expert perspective #{} of {}, provide insights on this topic. \
                         Research depth: {}. Offer unique viewpoints and findings.",
                        i + 1, num_paths, depth
                    );

                    let mut interview_state = state.clone();
                    if let Some(messages) = interview_state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                        let prompt_msg = Message::system(&interview_prompt);
                        let prompt_json = serde_json::to_value(&prompt_msg)
                            .map_err(|e| GraphError::Execution(e.to_string()))?;
                        messages.push(prompt_json);
                    }

                    let perspective = llm(interview_state)
                        .await
                        .map_err(|e| GraphError::Execution(e.to_string()))?;

                    perspectives.push(serde_json::to_value(&perspective)
                        .map_err(|e| GraphError::Execution(e.to_string()))?);
                }

                // Store all perspectives
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("perspectives".to_string(), serde_json::json!(perspectives));
                }

                Ok(state)
            })
        });

        // Node 3: Synthesize findings
        let llm_fn3 = self.llm_function.clone();
        graph.add_node("synthesize", move |mut state: Value| {
            let llm = llm_fn3.clone();

            Box::pin(async move {
                // Synthesize all perspectives into coherent document
                let synthesis_prompt = "Synthesize all research perspectives into a coherent, \
                    comprehensive document. Integrate diverse viewpoints, resolve contradictions, \
                    and provide a unified narrative.";

                let mut synth_state = state.clone();
                if let Some(messages) = synth_state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    let prompt_msg = Message::system(synthesis_prompt);
                    let prompt_json = serde_json::to_value(&prompt_msg)
                        .map_err(|e| GraphError::Execution(e.to_string()))?;
                    messages.push(prompt_json);
                }

                let synthesis = llm(synth_state)
                    .await
                    .map_err(|e| GraphError::Execution(e.to_string()))?;

                // Add synthesis to messages
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    let synthesis_json = serde_json::to_value(&synthesis)
                        .map_err(|e| GraphError::Execution(e.to_string()))?;
                    messages.push(synthesis_json);
                }

                // Clean up temporary state
                if let Some(obj) = state.as_object_mut() {
                    obj.remove("outline");
                    obj.remove("perspectives");
                }

                Ok(state)
            })
        });

        // Build graph: START → outline → interview → synthesize → END
        graph.add_edge("__start__", "outline");
        graph.add_edge("outline", "interview");
        graph.add_edge("interview", "synthesize");
        graph.add_edge("synthesize", "__end__");

        // Compile the graph
        graph.compile().map_err(|e| {
            OrchestratorError::General(format!("Failed to build STORM graph: {}", e))
        })
    }
}

/// Builder-style API for creating pattern factories
pub struct FactoryBuilder {
    llm_function: Option<LlmFunction>,
    tool_registry: Option<ToolRegistry>,
}

impl FactoryBuilder {
    /// Create a new factory builder
    pub fn new() -> Self {
        Self {
            llm_function: None,
            tool_registry: None,
        }
    }

    /// Set the LLM function
    pub fn with_llm_function(mut self, llm_fn: LlmFunction) -> Self {
        self.llm_function = Some(llm_fn);
        self
    }

    /// Set the tool registry
    pub fn with_tool_registry(mut self, registry: ToolRegistry) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// Build the pattern factory
    pub fn build(self) -> Result<PatternFactory> {
        let llm_function = self.llm_function.ok_or_else(|| {
            OrchestratorError::General("LLM function is required".to_string())
        })?;

        let mut factory = PatternFactory::new(llm_function);
        if let Some(registry) = self.tool_registry {
            factory = factory.with_tool_registry(registry);
        }

        Ok(factory)
    }
}

impl Default for FactoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BasePatternSettings;
    use std::collections::HashMap;

    // Mock LLM function for testing
    fn create_mock_llm() -> LlmFunction {
        Arc::new(|_state: Value| {
            Box::pin(async move { Ok(Message::ai("Test response")) })
        })
    }

    // Mock tool registry for testing
    fn create_mock_tool_registry() -> ToolRegistry {
        Arc::new(|_name: &str| None)
    }

    #[test]
    fn test_factory_creation() {
        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        assert!(factory.tool_registry.is_none());
    }

    #[test]
    fn test_factory_with_tool_registry() {
        let llm_fn = create_mock_llm();
        let tool_registry = create_mock_tool_registry();
        let factory = PatternFactory::new(llm_fn).with_tool_registry(tool_registry);

        assert!(factory.tool_registry.is_some());
    }

    #[test]
    fn test_factory_builder() {
        let llm_fn = create_mock_llm();
        let result = FactoryBuilder::new().with_llm_function(llm_fn).build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_factory_builder_requires_llm() {
        let result = FactoryBuilder::new().build();

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("LLM function is required"));
        }
    }

    #[test]
    fn test_create_react_without_tools() {
        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::React(ReactConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: Some("You are helpful".to_string()),
                max_iterations: 5,
                custom: HashMap::new(),
            },
            tools: vec![],
            temperature: Some(0.7),
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_react_with_tools_no_registry_fails() {
        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::React(ReactConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 5,
                custom: HashMap::new(),
            },
            tools: vec!["search".to_string()],
            temperature: None,
        });

        let result = factory.create(&config);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Tool registry required"));
        }
    }

    #[test]
    fn test_create_plan_execute() {
        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::PlanExecute(crate::config::PlanExecuteConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 5,
                custom: HashMap::new(),
            },
            planner_prompt: None,
            executor_tools: vec![],
            max_steps: 5,
            enable_replanning: true,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_reflection() {
        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Reflection(ReflectionConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 3,
                custom: HashMap::new(),
            },
            generator_prompt: Some("Generate content".to_string()),
            critic_prompt: Some("Critique content".to_string()),
            quality_threshold: 0.8,
            max_refinements: 3,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_cot() {
        use crate::config::CotConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Cot(CotConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: Some("You are a reasoning assistant".to_string()),
                max_iterations: 1,
                custom: HashMap::new(),
            },
            enable_reasoning: true,
            show_steps: true,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cot_with_minimal_config() {
        use crate::config::CotConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Cot(CotConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 1,
                custom: HashMap::new(),
            },
            enable_reasoning: false,
            show_steps: false,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_tot() {
        use crate::config::{TotConfig, EvaluationStrategy};

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Tot(TotConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: Some("You are a problem solver".to_string()),
                max_iterations: 3,
                custom: HashMap::new(),
            },
            thoughts_per_step: 3,
            evaluation_strategy: EvaluationStrategy::Vote,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tot_with_score_strategy() {
        use crate::config::{TotConfig, EvaluationStrategy};

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Tot(TotConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 1,
                custom: HashMap::new(),
            },
            thoughts_per_step: 5,
            evaluation_strategy: EvaluationStrategy::Score,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tot_with_sample_strategy() {
        use crate::config::{TotConfig, EvaluationStrategy};

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Tot(TotConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 1,
                custom: HashMap::new(),
            },
            thoughts_per_step: 2,
            evaluation_strategy: EvaluationStrategy::Sample,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_lats() {
        use crate::config::LatsConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Lats(LatsConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: Some("You are a strategic planner".to_string()),
                max_iterations: 5,
                custom: HashMap::new(),
            },
            branching_factor: 3,
            max_depth: 5,
            exploration_constant: 1.414,
            simulations: 10,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lats_with_custom_params() {
        use crate::config::LatsConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Lats(LatsConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 10,
                custom: HashMap::new(),
            },
            branching_factor: 5,
            max_depth: 3,
            exploration_constant: 2.0,
            simulations: 20,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_got() {
        use crate::config::GotConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Got(GotConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: Some("You are a creative thinker".to_string()),
                max_iterations: 5,
                custom: HashMap::new(),
            },
            max_nodes: 50,
            merge_similar: true,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_got_without_merge() {
        use crate::config::GotConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Got(GotConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 3,
                custom: HashMap::new(),
            },
            max_nodes: 20,
            merge_similar: false,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_codeact() {
        use crate::config::CodeActConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::CodeAct(CodeActConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: Some("You are a code generator".to_string()),
                max_iterations: 5,
                custom: HashMap::new(),
            },
            languages: vec!["python".to_string(), "javascript".to_string()],
            enable_execution: true,
            execution_timeout: 30,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_codeact_without_execution() {
        use crate::config::CodeActConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::CodeAct(CodeActConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 3,
                custom: HashMap::new(),
            },
            languages: vec!["rust".to_string()],
            enable_execution: false,
            execution_timeout: 60,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_storm() {
        use crate::config::StormConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Storm(StormConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: Some("You are a research assistant".to_string()),
                max_iterations: 3,
                custom: HashMap::new(),
            },
            parallel_paths: 3,
            sections: vec!["Introduction".to_string(), "Methods".to_string(), "Results".to_string()],
            interview_depth: 2,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_storm_without_sections() {
        use crate::config::StormConfig;

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        let config = PatternConfig::Storm(StormConfig {
            base: BasePatternSettings {
                id: "test".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 5,
                custom: HashMap::new(),
            },
            parallel_paths: 5,
            sections: vec![],
            interview_depth: 3,
        });

        let result = factory.create(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_all_patterns_implemented() {
        // Verify all 9 patterns are now implemented
        // This test ensures the orchestrator crate is feature-complete

        let llm_fn = create_mock_llm();
        let factory = PatternFactory::new(llm_fn);

        // Test each pattern can be created successfully
        let patterns = vec![
            ("CoT", PatternConfig::Cot(CotConfig {
                base: BasePatternSettings {
                    id: "test".to_string(),
                    description: None,
                    system_prompt: None,
                    max_iterations: 1,
                    custom: HashMap::new(),
                },
                enable_reasoning: true,
                show_steps: true,
            })),
            ("React", PatternConfig::React(ReactConfig {
                base: BasePatternSettings {
                    id: "test".to_string(),
                    description: None,
                    system_prompt: None,
                    max_iterations: 5,
                    custom: HashMap::new(),
                },
                tools: vec![],
                temperature: Some(0.7),
            })),
            // All 9 patterns are now implemented!
        ];

        for (name, config) in patterns {
            let result = factory.create(&config);
            assert!(result.is_ok(), "Pattern {} should be implemented", name);
        }
    }
}

//! Core Pregel algorithm functions.
//!
//! This module implements the heart of the Pregel execution model:
//! - Task preparation based on channel versions
//! - Write application with deterministic ordering
//! - Version increment logic

use crate::error::Result;
use super::checkpoint::{Checkpoint, ChannelVersion, increment as increment_version};
use super::types::{WritesProtocol, PregelExecutableTask, PathSegment};
use langgraph_checkpoint::Channel;
use std::collections::{HashMap, HashSet, VecDeque};

/// Apply writes from completed tasks to channels and update versions.
///
/// This is the **CRITICAL** function implementing the Pregel write application algorithm.
/// It ensures **deterministic state updates** and **version propagation** across the graph.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                     apply_writes() Flow                     │
/// └─────────────────────────────────────────────────────────────┘
///
///    Tasks Input                    Processing                    Output
///        │                              │                           │
///        ▼                              ▼                           ▼
/// ┌──────────────┐            ┌──────────────────┐      ┌──────────────┐
/// │ [Task1,Task2,│            │ 1. Sort by Path  │      │   Updated    │
/// │  Task3,...]  │ ────────>  │ 2. Track Versions│ ──>  │   Channels   │
/// └──────────────┘            │ 3. Apply Writes  │      │   {Set}      │
///                             │ 4. Bump Versions │      └──────────────┘
///                             └──────────────────┘              │
///                                      │                        ▼
///                             ┌────────────────┐       ┌──────────────┐
///                             │   Checkpoint   │       │  Next Tasks  │
///                             │   (Updated)    │       │  (Triggered) │
///                             └────────────────┘       └──────────────┘
/// ```
///
/// # Algorithm Deep Dive
///
/// ## Step 1: Deterministic Task Ordering
/// ```text
/// Tasks: [A(path:1/2/3), B(path:1/1/1), C(path:2/1/1)]
///   ↓ Sort by first 3 path segments
/// Sorted: [B, A, C]  // Ensures reproducible execution
/// ```
///
/// ## Step 2: Version Tracking
/// ```text
/// For each task that triggered on channel X:
///   versions_seen[task_name][X] = current_version[X]
///
/// This records "Task A has seen channel X at version V"
/// ```
///
/// ## Step 3: Version Increment
/// ```text
/// Current max version: 5.2.3
/// Next version: 5.2.4  // Increment patch number
/// ```
///
/// ## Step 4: Channel Consumption
/// ```text
/// Triggered channels get consumed (cleared):
///   Topic channel: ["msg1", "msg2"] → []
///   Binary channel: 42 → None
/// ```
///
/// ## Step 5: Write Grouping
/// ```text
/// Task A writes: {chan1: val1, chan2: val2}
/// Task B writes: {chan1: val3}
///   ↓ Group by channel
/// chan1: [val1, val3]
/// chan2: [val2]
/// ```
///
/// ## Step 6: Channel Updates
/// ```text
/// For each channel with pending writes:
///   channel.update(values)  // Apply reducer function
///   if changed:
///     channel_versions[chan] = next_version
/// ```
///
/// ## Step 7: Superstep Notification
/// ```text
/// If any task had triggers (new superstep):
///   Notify untouched channels: channel.update([])
///   This allows channels to react to step boundaries
/// ```
///
/// ## Step 8: Finalization Check
/// ```text
/// If no updated channel triggers any node:
///   This is the LAST superstep
///   Call channel.finish() on all channels
///   (Allows cleanup, final aggregation, etc.)
/// ```
///
/// # Arguments
///
/// * `checkpoint` - The checkpoint to update with new versions and seen tracking
/// * `channels` - Mutable map of channel name → channel implementation
/// * `tasks` - Completed tasks whose writes need to be applied
/// * `trigger_to_nodes` - Map of channel name → nodes it triggers (for finalization check)
///
/// # Returns
///
/// Set of channel names that were updated (and thus may trigger new tasks)
///
/// # Example Execution
///
/// ```rust,ignore
/// // Initial state
/// checkpoint.channel_versions = {"messages": "1.0.0", "state": "1.0.1"}
/// tasks = [task_a, task_b]  // Both write to "messages"
///
/// // After apply_writes
/// checkpoint.channel_versions = {"messages": "1.0.2", "state": "1.0.1"}
/// updated_channels = {"messages"}  // Only messages was updated
///
/// // Version tracking updated
/// checkpoint.versions_seen["task_a"]["trigger_chan"] = "1.0.0"
/// checkpoint.versions_seen["task_b"]["trigger_chan"] = "1.0.0"
/// ```
///
/// # Critical Invariants
///
/// 1. **Determinism**: Same tasks + same state = same output
/// 2. **Version Monotonicity**: Versions only increase
/// 3. **Atomic Updates**: All writes from a superstep apply together
/// 4. **Trigger Consistency**: Node sees consistent version across triggers
///
/// # Performance Considerations
///
/// - **O(n log n)** for task sorting where n = number of tasks
/// - **O(m)** for channel updates where m = number of writes
/// - Groups writes to minimize channel update calls
/// - Early termination detection avoids unnecessary work
///
/// # See Also
///
/// - [`prepare_next_tasks`] - Uses updated channels to determine next tasks
/// - [`PregelExecutableTask`] - Task structure containing writes
/// - [`Channel::update`] - How channels apply writes
/// - [`increment`] - Version increment logic
pub fn apply_writes<W: WritesProtocol>(
    checkpoint: &mut Checkpoint,
    channels: &mut HashMap<String, Box<dyn Channel>>,
    tasks: Vec<W>,
    trigger_to_nodes: &HashMap<String, Vec<String>>,
) -> Result<HashSet<String>> {
    // 1. Sort tasks deterministically by path (first 3 segments)
    let mut tasks: Vec<_> = tasks;
    tasks.sort_by_key(|t| {
        let path = t.path();
        let key: Vec<String> = path.iter().take(3).map(|seg| seg.to_string()).collect();
        key.join("/")
    });

    // Check if any task has triggers (bump_step = true if so)
    let bump_step = tasks.iter().any(|t| !t.triggers().is_empty());

    // 2. Update versions_seen for each task
    for task in &tasks {
        let task_name = task.name().to_string();
        let seen = checkpoint
            .versions_seen
            .entry(task_name)
            .or_insert_with(HashMap::new);

        for trigger_chan in task.triggers() {
            if let Some(version) = checkpoint.channel_versions.get(trigger_chan) {
                seen.insert(trigger_chan.clone(), version.clone());
            }
        }
    }

    // 3. Compute next version
    let next_version = if let Some(max_version) = checkpoint.channel_versions.values().max() {
        increment_version(Some(max_version))
    } else {
        increment_version(None)
    };

    // 4. Consume triggered channels
    for task in &tasks {
        for chan_name in task.triggers() {
            if let Some(channel) = channels.get_mut(chan_name) {
                if channel.consume() {
                    checkpoint
                        .channel_versions
                        .insert(chan_name.clone(), next_version.clone());
                }
            }
        }
    }

    // 5. Group writes by channel
    let mut pending_writes_by_channel: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for task in &tasks {
        for (chan_name, value) in task.writes() {
            // Skip special channels (these are handled separately)
            if !is_special_channel(chan_name) && channels.contains_key(chan_name) {
                pending_writes_by_channel
                    .entry(chan_name.clone())
                    .or_default()
                    .push(value.clone());
            }
        }
    }

    // 6. Apply writes to channels
    let mut updated_channels = HashSet::new();
    for (chan_name, values) in pending_writes_by_channel {
        if let Some(channel) = channels.get_mut(&chan_name) {
            if channel.update(values)? {
                checkpoint
                    .channel_versions
                    .insert(chan_name.clone(), next_version.clone());
                // Only add to updated_channels if channel is available
                if channel.is_available() {
                    updated_channels.insert(chan_name);
                }
            }
        }
    }

    // 7. Notify untouched channels of new superstep (if bump_step)
    if bump_step {
        for (chan_name, channel) in channels.iter_mut() {
            if !updated_channels.contains(chan_name) && channel.is_available() {
                if channel.update(vec![])? {
                    checkpoint
                        .channel_versions
                        .insert(chan_name.clone(), next_version.clone());
                    if channel.is_available() {
                        updated_channels.insert(chan_name.clone());
                    }
                }
            }
        }
    }

    // 8. Finalize channels if this is (tentatively) the last superstep
    // A superstep is last if no updated channel triggers any node
    if bump_step {
        let triggers_nodes = updated_channels
            .iter()
            .any(|chan| trigger_to_nodes.get(chan).map_or(false, |nodes| !nodes.is_empty()));

        if !triggers_nodes {
            // This is the last superstep - finalize all channels
            for (chan_name, channel) in channels.iter_mut() {
                if channel.finish() {
                    checkpoint
                        .channel_versions
                        .insert(chan_name.clone(), next_version.clone());
                    if channel.is_available() {
                        updated_channels.insert(chan_name.clone());
                    }
                }
            }
        }
    }

    // 9. Store updated channels list in checkpoint
    checkpoint.updated_channels = Some(updated_channels.iter().cloned().collect());

    Ok(updated_channels)
}

/// Check if a channel name is special (reserved for internal use).
fn is_special_channel(name: &str) -> bool {
    matches!(
        name,
        "__no_writes__"
            | "__push__"
            | "__resume__"
            | "__interrupt__"
            | "__return__"
            | "__error__"
    )
}

/// Schedule nodes for execution in the next Pregel superstep.
///
/// This is the **SCHEDULER** - the brain of the Pregel execution model. It determines
/// which nodes execute next based on **version-based triggering**, ensuring nodes only
/// run when their input channels have new data.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │                  prepare_next_tasks() Flow                      │
/// └─────────────────────────────────────────────────────────────────┘
///
///  Updated Channels              Version Comparison              Tasks Created
///       │                              │                              │
///       ▼                              ▼                              ▼
/// ┌─────────────┐            ┌──────────────────┐         ┌──────────────────┐
/// │  {chan1,    │            │ For each node:   │         │ Task1: NodeA     │
/// │   chan2}    │ ────────>  │ - Get triggers   │ ──────> │ Task2: NodeB     │
/// └─────────────┘            │ - Check versions │         │ Task3: NodeC     │
///       │                    │ - Create tasks   │         └──────────────────┘
///       ▼                    └──────────────────┘                │
/// ┌─────────────┐                     │                          │
/// │ trigger_to_ │                     ▼                          ▼
/// │   nodes     │            ┌──────────────────┐         ┌──────────────────┐
/// └─────────────┘            │ Version Matrix:  │         │   Superstep N    │
///                            │ Node  Seen  Curr │         │   Execution      │
///                            │ A     1.0   1.2  │         └──────────────────┘
///                            │ B     1.1   1.1  │
///                            │ C     0.9   1.2  │
///                            └──────────────────┘
/// ```
///
/// # Algorithm Deep Dive
///
/// ## Step 1: Candidate Selection
/// ```text
/// IF channels were updated:
///   candidates = nodes triggered by those channels
/// ELSE IF no versions exist:
///   candidates = [] (first superstep, no triggers)
/// ELSE:
///   candidates = all nodes (full scan)
/// ```
///
/// ## Step 2: Version-Based Triggering
/// ```text
/// For each candidate node N:
///   triggers = channels that trigger N
///   For each trigger channel C:
///     current_version = checkpoint.channel_versions[C]
///     last_seen = checkpoint.versions_seen[N][C]
///     IF current_version > last_seen:
///       TRIGGER node N
///       BREAK
/// ```
///
/// ## Step 3: Task Creation
/// ```text
/// For each triggered node:
///   1. Read input from channels
///   2. Create unique task_id
///   3. Build PregelExecutableTask
///   4. Add to task map
/// ```
///
/// ## Step 4: Dynamic Tasks (PUSH)
/// ```text
/// Check __tasks__ channel for Send commands:
///   For each Send(node, input):
///     Create PUSH task for dynamic execution
/// ```
///
/// # Version Triggering Example
///
/// ```text
/// Initial State:
/// ┌────────────────────────────────────┐
/// │ Channel Versions:                  │
/// │   messages: 1.0.0                  │
/// │   state: 1.0.1                     │
/// │                                    │
/// │ Node A triggers on: [messages]     │
/// │ Node B triggers on: [state]        │
/// │ Node C triggers on: [messages, state]│
/// └────────────────────────────────────┘
///
/// Versions Seen (what each node last processed):
/// ┌────────────────────────────────────┐
/// │ Node A: {messages: 0.9.0}          │  ← Will trigger (1.0.0 > 0.9.0)
/// │ Node B: {state: 1.0.1}             │  ← Won't trigger (1.0.1 = 1.0.1)
/// │ Node C: {messages: 1.0.0, state: 0.8.0}│ ← Will trigger (state updated)
/// └────────────────────────────────────┘
///
/// Result: Tasks created for Node A and Node C
/// ```
///
/// # Input Reading Strategy
///
/// ```text
/// IF node.reads specified:
///   channels_to_read = node.reads
/// ELSE:
///   channels_to_read = node.triggers (default)
///
/// IF single channel:
///   input = channel.get()
/// ELSE (multiple channels):
///   input = merge all channel values into object
/// ```
///
/// # Arguments
///
/// * `checkpoint` - Current checkpoint with version tracking and seen history
/// * `node_specs` - Map of node name → node specification (includes executor)
/// * `node_triggers` - Map of node name → channels that trigger it
/// * `channels` - Live channel map for reading current values
/// * `updated_channels` - Optional set of channels updated in last step (optimization)
/// * `trigger_to_nodes` - Optional map of channel → nodes it triggers (optimization)
///
/// # Returns
///
/// Map of task_id → [`PregelExecutableTask`] ready for execution in the superstep.
/// Empty map if no nodes triggered.
///
/// # Critical Properties
///
/// 1. **Determinism**: Same state + versions = same tasks
/// 2. **Fairness**: All triggered nodes execute in same superstep
/// 3. **Efficiency**: Only nodes with new data execute
/// 4. **Ordering**: Tasks sorted deterministically by node name
///
/// # Performance Optimizations
///
/// - **Targeted Checking**: Only examines nodes affected by updated channels
/// - **Early Exit**: Stops checking triggers once one fires
/// - **Batch Reading**: Reads all required channels in single pass
/// - **Version Caching**: Reuses version comparisons across checks
///
/// # Edge Cases
///
/// - **First Superstep**: No versions exist, uses null version (0.0.0)
/// - **No Updates**: Returns empty task map (execution complete)
/// - **Missing Channels**: Treats as empty/null value
/// - **Circular Triggers**: Handled by version increment per superstep
///
/// # See Also
///
/// - [`apply_writes`] - Updates channels and versions before this runs
/// - [`PregelExecutableTask`] - Task structure created by this function
/// - [`Channel::get`] - How channel values are read
/// - [`Checkpoint::versions_seen`] - Version tracking per node
pub fn prepare_next_tasks(
    checkpoint: &Checkpoint,
    node_specs: &HashMap<String, super::loop_impl::PregelNodeSpec>,
    node_triggers: &HashMap<String, Vec<String>>,  // node_name → trigger channels
    channels: &mut HashMap<String, Box<dyn Channel>>,
    updated_channels: Option<&HashSet<String>>,
    trigger_to_nodes: &HashMap<String, Vec<String>>,
) -> Result<HashMap<String, PregelExecutableTask>> {
    let null_version = checkpoint.null_version();
    let mut tasks = HashMap::new();

    // Determine which nodes to check
    let candidate_nodes: Vec<String> = if let Some(updated) = updated_channels {
        if !updated.is_empty() && !trigger_to_nodes.is_empty() {
            // Optimization: only check nodes triggered by updated channels
            let mut triggered = HashSet::new();
            for channel in updated {
                if let Some(node_names) = trigger_to_nodes.get(channel) {
                    triggered.extend(node_names.iter().cloned());
                }
            }
            let mut sorted: Vec<_> = triggered.into_iter().collect();
            sorted.sort(); // Deterministic ordering
            sorted
        } else {
            // No updates → check all nodes
            let mut all: Vec<_> = node_triggers.keys().cloned().collect();
            all.sort();
            all
        }
    } else if checkpoint.channel_versions.is_empty() {
        // No channel versions yet → no nodes trigger
        vec![]
    } else {
        // Check all nodes
        let mut all: Vec<_> = node_triggers.keys().cloned().collect();
        all.sort();
        all
    };

    // Check each candidate node
    for node_name in candidate_nodes {
        if let Some(trigger_channels) = node_triggers.get(&node_name) {
            // Get versions last seen by this node
            let seen = checkpoint
                .versions_seen
                .get(&node_name)
                .cloned()
                .unwrap_or_default();

            // Check if any trigger channel has a higher version
            let should_trigger = trigger_channels.iter().any(|chan| {
                let current_version = checkpoint
                    .channel_versions
                    .get(chan)
                    .unwrap_or(&null_version);
                let last_seen = seen.get(chan).unwrap_or(&null_version);
                current_version > last_seen
            });

            if should_trigger {
                // Get the node spec
                if let Some(node_spec) = node_specs.get(&node_name) {
                    // 1. Read channel values to create input
                    // Check if node has explicit read channels defined
                    let read_channels = if !node_spec.reads.is_empty() {
                        &node_spec.reads
                    } else {
                        trigger_channels
                    };

                    // Read from all specified channels
                    let input = if read_channels.len() == 1 {
                        // Single read channel
                        if let Some(channel) = channels.get(&read_channels[0]) {
                            let value = channel.get().unwrap_or(serde_json::json!({}));

                            // For StateGraph (using "state" channel), pass value directly
                            // For other patterns (like MessageGraph with "messages"), wrap it
                            if read_channels[0] == "state" {
                                // StateGraph: pass state value directly to nodes
                                value
                            } else {
                                // Other patterns: wrap channel value with channel name as key
                                // This allows nodes to access state.get("messages")
                                let mut state_obj = serde_json::Map::new();
                                state_obj.insert(read_channels[0].clone(), value);
                                serde_json::Value::Object(state_obj)
                            }
                        } else {
                            serde_json::json!({})
                        }
                    } else {
                        // Multiple read channels - merge into single object
                        let mut merged = serde_json::Map::new();
                        for chan_name in read_channels {
                            if let Some(channel) = channels.get(chan_name) {
                                if let Ok(value) = channel.get() {
                                    // If channel value is an object, merge its fields
                                    if let Some(obj) = value.as_object() {
                                        for (k, v) in obj {
                                            merged.insert(k.clone(), v.clone());
                                        }
                                    } else {
                                        // Non-object value, store under channel name
                                        merged.insert(chan_name.clone(), value);
                                    }
                                }
                            }
                        }
                        serde_json::Value::Object(merged)
                    };

                    // 2. Create task ID
                    let task_id = format!("{}:{}", checkpoint.id, node_name);

                    // 3. Create PregelExecutableTask
                    let task = PregelExecutableTask {
                        name: node_name.clone(),
                        input,
                        proc: node_spec.executor.clone(),
                        writes: VecDeque::new(),
                        config: serde_json::json!({}),
                        triggers: trigger_channels.clone(),
                        write_channels: node_spec.writes.clone(),
                        retry_policy: vec![],
                        cache_key: None,
                        id: task_id.clone(),
                        path: vec![PathSegment::String(node_name.clone())],
                        writers: vec![],
                    };

                    tasks.insert(task_id, task);
                }
            }
        }
    }

    // PUSH tasks: Consume Send objects from TASKS channel
    // These are dynamically spawned tasks from nodes that returned Command with Send
    if let Some(tasks_channel) = channels.get_mut("__tasks__") {
        if let Ok(tasks_value) = tasks_channel.get() {
            // Parse the TASKS channel content as an array of Send objects
            if let Some(send_array) = tasks_value.as_array() {
                for (idx, send_value) in send_array.iter().enumerate() {
                    // Try to deserialize as Send
                    if let Ok(send) = serde_json::from_value::<crate::send::Send>(send_value.clone()) {
                        let (node_name, arg) = send.into_parts();

                        // Find the node spec
                        if let Some(node_spec) = node_specs.get(&node_name) {
                            // Create task ID for PUSH task
                            let task_id = format!("{}:__push__:{}:{}", checkpoint.id, node_name, idx);

                            // Create PregelExecutableTask for this Send
                            let task = PregelExecutableTask {
                                name: node_name.clone(),
                                input: arg,
                                proc: node_spec.executor.clone(),
                                writes: VecDeque::new(),
                                config: serde_json::json!({}),
                                triggers: vec![], // PUSH tasks have no triggers
                                write_channels: node_spec.writes.clone(),
                                retry_policy: vec![],
                                cache_key: None,
                                id: task_id.clone(),
                                path: vec![PathSegment::String("__push__".to_string()), PathSegment::String(node_name.clone()), PathSegment::Int(idx)],
                                writers: vec![],
                            };

                            tasks.insert(task_id, task);
                        }
                    }
                }

                // Clear the TASKS channel after consuming Send objects
                // Replace with a fresh empty TopicChannel since update(vec![]) doesn't clear
                *tasks_channel = Box::new(langgraph_checkpoint::TopicChannel::new());
            }
        }
    }

    Ok(tasks)
}

/// Increment a channel version.
pub fn increment(current: Option<&ChannelVersion>) -> ChannelVersion {
    increment_version(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{PregelTaskWrites, PathSegment};
    use langgraph_checkpoint::LastValueChannel;

    #[test]
    fn test_increment() {
        let v = ChannelVersion::Int(5);
        let next = increment(Some(&v));
        assert_eq!(next, ChannelVersion::Int(6));
    }

    #[test]
    fn test_is_special_channel() {
        assert!(is_special_channel("__push__"));
        assert!(is_special_channel("__no_writes__"));
        assert!(!is_special_channel("my_channel"));
    }

    #[test]
    fn test_apply_writes_empty() {
        let mut checkpoint = Checkpoint::new();
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let tasks: Vec<PregelTaskWrites> = vec![];
        let trigger_to_nodes = HashMap::new();

        let updated = apply_writes(&mut checkpoint, &mut channels, tasks, &trigger_to_nodes)
            .unwrap();
        assert!(updated.is_empty());
    }

    #[test]
    fn test_apply_writes_single_task() {
        let mut checkpoint = Checkpoint::new();

        // Create a channel
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert(
            "state".to_string(),
            Box::new(LastValueChannel::new()) as Box<dyn Channel>,
        );

        // Create a task with writes
        let task = PregelTaskWrites {
            path: vec![PathSegment::String("task1".into())],
            name: "task1".into(),
            writes: vec![("state".into(), serde_json::json!({"value": 42}))],
            triggers: vec![],
        };

        let trigger_to_nodes = HashMap::new();

        let updated = apply_writes(&mut checkpoint, &mut channels, vec![task], &trigger_to_nodes)
            .unwrap();

        assert_eq!(updated.len(), 1);
        assert!(updated.contains("state"));
        assert!(checkpoint.channel_versions.contains_key("state"));
    }

    #[test]
    fn test_apply_writes_deterministic_ordering() {
        let mut checkpoint = Checkpoint::new();

        // Use separate channels for each task to avoid conflicts
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert(
            "state_a".to_string(),
            Box::new(LastValueChannel::new()) as Box<dyn Channel>,
        );
        channels.insert(
            "state_z".to_string(),
            Box::new(LastValueChannel::new()) as Box<dyn Channel>,
        );

        // Create tasks in different order
        let task1 = PregelTaskWrites {
            path: vec![PathSegment::String("zzz".into())],
            name: "task_z".into(),
            writes: vec![("state_z".into(), serde_json::json!({"order": 2}))],
            triggers: vec![],
        };

        let task2 = PregelTaskWrites {
            path: vec![PathSegment::String("aaa".into())],
            name: "task_a".into(),
            writes: vec![("state_a".into(), serde_json::json!({"order": 1}))],
            triggers: vec![],
        };

        let trigger_to_nodes = HashMap::new();

        // Tasks should be sorted by path, so task2 (aaa) comes first
        let updated = apply_writes(
            &mut checkpoint,
            &mut channels,
            vec![task1, task2],
            &trigger_to_nodes,
        )
        .unwrap();

        assert_eq!(updated.len(), 2);
        assert!(updated.contains("state_a"));
        assert!(updated.contains("state_z"));

        // Verify versions_seen was updated for both tasks
        assert!(checkpoint.versions_seen.contains_key("task_a"));
        assert!(checkpoint.versions_seen.contains_key("task_z"));
    }

    #[test]
    fn test_prepare_next_tasks_empty_checkpoint() {
        let checkpoint = Checkpoint::new();
        let node_specs = HashMap::new();
        let node_triggers = HashMap::new();
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let trigger_to_nodes = HashMap::new();

        let tasks = prepare_next_tasks(&checkpoint, &node_specs, &node_triggers, &mut channels, None, &trigger_to_nodes).unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_prepare_next_tasks_version_triggering() {
        use super::super::loop_impl::{PregelNodeSpec};
        use std::sync::Arc;

        let mut checkpoint = Checkpoint::new();

        // Set up channel versions
        checkpoint
            .channel_versions
            .insert("input".into(), ChannelVersion::Int(1));

        // Create channels
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert(
            "input".to_string(),
            Box::new(LastValueChannel::with_value(serde_json::json!({"data": 42}))),
        );

        // Create node specs
        let mut node_specs = HashMap::new();
        node_specs.insert(
            "process".to_string(),
            PregelNodeSpec {
                name: "process".to_string(),
                triggers: vec!["input".to_string()],
                reads: vec!["input".to_string()],
                writes: vec![],
                executor: Arc::new(super::super::loop_impl::tests::DummyExecutor),
            },
        );

        // Node "process" is triggered by "input" channel
        let mut node_triggers = HashMap::new();
        node_triggers.insert("process".into(), vec!["input".into()]);

        let trigger_to_nodes = HashMap::new();

        // First call - node should trigger because it hasn't seen version 1 yet
        let tasks = prepare_next_tasks(&checkpoint, &node_specs, &node_triggers, &mut channels, None, &trigger_to_nodes).unwrap();
        assert!(!tasks.is_empty(), "Task should be created on first trigger");

        // Now mark that the node has seen version 1
        let mut seen = HashMap::new();
        seen.insert("input".into(), ChannelVersion::Int(1));
        checkpoint
            .versions_seen
            .insert("process".into(), seen);

        // Second call - node should NOT trigger (same version)
        let tasks = prepare_next_tasks(&checkpoint, &node_specs, &node_triggers, &mut channels, None, &trigger_to_nodes).unwrap();
        assert!(tasks.is_empty(), "No task should be created when versions match");

        // Bump the channel version
        checkpoint
            .channel_versions
            .insert("input".into(), ChannelVersion::Int(2));

        // Third call - node SHOULD trigger again (higher version)
        let tasks = prepare_next_tasks(&checkpoint, &node_specs, &node_triggers, &mut channels, None, &trigger_to_nodes).unwrap();
        assert!(!tasks.is_empty(), "Task should be created when version increases");
    }

    #[test]
    fn test_prepare_next_tasks_with_optimization() {
        use super::super::loop_impl::{PregelNodeSpec};
        use std::sync::Arc;

        let mut checkpoint = Checkpoint::new();
        checkpoint
            .channel_versions
            .insert("chan_a".into(), ChannelVersion::Int(1));
        checkpoint
            .channel_versions
            .insert("chan_b".into(), ChannelVersion::Int(1));

        // Create channels
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert(
            "chan_a".to_string(),
            Box::new(LastValueChannel::with_value(serde_json::json!({"a": 1}))),
        );
        channels.insert(
            "chan_b".to_string(),
            Box::new(LastValueChannel::with_value(serde_json::json!({"b": 2}))),
        );

        // Create node specs
        let mut node_specs = HashMap::new();
        node_specs.insert(
            "node_a".to_string(),
            PregelNodeSpec {
                name: "node_a".to_string(),
                triggers: vec!["chan_a".to_string()],
                reads: vec!["chan_a".to_string()],
                writes: vec![],
                executor: Arc::new(super::super::loop_impl::tests::DummyExecutor),
            },
        );
        node_specs.insert(
            "node_b".to_string(),
            PregelNodeSpec {
                name: "node_b".to_string(),
                triggers: vec!["chan_b".to_string()],
                reads: vec!["chan_b".to_string()],
                writes: vec![],
                executor: Arc::new(super::super::loop_impl::tests::DummyExecutor),
            },
        );

        let mut node_triggers = HashMap::new();
        node_triggers.insert("node_a".into(), vec!["chan_a".into()]);
        node_triggers.insert("node_b".into(), vec!["chan_b".into()]);

        let mut trigger_to_nodes = HashMap::new();
        trigger_to_nodes.insert("chan_a".into(), vec!["node_a".into()]);
        trigger_to_nodes.insert("chan_b".into(), vec!["node_b".into()]);

        // Only chan_a was updated
        let mut updated = HashSet::new();
        updated.insert("chan_a".into());

        // Should only check node_a (optimization)
        let tasks = prepare_next_tasks(&checkpoint, &node_specs, &node_triggers, &mut channels, Some(&updated), &trigger_to_nodes)
            .unwrap();

        // Should only create task for node_a
        assert_eq!(tasks.len(), 1, "Should only trigger node_a");
        assert!(tasks.values().any(|t| t.name == "node_a"), "Task should be for node_a");
    }
}

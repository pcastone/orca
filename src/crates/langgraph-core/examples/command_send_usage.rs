//! Example demonstrating Command and Send usage
//!
//! This example shows how to use Command for advanced graph control
//! and Send for dynamic task creation (map-reduce patterns).

use langgraph_core::{Command, Send, NodeResult};
use serde_json::json;

fn main() {
    println!("=== Command and Send Usage Examples ===\n");

    // Example 1: Simple state update (no command needed)
    println!("1. Simple State Update:");
    let result = NodeResult::State(json!({
        "count": 42,
        "status": "processed"
    }));
    println!("   Result: {:?}\n", result.get_state_update());

    // Example 2: Command with state update and navigation
    println!("2. Command with Update and Goto:");
    let cmd = Command::new()
        .with_update(json!({"status": "ready"}))
        .with_goto("next_step");
    let result = NodeResult::Command(cmd);
    println!("   Has goto: {}", result.has_goto());
    println!("   Update: {:?}\n", result.get_state_update());

    // Example 3: Map-reduce pattern with Send
    println!("3. Map-Reduce Pattern:");
    let items = vec![1, 2, 3, 4, 5];
    let sends: Vec<Send> = items
        .iter()
        .map(|i| Send::new("process_item", json!({"value": i, "operation": "square"})))
        .collect();

    println!("   Created {} Send tasks:", sends.len());
    for (idx, send) in sends.iter().enumerate() {
        println!("     Task {}: {} with {:?}", idx, send.node(), send.arg());
    }

    let cmd = Command::new()
        .with_update(json!({"phase": "mapping"}))
        .with_goto(sends);
    println!("   Command ready for execution\n");

    // Example 4: Resume from interrupt
    println!("4. Resume from Interrupt:");
    let cmd = Command::new()
        .with_resume(json!({"approved": true, "comments": "Looks good"}))
        .with_goto("continue_workflow");
    let result = NodeResult::Command(cmd);
    println!("   Has resume: {}", result.has_resume());
    println!("   Has goto: {}\n", result.has_goto());

    // Example 5: Multiple conditional sends
    println!("5. Conditional Routing with Multiple Sends:");
    let conditions = vec![
        ("high_priority", json!({"priority": "high", "id": 1})),
        ("high_priority", json!({"priority": "high", "id": 2})),
        ("low_priority", json!({"priority": "low", "id": 3})),
    ];

    let sends: Vec<Send> = conditions
        .iter()
        .map(|(node, state)| Send::new(*node, state.clone()))
        .collect();

    println!("   Routing to {} different handlers:", sends.len());
    for send in sends.iter() {
        println!("     -> {}", send.node());
    }
    println!();

    // Example 6: Command without state update (pure navigation)
    println!("6. Pure Navigation Command:");
    let cmd = Command::new().with_goto("error_handler");
    println!("   Goto: error_handler");
    println!("   Has update: {}\n", cmd.update.is_some());

    // Example 7: Complex command with all fields
    println!("7. Complex Command:");
    let cmd = Command::new()
        .with_update(json!({"attempt": 2, "status": "retrying"}))
        .with_resume(json!({"retry": true}))
        .with_goto(vec![
            Send::new("validator", json!({"type": "schema"})),
            Send::new("validator", json!({"type": "business_rules"})),
        ]);
    println!("   Full command with update, resume, and multiple sends");
    println!("   Update: {:?}", cmd.update.is_some());
    println!("   Resume: {:?}", cmd.resume.is_some());
    println!("   Goto: {:?}\n", cmd.goto.is_some());

    println!("=== Integration Points (to be implemented in Pregel loop) ===\n");
    println!("1. Node execution must support returning NodeResult instead of just Value");
    println!("2. After node execution, check if result contains Command");
    println!("3. Extract Command.update and apply to state");
    println!("4. Process Command.goto to create dynamic tasks (Send)");
    println!("5. Handle Command.resume to resolve pending interrupts");
    println!("6. Create PregelExecutableTask for each Send in Command.goto");
    println!("7. Maintain task hierarchy with proper path segments");
}

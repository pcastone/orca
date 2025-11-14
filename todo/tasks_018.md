# Task 018: Implement Task CLI Commands

## Objective
Implement all task-related CLI commands.

## Dependencies
- Task 017 (CLI framework)

## Commands
- `aco task create <title> [OPTIONS]` - Create task
- `aco task list [--status <status>]` - List tasks
- `aco task get <id>` - Get task details
- `aco task delete <id>` - Delete task
- `aco task execute <id> [--stream]` - Execute with streaming

## Implementation
- Call gRPC methods from client
- Handle streaming for execute command
- Format output based on --format flag
- Error handling and user-friendly messages

## Complexity: Moderate | Effort: 6-8 hours

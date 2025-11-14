# Task 017: Implement CLI Framework (clap)

## Objective
Set up CLI structure using clap with subcommands for task, workflow, and auth operations.

## Dependencies
- Task 016 (Client infrastructure)

## Key Files
- `src/crates/aco/src/cli/mod.rs` - CLI definition
- `src/crates/aco/src/cli/output.rs` - Output formatters

## Implementation
```
aco task <subcommand>
aco workflow <subcommand>
aco auth <subcommand>
```

Output formats: --format json|table|plain
Global flags: --server <url>, --verbose

## Complexity: Simple | Effort: 4-5 hours

# Task 020: Implement Auth CLI Commands

## Objective
Implement authentication CLI commands.

## Dependencies
- Task 017 (CLI framework)

## Commands
- `aco login` - Prompt for credentials, save token
- `aco logout` - Clear saved token
- `aco whoami` - Show current user from token

## Implementation
- Interactive password prompt (no echo)
- Token persistence to ~/.aco/token
- Token validation on whoami

## Complexity: Simple | Effort: 3-4 hours

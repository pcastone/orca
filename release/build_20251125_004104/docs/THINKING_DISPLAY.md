# Thinking Display Feature

## Overview

Orca and ACO now support displaying LLM thinking/reasoning output, similar to Claude Code's thinking blocks. This feature shows the model's internal reasoning process before providing the final answer.

## Supported Models

Models with explicit thinking/reasoning output:
- **Claude** (Anthropic) - Extended thinking with `thinking_budget` parameter
- **DeepSeek R1** - Reasoning wrapped in `<think>` tags
- **OpenAI o1** - Reasoning in separate content blocks

## Configuration

### TOML Configuration

Add to `~/.orca/orca.toml`:

```toml
[execution]
show_thinking = true  # Default: true
```

Add to `~/.orca/aco.toml`:

```toml
[ui]
show_thinking = true  # Default: true
```

### CLI Flags

Override config for a single execution:

```bash
# Force show thinking
orca --show-thinking --prompt "Your question"

# Force hide thinking
orca --no-thinking --prompt "Your question"
```

## Display Format

### Orca (Terminal)

Thinking is displayed with styled box-drawing characters:

```
╭─ Model Thinking ─╮
│ First, I need to consider...
│ Then I should analyze...
│ Finally, I conclude...
╰─ 156 tokens, 2.34s ─╯

[Final Answer Here]
```

### ACO TUI

Thinking events appear in the execution stream with:
- Icon: 💭 (thought bubble)
- Color: Gray
- Format: `[timestamp] 💭 REASONING: [content]`

## Implementation Details

### Direct LLM Calls

When calling LLMs directly (not through agents):
- Reasoning is extracted from `ChatResponse.reasoning` field
- Display controlled by `config.execution.show_thinking`
- Styled output with dimmed gray text

### Streaming Execution

When using streaming mode:
- `StreamEvent::Reasoning` events are emitted
- Content displayed in real-time as it arrives
- Same styled format as direct calls

### Agent Framework Limitation

**Known Issue**: The prebuilt agent framework (ReAct, PlanExecute, Reflection) currently loses reasoning information because the `LlmFunction` interface only returns `Message`, not the full `ChatResponse`.

**Workaround**: Direct LLM calls and custom agent implementations can access reasoning by using the LLM provider's `chat()` method directly, which returns `ChatResponse` with the `reasoning` field.

## Architecture

### Components Modified

1. **langgraph-core** - Added `StreamEvent::Reasoning` variant
2. **llm/claude** - Extracts thinking blocks from Claude API responses
3. **orca/task_executor** - Displays thinking in direct calls and streaming
4. **aco/tui** - Renders thinking events with special styling

### Data Flow

```
LLM Provider
    ↓ (returns ChatResponse with reasoning)
Task Executor / Stream Handler
    ↓ (checks config.show_thinking)
Display Layer
    ↓ (styled output)
Terminal / TUI
```

## Examples

### Example 1: Claude Extended Thinking

```bash
# Configure Claude
export ANTHROPIC_API_KEY="your-key"

# Edit ~/.orca/orca.toml
[llm]
provider = "claude"
model = "claude-3-7-sonnet-20250219"

# Run with thinking
orca --prompt "Explain quantum entanglement in simple terms"
```

Expected output shows thinking process before final answer.

### Example 2: DeepSeek R1 Reasoning

```bash
# Configure DeepSeek
export DEEPSEEK_API_KEY="your-key"

# Edit ~/.orca/orca.toml
[llm]
provider = "deepseek"
model = "deepseek-reasoner"

# Run
orca --prompt "Solve: What is 15% of 240?"
```

Shows step-by-step reasoning in `<think>` tags.

### Example 3: Disable Thinking Display

```bash
# Hide thinking for this execution
orca --no-thinking --prompt "Quick question"

# Or update config permanently
# ~/.orca/orca.toml
[execution]
show_thinking = false
```

## Testing

### Verify Configuration

```bash
# Check CLI flags exist
orca --help | grep thinking

# Expected output:
#   --show-thinking    Show LLM thinking/reasoning output
#   --no-thinking      Hide LLM thinking/reasoning output
```

### Test with Mock Reasoning

The Claude client tests include reasoning extraction:
```bash
cargo test --package llm test_response_conversion_with_thinking
```

## Troubleshooting

### Thinking Not Displayed

1. **Check config**: Ensure `show_thinking = true` in config file
2. **Check model**: Only certain models provide reasoning (see Supported Models)
3. **Check mode**: Agent framework has known limitations (see above)

### Incorrect Formatting

1. **Terminal colors**: Ensure terminal supports ANSI colors
2. **Unicode support**: Box-drawing characters require UTF-8 terminal

## Future Enhancements

- [ ] Support reasoning in agent framework by extending `LlmFunction` interface
- [ ] Add streaming thinking display for Claude extended thinking
- [ ] Configurable thinking display style (verbose, compact, hidden)
- [ ] Export thinking to logs or files for analysis
- [ ] Integration tests with actual LLM providers

## Related Files

- `src/crates/orca/src/config/schema.rs` - Configuration schema
- `src/crates/orca/src/executor/task_executor.rs` - Display implementation
- `src/crates/llm/src/remote/claude.rs` - Claude thinking extraction
- `src/crates/langgraph-core/src/stream.rs` - Streaming events
- `src/crates/aco/src/tui/ui.rs` - TUI rendering

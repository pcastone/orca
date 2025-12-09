# Unit Test Plan: Orca Project

Generated: 2025-11-25
Author: Claude (AI-Generated Test Plan)
Updated: 2025-11-25 (Tests implemented)

## Implementation Progress

### Completed (2025-11-25)
- ✅ **context/manager.rs**: Added 9 gap tests (21 total tests now pass)
- ✅ **context/token_counter.rs**: Added 8 gap tests (15 total tests now pass)
- ✅ **conversation_service.rs**: Added 9 tests (9 total tests now pass)

### Pending
- ⏳ LLM provider error handling tests
- ⏳ Agent pattern tests (existing coverage is good)

---

## Table of Contents

1. [langgraph-core Crate](#langgraph-core-crate)
   - [Context Module](#context-module-context)
2. [orca Crate](#orca-crate)
   - [Services Module](#services-module)
3. [langgraph-prebuilt Crate](#langgraph-prebuilt-crate)
   - [Agents Module](#agents-module)
4. [llm Crate](#llm-crate)
   - [Remote Providers](#remote-providers)
   - [Local Providers](#local-providers)
5. [Regression Detection Tests](#regression-detection-tests)
6. [Quality Checklist](#quality-checklist)

---

## langgraph-core Crate

### Context Module (`context/`)

The context module provides automatic context summarization to prevent token overflow in long-running agent sessions.

#### context/manager.rs ✅ IMPLEMENTED

**Current Tests:** 21 tests (12 original + 9 new gap tests)

**Gap Tests Implemented:**

##### ContextConfig Tests

- [x] **Test: `test_config_threshold_boundary_values`** ✅
  - Purpose: Validate threshold clamping at boundaries
  - Input: threshold values of 0.0, 0.1, 0.99, 1.0, -0.5, 1.5
  - Expected Output: Values clamped to [0.1, 0.99] range
  - Validation Criteria:
    - [x] 0.0 input results in 0.1 (clamped)
    - [x] 1.0 input results in 0.99 (clamped)
    - [x] Negative values clamped to 0.1
    - [x] Values > 1.0 clamped to 0.99

- [x] **Test: `test_config_target_ratio_calculation`** ✅
  - Purpose: Verify target_after_summarization calculation
  - Input: Various max_tokens and target_ratio_after_summarization values
  - Expected Output: Correct token count after summarization
  - Validation Criteria:
    - [ ] 100,000 max with 0.5 ratio = 50,000 target
    - [ ] Edge case: 0 max_tokens
    - [ ] Edge case: 1.0 ratio (full context)

- [ ] **Test: `test_config_with_preserve_system_toggle`**
  - Purpose: Test preserve_system_message flag behavior
  - Input: Various messages with system messages
  - Expected Output: System message preserved or not based on flag
  - Validation Criteria:
    - [ ] When true, system message always preserved
    - [ ] When false, system message can be summarized

##### ContextManager Tests

- [ ] **Test: `test_count_tokens_with_tool_calls`**
  - Purpose: Verify token counting includes tool call overhead
  - Input: Messages with tool_calls
  - Expected Output: Token count includes tool call tokens
  - Validation Criteria:
    - [ ] Tool call name tokens counted
    - [ ] Tool call arguments tokens counted
    - [ ] Overhead tokens (4 per tool call) added

- [ ] **Test: `test_should_summarize_exactly_at_threshold`**
  - Purpose: Test behavior when exactly at summarization threshold
  - Input: Messages with token count == trigger_threshold
  - Expected Output: Summarization triggered (>= comparison)
  - Validation Criteria:
    - [ ] Returns true when current_tokens == threshold
    - [ ] Returns false when current_tokens < threshold

- [ ] **Test: `test_summarize_preserves_recent_count_boundary`**
  - Purpose: Test preserve_recent_count edge cases
  - Input: Message lists where len() == preserve_recent_count
  - Expected Output: No summarization when not enough messages
  - Validation Criteria:
    - [ ] When messages == preserve_recent_count, all kept
    - [ ] When messages < preserve_recent_count, all kept

- [ ] **Test: `test_summarize_with_empty_messages`**
  - Purpose: Test summarization with empty message list
  - Input: Empty Vec<Message>
  - Expected Output: No crash, returns early
  - Validation Criteria:
    - [ ] summarized = false
    - [ ] messages_before == messages_after == 0

- [ ] **Test: `test_create_truncation_summary_long_content`**
  - Purpose: Test truncation with messages exceeding 200 chars
  - Input: Messages with content > 200 characters
  - Expected Output: Content truncated with "..."
  - Validation Criteria:
    - [ ] Truncated content ends with "..."
    - [ ] Truncated length is exactly 203 chars (200 + "...")

- [ ] **Test: `test_create_truncation_summary_skipped_messages_indicator`**
  - Purpose: Verify "(X messages omitted)" indicator
  - Input: Large message list requiring truncation
  - Expected Output: Summary includes omission indicator
  - Validation Criteria:
    - [ ] Indicator shows correct count
    - [ ] Head and tail messages present

- [ ] **Test: `test_summarize_rebuilds_message_list_correctly`** (async)
  - Purpose: Test message list structure after summarization
  - Input: Messages triggering summarization
  - Expected Output: [system?, summary_message, recent_messages...]
  - Validation Criteria:
    - [ ] System message at index 0 if preserved
    - [ ] Summary message marked as system with "[Previous conversation summary]" prefix
    - [ ] Recent messages preserved in order

#### context/token_counter.rs

**Existing Tests:** 10 tests covering basic token counting

**Additional Tests Needed:**

##### TiktokenCounter Tests

- [ ] **Test: `test_tiktoken_count_unicode_text`**
  - Purpose: Verify correct token counting for Unicode/non-ASCII
  - Input: Unicode strings (emojis, CJK characters, etc.)
  - Expected Output: Correct token count
  - Validation Criteria:
    - [ ] Emoji handling (likely 1+ tokens each)
    - [ ] CJK characters handled correctly
    - [ ] Mixed ASCII/Unicode text

- [ ] **Test: `test_tiktoken_count_message_with_all_content_types`**
  - Purpose: Test counting with MessageContent::Parts
  - Input: Message with text parts and image placeholders
  - Expected Output: Accurate token estimate
  - Validation Criteria:
    - [ ] Text parts counted correctly
    - [ ] Image placeholder "[image]" counted
    - [ ] Custom data placeholder counted

- [ ] **Test: `test_tiktoken_message_overhead_accuracy`**
  - Purpose: Verify 4-token overhead per message is correct
  - Input: Known test messages
  - Expected Output: content_tokens + 4 overhead
  - Validation Criteria:
    - [ ] Overhead is consistently 4 tokens
    - [ ] Matches GPT-4 message format

##### SimpleTokenCounter Tests

- [ ] **Test: `test_simple_counter_edge_cases`**
  - Purpose: Test edge cases for character-based counting
  - Input: Various edge case strings
  - Expected Output: Correct ceiling-based count
  - Validation Criteria:
    - [ ] 1 character = 1 token (ceil(1/4))
    - [ ] 4 characters = 1 token
    - [ ] 5 characters = 2 tokens
    - [ ] Whitespace-only strings

- [ ] **Test: `test_message_to_string_multipart_content`**
  - Purpose: Test helper function with complex content
  - Input: Message with multipart content
  - Expected Output: Concatenated string representation
  - Validation Criteria:
    - [ ] Text parts joined with spaces
    - [ ] Image parts converted to "[image]"
    - [ ] Custom parts formatted correctly

---

## orca Crate

### Services Module (`services/`)

#### conversation_service.rs

**Existing Tests:** 1 test for context config

**Additional Tests Needed:**

##### ConversationService Tests

- [ ] **Test: `test_new_with_default_context_window`**
  - Purpose: Verify default 128k context window
  - Input: OrcaConfig with default settings
  - Expected Output: ContextManager with 128k max_tokens
  - Validation Criteria:
    - [ ] context_manager.config().max_tokens == 128,000
    - [ ] Default threshold is 0.8

- [ ] **Test: `test_with_system_prompt_sets_prompt`**
  - Purpose: Verify system prompt is stored
  - Input: ConversationService with system prompt
  - Expected Output: system_prompt field set
  - Validation Criteria:
    - [ ] system_prompt is Some with correct value
    - [ ] Prompt is added to first message

- [ ] **Test: `test_send_message_empty_returns_error`** (async)
  - Purpose: Verify empty message validation
  - Input: Empty string ""
  - Expected Output: OrcaError::Config with "empty" message
  - Validation Criteria:
    - [ ] Returns Err variant
    - [ ] Error message contains "empty"

- [ ] **Test: `test_send_message_adds_to_history`** (async)
  - Purpose: Verify messages accumulate in history
  - Input: Multiple messages sent sequentially
  - Expected Output: History contains all messages
  - Validation Criteria:
    - [ ] message_count() increases
    - [ ] Messages in correct order

- [ ] **Test: `test_clear_history_empties_messages`** (async)
  - Purpose: Verify clear_history() works
  - Input: ConversationService with messages, then clear
  - Expected Output: Empty message list
  - Validation Criteria:
    - [ ] message_count() == 0 after clear
    - [ ] Can send new messages after clear

- [ ] **Test: `test_get_context_stats_returns_tuple`** (async)
  - Purpose: Verify stats calculation
  - Input: ConversationService with messages
  - Expected Output: (current_tokens, max_tokens, ratio)
  - Validation Criteria:
    - [ ] current_tokens > 0 for non-empty
    - [ ] max_tokens matches config
    - [ ] ratio = current/max

- [ ] **Test: `test_message_to_json_all_roles`**
  - Purpose: Test message_to_json helper for all roles
  - Input: Messages with all MessageRole variants
  - Expected Output: Correct JSON type field
  - Validation Criteria:
    - [ ] System -> "system"
    - [ ] Human -> "human"
    - [ ] Assistant -> "ai"
    - [ ] Tool -> "tool"
    - [ ] Custom -> "custom"

#### task_classifier.rs

**Existing Tests:** 17 tests covering classification and confidence

**Additional Tests Needed:**

##### TaskCategory Tests

- [ ] **Test: `test_task_category_custom_variant`**
  - Purpose: Test Custom variant behavior
  - Input: TaskCategory::Custom("my_category")
  - Expected Output: Correct as_str and display_name
  - Validation Criteria:
    - [ ] as_str() returns inner string
    - [ ] display_name() returns "Custom"
    - [ ] default_pattern_config_id() returns "default_react"

- [ ] **Test: `test_all_categories_have_pattern_config`**
  - Purpose: Verify all categories map to valid pattern configs
  - Input: All TaskCategory variants
  - Expected Output: Non-empty pattern config IDs
  - Validation Criteria:
    - [ ] Each variant returns a non-empty string
    - [ ] All IDs start with "default_"

##### TaskClassifier Tests

- [ ] **Test: `test_classifier_rule_priority_ordering`**
  - Purpose: Verify rules are sorted by priority
  - Input: TaskClassifier after construction
  - Expected Output: Rules ordered high to low priority
  - Validation Criteria:
    - [ ] CodeGeneration (90) checked before SimpleQuery (50)
    - [ ] Research (85) checked before FileOperation (75)

- [ ] **Test: `test_classify_case_insensitivity`**
  - Purpose: Verify patterns are case insensitive
  - Input: Same query in different cases
  - Expected Output: Same classification
  - Validation Criteria:
    - [ ] "WRITE A FUNCTION" == "write a function"
    - [ ] "Research How" == "research how"

- [ ] **Test: `test_classify_with_confidence_multiple_matches`**
  - Purpose: Test confidence with multiple pattern matches
  - Input: Query matching multiple patterns
  - Expected Output: Higher confidence (0.75-0.95)
  - Validation Criteria:
    - [ ] 2 matches -> 0.75 confidence
    - [ ] 3 matches -> 0.85 confidence
    - [ ] 4+ matches -> 0.95 confidence

- [ ] **Test: `test_classify_with_llm_unexpected_response`** (async)
  - Purpose: Test fallback when LLM returns invalid category
  - Input: Mock LLM returning "invalid_category"
  - Expected Output: Falls back to keyword classification
  - Validation Criteria:
    - [ ] Returns keyword-based result
    - [ ] Logs warning message

- [ ] **Test: `test_classify_smart_with_llm`** (async)
  - Purpose: Test classify_smart when LLM is configured
  - Input: Classifier with mock LLM
  - Expected Output: Uses LLM classification
  - Validation Criteria:
    - [ ] LLM chat() called
    - [ ] Returns LLM's category

- [ ] **Test: `test_add_rule_compiles_patterns`**
  - Purpose: Test that add_rule correctly compiles regex patterns
  - Input: Valid and invalid regex patterns
  - Expected Output: Valid patterns added, invalid skipped
  - Validation Criteria:
    - [ ] Valid patterns increase rules count
    - [ ] Invalid patterns silently skipped

#### pattern_router.rs

**Existing Tests:** 12 tests covering routing scenarios

**Additional Tests Needed:**

##### PatternRouter Tests

- [ ] **Test: `test_router_default_category_map_completeness`**
  - Purpose: Verify all TaskCategory variants have mappings
  - Input: default_category_map()
  - Expected Output: Map with all standard categories
  - Validation Criteria:
    - [ ] Contains SimpleQuery, FileOperation, CodeGeneration
    - [ ] Contains Research, DataAnalysis, SystemCommand, General

- [ ] **Test: `test_route_increments_usage_count`** (async)
  - Purpose: Verify usage counting on successful route
  - Input: Task routing to valid config
  - Expected Output: increment_usage called
  - Validation Criteria:
    - [ ] Config usage count increases
    - [ ] Works for both explicit and classified routes

- [ ] **Test: `test_route_fallback_chain`** (async)
  - Purpose: Test complete fallback chain
  - Input: Task with invalid explicit config, unknown classification
  - Expected Output: Falls through to hardcoded fallback
  - Validation Criteria:
    - [ ] Tries explicit config first
    - [ ] Falls to classification
    - [ ] Falls to default config
    - [ ] Falls to hardcoded "Fallback"

- [ ] **Test: `test_get_default_config_creates_fallback`** (async)
  - Purpose: Test fallback config creation
  - Input: Empty database
  - Expected Output: Returns fallback PatternConfig
  - Validation Criteria:
    - [ ] Name is "Fallback"
    - [ ] Pattern type is React
    - [ ] max_iterations is 10

- [ ] **Test: `test_set_category_mapping_updates_router`**
  - Purpose: Test dynamic category mapping updates
  - Input: set_category_mapping with new mapping
  - Expected Output: Routing uses new mapping
  - Validation Criteria:
    - [ ] map_category_to_config returns new config ID
    - [ ] Original mappings preserved

- [ ] **Test: `test_classify_with_confidence_passthrough`**
  - Purpose: Verify classifier passthrough
  - Input: Various descriptions
  - Expected Output: Same result as direct classifier call
  - Validation Criteria:
    - [ ] Category matches
    - [ ] Confidence matches

#### prompt_service.rs

**Existing Tests:** 3 tests covering creation and validation

**Additional Tests Needed:**

##### PromptService Tests

- [ ] **Test: `test_send_prompt_whitespace_only`** (async)
  - Purpose: Test prompt with only whitespace
  - Input: "   " (spaces only)
  - Expected Output: Error for empty prompt
  - Validation Criteria:
    - [ ] Whitespace-only treated as empty
    - [ ] Returns Config error

- [ ] **Test: `test_new_with_workspace_root`**
  - Purpose: Test workspace_root configuration
  - Input: Config with explicit workspace_root
  - Expected Output: Uses configured path
  - Validation Criteria:
    - [ ] DirectToolBridge created with path
    - [ ] Session ID is unique

- [ ] **Test: `test_send_prompt_result_extraction`** (async)
  - Purpose: Test response extraction logic
  - Input: Agent execution with various result structures
  - Expected Output: Correctly extracted response
  - Validation Criteria:
    - [ ] Extracts from result field if present
    - [ ] Falls back to messages if no result
    - [ ] Returns "No response generated" as last resort

---

## langgraph-prebuilt Crate

### Agents Module (`agents/`)

#### react.rs

**Existing Tests:** 31 comprehensive tests

**Additional Tests Needed:**

##### ReactAgentConfig Tests

- [ ] **Test: `test_react_agent_zero_max_iterations`** (async)
  - Purpose: Test behavior with max_iterations = 0
  - Input: Config with max_iterations = 0
  - Expected Output: Agent builds but may terminate immediately
  - Validation Criteria:
    - [ ] Graph compiles successfully
    - [ ] Execution terminates without infinite loop

- [ ] **Test: `test_react_agent_empty_system_prompt`**
  - Purpose: Test with empty string system prompt
  - Input: with_system_prompt("")
  - Expected Output: Empty system prompt stored
  - Validation Criteria:
    - [ ] system_prompt = Some("")
    - [ ] No system message added if empty

##### Tool Execution Tests

- [ ] **Test: `test_react_parallel_tool_execution`** (async)
  - Purpose: Test multiple tool calls in single message
  - Input: LLM returns message with multiple tool_calls
  - Expected Output: All tools executed, results combined
  - Validation Criteria:
    - [ ] Each tool call gets result message
    - [ ] Tool results in correct order
    - [ ] Agent sees all results

- [ ] **Test: `test_react_tool_not_found_handling`** (async)
  - Purpose: Test behavior when tool_call references unknown tool
  - Input: Tool call for non-existent tool
  - Expected Output: Error message in tool result
  - Validation Criteria:
    - [ ] Agent receives error message
    - [ ] Agent can attempt recovery

##### State Management Tests

- [ ] **Test: `test_react_state_messages_preserved_across_iterations`** (async)
  - Purpose: Test message history grows correctly
  - Input: Multi-iteration execution
  - Expected Output: All messages present in final state
  - Validation Criteria:
    - [ ] Human message present
    - [ ] All AI messages present
    - [ ] All tool messages present
    - [ ] Order preserved

#### plan_execute.rs

**Existing Tests:** 37 comprehensive tests

**Additional Tests Needed:**

##### PlanStep Tests

- [ ] **Test: `test_plan_step_all_fields_nullable`**
  - Purpose: Test PlanStep with all optional fields None
  - Input: PlanStep with only required fields
  - Expected Output: Serializes/deserializes correctly
  - Validation Criteria:
    - [ ] tool = None allowed
    - [ ] tool_args = None allowed
    - [ ] result = None allowed

- [ ] **Test: `test_plan_step_completion_state_transitions`**
  - Purpose: Test step completion workflow
  - Input: Step transitioning incomplete -> complete
  - Expected Output: State changes correctly
  - Validation Criteria:
    - [ ] completed: false -> true
    - [ ] result: None -> Some
    - [ ] Cannot uncomplete

##### PlanExecuteState Tests

- [ ] **Test: `test_state_current_step_bounds`**
  - Purpose: Test current_step beyond plan length
  - Input: current_step > plan.len()
  - Expected Output: Handled gracefully (plan complete)
  - Validation Criteria:
    - [ ] No out-of-bounds access
    - [ ] Treated as all steps complete

- [ ] **Test: `test_state_replan_count_max`**
  - Purpose: Test behavior at max replan count
  - Input: replan_count == max_replans
  - Expected Output: No more replanning
  - Validation Criteria:
    - [ ] Final answer set
    - [ ] Message indicates max reached

##### Replanning Logic Tests

- [ ] **Test: `test_should_replan_case_sensitivity`**
  - Purpose: Test error detection is case insensitive
  - Input: Results with "ERROR", "Error", "error"
  - Expected Output: All trigger replanning
  - Validation Criteria:
    - [ ] All case variants detected

- [ ] **Test: `test_should_replan_false_positives`**
  - Purpose: Test that "error" in context doesn't trigger
  - Input: Result like "No error occurred"
  - Expected Output: Triggers replan (current behavior)
  - Validation Criteria:
    - [ ] Understand current simple implementation
    - [ ] Document potential false positives

#### reflection.rs

**Existing Tests:** 43 comprehensive tests

**Additional Tests Needed:**

##### ReflectionCritique Tests

- [ ] **Test: `test_critique_score_boundary_validation`**
  - Purpose: Test scores at 0.0 and 1.0 boundaries
  - Input: Critiques with boundary scores
  - Expected Output: Correct is_satisfactory evaluation
  - Validation Criteria:
    - [ ] 0.0 score is never satisfactory
    - [ ] 1.0 score is always satisfactory
    - [ ] Threshold comparison is >=

- [ ] **Test: `test_critique_empty_feedback_arrays`**
  - Purpose: Test with empty strengths/weaknesses/suggestions
  - Input: Critique with all empty arrays
  - Expected Output: Valid object, serializes correctly
  - Validation Criteria:
    - [ ] Empty arrays allowed
    - [ ] No null issues

##### QualityMetrics Tests

- [ ] **Test: `test_quality_metrics_serialization_precision`**
  - Purpose: Test float precision in serialization
  - Input: QualityMetrics with many decimal places
  - Expected Output: Precision preserved
  - Validation Criteria:
    - [ ] 0.923456 roundtrips correctly
    - [ ] Negative deltas preserved

##### ReflectionState Tests

- [ ] **Test: `test_state_response_history_accumulation`**
  - Purpose: Test response_history grows correctly
  - Input: Multiple generator iterations
  - Expected Output: All responses in history
  - Validation Criteria:
    - [ ] History contains all versions
    - [ ] Order is chronological

- [ ] **Test: `test_state_critique_history_matches_iterations`**
  - Purpose: Test critique/iteration count alignment
  - Input: State after N iterations
  - Expected Output: critique_history.len() == iteration_count
  - Validation Criteria:
    - [ ] One critique per iteration
    - [ ] Counts match

##### Threshold Tests

- [ ] **Test: `test_quality_threshold_clamping`**
  - Purpose: Test threshold clamping to [0.0, 1.0]
  - Input: Thresholds of -0.5, 0.0, 1.0, 1.5
  - Expected Output: Values clamped
  - Validation Criteria:
    - [ ] -0.5 becomes 0.0
    - [ ] 1.5 becomes 1.0
    - [ ] Valid values unchanged

---

## llm Crate

### Remote Providers

#### claude.rs

**Existing Tests:** 16 tests covering message/response conversion

**Additional Tests Needed:**

##### Message Conversion Tests

- [ ] **Test: `test_convert_messages_empty_list`**
  - Purpose: Test with empty message list
  - Input: Empty Vec<Message>
  - Expected Output: (None, empty vec)
  - Validation Criteria:
    - [ ] No system prompt
    - [ ] Empty claude_messages

- [ ] **Test: `test_convert_messages_only_system`**
  - Purpose: Test with only system messages
  - Input: [Message::system("...")]
  - Expected Output: (Some(system), empty vec)
  - Validation Criteria:
    - [ ] System extracted
    - [ ] No conversation messages

- [ ] **Test: `test_convert_messages_interleaved_system`**
  - Purpose: Test system messages not at start
  - Input: [human, system, assistant]
  - Expected Output: System still combined correctly
  - Validation Criteria:
    - [ ] All system messages combined
    - [ ] Other messages preserved

##### Response Conversion Tests

- [ ] **Test: `test_convert_response_empty_content`**
  - Purpose: Test response with empty content array
  - Input: ClaudeResponse with content = []
  - Expected Output: Empty message content
  - Validation Criteria:
    - [ ] message.text() = Some("")
    - [ ] No panic

- [ ] **Test: `test_convert_response_missing_stop_reason`**
  - Purpose: Test response with None stop_reason
  - Input: ClaudeResponse with stop_reason = None
  - Expected Output: Default empty string in metadata
  - Validation Criteria:
    - [ ] metadata["stop_reason"] = ""

##### Error Handling Tests

- [ ] **Test: `test_chat_authentication_error`** (async, integration)
  - Purpose: Test 401 response handling
  - Input: Invalid API key
  - Expected Output: LlmError::AuthenticationError
  - Validation Criteria:
    - [ ] Error type is AuthenticationError
    - [ ] Error contains response body

- [ ] **Test: `test_chat_rate_limit_error`** (async, integration)
  - Purpose: Test 429 response handling
  - Input: Rate limited request
  - Expected Output: LlmError::RateLimitExceeded
  - Validation Criteria:
    - [ ] Error type is RateLimitExceeded

#### openai.rs

**Existing Tests:** 14 tests covering message/response conversion

**Additional Tests Needed:**

##### Message Conversion Tests

- [ ] **Test: `test_convert_message_tool_role`**
  - Purpose: Test tool message conversion
  - Input: Message with role = Tool and tool_call_id
  - Expected Output: OpenAiMessage with tool role
  - Validation Criteria:
    - [ ] role = "tool"
    - [ ] tool_call_id preserved

- [ ] **Test: `test_convert_message_empty_content`**
  - Purpose: Test message with empty content
  - Input: Message with MessageContent::Text("")
  - Expected Output: OpenAiMessage with content = Some("")
  - Validation Criteria:
    - [ ] content is Some, not None

##### Response Conversion Tests

- [ ] **Test: `test_convert_response_no_usage`**
  - Purpose: Test response without usage data
  - Input: OpenAiResponse with usage = None
  - Expected Output: ChatResponse with usage = None
  - Validation Criteria:
    - [ ] No panic on None usage
    - [ ] Other fields populated

- [ ] **Test: `test_convert_response_gpt4_vs_o1`**
  - Purpose: Test different model handling
  - Input: Same request to GPT-4 vs o1 model
  - Expected Output: o1 has reasoning extraction
  - Validation Criteria:
    - [ ] GPT-4: no reasoning extraction
    - [ ] o1: reasoning extracted if present

##### Streaming Tests

- [ ] **Test: `test_stream_builds_correct_request`** (async)
  - Purpose: Verify streaming request format
  - Input: ChatRequest for streaming
  - Expected Output: Request has stream: true
  - Validation Criteria:
    - [ ] Body contains "stream": true
    - [ ] Messages converted correctly

### Local Providers

#### ollama.rs

**Existing Tests:** 12 tests covering basic functionality

**Additional Tests Needed:**

##### Health Check Tests

- [ ] **Test: `test_check_health_server_down`** (async)
  - Purpose: Test health check when server unreachable
  - Input: Invalid/unreachable URL
  - Expected Output: Returns Ok(false)
  - Validation Criteria:
    - [ ] No panic or error
    - [ ] Returns false, not Err

##### Message Conversion Tests

- [ ] **Test: `test_convert_message_empty_content`**
  - Purpose: Test message with None text
  - Input: Message where text() returns None
  - Expected Output: Empty string content
  - Validation Criteria:
    - [ ] content = "" not None

##### Model Management Tests

- [ ] **Test: `test_use_model_updates_both_fields`** (async)
  - Purpose: Verify use_model updates config and current_model
  - Input: use_model("new-model")
  - Expected Output: Both fields updated
  - Validation Criteria:
    - [ ] current_model updated
    - [ ] config.model updated
    - [ ] Returns new model name

- [ ] **Test: `test_fetch_models_parses_response`** (async, integration)
  - Purpose: Test model list parsing
  - Input: Valid Ollama server response
  - Expected Output: Vec<ModelInfo>
  - Validation Criteria:
    - [ ] Model IDs extracted
    - [ ] Size metadata present
    - [ ] Modified date present

##### Response Conversion Tests

- [ ] **Test: `test_convert_response_no_duration`**
  - Purpose: Test response without timing data
  - Input: OllamaResponse with total_duration = None
  - Expected Output: No duration in metadata
  - Validation Criteria:
    - [ ] metadata lacks "total_duration_ns"
    - [ ] Other fields populated

---

## Regression Detection Tests

### Critical Path Protection

- [ ] **Test: `test_react_agent_always_terminates`** (async)
  - Protects Against: Infinite loop in ReAct execution
  - Trigger Conditions: Bug in should_continue routing logic
  - Validation Criteria:
    - [ ] Agent terminates within max_iterations
    - [ ] No infinite tool call loops
    - [ ] Clean termination on END node

- [ ] **Test: `test_context_summarization_preserves_meaning`** (async)
  - Protects Against: Data loss during summarization
  - Trigger Conditions: Changes to summarize_messages logic
  - Validation Criteria:
    - [ ] System prompt always preserved if configured
    - [ ] Recent messages always preserved
    - [ ] Token count reduced

- [ ] **Test: `test_message_to_json_roundtrip`**
  - Protects Against: Message serialization bugs
  - Trigger Conditions: Changes to Message struct or JSON conversion
  - Validation Criteria:
    - [ ] All message types roundtrip correctly
    - [ ] Role preserved
    - [ ] Content preserved
    - [ ] Tool calls preserved

- [ ] **Test: `test_llm_provider_error_categorization`** (async)
  - Protects Against: Error type misclassification
  - Trigger Conditions: Changes to error handling in providers
  - Validation Criteria:
    - [ ] 401 -> AuthenticationError
    - [ ] 429 -> RateLimitExceeded
    - [ ] Other -> ProviderError

- [ ] **Test: `test_task_classifier_determinism`**
  - Protects Against: Non-deterministic classification
  - Trigger Conditions: Changes to classification rules or ordering
  - Validation Criteria:
    - [ ] Same input always produces same category
    - [ ] Same input always produces same confidence

---

## Quality Checklist

Before completing this test plan, verify:

- [ ] Every public function in target modules has at least one test planned
- [ ] All error conditions have test coverage
- [ ] Edge cases are explicitly identified
- [ ] Validation criteria are specific and measurable
- [ ] Test names are descriptive and follow Rust conventions (snake_case, test_ prefix)
- [ ] Integration tests cover module boundaries
- [ ] Regression tests protect critical functionality
- [ ] Async tests marked appropriately
- [ ] Mock requirements documented for tests needing external dependencies

---

## Implementation Priority

### High Priority (Critical Paths)

1. **ContextManager summarization tests** - Prevents token overflow
2. **ReAct agent termination tests** - Prevents infinite loops
3. **LLM provider error handling** - Prevents silent failures
4. **Task classifier determinism** - Ensures reliable routing

### Medium Priority (Core Functionality)

1. **TokenCounter edge cases** - Accurate token estimation
2. **PatternRouter fallback chain** - Reliable pattern selection
3. **Plan-Execute state management** - Correct workflow execution
4. **Reflection quality metrics** - Accurate quality tracking

### Lower Priority (Edge Cases)

1. **Empty input handling** - Graceful degradation
2. **Unicode/special character handling** - International support
3. **Metadata serialization** - Complete data preservation
4. **Custom role handling** - Extension support

---

## Notes

- Tests marked `(async)` require `#[tokio::test]`
- Tests marked `(integration)` require external services (mock or real)
- Many existing tests provide good coverage; focus on gaps identified
- Consider property-based testing for numeric edge cases (proptest)
- Stream tests may need mock servers for full coverage

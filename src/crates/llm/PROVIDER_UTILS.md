# Provider Utilities

The `llm` crate now includes additional utility functions for all LLM providers through the `ProviderUtils` trait.

## Functions

### 1. `ping()` - Check Connection

Test if the provider is reachable and responsive.

```rust
use llm::local::OllamaClient;
use llm::config::LocalLlmConfig;
use llm::ProviderUtils;

let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
let client = OllamaClient::new(config);

if client.ping().await? {
    println!("✓ Provider is online");
} else {
    println!("✗ Provider is unreachable");
}
```

**Returns:**
- `Ok(true)` - Provider is available
- `Ok(false)` - Provider is unreachable  
- `Err(...)` - Authentication or configuration error

### 2. `fetch_models()` - Get Available Models

Retrieve a list of models available from the provider.

```rust
use llm::local::OllamaClient;
use llm::config::LocalLlmConfig;
use llm::ProviderUtils;

let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
let client = OllamaClient::new(config);

let models = client.fetch_models().await?;
for model in models {
    println!("Model: {}", model.id);
    println!("  Name: {}", model.name);
    if let Some(desc) = model.description {
        println!("  Description: {}", desc);
    }
}
```

**Returns:**
- `Vec<ModelInfo>` - List of available models

**Note:** Not all providers support model listing. Some providers return only the currently configured model.

**Model Info Structure:**
```rust
pub struct ModelInfo {
    pub id: String,              // Model identifier
    pub name: String,            // Human-readable name
    pub description: Option<String>,
    pub capabilities: Vec<String>,
    pub metadata: serde_json::Map<String, serde_json::Value>,
}
```

### 3. `use_model()` - Switch Models

Change the model being used by the client.

```rust
use llm::local::OllamaClient;
use llm::config::LocalLlmConfig;
use llm::ProviderUtils;

let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
let mut client = OllamaClient::new(config);

println!("Current model: {}", client.current_model());

// Switch to a different model
client.use_model("mistral").await?;

println!("New model: {}", client.current_model());
```

**Returns:**
- `String` - The new model name that is now active

### 4. `current_model()` - Get Current Model

Get the currently active model name.

```rust
use llm::ProviderUtils;

let model_name = client.current_model();
println!("Using model: {}", model_name);
```

## Complete Example

Here's a complete example demonstrating all provider utility functions:

```rust
use llm::local::OllamaClient;
use llm::config::LocalLlmConfig;
use llm::ProviderUtils;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
    let mut client = OllamaClient::new(config);

    // 1. Check if provider is online
    println!("Checking connection...");
    if !client.ping().await? {
        eprintln!("Error: Ollama server is not running");
        return Ok(());
    }
    println!("✓ Connected to Ollama");

    // 2. List available models
    println!("\nAvailable models:");
    let models = client.fetch_models().await?;
    for model in &models {
        println!("  - {}", model.id);
    }

    // 3. Show current model
    println!("\nCurrent model: {}", client.current_model());

    // 4. Switch to a different model (if available)
    if models.len() > 1 {
        let new_model = &models[1].id;
        println!("\nSwitching to model: {}", new_model);
        client.use_model(new_model).await?;
        println!("✓ Now using: {}", client.current_model());
    }

    // 5. Use the client to generate a response
    let request = ChatRequest::new(vec![
        Message::human("Hello! What model are you?")
    ]);
    let response = client.chat(request).await?;
    println!("\nResponse: {}", response.message.text().unwrap());

    Ok(())
}
```

## Provider Support

### Local Providers

| Provider | `ping()` | `fetch_models()` | `use_model()` |
|----------|----------|------------------|---------------|
| **Ollama** | ✓ Full | ✓ Lists all installed models | ✓ |
| **llama.cpp** | ✓ Full | ⚠️ Returns current model only | ✓ |
| **LM Studio** | ✓ Full | ✓ Lists available models | ✓ |

### Remote Providers

| Provider | `ping()` | `fetch_models()` | `use_model()` |
|----------|----------|------------------|---------------|
| **Claude** | ⚠️ Basic | ⚠️ Returns current model only | ✓ |
| **OpenAI** | ⚠️ Basic | ⚠️ Returns current model only | ✓ |
| **Gemini** | ⚠️ Basic | ⚠️ Returns current model only | ✓ |
| **Grok** | ⚠️ Basic | ⚠️ Returns current model only | ✓ |
| **Deepseek** | ⚠️ Basic | ⚠️ Returns current model only | ✓ |
| **OpenRouter** | ⚠️ Basic | ⚠️ Returns current model only | ✓ |

**Legend:**
- ✓ Full - Complete implementation with provider API support
- ⚠️ Basic - Basic implementation (may not call provider API)

## Use Cases

### Health Checks
```rust
// Before making requests, ensure provider is available
if !client.ping().await? {
    // Fall back to a different provider or return error
    return Err("Provider unavailable".into());
}
```

### Model Discovery
```rust
// Let users choose from available models
let models = client.fetch_models().await?;
println!("Select a model:");
for (i, model) in models.iter().enumerate() {
    println!("{}: {} - {}", i+1, model.id, model.name);
}
```

### Dynamic Model Selection
```rust
// Switch models based on task
match task_type {
    TaskType::Code => client.use_model("codellama").await?,
    TaskType::Chat => client.use_model("llama2-chat").await?,
    TaskType::Reasoning => client.use_model("deepseek-r1").await?,
};
```

### Monitoring
```rust
// Log current configuration
tracing::info!(
    provider = "ollama",
    model = client.current_model(),
    available = client.ping().await?,
    "LLM provider status"
);
```

## Implementation Notes

1. **Implemented Providers:** Currently, `ProviderUtils` is fully implemented for:
   - Ollama (with model listing)
   - llama.cpp (basic)
   - LM Studio (with model listing)

2. **Remaining Providers:** Remote providers (Claude, OpenAI, Gemini, Grok, Deepseek, OpenRouter) follow the same pattern and can be implemented using the examples provided.

3. **Pattern:** All implementations follow this structure:
   - Add `current_model` field to client struct
   - Initialize in `new()`
   - Implement `ProviderUtils` trait
   - For providers with model listing APIs, implement `fetch_models()` properly
   - For others, return the current model

4. **Testing:** Each implementation includes tests for `current_model()` functionality.


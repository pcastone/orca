# LLM Crate

Concrete LLM provider implementations for acolib.

## Overview

This crate provides implementations of the `ChatModel` trait from `langgraph-core` for various LLM providers, both local and remote.

## Features

### Local Providers

Local providers connect to LLM servers running on localhost or local networks:

- **Ollama** - Popular local LLM runner (`OllamaClient`)
  - Default URL: `http://localhost:11434`
  - Supports Llama 2, Mistral, Mixtral, and more
  
- **llama.cpp** - Direct llama.cpp server integration (`LlamaCppClient`)
  - Default URL: `http://localhost:8080`
  - OpenAI-compatible API
  
- **LM Studio** - User-friendly local LLM interface (`LmStudioClient`)
  - Default URL: `http://localhost:1234/v1`
  - OpenAI-compatible API

### Remote Providers

Remote providers connect to cloud-hosted LLM APIs:

- **Claude** - Anthropic's Claude models (`ClaudeClient`)
  - Claude 3 Opus, Sonnet, Haiku
  - Claude 3.5 Sonnet
  
- **OpenAI** - OpenAI models (`OpenAiClient`)
  - GPT-4, GPT-4 Turbo, GPT-3.5 Turbo
  - o1, o1-mini (thinking models with reasoning support)
  
- **Gemini** - Google's Gemini models (`GeminiClient`)
  - Gemini Pro, Gemini Pro Vision
  - Gemini 1.5 Pro, Gemini 1.5 Flash
  
- **Grok** - xAI's Grok models (`GrokClient`)
  - OpenAI-compatible API
  
- **Deepseek** - Deepseek models (`DeepseekClient`)
  - Deepseek Chat, Deepseek Coder
  - Deepseek R1 (thinking model with extended reasoning)
  
- **OpenRouter** - Unified API for multiple providers (`OpenRouterClient`)
  - Route to any supported provider
  - Single API key for multiple models

## Usage

### Local Provider (Ollama)

```rust
use llm::local::OllamaClient;
use llm::config::LocalLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
    let client = OllamaClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("What is Rust?")
    ]);

    let response = client.chat(request).await?;
    println!("Response: {}", response.message.text().unwrap());

    Ok(())
}
```

### Remote Provider (OpenAI)

```rust
use llm::remote::OpenAiClient;
use llm::config::RemoteLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RemoteLlmConfig::from_env(
        "OPENAI_API_KEY",
        "https://api.openai.com/v1",
        "gpt-4"
    )?;
    let client = OpenAiClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("Explain quantum computing briefly")
    ]).with_temperature(0.7);

    let response = client.chat(request).await?;
    println!("Response: {}", response.message.text().unwrap());

    Ok(())
}
```

### Remote Provider (Google Gemini)

```rust
use llm::remote::GeminiClient;
use llm::config::RemoteLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RemoteLlmConfig::from_env(
        "GOOGLE_API_KEY",
        "https://generativelanguage.googleapis.com/v1beta",
        "gemini-pro"
    )?;
    let client = GeminiClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("What is machine learning?")
    ]).with_temperature(0.7);

    let response = client.chat(request).await?;
    println!("Response: {}", response.message.text().unwrap());

    Ok(())
}
```

### Thinking Model with Reasoning (Deepseek R1)

```rust
use llm::remote::DeepseekClient;
use llm::config::RemoteLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest, ReasoningMode};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RemoteLlmConfig::from_env(
        "DEEPSEEK_API_KEY",
        "https://api.deepseek.com",
        "deepseek-reasoner"
    )?;
    let client = DeepseekClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("Solve this logic puzzle: ...")
    ]).with_reasoning(ReasoningMode::Separated);

    let response = client.chat(request).await?;
    
    // Access the thinking process
    if let Some(reasoning) = response.reasoning {
        println!("Model's thinking: {}", reasoning.content);
    }
    
    // Access the final answer
    println!("Final answer: {}", response.message.text().unwrap());

    Ok(())
}
```

## Configuration

All providers support:
- Request timeouts
- Retry logic (configurable max retries)
- Environment variable loading for API keys
- Temperature, max_tokens, and other generation parameters

## Provider Utilities

All providers now support additional utility functions via the `ProviderUtils` trait:

### Check Connection (`ping`)
```rust
use llm::ProviderUtils;

if client.ping().await? {
    println!("Provider is online");
}
```

### List Available Models (`fetch_models`)
```rust
use llm::ProviderUtils;

let models = client.fetch_models().await?;
for model in models {
    println!("Model: {} - {}", model.id, model.name);
}
```

### Switch Models (`use_model`)
```rust
use llm::ProviderUtils;

client.use_model("different-model").await?;
println!("Now using: {}", client.current_model());
```

See [PROVIDER_UTILS.md](./PROVIDER_UTILS.md) for complete documentation and examples.

## Features

- `default` - Enables both local and remote providers
- `local` - Local providers only
- `remote` - Remote providers only

## Dependencies

This crate depends on:
- `langgraph-core` - For the `ChatModel` trait and message types
- `reqwest` - HTTP client
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization


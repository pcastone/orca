# LLM Provider Configuration Guide

**Last Updated**: 2025-11-16
**acolib Version**: 0.1.0

This guide covers all LLM providers supported by acolib, including configuration, usage examples, and best practices.

---

## Table of Contents

- [Overview](#overview)
- [Remote Providers](#remote-providers)
  - [Anthropic Claude](#anthropic-claude)
  - [OpenAI](#openai)
  - [Google Gemini](#google-gemini)
  - [xAI Grok](#xai-grok)
  - [Deepseek](#deepseek)
  - [OpenRouter](#openrouter)
- [Local Providers](#local-providers)
  - [Ollama](#ollama)
  - [llama.cpp](#llamacpp)
  - [LM Studio](#lm-studio)
- [Advanced Features](#advanced-features)
  - [Streaming Responses](#streaming-responses)
  - [Thinking Models](#thinking-models)
  - [Token Counting](#token-counting)
- [Configuration Reference](#configuration-reference)
- [Troubleshooting](#troubleshooting)

---

## Overview

acolib supports **9 LLM providers** divided into two categories:

### Remote Providers (6)
Cloud-hosted APIs requiring API keys:
- **Claude** - Anthropic's Claude models
- **OpenAI** - GPT-4, GPT-3.5, o1
- **Gemini** - Google's Gemini models
- **Grok** - xAI's Grok models
- **Deepseek** - Deepseek models including R1
- **OpenRouter** - Unified API for multiple providers

### Local Providers (3)
Local servers, no API keys required:
- **Ollama** - Popular local LLM runner
- **llama.cpp** - Direct llama.cpp integration
- **LM Studio** - User-friendly local interface

---

## Remote Providers

### Anthropic Claude

**Models**: Claude 3 Opus, Sonnet, Haiku, Claude 3.5, etc.

**API Documentation**: https://docs.anthropic.com/

#### Configuration

**Environment Variable**:
```bash
export ANTHROPIC_API_KEY="your-api-key-here"
```

**Config File** (`~/.orca/orca.toml` or `./.orca/orca.toml`):
```toml
[llm]
provider = "anthropic"
model = "claude-3-opus-20240229"
api_key = "${ANTHROPIC_API_KEY}"
```

#### Code Example

```rust
use llm::remote::ClaudeClient;
use llm::config::RemoteLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RemoteLlmConfig::from_env(
        "ANTHROPIC_API_KEY",
        "https://api.anthropic.com/v1",
        "claude-3-opus-20240229"
    )?;

    let client = ClaudeClient::new(config);

    let request = ChatRequest::new(vec![
        Message::system("You are a helpful assistant."),
        Message::human("Explain Rust ownership in one sentence.")
    ])
    .with_temperature(0.7)
    .with_max_tokens(200);

    let response = client.chat(request).await?;
    println!("Claude: {}", response.message.text().unwrap());

    Ok(())
}
```

#### Available Models
- `claude-3-opus-20240229` - Most capable, best for complex tasks
- `claude-3-sonnet-20240229` - Balanced performance and speed
- `claude-3-haiku-20240307` - Fastest, most cost-effective
- `claude-3-5-sonnet-20240620` - Latest enhanced version

---

### OpenAI

**Models**: GPT-4, GPT-4 Turbo, GPT-3.5 Turbo, o1, o1-mini

**API Documentation**: https://platform.openai.com/docs/

#### Configuration

**Environment Variable**:
```bash
export OPENAI_API_KEY="your-api-key-here"
```

**Config File**:
```toml
[llm]
provider = "openai"
model = "gpt-4"
api_key = "${OPENAI_API_KEY}"
```

#### Code Example

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
        Message::human("What is the capital of France?")
    ])
    .with_temperature(0.5);

    let response = client.chat(request).await?;
    println!("GPT-4: {}", response.message.text().unwrap());

    Ok(())
}
```

#### Available Models
- `gpt-4` - Most capable GPT-4 model
- `gpt-4-turbo` - Faster GPT-4 with larger context
- `gpt-3.5-turbo` - Fast and cost-effective
- `o1-preview` - Advanced reasoning model
- `o1-mini` - Faster reasoning model

---

### Google Gemini

**Models**: Gemini Pro, Gemini Pro Vision

**API Documentation**: https://ai.google.dev/docs

#### Configuration

**Environment Variable**:
```bash
export GOOGLE_API_KEY="your-api-key-here"
```

**Config File**:
```toml
[llm]
provider = "gemini"
model = "gemini-pro"
api_key = "${GOOGLE_API_KEY}"
```

#### Code Example

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
        Message::human("Explain quantum entanglement simply.")
    ]);

    let response = client.chat(request).await?;
    println!("Gemini: {}", response.message.text().unwrap());

    Ok(())
}
```

#### Available Models
- `gemini-pro` - Text generation
- `gemini-pro-vision` - Multimodal (text + images)

---

### xAI Grok

**Models**: Grok Beta

**API Documentation**: https://docs.x.ai/

#### Configuration

**Environment Variable**:
```bash
export XAI_API_KEY="your-api-key-here"
```

**Config File**:
```toml
[llm]
provider = "grok"
model = "grok-beta"
api_key = "${XAI_API_KEY}"
```

#### Code Example

```rust
use llm::remote::GrokClient;
use llm::config::RemoteLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RemoteLlmConfig::from_env(
        "XAI_API_KEY",
        "https://api.x.ai/v1",
        "grok-beta"
    )?;

    let client = GrokClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("What's the latest in AI research?")
    ]);

    let response = client.chat(request).await?;
    println!("Grok: {}", response.message.text().unwrap());

    Ok(())
}
```

#### Available Models
- `grok-beta` - Latest Grok model

---

### Deepseek

**Models**: Deepseek Chat, Deepseek Reasoner (R1)

**API Documentation**: https://platform.deepseek.com/

**Special Feature**: R1 is a thinking model that shows its reasoning process.

#### Configuration

**Environment Variable**:
```bash
export DEEPSEEK_API_KEY="your-api-key-here"
```

**Config File**:
```toml
[llm]
provider = "deepseek"
model = "deepseek-reasoner"
api_key = "${DEEPSEEK_API_KEY}"
```

#### Code Example (Thinking Model)

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

    // Enable reasoning mode to see the model's thinking
    let request = ChatRequest::new(vec![
        Message::human("Solve: If a train travels 120 miles in 2 hours, what's its speed?")
    ])
    .with_reasoning(ReasoningMode::Separated);

    let response = client.chat(request).await?;

    // The thinking process
    if let Some(reasoning) = response.reasoning {
        println!("Reasoning:\n{}", reasoning.content);
    }

    // The final answer
    println!("\nAnswer: {}", response.message.text().unwrap());

    Ok(())
}
```

#### Available Models
- `deepseek-chat` - Standard chat model
- `deepseek-reasoner` - R1 thinking model with reasoning

---

### OpenRouter

**Models**: Access to 100+ models from multiple providers

**API Documentation**: https://openrouter.ai/docs

**Special Feature**: Single API for OpenAI, Anthropic, Google, Meta, and more.

#### Configuration

**Environment Variable**:
```bash
export OPENROUTER_API_KEY="your-api-key-here"
```

**Config File**:
```toml
[llm]
provider = "openrouter"
model = "anthropic/claude-3-opus"  # Can use any OpenRouter model
api_key = "${OPENROUTER_API_KEY}"
```

#### Code Example

```rust
use llm::remote::OpenRouterClient;
use llm::config::RemoteLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RemoteLlmConfig::from_env(
        "OPENROUTER_API_KEY",
        "https://openrouter.ai/api/v1",
        "anthropic/claude-3-opus"  // Use any model ID from OpenRouter
    )?;

    let client = OpenRouterClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("Compare Python and Rust for web development.")
    ]);

    let response = client.chat(request).await?;
    println!("Response: {}", response.message.text().unwrap());

    Ok(())
}
```

#### Popular Models
- `anthropic/claude-3-opus` - Claude 3 Opus
- `openai/gpt-4` - GPT-4
- `google/gemini-pro` - Gemini Pro
- `meta-llama/llama-3-70b` - Llama 3 70B
- And 100+ more...

---

## Local Providers

### Ollama

**Description**: Popular local LLM runner with easy model management

**Website**: https://ollama.ai/

**Advantages**:
- No API costs
- Privacy (data stays local)
- Offline operation
- Wide model support

#### Installation

```bash
# macOS
brew install ollama

# Linux
curl -fsSL https://ollama.ai/install.sh | sh

# Windows
# Download from https://ollama.ai/download
```

#### Configuration

**No API key needed!**

**Config File**:
```toml
[llm]
provider = "ollama"
model = "llama2"  # or any Ollama model
base_url = "http://localhost:11434"
```

#### Code Example

```rust
use llm::local::OllamaClient;
use llm::config::LocalLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LocalLlmConfig::new(
        "http://localhost:11434",
        "llama2"
    );

    let client = OllamaClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("What is Rust?")
    ]);

    let response = client.chat(request).await?;
    println!("Ollama: {}", response.message.text().unwrap());

    Ok(())
}
```

#### Popular Models
```bash
# Pull models before using
ollama pull llama2           # Meta's Llama 2
ollama pull mistral          # Mistral 7B
ollama pull codellama        # Code-focused
ollama pull llama3           # Meta's Llama 3
ollama pull phi              # Microsoft Phi
```

---

### llama.cpp

**Description**: Direct integration with llama.cpp server

**Website**: https://github.com/ggerganov/llama.cpp

**Advantages**:
- Maximum performance
- Low resource usage
- CPU and GPU support
- GGUF model format

#### Installation

```bash
# Clone and build
git clone https://github.com/ggerganov/llama.cpp
cd llama.cpp
make

# Run server
./server -m models/llama-2-7b.gguf -c 2048
```

#### Configuration

**Config File**:
```toml
[llm]
provider = "llama_cpp"
model = "llama-2-7b"
base_url = "http://localhost:8080"
```

#### Code Example

```rust
use llm::local::LlamaCppClient;
use llm::config::LocalLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LocalLlmConfig::new(
        "http://localhost:8080",
        "llama-2-7b"
    );

    let client = LlamaCppClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("Explain machine learning.")
    ]);

    let response = client.chat(request).await?;
    println!("llama.cpp: {}", response.message.text().unwrap());

    Ok(())
}
```

---

### LM Studio

**Description**: User-friendly desktop app for running local LLMs

**Website**: https://lmstudio.ai/

**Advantages**:
- Easy GUI interface
- Model discovery and download
- No command line needed
- Cross-platform (macOS, Windows, Linux)

#### Installation

Download from https://lmstudio.ai/

#### Configuration

**Config File**:
```toml
[llm]
provider = "lmstudio"
model = "local-model"
base_url = "http://localhost:1234/v1"
```

#### Code Example

```rust
use llm::local::LmStudioClient;
use llm::config::LocalLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LocalLlmConfig::new(
        "http://localhost:1234/v1",
        "local-model"  // Name shown in LM Studio
    );

    let client = LmStudioClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("Write a haiku about coding.")
    ]);

    let response = client.chat(request).await?;
    println!("LM Studio: {}", response.message.text().unwrap());

    Ok(())
}
```

---

## Advanced Features

### Streaming Responses

All providers support streaming for real-time token delivery:

```rust
use llm::remote::OpenAiClient;
use llm::config::RemoteLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RemoteLlmConfig::from_env(
        "OPENAI_API_KEY",
        "https://api.openai.com/v1",
        "gpt-4"
    )?;

    let client = OpenAiClient::new(config);

    let request = ChatRequest::new(vec![
        Message::human("Write a short story about a robot.")
    ]);

    // Stream the response
    let mut stream = client.stream(request).await?;

    print!("GPT-4: ");
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(text) = chunk.delta.text() {
            print!("{}", text);
        }
    }
    println!();

    Ok(())
}
```

### Thinking Models

OpenAI's o1 and Deepseek's R1 models support reasoning modes:

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
        Message::human("Solve this math problem step by step: ...")
    ])
    .with_reasoning(ReasoningMode::Separated);

    let response = client.chat(request).await?;

    // Access thinking process
    if let Some(reasoning) = response.reasoning {
        println!("=== Thinking Process ===");
        println!("{}", reasoning.content);
        println!("\n=== Final Answer ===");
    }

    println!("{}", response.message.text().unwrap());

    Ok(())
}
```

### Token Counting

Track token usage for cost estimation:

```rust
let response = client.chat(request).await?;

if let Some(usage) = response.usage {
    println!("Tokens used:");
    println!("  Input: {}", usage.input_tokens);
    println!("  Output: {}", usage.output_tokens);
    println!("  Total: {}", usage.total_tokens);
}
```

---

## Configuration Reference

### RemoteLlmConfig

```rust
pub struct RemoteLlmConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl RemoteLlmConfig {
    // Load from environment variable
    pub fn from_env(env_var: &str, base_url: &str, model: &str) -> Result<Self>;

    // Create directly
    pub fn new(api_key: String, base_url: String, model: String) -> Self;
}
```

### LocalLlmConfig

```rust
pub struct LocalLlmConfig {
    pub base_url: String,
    pub model: String,
}

impl LocalLlmConfig {
    pub fn new(base_url: &str, model: &str) -> Self;
}
```

### ChatRequest Options

```rust
ChatRequest::new(messages)
    .with_temperature(0.7)          // 0.0 to 2.0, default 1.0
    .with_max_tokens(500)            // Max output tokens
    .with_top_p(0.9)                 // Nucleus sampling
    .with_stop_sequences(vec!["###"]) // Stop generation
    .with_reasoning(ReasoningMode::Separated)  // For thinking models
```

---

## Troubleshooting

### API Key Not Found

**Error**: `LlmError: API key not found`

**Solution**:
```bash
# Check environment variable is set
echo $OPENAI_API_KEY

# Set if missing
export OPENAI_API_KEY="your-key-here"

# Or add to .bashrc/.zshrc for persistence
echo 'export OPENAI_API_KEY="your-key-here"' >> ~/.bashrc
```

### Connection Refused (Local Providers)

**Error**: `Connection refused`

**Solution**:
```bash
# Check if server is running
curl http://localhost:11434/api/version  # Ollama

# Start Ollama
ollama serve

# Or llama.cpp
./server -m models/model.gguf -c 2048
```

### Model Not Found

**Error**: `Model not found`

**Solution**:
```bash
# For Ollama, pull the model first
ollama pull llama2

# List available models
ollama list
```

### Rate Limiting

**Error**: `429 Too Many Requests`

**Solution**:
- Add delay between requests
- Reduce concurrent requests
- Upgrade API plan
- Switch to local provider

### Out of Memory (Local Providers)

**Error**: `Failed to allocate memory`

**Solution**:
- Use smaller model (7B instead of 70B)
- Reduce context length
- Enable quantization (GGUF Q4/Q5)
- Add more RAM or use GPU

---

## Best Practices

1. **Start with local providers** for development (free, fast iteration)
2. **Use environment variables** for API keys (never hardcode)
3. **Enable streaming** for better UX in interactive applications
4. **Monitor token usage** to control costs
5. **Handle errors gracefully** with retries and backoff
6. **Choose model size appropriately** (don't use GPT-4 for simple tasks)
7. **Test with multiple providers** to find best fit for your use case

---

## Next Steps

- [Architecture Documentation](architecture.md) - System design
- [CLAUDE.md](../CLAUDE.md) - Development guide
- [API Reference](endpoints.md) - REST API documentation

---

**Need Help?** Check the [GitHub Issues](https://github.com/pcastone/orca/issues) or file a new issue.

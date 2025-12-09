# Orca Release

This is a compiled release of the Orca orchestrator for building and executing stateful AI agent workflows.

## Quick Start

1. **Install the binary:**
   ```bash
   cp bin/orca /usr/local/bin/
   # or add bin/ to your PATH
   ```

2. **Configure Orca:**
   ```bash
   mkdir -p ~/.orca
   cp config/orca.toml.sample ~/.orca/orca.toml
   # Edit with your API keys and preferences
   ```

3. **Run Orca:**
   ```bash
   orca --help
   ```

## Directory Structure

- **bin/** - Compiled binary
- **config/** - Configuration templates
- **templates/** - Reusable workflow templates
- **workflows/** - Example workflows
- **playground/** - Sandbox examples for learning
- **docs/** - Documentation

## Configuration

See `config/orca.toml.sample` for all available options. Configuration files are loaded from:
1. `./.orca/orca.toml` (project-level)
2. `~/.orca/orca.toml` (user-level)

## LLM Providers

Orca supports multiple LLM providers:
- **Anthropic** - Claude models (requires `ANTHROPIC_API_KEY`)
- **OpenAI** - GPT models (requires `OPENAI_API_KEY`)
- **Google** - Gemini models (requires `GOOGLE_API_KEY`)
- **Ollama** - Local models (requires local Ollama running)
- **llama.cpp** - Local LLaMA models

## Building from Source

To build from source:
```bash
cd /path/to/orca
cargo build -p orca --release
```

## Documentation

See the docs/ directory for detailed documentation on:
- Architecture and design
- Building workflows
- Configuration options
- Examples and templates

## Support

For issues and questions:
- Check the documentation in docs/
- Review examples in playground/
- Check workflow templates in templates/

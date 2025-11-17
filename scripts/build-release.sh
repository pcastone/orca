#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_DIR="${PROJECT_ROOT}/release"
BUILD_DATE=$(date +%Y%m%d_%H%M%S)
RELEASE_BUILD_DIR="${RELEASE_DIR}/build_${BUILD_DATE}"

echo -e "${YELLOW}🔨 Starting Orca Build and Release Process${NC}"
echo "Project Root: $PROJECT_ROOT"
echo "Release Directory: $RELEASE_BUILD_DIR"
echo ""

# Step 1: Clean previous builds
echo -e "${YELLOW}📦 Cleaning previous builds...${NC}"
cd "$PROJECT_ROOT"
cargo clean --release 2>/dev/null || true
echo -e "${GREEN}✓ Clean complete${NC}"
echo ""

# Step 2: Build the project
echo -e "${YELLOW}🏗️  Building all binaries in release mode...${NC}"
cargo build -p orca -p aco -p orchestrator --release
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi
echo ""

# Step 3: Create release directory structure
echo -e "${YELLOW}📁 Creating release directory structure...${NC}"
mkdir -p "${RELEASE_BUILD_DIR}"
mkdir -p "${RELEASE_BUILD_DIR}/bin"
mkdir -p "${RELEASE_BUILD_DIR}/config"
mkdir -p "${RELEASE_BUILD_DIR}/templates"
mkdir -p "${RELEASE_BUILD_DIR}/workflows"
mkdir -p "${RELEASE_BUILD_DIR}/playground"
mkdir -p "${RELEASE_BUILD_DIR}/docs"
echo -e "${GREEN}✓ Directory structure created${NC}"
echo ""

# Step 4: Copy binaries
echo -e "${YELLOW}📋 Copying binaries...${NC}"

# Copy orca
ORCA_SRC="${PROJECT_ROOT}/target/release/orca"
if [ -f "$ORCA_SRC" ]; then
    cp "$ORCA_SRC" "${RELEASE_BUILD_DIR}/bin/"
    chmod +x "${RELEASE_BUILD_DIR}/bin/orca"
    echo -e "${GREEN}✓ Binary copied: orca${NC}"
else
    echo -e "${RED}✗ Orca binary not found${NC}"
    exit 1
fi

# Copy aco
ACO_SRC="${PROJECT_ROOT}/target/release/aco"
if [ -f "$ACO_SRC" ]; then
    cp "$ACO_SRC" "${RELEASE_BUILD_DIR}/bin/"
    chmod +x "${RELEASE_BUILD_DIR}/bin/aco"
    echo -e "${GREEN}✓ Binary copied: aco${NC}"
else
    echo -e "${YELLOW}⚠ Aco binary not found${NC}"
fi

# Copy orchestrator
ORCHESTRATOR_SRC="${PROJECT_ROOT}/target/release/orchestrator"
if [ -f "$ORCHESTRATOR_SRC" ]; then
    cp "$ORCHESTRATOR_SRC" "${RELEASE_BUILD_DIR}/bin/"
    chmod +x "${RELEASE_BUILD_DIR}/bin/orchestrator"
    echo -e "${GREEN}✓ Binary copied: orchestrator${NC}"
else
    echo -e "${YELLOW}⚠ Orchestrator binary not found${NC}"
fi

echo ""

# Step 5: Copy templates
echo -e "${YELLOW}📚 Copying templates...${NC}"
if [ -d "${PROJECT_ROOT}/templates" ]; then
    cp -r "${PROJECT_ROOT}/templates"/* "${RELEASE_BUILD_DIR}/templates/"
    echo -e "${GREEN}✓ Templates copied${NC}"
else
    echo -e "${YELLOW}⚠ No templates directory found${NC}"
fi
echo ""

# Step 6: Copy workflows
echo -e "${YELLOW}⚙️  Copying workflows...${NC}"
if [ -d "${PROJECT_ROOT}/workflows" ]; then
    cp -r "${PROJECT_ROOT}/workflows"/* "${RELEASE_BUILD_DIR}/workflows/"
    echo -e "${GREEN}✓ Workflows copied${NC}"
else
    echo -e "${YELLOW}⚠ No workflows directory found${NC}"
fi
echo ""

# Step 7: Copy playground
echo -e "${YELLOW}🎮 Copying playground...${NC}"
if [ -d "${PROJECT_ROOT}/playground" ]; then
    cp -r "${PROJECT_ROOT}/playground"/* "${RELEASE_BUILD_DIR}/playground/"
    echo -e "${GREEN}✓ Playground copied${NC}"
else
    echo -e "${YELLOW}⚠ No playground directory found${NC}"
fi
echo ""

# Step 8: Create sample config
echo -e "${YELLOW}⚙️  Creating sample configuration...${NC}"
cat > "${RELEASE_BUILD_DIR}/config/orca.toml.sample" << 'EOF'
# Orca Configuration Sample
# Copy this file to ~/.orca/orca.toml or ./.orca/orca.toml for project-level config

[llm]
# Provider options: anthropic, openai, gemini, ollama, llama_cpp
provider = "anthropic"
model = "claude-3-5-sonnet-20241022"

# API key can use environment variable expansion
api_key = "${ANTHROPIC_API_KEY}"

[execution]
# Enable streaming for token-by-token output
streaming = true

# Maximum tokens for response
max_tokens = 4096

# Temperature for sampling (0.0 to 2.0)
temperature = 0.7

[database]
# SQLite database location
path = "~/.orca/orca.db"

[logging]
# Log level: trace, debug, info, warn, error
level = "info"
EOF
echo -e "${GREEN}✓ Sample configuration created${NC}"
echo ""

# Step 9: Create README for release
echo -e "${YELLOW}📖 Creating release README...${NC}"
cat > "${RELEASE_BUILD_DIR}/README.md" << 'EOF'
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
EOF
echo -e "${GREEN}✓ Release README created${NC}"
echo ""

# Step 10: Copy documentation
echo -e "${YELLOW}📚 Copying documentation...${NC}"
if [ -d "${PROJECT_ROOT}/docs" ]; then
    cp -r "${PROJECT_ROOT}/docs"/* "${RELEASE_BUILD_DIR}/docs/" 2>/dev/null || true
    echo -e "${GREEN}✓ Documentation copied${NC}"
else
    echo -e "${YELLOW}⚠ No docs directory found${NC}"
fi
echo ""

# Step 11: Create version info
echo -e "${YELLOW}📝 Creating version info...${NC}"
cat > "${RELEASE_BUILD_DIR}/VERSION" << EOF
Build Date: $(date)
Git Commit: $(git -C "$PROJECT_ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
Branch: $(git -C "$PROJECT_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
EOF
echo -e "${GREEN}✓ Version info created${NC}"
echo ""

# Step 12: Create tarball
echo -e "${YELLOW}📦 Creating release tarball...${NC}"
cd "${RELEASE_DIR}"
TARBALL_NAME="orca_${BUILD_DATE}.tar.gz"
tar -czf "$TARBALL_NAME" "build_${BUILD_DATE}/"
echo -e "${GREEN}✓ Tarball created: ${TARBALL_NAME}${NC}"
echo ""

# Step 13: Create symlink to latest build
echo -e "${YELLOW}🔗 Creating symlink to latest build...${NC}"
cd "${RELEASE_DIR}"
# Remove old symlink if it exists
if [ -L "lastbuild" ]; then
    rm "lastbuild"
fi
# Create new symlink
ln -s "build_${BUILD_DATE}" "lastbuild"
echo -e "${GREEN}✓ Symlink created: ${RELEASE_DIR}/lastbuild → build_${BUILD_DATE}${NC}"
echo ""

# Summary
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ Build and Release Complete!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo "Release Location: ${RELEASE_BUILD_DIR}"
echo "Tarball: ${RELEASE_DIR}/${TARBALL_NAME}"
echo ""
echo "Contents:"
echo "  • Binaries: bin/orca, bin/aco, bin/orchestrator"
echo "  • Configuration: config/orca.toml.sample"
echo "  • Templates: templates/"
echo "  • Workflows: workflows/"
echo "  • Playground: playground/"
echo "  • Documentation: docs/"
echo "  • Version info: VERSION"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Quick access: ${RELEASE_DIR}/lastbuild/bin/"
echo "2. Review and test the release in ${RELEASE_BUILD_DIR}"
echo "3. Archive: tar -xzf ${RELEASE_DIR}/${TARBALL_NAME}"
echo "4. Install binaries:"
echo "   cp ${RELEASE_DIR}/lastbuild/bin/orca /usr/local/bin/"
echo "   cp ${RELEASE_DIR}/lastbuild/bin/aco /usr/local/bin/"
echo "   cp ${RELEASE_DIR}/lastbuild/bin/orchestrator /usr/local/bin/"
echo ""
echo -e "${YELLOW}Run commands:${NC}"
echo "   ./release/lastbuild/bin/orca"
echo "   ./release/lastbuild/bin/aco"
echo "   ./release/lastbuild/bin/orchestrator"
echo ""

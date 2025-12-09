#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_DIR="${PROJECT_ROOT}/release"
DIST_DIR="${RELEASE_DIR}/dist"
BUILD_NUMBER_FILE="${RELEASE_DIR}/.build_number"
MAX_VERSIONS=3

# Parse command line arguments
FULL_BUILD=false
SHOW_HELP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --full|-f)
            FULL_BUILD=true
            shift
            ;;
        --help|-h)
            SHOW_HELP=true
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            SHOW_HELP=true
            shift
            ;;
    esac
done

if [ "$SHOW_HELP" = true ]; then
    echo "Usage: $(basename $0) [OPTIONS]"
    echo ""
    echo "Build modes:"
    echo "  (default)     Incremental build - only builds changed code"
    echo "                Updates existing release in-place"
    echo ""
    echo "  --full, -f    Full release build - clean build from scratch"
    echo "                Creates new timestamped build directory"
    echo "                Increments build number"
    echo ""
    echo "Options:"
    echo "  --help, -h    Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./scripts/build-release.sh          # Quick incremental build"
    echo "  ./scripts/build-release.sh --full   # Full clean release build"
    exit 0
fi

# Get or create build number
get_build_number() {
    if [ -f "$BUILD_NUMBER_FILE" ]; then
        cat "$BUILD_NUMBER_FILE"
    else
        echo "0"
    fi
}

increment_build_number() {
    local current=$(get_build_number)
    local next=$((current + 1))
    echo "$next" > "$BUILD_NUMBER_FILE"
    echo "$next"
}

# Ensure release directory exists
mkdir -p "${RELEASE_DIR}"
mkdir -p "${DIST_DIR}"

if [ "$FULL_BUILD" = true ]; then
    #==========================================================================
    # FULL BUILD MODE
    #==========================================================================
    BUILD_DATE=$(date +%Y%m%d_%H%M%S)
    BUILD_NUM=$(increment_build_number)
    RELEASE_BUILD_DIR="${RELEASE_DIR}/build_${BUILD_DATE}"

    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}🚀 FULL RELEASE BUILD (Build #${BUILD_NUM})${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo "Project Root: $PROJECT_ROOT"
    echo "Release Directory: $RELEASE_BUILD_DIR"
    echo ""

    # Step 1: Clean old builds (keep only MAX_VERSIONS)
    echo -e "${YELLOW}🧹 Cleaning old builds (keeping ${MAX_VERSIONS} versions)...${NC}"
    cd "${RELEASE_DIR}"
    BUILD_COUNT=$(ls -d build_* 2>/dev/null | wc -l | tr -d ' ')
    if [ "$BUILD_COUNT" -gt "$MAX_VERSIONS" ]; then
        BUILDS_TO_DELETE=$((BUILD_COUNT - MAX_VERSIONS))
        ls -dt build_* | tail -n "$BUILDS_TO_DELETE" | xargs rm -rf
        echo -e "${GREEN}✓ Removed ${BUILDS_TO_DELETE} old build directories${NC}"
    else
        echo -e "${GREEN}✓ No old build directories to remove${NC}"
    fi

    # Clean old tarballs in dist
    cd "${DIST_DIR}"
    TARBALL_COUNT=$(ls orca_*.tar.gz 2>/dev/null | wc -l | tr -d ' ')
    if [ "$TARBALL_COUNT" -gt "$MAX_VERSIONS" ]; then
        TARBALLS_TO_DELETE=$((TARBALL_COUNT - MAX_VERSIONS))
        ls -t orca_*.tar.gz | tail -n "$TARBALLS_TO_DELETE" | xargs rm -f
        echo -e "${GREEN}✓ Removed ${TARBALLS_TO_DELETE} old tarballs${NC}"
    else
        echo -e "${GREEN}✓ No old tarballs to remove${NC}"
    fi
    echo ""

    # Step 2: Clean cargo release artifacts
    echo -e "${YELLOW}📦 Cleaning ALL cargo release artifacts...${NC}"
    cd "$PROJECT_ROOT"
    cargo clean --release 2>/dev/null || true
    echo -e "${GREEN}✓ Clean complete${NC}"
    echo ""

    # Step 3: Full build
    echo -e "${YELLOW}🏗️  Building all binaries in release mode (full rebuild)...${NC}"
    cargo build -p orca -p aco -p orchestrator -p orca_install --release
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Build successful${NC}"
    else
        echo -e "${RED}✗ Build failed${NC}"
        exit 1
    fi
    echo ""

    # Step 4: Create release directory structure
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

    # Step 5: Copy binaries
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

    # Copy orchestrator-server
    ORCHESTRATOR_SRC="${PROJECT_ROOT}/target/release/orchestrator-server"
    if [ -f "$ORCHESTRATOR_SRC" ]; then
        cp "$ORCHESTRATOR_SRC" "${RELEASE_BUILD_DIR}/bin/"
        chmod +x "${RELEASE_BUILD_DIR}/bin/orchestrator-server"
        echo -e "${GREEN}✓ Binary copied: orchestrator-server${NC}"
    else
        echo -e "${YELLOW}⚠ Orchestrator-server binary not found${NC}"
    fi

    # Copy orca_install
    INSTALL_SRC="${PROJECT_ROOT}/target/release/orca_install"
    if [ -f "$INSTALL_SRC" ]; then
        cp "$INSTALL_SRC" "${RELEASE_BUILD_DIR}/bin/"
        chmod +x "${RELEASE_BUILD_DIR}/bin/orca_install"
        echo -e "${GREEN}✓ Binary copied: orca_install${NC}"
    else
        echo -e "${YELLOW}⚠ orca_install binary not found${NC}"
    fi
    echo ""

    # Step 5.5: Run orca_install to reset/recreate user database from config
    echo -e "${YELLOW}🔧 Running orca_install to reset database...${NC}"
    cd "$PROJECT_ROOT"
    "${RELEASE_BUILD_DIR}/bin/orca_install" --force reset orca
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ orca_install completed${NC}"
    else
        echo -e "${YELLOW}⚠ orca_install had issues (non-fatal)${NC}"
    fi
    echo ""

    # Step 6: Copy templates
    echo -e "${YELLOW}📚 Copying templates...${NC}"
    if [ -d "${PROJECT_ROOT}/templates" ]; then
        cp -r "${PROJECT_ROOT}/templates"/* "${RELEASE_BUILD_DIR}/templates/" 2>/dev/null || true
        echo -e "${GREEN}✓ Templates copied${NC}"
    else
        echo -e "${YELLOW}⚠ No templates directory found${NC}"
    fi
    echo ""

    # Step 7: Copy workflows
    echo -e "${YELLOW}⚙️  Copying workflows...${NC}"
    if [ -d "${PROJECT_ROOT}/workflows" ]; then
        cp -r "${PROJECT_ROOT}/workflows"/* "${RELEASE_BUILD_DIR}/workflows/" 2>/dev/null || true
        echo -e "${GREEN}✓ Workflows copied${NC}"
    else
        echo -e "${YELLOW}⚠ No workflows directory found${NC}"
    fi
    echo ""

    # Step 8: Copy playground
    echo -e "${YELLOW}🎮 Copying playground...${NC}"
    if [ -d "${PROJECT_ROOT}/playground" ]; then
        cp -r "${PROJECT_ROOT}/playground"/* "${RELEASE_BUILD_DIR}/playground/" 2>/dev/null || true
        echo -e "${GREEN}✓ Playground copied${NC}"
    else
        echo -e "${YELLOW}⚠ No playground directory found${NC}"
    fi
    echo ""

    # Step 9: Create sample config
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

    # Step 10: Create README for release
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
EOF
    echo -e "${GREEN}✓ Release README created${NC}"
    echo ""

    # Step 11: Copy documentation
    echo -e "${YELLOW}📚 Copying documentation...${NC}"
    if [ -d "${PROJECT_ROOT}/docs" ]; then
        cp -r "${PROJECT_ROOT}/docs"/* "${RELEASE_BUILD_DIR}/docs/" 2>/dev/null || true
        echo -e "${GREEN}✓ Documentation copied${NC}"
    else
        echo -e "${YELLOW}⚠ No docs directory found${NC}"
    fi
    echo ""

    # Step 12: Create version info
    echo -e "${YELLOW}📝 Creating version info...${NC}"
    cat > "${RELEASE_BUILD_DIR}/VERSION" << EOF
Build Number: ${BUILD_NUM}
Build Date: $(date)
Git Commit: $(git -C "$PROJECT_ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
Branch: $(git -C "$PROJECT_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
Build Type: Full Release
EOF
    echo -e "${GREEN}✓ Version info created${NC}"
    echo ""

    # Step 13: Create tarball in dist directory
    echo -e "${YELLOW}📦 Creating release tarball...${NC}"
    cd "${RELEASE_DIR}"
    TARBALL_NAME="orca_${BUILD_DATE}.tar.gz"
    tar -czf "${DIST_DIR}/${TARBALL_NAME}" "build_${BUILD_DATE}/"
    echo -e "${GREEN}✓ Tarball created: dist/${TARBALL_NAME}${NC}"
    echo ""

    # Step 14: Create symlink to latest build
    echo -e "${YELLOW}🔗 Creating symlink to latest build...${NC}"
    cd "${RELEASE_DIR}"
    if [ -L "lastbuild" ]; then
        rm "lastbuild"
    fi
    ln -s "build_${BUILD_DATE}" "lastbuild"
    echo -e "${GREEN}✓ Symlink created: ${RELEASE_DIR}/lastbuild → build_${BUILD_DATE}${NC}"
    echo ""

    # Summary
    echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}✓ Full Release Build #${BUILD_NUM} Complete!${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Release Location: ${RELEASE_BUILD_DIR}"
    echo "Tarball: ${DIST_DIR}/${TARBALL_NAME}"
    echo ""
    echo "Contents:"
    echo "  • Binaries: bin/orca, bin/aco, bin/orchestrator-server, bin/orca_install"
    echo "  • Configuration: config/orca.toml.sample"
    echo "  • Templates: templates/"
    echo "  • Workflows: workflows/"
    echo "  • Playground: playground/"
    echo "  • Documentation: docs/"
    echo "  • Version info: VERSION"
    echo ""

else
    #==========================================================================
    # INCREMENTAL BUILD MODE (default)
    #==========================================================================
    BUILD_NUM=$(get_build_number)

    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}⚡ INCREMENTAL BUILD (Build #${BUILD_NUM})${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo "Project Root: $PROJECT_ROOT"
    echo "Only building changed files..."
    echo ""

    # Step 1: Incremental build (no clean)
    echo -e "${YELLOW}🏗️  Building changed binaries in release mode...${NC}"
    cd "$PROJECT_ROOT"
    cargo build -p orca -p aco -p orchestrator -p orca_install --release
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Build successful${NC}"
    else
        echo -e "${RED}✗ Build failed${NC}"
        exit 1
    fi
    echo ""

    # Step 2: Ensure lastbuild directory exists
    LASTBUILD_DIR="${RELEASE_DIR}/lastbuild"
    if [ ! -d "$LASTBUILD_DIR" ] && [ ! -L "$LASTBUILD_DIR" ]; then
        echo -e "${YELLOW}⚠ No existing release found. Creating new release directory...${NC}"
        mkdir -p "${LASTBUILD_DIR}/bin"
        mkdir -p "${LASTBUILD_DIR}/config"
        mkdir -p "${LASTBUILD_DIR}/docs"
    fi

    # Resolve symlink if it exists
    if [ -L "$LASTBUILD_DIR" ]; then
        ACTUAL_BUILD_DIR=$(readlink -f "$LASTBUILD_DIR")
    else
        ACTUAL_BUILD_DIR="$LASTBUILD_DIR"
    fi

    # Step 3: Update binaries in-place
    echo -e "${YELLOW}📋 Updating binaries...${NC}"

    # Copy orca
    ORCA_SRC="${PROJECT_ROOT}/target/release/orca"
    if [ -f "$ORCA_SRC" ]; then
        mkdir -p "${ACTUAL_BUILD_DIR}/bin"
        cp "$ORCA_SRC" "${ACTUAL_BUILD_DIR}/bin/"
        chmod +x "${ACTUAL_BUILD_DIR}/bin/orca"
        echo -e "${GREEN}✓ Updated: orca${NC}"
    else
        echo -e "${RED}✗ Orca binary not found${NC}"
        exit 1
    fi

    # Copy aco
    ACO_SRC="${PROJECT_ROOT}/target/release/aco"
    if [ -f "$ACO_SRC" ]; then
        cp "$ACO_SRC" "${ACTUAL_BUILD_DIR}/bin/"
        chmod +x "${ACTUAL_BUILD_DIR}/bin/aco"
        echo -e "${GREEN}✓ Updated: aco${NC}"
    else
        echo -e "${YELLOW}⚠ Aco binary not found${NC}"
    fi

    # Copy orchestrator-server
    ORCHESTRATOR_SRC="${PROJECT_ROOT}/target/release/orchestrator-server"
    if [ -f "$ORCHESTRATOR_SRC" ]; then
        cp "$ORCHESTRATOR_SRC" "${ACTUAL_BUILD_DIR}/bin/"
        chmod +x "${ACTUAL_BUILD_DIR}/bin/orchestrator-server"
        echo -e "${GREEN}✓ Updated: orchestrator-server${NC}"
    else
        echo -e "${YELLOW}⚠ Orchestrator-server binary not found${NC}"
    fi

    # Copy orca_install
    INSTALL_SRC="${PROJECT_ROOT}/target/release/orca_install"
    if [ -f "$INSTALL_SRC" ]; then
        cp "$INSTALL_SRC" "${ACTUAL_BUILD_DIR}/bin/"
        chmod +x "${ACTUAL_BUILD_DIR}/bin/orca_install"
        echo -e "${GREEN}✓ Updated: orca_install${NC}"
    else
        echo -e "${YELLOW}⚠ orca_install binary not found${NC}"
    fi
    echo ""

    # Step 3.5: Run orca_install to reset/recreate user database from config
    echo -e "${YELLOW}🔧 Running orca_install to reset database...${NC}"
    "${ACTUAL_BUILD_DIR}/bin/orca_install" --force reset orca
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ orca_install completed${NC}"
    else
        echo -e "${YELLOW}⚠ orca_install had issues (non-fatal)${NC}"
    fi
    echo ""

    # Step 4: Update VERSION file with incremental info
    echo -e "${YELLOW}📝 Updating version info...${NC}"
    cat > "${ACTUAL_BUILD_DIR}/VERSION" << EOF
Build Number: ${BUILD_NUM}
Last Updated: $(date)
Git Commit: $(git -C "$PROJECT_ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
Branch: $(git -C "$PROJECT_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
Build Type: Incremental
EOF
    echo -e "${GREEN}✓ Version info updated${NC}"
    echo ""

    # Summary
    echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}✓ Incremental Build Complete!${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Binaries updated in: ${ACTUAL_BUILD_DIR}/bin/"
    echo ""
fi

# Common output for both modes
echo -e "${YELLOW}Run commands:${NC}"
echo "   ./release/lastbuild/bin/orca"
echo "   ./release/lastbuild/bin/aco"
echo "   ./release/lastbuild/bin/orchestrator-server"
echo "   ./release/lastbuild/bin/orca_install"
echo ""
echo -e "${YELLOW}Build modes:${NC}"
echo "   ./scripts/build-release.sh          # Quick incremental"
echo "   ./scripts/build-release.sh --full   # Full clean release"
echo ""

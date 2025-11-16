#!/bin/bash

# rLangGraph Documentation Website Build Script
# This script helps build and serve the documentation website

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Functions
print_header() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  rLangGraph Documentation Website Builder${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

check_node() {
    if ! command -v node &> /dev/null; then
        print_error "Node.js is not installed. Please install Node.js 16+ to continue."
        exit 1
    fi

    NODE_VERSION=$(node --version)
    print_success "Node.js found: $NODE_VERSION"
}

check_npm() {
    if ! command -v npm &> /dev/null; then
        print_error "npm is not installed. Please install npm to continue."
        exit 1
    fi

    NPM_VERSION=$(npm --version)
    print_success "npm found: v$NPM_VERSION"
}

install_dependencies() {
    print_info "Installing dependencies..."

    if [ ! -d "node_modules" ]; then
        npm install
        print_success "Dependencies installed"
    else
        print_info "Dependencies already installed (use --clean to reinstall)"
    fi
}

build_css() {
    print_info "Building CSS..."
    npm run build
    print_success "CSS built successfully"
}

dev_mode() {
    print_info "Starting development mode (watch + serve)..."
    print_info "CSS will be rebuilt automatically on changes"
    print_info "Server will start on http://localhost:8000"
    echo ""

    # Start watch mode in background
    npm run dev &
    DEV_PID=$!

    # Give it a second to start
    sleep 2

    # Start server
    npm run serve

    # Clean up on exit
    kill $DEV_PID 2>/dev/null || true
}

serve_only() {
    print_info "Starting server on http://localhost:8000..."
    npm run serve
}

clean_build() {
    print_info "Cleaning build artifacts..."
    rm -rf node_modules dist package-lock.json
    print_success "Build artifacts cleaned"
}

# Main script
print_header

# Parse arguments
MODE="build"
while [[ $# -gt 0 ]]; do
    case $1 in
        --dev|-d)
            MODE="dev"
            shift
            ;;
        --serve|-s)
            MODE="serve"
            shift
            ;;
        --clean|-c)
            MODE="clean"
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --dev, -d      Development mode (watch CSS + serve)"
            echo "  --serve, -s    Serve only (no build)"
            echo "  --clean, -c    Clean build artifacts and rebuild"
            echo "  --help, -h     Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0              # Build production CSS"
            echo "  $0 --dev        # Development mode with watch and serve"
            echo "  $0 --serve      # Serve existing build"
            echo "  $0 --clean      # Clean and rebuild everything"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Execute based on mode
case $MODE in
    dev)
        check_node
        check_npm
        install_dependencies
        dev_mode
        ;;
    serve)
        print_info "Serving existing build..."
        serve_only
        ;;
    clean)
        clean_build
        check_node
        check_npm
        install_dependencies
        build_css
        print_success "Clean build completed!"
        ;;
    build)
        check_node
        check_npm
        install_dependencies
        build_css
        print_success "Production build completed!"
        echo ""
        print_info "To serve the website, run:"
        echo "  $0 --serve"
        echo "  or: npm run serve"
        ;;
esac

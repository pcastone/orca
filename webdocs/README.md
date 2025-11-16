# Orca Documentation Website

This directory contains the static HTML documentation website for Orca, Orchestrator, and ACO.

## Overview

The documentation website provides comprehensive guides, examples, and references for:

- **Orca**: Standalone orchestrator for local development
- **Orchestrator**: Distributed orchestration engine for production
- **ACO**: Terminal UI client for monitoring and execution

## Structure

```
webdocs/
├── index.html              # Landing page with overview
├── getting-started.html    # Quick start guide
├── orca.html              # Orca documentation
├── orchestrator.html      # Orchestrator documentation
├── aco.html               # ACO documentation
├── examples.html          # Code examples for all crates
└── assets/                # Static assets (if needed)
```

## Features

- **Responsive Design**: Built with Tailwind CSS for mobile-friendly layouts
- **Syntax Highlighting**: Code examples with highlight.js
- **Clean Navigation**: Consistent navigation across all pages
- **Comprehensive Coverage**: Setup, usage, configuration, and examples
- **No Build Step**: Pure HTML/CSS/JS using CDN resources

## Viewing the Documentation

### Option 1: Local HTTP Server (Recommended)

```bash
# Using Python 3
cd webdocs
python3 -m http.server 8000

# Or using Python 2
python -m SimpleHTTPServer 8000

# Visit http://localhost:8000 in your browser
```

### Option 2: File Protocol

Simply open `index.html` in your browser:

```bash
open webdocs/index.html  # macOS
xdg-open webdocs/index.html  # Linux
start webdocs/index.html  # Windows
```

## Technology Stack

- **Tailwind CSS 3.x**: Styling via CDN
- **Highlight.js 11.9**: Code syntax highlighting
- **Pure HTML/CSS/JS**: No build process required

## Pages

### index.html
Landing page with:
- Overview of all three tools
- Feature highlights
- Quick start section
- Navigation to detailed docs

### getting-started.html
Step-by-step setup guide:
- Prerequisites
- Installation
- Configuration
- First task creation

### orca.html
Complete Orca documentation:
- Overview and features
- Installation and setup
- CLI commands
- Agent patterns (ReAct, Plan-Execute, Reflection)
- Configuration examples

### orchestrator.html
Orchestrator documentation:
- Distributed architecture
- Database schema
- Deployment options
- API reference
- Production considerations

### aco.html
ACO TUI documentation:
- Terminal UI features
- Keyboard shortcuts
- Views and navigation
- Deployment scenarios
- Performance characteristics

### examples.html
Real-world examples:
- Orca task creation and execution
- Orchestrator setup and deployment
- ACO client usage
- Configuration examples
- Complete workflows

## Updating Documentation

To update the documentation:

1. Edit the relevant HTML file
2. Test locally using a web server
3. Commit changes to git
4. Deploy to production (if applicable)

## Deployment

### GitHub Pages

```bash
# The webdocs/ directory can be served directly via GitHub Pages
# Configure in repository settings: Settings → Pages → Source: /webdocs
```

### Static Hosting

Upload the entire `webdocs/` directory to any static hosting service:
- Netlify
- Vercel
- AWS S3 + CloudFront
- GitHub Pages
- GitLab Pages

### Nginx

```nginx
server {
    listen 80;
    server_name docs.example.com;
    root /path/to/orca/webdocs;
    index index.html;

    location / {
        try_files $uri $uri/ =404;
    }
}
```

## Contributing

To contribute to the documentation:

1. Follow the existing HTML structure and styling
2. Use Tailwind CSS utility classes for styling
3. Ensure code examples are syntax-highlighted
4. Test on multiple browsers
5. Keep navigation consistent across pages

## License

Same as the main Orca project (MIT OR Apache-2.0)

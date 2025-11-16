# Documentation Website Build Plan

## Overview
Build a modern documentation website using Tailwind CSS to showcase the LLM and Tooling crates.

## Goals
1. Create clean, professional documentation site
2. Use Tailwind CSS for styling
3. Document LLM providers (local and remote)
4. Document Tooling utilities and modules
5. Make it responsive and accessible
6. Include code examples with syntax highlighting

## Structure

```
webdoc/
├── index.html              # Homepage with overview
├── llm.html               # LLM documentation
├── tooling.html           # Tooling documentation
├── assets/
│   ├── css/
│   │   └── custom.css     # Custom styles (if needed)
│   └── js/
│       └── main.js        # Any interactive features
└── README.md
```

## Pages

### 1. Homepage (index.html)
- Project overview
- Quick navigation to LLM and Tooling docs
- Key features highlight
- Getting started section
- Links to GitHub, crates.io, etc.

### 2. LLM Documentation (llm.html)
Sections:
- Overview
- Local Providers
  - Ollama
  - llama.cpp
  - LM Studio
- Remote Providers
  - Claude (Anthropic)
  - OpenAI
  - Gemini (Google)
  - Grok (xAI)
  - Deepseek
  - OpenRouter
- Configuration
- Code examples for each provider
- Provider utilities (ping, fetch_models, use_model)

### 3. Tooling Documentation (tooling.html)
Sections:
- Overview
- Configuration Management
- Error Handling
- Async Utilities (retry, timeout)
- Validation
- Serialization
- Rate Limiting
- Logging
- Code examples for each module

## Design Elements

### Layout
- Fixed sidebar navigation
- Main content area with sections
- Sticky header
- Footer with links

### Color Scheme (Tailwind)
- Primary: Blue (slate/blue palette)
- Accent: Indigo
- Code blocks: Dark theme (gray-900 bg)
- Success: Green
- Warning: Amber

### Components
- Navigation menu
- Code blocks with copy button
- Section headers with anchor links
- Cards for features
- Tables for reference
- Responsive grid layouts

## Technical Stack
- HTML5
- Tailwind CSS (via CDN for simplicity)
- Prism.js or Highlight.js for syntax highlighting
- Vanilla JavaScript for interactivity

## Tasks Checklist
- [ ] Set up base HTML structure with Tailwind CSS CDN
- [ ] Create navigation and layout components
- [ ] Build LLM documentation page
- [ ] Build Tooling documentation page
- [ ] Create homepage/index
- [ ] Add responsive design
- [ ] Add syntax highlighting
- [ ] Test all pages
- [ ] Commit changes

## Notes
- Keep it simple and maintainable
- Focus on clarity and readability
- Ensure mobile responsiveness
- Use semantic HTML
- Optimize for fast loading

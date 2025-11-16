# acolib Documentation Website

Modern documentation website for the acolib Rust project, specifically covering the **LLM** and **Tooling** crates.

## Overview

This documentation website provides comprehensive guides and examples for:

- **LLM Crate**: Multi-provider LLM integrations (local and remote providers)
- **Tooling Crate**: Production-ready utilities for configuration, validation, async operations, and more

## Features

- **Modern Design**: Built with Tailwind CSS for a clean, professional look
- **Responsive**: Mobile-friendly layout that works on all devices
- **Syntax Highlighting**: Code examples with Prism.js syntax highlighting
- **Interactive**: Copy-to-clipboard for code snippets, smooth scrolling navigation
- **Accessible**: Keyboard navigation and semantic HTML

## Structure

```
webdocs/
├── index.html              # Homepage with project overview
├── llm.html               # LLM crate documentation
├── tooling.html           # Tooling crate documentation
├── assets/
│   ├── css/               # Custom CSS (if needed)
│   └── js/
│       └── main.js        # Interactive features
└── README.md              # This file
```

## Pages

### Homepage (`index.html`)
- Project overview and key features
- Quick links to LLM and Tooling documentation
- Getting started guide with installation and examples
- Build commands and usage instructions

### LLM Documentation (`llm.html`)
Complete documentation for the LLM crate including:
- **Local Providers**: Ollama, llama.cpp, LM Studio
- **Remote Providers**: Claude, OpenAI, Gemini, Grok, Deepseek, OpenRouter
- **Advanced Features**: Thinking models with reasoning, provider utilities
- **Configuration**: API keys, timeouts, retry logic, and more

### Tooling Documentation (`tooling.html`)
Complete documentation for the Tooling crate including:
- **Configuration Management**: Environment variable loading, config builders
- **Error Handling**: Error context and chain formatting
- **Async Utilities**: Retry policies, timeouts, exponential backoff
- **Validation**: Fluent validation API with chainable rules
- **Serialization**: Stable JSON serialization and hashing
- **Rate Limiting**: Token bucket and sliding window limiters
- **Logging**: Structured logging with timing and formatting

## Technology Stack

- **HTML5**: Semantic markup
- **Tailwind CSS**: Utility-first CSS framework (via CDN)
- **Prism.js**: Syntax highlighting for code blocks
- **Vanilla JavaScript**: Interactive features and navigation

## Features

### Navigation
- Sticky header with main navigation
- Sidebar navigation on documentation pages (desktop)
- Smooth scrolling to sections
- Active link highlighting based on scroll position

### Code Blocks
- Syntax highlighting for Rust, TOML, and Bash
- Copy-to-clipboard button on all code blocks
- Visual feedback when code is copied

### Keyboard Shortcuts
- `Alt + ←`: Previous page
- `Alt + →`: Next page

## Development

### Viewing Locally

To view the documentation locally, simply open any HTML file in a web browser:

```bash
# Using Python
cd webdocs
python3 -m http.server 8000
# Open http://localhost:8000

# Using Node.js
npx http-server webdocs -p 8000
# Open http://localhost:8000
```

### Adding New Content

1. **Update HTML files**: Add new sections to the appropriate HTML file
2. **Update sidebar navigation**: Add links to new sections in the sidebar
3. **Add code examples**: Use Prism.js supported language classes
4. **Test responsiveness**: Verify on mobile and desktop viewports

### Code Example Format

```html
<pre class="bg-gray-900 text-gray-100 p-4 rounded-lg overflow-x-auto">
<code class="language-rust">
// Your Rust code here
fn main() {
    println!("Hello, world!");
}
</code>
</pre>
```

## Browser Support

- Chrome/Edge (latest)
- Firefox (latest)
- Safari (latest)
- Mobile browsers (iOS Safari, Chrome Mobile)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

## Contributing

When updating documentation:

1. Maintain consistent styling and structure
2. Test all code examples for correctness
3. Ensure responsive design works on mobile
4. Update the sidebar navigation for new sections
5. Keep examples simple and well-commented

## Deployment

The documentation can be deployed to:

- **GitHub Pages**: Push to `gh-pages` branch
- **Netlify**: Connect GitHub repo and deploy
- **Vercel**: Import GitHub repo and deploy
- **Any static hosting**: Upload the `webdocs/` directory

### GitHub Pages Deployment

```bash
# From project root
git subtree push --prefix webdocs origin gh-pages
```

## Links

- [GitHub Repository](https://github.com/pcastone/orca)
- [Rust Documentation](https://doc.rust-lang.org/)
- [Tailwind CSS](https://tailwindcss.com/)
- [Prism.js](https://prismjs.com/)

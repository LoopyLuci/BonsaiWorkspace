# Omnisystem Polyglot - VS Code Extension

**Unified development environment for 1000+ programming languages**

## Features

### 🌍 750+ Language Support
- Full language support across 750+ programming languages
- Intelligent syntax highlighting for all languages
- Language detection from code snippets
- Seamless switching between languages

### 🔗 Language Chain Visualization
- View the complete language chain (Assembly → Modern Languages)
- Understand language relationships and evolution
- Navigate between related languages
- Historical context for each language

### 🎯 Multi-Language Execution
- Execute code in any supported language
- Integrated terminal with language-specific runners
- Cross-language code conversion
- Batch execution across languages

### 📦 Module Marketplace
- Browse 1000+ polyglot modules
- One-click installation
- Rating and download tracking
- Search by language, category, or keywords

### 🔄 Code Conversion
- Convert code between any two languages
- Intelligent syntax mapping
- Preserve logic and structure
- Preview conversion results

### 📚 Documentation
- Integrated language documentation
- Quick reference guides
- Code examples for all languages
- Links to official language resources

### 🐛 Debugging
- Debugger support for supported languages
- Breakpoints and stepping
- Variable inspection
- Async debugging support

## Installation

1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "Omnisystem Polyglot"
4. Click Install

Or install from command line:
```bash
code --install-extension omnisystem.omnisystem-polyglot
```

## Quick Start

### Select a Language
- Press `Ctrl+Shift+P` (Windows/Linux) or `Cmd+Shift+P` (Mac)
- Type "Omnisystem: Select Language"
- Choose from 750+ languages

### Execute Code
- Write code in any language
- Press `Ctrl+Shift+Enter` to execute
- Results appear in integrated terminal

### Convert Code
- Select code in editor
- Press `Ctrl+Shift+C`
- Choose source and target languages
- View converted code

### Browse Marketplace
- Press `Ctrl+Shift+P`
- Type "Omnisystem: Search Module Marketplace"
- Browse and install modules

## Commands

| Command | Shortcut | Description |
|---------|----------|-------------|
| Select Language | Ctrl+Shift+P | Choose language for current file |
| Execute Polyglot | Ctrl+Shift+Enter | Run code in selected language |
| Show Language Chain | Ctrl+Shift+L | View language relationships |
| Search Marketplace | Ctrl+Shift+P | Find polyglot modules |
| Convert Language | Ctrl+Shift+C | Convert between languages |
| Show Documentation | Ctrl+Shift+P | View language docs |

## Settings

Configure Omnisystem in VS Code settings:

```json
{
  "omnisystem.enableAutoComplete": true,
  "omnisystem.enableLinting": true,
  "omnisystem.enableFormatting": true,
  "omnisystem.defaultLanguage": "assembly",
  "omnisystem.marketplaceUrl": "https://marketplace.omnisystem.dev",
  "omnisystem.enableDebugger": true
}
```

## Supported Languages

- **Foundation**: Assembly, FORTRAN, COBOL, C, C++, Pascal, Ada
- **Scientific**: MATLAB, R, Julia, Haskell, OCaml, Rust, Go
- **Enterprise**: Java, Python, JavaScript, C#, Ruby, PHP
- **Modern**: TypeScript, Kotlin, Swift, Dart, Zig, Nim
- **Emerging**: Solidity, Move, Cadence, Q#, Cairo, Noir
- **And 700+ more...**

## Tips & Tricks

1. **Fast Language Switching**: Use the language dropdown in the status bar
2. **Batch Conversion**: Select multiple files and convert all at once
3. **Marketplace Search**: Filter by rating, downloads, or language support
4. **Smart Completion**: Enable AI-powered completion for all languages
5. **Performance**: Use `omnisystem.enableLinting: false` on slow machines

## Troubleshooting

### Extension not activating
- Ensure you have the latest version installed
- Restart VS Code
- Check that you have Node.js 14+ installed

### Execution failing
- Verify the language is installed on your system
- Check marketplace for required dependencies
- See language documentation for setup requirements

### Performance issues
- Disable linting for large files
- Reduce syntax highlighting for complex code
- Check CPU usage in VS Code settings

## Support

- 📖 [Documentation](https://docs.omnisystem.dev)
- 🐛 [Report Issues](https://github.com/omnisystem/vscode-polyglot/issues)
- 💬 [Community Chat](https://discord.gg/omnisystem)
- 📧 [Email Support](mailto:support@omnisystem.dev)

## License

MIT License - See LICENSE file for details

## Contributing

We welcome contributions! Please see CONTRIBUTING.md for guidelines.

---

**Omnisystem Polyglot v1.0.0** — *Unify your polyglot development*

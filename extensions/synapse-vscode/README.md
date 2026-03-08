# Synapse (.mnm) Syntax Highlighter

Syntax highlighting for the Synapse memory configuration language (`.mnm` files).

## Installation

### From workspace (development)

1. Open the Synapse project in VS Code/Cursor
2. Open `extensions/synapse-vscode` in the editor
3. Press `F5` or Run > Start Debugging to launch a new window with the extension loaded
4. Open any `.mnm` file to see syntax highlighting

### Install locally (packaged)

```bash
cd extensions/synapse-vscode
npm install
vsce package
code --install-extension synapse-mnm-0.1.0.vsix
```

## Features

- **Comments**: `#` line comments
- **Keywords**: `config`, `memory`, `namespace`, `query`, `update`, `on`, `fn`, `policy`, `if`, `else`, `let`, `for`, `in`, etc.
- **Types**: `string`, `int`, `float`, `bool`, `timestamp`
- **Literals**: numbers, durations (`24h`, `7d`), booleans, `null`
- **Strings**: double- and single-quoted
- **Decorators**: `@extern`, `@index`, `@invariant`
- **Built-ins**: `store`, `now`, `supersede`, `discard`, `emit`, `extract`, `map`, `filter`, etc.
- **Operators**: `|>`, `->`, `=>`, `==`, `!=`, arithmetic

## Language configuration

- Line comment: `#`
- Bracket matching for `{}`, `[]`, `()`
- Auto-closing pairs for quotes and brackets

<p align="center">
  <h1 align="center">
    <code>sprocket-zed</code>
  </h1>

  <p align="center">
    <a href="https://github.com/stjude-rust-labs/sprocket-zed/blob/main/LICENSE-APACHE"><img alt="Apache 2.0" src="https://img.shields.io/badge/license-Apache%202.0-blue.svg"></a>
    <a href="https://github.com/stjude-rust-labs/sprocket-zed/blob/main/LICENSE-MIT"><img alt="MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  </p>

  <p align="center">
    <a href="https://openwdl.org/">WDL</a> language support for <a href="https://zed.dev/">Zed</a> via the <a href="https://github.com/stjude-rust-labs/sprocket">Sprocket</a> LSP.
    <br />
    <br />
    <a href="https://github.com/stjude-rust-labs/sprocket-zed/issues/new?labels=enhancement">Request Feature</a>
    ·
    <a href="https://github.com/stjude-rust-labs/sprocket-zed/issues/new?labels=bug">Report Bug</a>
  </p>
</p>

## 🏠 Overview

`sprocket-zed` provides comprehensive WDL support for the Zed editor, powered by
the [Sprocket](https://github.com/stjude-rust-labs/sprocket) language server. 

## 🎨 Features

- **LSP Integration.** Completions, diagnostics, hover documentation,
  go-to-definition, and references powered by Sprocket.
- **Syntax Highlighting.** Full support for WDL 1.0 through 1.3 via
  [tree-sitter-wdl](https://github.com/stjude-rust-labs/tree-sitter-wdl).
- **Bash Injection.** Command blocks are highlighted as bash.
- **Auto-Download.** Automatically downloads the latest Sprocket release if not
  found on `PATH`.

## 📚 Getting Started

### Installation

1. Clone this repository:

   ```bash
   git clone https://github.com/stjude-rust-labs/sprocket-zed.git
   ```

2. In Zed, open the command palette (`Cmd+Shift+P`) and run **zed: install dev
   extension**.

3. Select the cloned `sprocket-zed` directory.

4. When prompted, trust the worktree to start the language server.

The extension will automatically download the latest
[Sprocket](https://github.com/stjude-rust-labs/sprocket/releases) release. To
use a locally installed binary instead, ensure `sprocket` is on your `PATH` or
set the `binaryPath` option below.

### Configuration

Configure the extension in your Zed `settings.json` under the `lsp.sprocket`
key:

```json
{
  "lsp": {
    "sprocket": {
      "binaryPath": "/path/to/sprocket",
      "checkForUpdates": true,
      "server": {
        "logLevel": "info",
        "lint": {
          "enabled": true
        }
      }
    }
  }
}
```

| Setting               | Type     | Default   | Description                                                                 |
|-----------------------|----------|-----------|-----------------------------------------------------------------------------|
| `binaryPath`          | `string` | —         | Path to a locally installed Sprocket binary                                 |
| `checkForUpdates`     | `bool`   | `true`    | Check for new Sprocket releases on startup                                  |
| `server.logLevel`     | `string` | `"error"` | Server output level: `"error"`, `"warn"`, `"info"`, `"debug"`, or `"trace"` |
| `server.lint.enabled` | `bool`   | `false`   | Enable additional linting checks                                            |

## 📝 License and Legal

This project is licensed as either [Apache 2.0][license-apache] or
[MIT][license-mit] at your discretion. Additionally, please see [the
disclaimer](https://github.com/stjude-rust-labs#disclaimer) that applies to all
crates and command line tools made available by St. Jude Rust Labs.

Copyright © 2026-Present [St. Jude Children's Research Hospital](https://github.com/stjude).

[license-apache]: https://github.com/stjude-rust-labs/sprocket-zed/blob/main/LICENSE-APACHE
[license-mit]: https://github.com/stjude-rust-labs/sprocket-zed/blob/main/LICENSE-MIT

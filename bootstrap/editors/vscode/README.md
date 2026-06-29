# Jazyk for VS Code

Language support for [Jazyk](https://jazyk.org) — natural language as a programming language.
The extension is a thin client: it launches `jazyk lsp` and relays LSP traffic, so all analysis
(diagnostics, go-to-definition, find-references, hover, completion) comes from the compiler.

## Requirements

The `jazyk` binary must be on your `PATH`, or set `jazyk.server.path` in settings.

```sh
cd ../..        # the bootstrap crate
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

## Build & run the extension

```sh
npm install
npm run compile
```

Then press <kbd>F5</kbd> in VS Code to launch an Extension Development Host, and open a folder
containing a `jazyk.toml`.

## Settings

- `jazyk.server.path` — path to the `jazyk` binary (default: `jazyk` on `PATH`).
- `jazyk.llm.baseUrl` — override the LLM base URL (`--llm-base-url`).
- `jazyk.llm.model` — override the LLM model (`--model`).

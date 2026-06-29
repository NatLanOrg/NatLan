# VS Code

A small VS Code extension makes the [language server](../lsp.md#language-server) available in the
editor. The extension does no analysis itself: it launches `jazyk lsp` and forwards LSP traffic using
the [`vscode-languageclient`](https://github.com/microsoft/vscode-languageserver-node) library.

## Anatomy

- `package.json`
  - `contributes.languages`. Declares the Jazyk language id and the files it applies to. Jazyk
    documentation is plain Markdown, so binding by file extension alone (`.md`) would capture every
    Markdown file. Bind instead to files inside a Jazyk project (a folder containing `jazyk.toml`), or
    to a dedicated language id the user opts into, so the server only activates where it makes sense.
  - `activationEvents`. Activate on the Jazyk language and on the presence of a `jazyk.toml` in the
    workspace (e.g. `onLanguage:jazyk`, `workspaceContains:**/jazyk.toml`).
  - `contributes.configuration`. Settings for the server path and the
    [LLM overrides](../../compiler/project-settings.md#llm) (`--llm-base-url`, `--model`).
- Extension entry point (`activate` / `deactivate`)
  - Build `ServerOptions` that run the binary over stdio:

    ```ts
    const serverOptions: ServerOptions = {
      run:   { command: jazykPath, args: ['lsp', '--stdio'], transport: TransportKind.stdio },
      debug: { command: jazykPath, args: ['lsp', '--stdio'], transport: TransportKind.stdio },
    };
    ```
  - Build `LanguageClientOptions` with a `documentSelector` for the Jazyk documents.
  - `const client = new LanguageClient('jazyk', 'Jazyk', serverOptions, clientOptions);` then
    `await client.start();`. `deactivate` stops the client so the server process exits.

## Locating the binary

The extension resolves the `jazyk` binary in this order: an explicit path from settings, then the
`PATH`, then (optionally) a binary bundled with the extension. This mirrors how the rust-analyzer
extension bundles or discovers its server. The chosen path becomes `command` in `ServerOptions`.

## Behavior

Once started, the [capabilities](../capabilities/diagnostics.md#diagnostics) appear natively:
diagnostics in the Problems panel and inline, go to definition and find references on entities, hover,
and completion. The [lifecycle](../lifecycle.md#lifecycle) (project discovery, incremental recompile,
cancellation) is handled by the server; the extension only relays.

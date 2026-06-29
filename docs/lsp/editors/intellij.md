# IntelliJ

JetBrains IDEs run the [language server](../lsp.md#language-server) through a plugin that launches
`jazyk lsp` and maps LSP features onto the IDE.

## Via LSP4IJ

We target [LSP4IJ](https://github.com/redhat-developer/lsp4ij), an open LSP client plugin for IntelliJ.
LSP4IJ works in all JetBrains IDEs, including the free IntelliJ IDEA Community Edition, and lets a
language server be declared without writing a custom plugin.

A language server is registered by pointing LSP4IJ at the command to run and the files to associate:

- Server definition. The command line `jazyk lsp --stdio` (with the `jazyk` binary resolved from a
  configured path or `PATH`).
- File association. The Jazyk documents in a project (a folder containing `jazyk.toml`), so the server
  is not started for unrelated Markdown.

This can be configured by the user from the LSP4IJ settings, or shipped as a small plugin that declares
the server definition so users get it preconfigured.

## Why not the native LSP API

JetBrains also ships a native [LSP API](https://plugins.jetbrains.com/docs/intellij/language-server-protocol.html)
(`LspServerSupportProvider` + `LspServerDescriptor`). It is available only in the commercial IDEs
(IntelliJ IDEA Ultimate, WebStorm, GoLand, and so on) and not in Community Edition or Android Studio.
We prefer LSP4IJ so the integration also works in the free IDE. If we later ship a paid-IDE plugin, the
native API is the equivalent path: implement a provider whose descriptor runs `jazyk lsp`.

## Behavior

Either path launches the same server over [stdio](../transport.md#stdio), so the
[capabilities](../capabilities/diagnostics.md#diagnostics) and
[lifecycle](../lifecycle.md#lifecycle) are identical to the [VS Code](./vscode.md#vs-code)
integration; only the client wiring differs.

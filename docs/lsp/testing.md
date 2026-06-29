# Testing

Three ways to exercise the [language server](./lsp.md#language-server), from fastest to most
realistic.

## By hand over stdio

The server speaks JSON-RPC over [stdio](./transport.md#stdio), so it can be driven without an editor.
Start `jazyk lsp --stdio` and send a scripted session:

1. `initialize` with the workspace root, then `initialized`.
2. `textDocument/didOpen` for a document in the fixture project.
3. Expect `textDocument/publishDiagnostics` for that document.
4. `textDocument/hover` / `definition` / `completion` at known positions and check the responses.

This is the tightest loop for developing the protocol layer and is the easiest to assert on in an
automated test, since it is just request and response. Mock or point `--llm-base-url` at a local model
so the LLM stages are deterministic and offline.

## Fixture project

Tests run against a small Jazyk project committed for the purpose: a `jazyk.toml` plus a couple of
Markdown documents that together produce a known graph and at least one deliberate
[`missing-link`](../compiler/linking/resolve-entities.md#resolve-entities) so diagnostics, hover, and
completion all have something to show. The same fixture is reused by every editor below.

## In VS Code

Run the [extension](./editors/vscode.md#vs-code) in the Extension Development Host and open the fixture
project. Verify diagnostics appear in the Problems panel, go to definition jumps to the defining
section, hover shows the entity, and completion offers existing entities. This confirms the extension
wiring (binary discovery, `documentSelector`, activation) on top of the protocol.

## In IntelliJ

Load the server through [LSP4IJ](./editors/intellij.md#via-lsp4ij) against the same fixture and check
the same capabilities. Because every client launches the same `jazyk lsp` process, a behavior
difference between editors points at client wiring, not at the server.

# Language Server

The Language Server is the IDE frontend over the [compiler](./compiler.md#compiler). It implements
the [Language Server Protocol](https://en.wikipedia.org/wiki/Language_Server_Protocol) so an editor
can author Jazyk documentation with live feedback.

Like the [CLI](./cli.md#cli) and the [MCP server](./mcp.md#mcp), it is a thin frontend that embeds
the compiler library.

## Definition

The server is started by the editor as `jazyk lsp` [CLI](./cli.md#cli)) and speaks LSP over
stdio. It watches a Jazyk project and recompiles
[incrementally](./compiler/concepts/incremental.md#incrementality) as files change, so feedback tracks
edits without rebuilding the whole project.

### Capabilities

- **[Diagnostics](./compiler/model/diagnostic.md#diagnostic)**: Surface compiler warnings and errors inline at the
  relevant section, with related sections linked.
- **[Definition and references](./lsp/capabilities/definition.md#definition-and-references)**: Navigate the
  [entity](./compiler/model/entity.md#entity) graph: jump from a reference to the entity's
  defining section, or list everything that relates to an entity.
- **[Hover](./lsp/capabilities/hover.md#hover)**: mouse over an entity's
  [definition](./lsp/capabilities/definition.md#definition-and-references), its
  [relationships](./compiler/model/relationship.md#relationship), and its
  [diagnostics](./compiler/model/diagnostic.md#diagnostic).
- **[Completion](./lsp/capabilities/completion.md#completion)**. Suggest existing entities when authoring
  a cross-reference, to reduce [missing links](./compiler/linking/resolve-entities.md#resolve-entities).

## Internals

- [Lifecycle](./lsp/lifecycle.md#lifecycle). Project discovery, initialization, file sync, incremental
  recompile, and cancellation.
- [Transport](./lsp/transport.md#transport). Communication between editor and Jazyk via stdio JSON-RPC.

## Editors

- [VS Code](./lsp/editors/vscode.md#vs-code)
- [IntelliJ](./lsp/editors/intellij.md#intellij)

## Testing

- [Testing](./lsp/testing.md#testing). Driving the server by hand and through each editor.

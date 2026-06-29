# Coding agents

Coding agents (opencode, Codex, Claude Code, and similar) consume Jazyk the way an editor does, but
*headless*: there is no GUI, no mouse hover, no Problems panel. Every
[diagnostic](../capabilities/diagnostics.md#diagnostics),
[definition](../capabilities/definition.md#definition-and-references), or
[hover](../capabilities/hover.md#hover) becomes text in the model's context or a tool result. That
shifts what matters from an editor's priorities to an agent's: the **edit, check, fix loop** (make a
change, ask for diagnostics, repair until clean) and **navigation as cheap search** (resolve an entity
across files without reading every document).

An agent reaches Jazyk three ways. Which one applies depends on the agent, not on Jazyk: the same
`jazyk lsp` and the same [MCP server](../../mcp.md#mcp) serve all of them.

## Integration modes

### Natively over LSP

The agent is itself an LSP client. It spawns `jazyk lsp` (see [transport](../transport.md#stdio)),
manages the process, and exposes the [capabilities](../capabilities/diagnostics.md#diagnostics) to the
model as tools. This is the best fit for the diagnostics loop: after an edit the agent reads
[published diagnostics](../capabilities/diagnostics.md#republish) directly. Configured in the agent's
own language-server settings.

### Via MCP

The agent talks to Jazyk's [MCP server](../../mcp.md#mcp) instead of (or alongside) the LSP. MCP is
query-shaped and purpose-built for agents: `get_entity`, `requirements_for`, `relationships_for`,
`search`, and `diagnostics`. An agent more naturally asks "tell me about entity *Customer*" (one MCP
call) than "what is defined at line 12, column 7" (an LSP position request), so MCP is the natural path
for graph navigation. Any MCP-capable agent can use it with no LSP support at all.

### Via an IDE

The agent has no LSP client of its own but is attached to an editor that does. The
[VS Code](./vscode.md#vs-code) or [IntelliJ](./intellij.md#intellij) extension runs `jazyk lsp`, Jazyk
diagnostics land in the editor, and the agent reads them through the editor's own diagnostics bridge.
The editor extension does double duty: it serves the human and feeds the agent.

## The three agents

### opencode — natively

opencode is a first-class LSP client. It manages multiple concurrent language servers, caches each
server's `publishDiagnostics`, and exposes diagnostics, hover, symbols, definition, and references to
the model. Add `jazyk lsp` to its language-server configuration and it uses Jazyk's LSP as an LSP. It
can also use the [MCP server](../../mcp.md#mcp) for graph queries; the two complement each other.

### Codex — via MCP

Codex has no built-in LSP client (native LSP support is a requested, not yet shipped, feature). It does
speak MCP, so it consumes Jazyk through the [MCP server](../../mcp.md#mcp) today — diagnostics and the
full entity graph, without an editor. Community bridges that expose a language server's diagnostics as
MCP tools exist as well. If Codex gains a native LSP client later, the native mode above opens up with
no change to Jazyk.

### Claude Code — via an IDE or via MCP

Claude Code is not a generic LSP client; it never spawns language servers itself. It reaches Jazyk two
ways, both MCP underneath:

- Via an IDE. With the [VS Code](./vscode.md#vs-code) or [IntelliJ](./intellij.md#intellij) extension
  connected, the editor runs `jazyk lsp`, Jazyk diagnostics appear in the editor, and Claude Code reads
  them through the IDE integration's diagnostics tool (the editor exposes a local `ide` MCP server with
  a `getDiagnostics` tool). The editor extension we already ship is the bridge; nothing extra is needed.
- Via MCP. Connect Jazyk's [MCP server](../../mcp.md#mcp) directly, with no editor in the loop, for the
  full set of graph-navigation tools as well as diagnostics.

## Build latency

Jazyk diagnostics come from a compile that calls an LLM, so they are not instant (see
[lifecycle](../lifecycle.md#incremental-recompile)). An agent that expects a synchronous answer should
treat diagnostics as a pull: ask, and take the latest completed build, rather than assuming results are
ready the instant an edit lands. This suits agents well, since they already work in an edit-then-check
rhythm.

## Choosing a path

- Graph navigation and semantic queries: prefer [MCP](../../mcp.md#mcp). It is the surface built for
  agents and works regardless of LSP support.
- The post-edit diagnostics loop in an LSP-native agent: use the LSP directly.
- An agent with neither, but attached to an editor: rely on the IDE bridge.

The modes overlap by design — diagnostics flow through all three — so whatever a given agent supports,
Jazyk has a path to it.

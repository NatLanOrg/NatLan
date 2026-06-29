# Lifecycle

How the [language server](./lsp.md#language-server) starts, syncs files, recompiles, and shuts down.

The server keeps the compiler's analysis in memory for one project and updates it as the editor sends
edits. Like [rust-analyzer](https://rust-analyzer.github.io/book/contributing/architecture.html), the
protocol layer is thin: it routes requests and owns the document state, while the compiler library is a
pure function of its inputs.

## Project discovery

On `initialize` the client sends a workspace root. The server finds the Jazyk project by walking up
from that root to the nearest `jazyk.toml`, the same rule the [CLI](../cli.md#cli) and
[MCP server](../mcp.md#mcp) use. The project's [settings](../compiler/project-settings.md#project-settings)
(docs glob, scopes, linting rules, LLM endpoint) are loaded from that file. `--llm-base-url` and
`--model` passed to `jazyk lsp` override the `[llm]` settings, e.g. to point at a local Ollama.

## Initialization

- `initialize` / `initialized`. The server advertises its capabilities: incremental text sync,
  diagnostics, definition, references, hover, and completion (see
  [capabilities](./capabilities/diagnostics.md#diagnostics)).
- First build. The server compiles the project once so navigation and hover have a graph to read.
  Deterministic results (sections from the parser) are available immediately; LLM-derived results
  (entities, requirements, relationships, semantic diagnostics) arrive as the build completes.
- `shutdown` / `exit`. The server stops watching and releases the project.

## File sync

The editor is the source of truth for open files, not the disk.

- `didOpen`, `didChange`, `didClose`. The server keeps an in-memory overlay of open documents. Edits
  apply to the overlay, so unsaved buffers compile without writing to disk.
- `didSave` and watched-file events. Changes to files that are not open in the editor (saved on disk,
  changed by another tool) are picked up against the docs glob.
- Files outside the [docs glob](../compiler/project-settings.md#docs) are ignored.

## Incremental recompile

On each change the server reuses the compiler's [incrementality](../compiler/concepts/incremental.md#incrementality),
not a fresh pipeline:

- Only changed files recompile; per-file results are cached by content hash.
- Linking re-resolves only the entities touched by the change and their neighbors.
- A short debounce coalesces rapid keystrokes into one recompile.

Because some stages call an LLM, a recompile is not instant. Diagnostics and graph queries reflect the
last completed build and update when the new one lands; the server does not block editor requests on a
build in flight.

### Progressive results

The server does not wait for the whole build before telling the editor anything. Results are published
in phases as they become available, so feedback appears without waiting for the slowest stage:

- Deterministic [parse](../compiler/compilation/parse.md#parse) results (sections) are ready
  immediately.
- Per-file [compilation](../compiler/compilation.md#compilation) diagnostics are published as each
  file finishes compiling.
- Cross-file [linking](../compiler/linking.md#linking) diagnostics (resolution, contradictions,
  reachability) are published when linking completes.

A file's diagnostics are republished only when they change, so the editor's list stays stable
in between.

## Persisted output

When a build completes (and is not cancelled), the server writes the whole-program finals to the out
directory: `linked.yaml`, `reviewed.yaml`, and `diagnostics.yaml`. These are the same
[build artifacts](../compiler/artifacts.md#storage-layout) `jazyk build` writes. The per-stage cache
files under `target/` are written during compilation regardless; this step adds the out-dir-root finals
so they reflect the latest completed build, not just the last `jazyk build`.

This lets other processes that read the out directory, in particular the [MCP server](../mcp.md#mcp)
and CI, see the same build the editor sees without recompiling. The finals are written only for a
completed build, never for the progressive partial results published mid-build.

Because the editor is the source of truth for open files, the persisted finals reflect the server's
current analysis including unsaved overlays. The artifacts embed verbatim source, so a persisted build
from an unsaved buffer is self-consistent: it describes exactly the text the server analyzed.

## Cancellation

When a newer edit arrives while a build or request is still running, the in-flight work is cancelled
rather than allowed to publish stale results, mirroring rust-analyzer's revision model. The editor's
`$/cancelRequest` is honored the same way. Queries always answer against a consistent snapshot of the
analysis.

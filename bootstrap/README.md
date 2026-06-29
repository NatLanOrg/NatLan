# Jazyk bootstrap

A self-contained Rust implementation of the Jazyk compiler and its frontends, built from the
specification under [`../docs`](../docs). It turns natural-language documentation into a
machine-readable entity graph by treating docs as source code: **separate compilation per file,
then linking across files** (see [`../PLAN.md`](../PLAN.md) and
[`../docs/compiler.md`](../docs/compiler.md)).

The crate has only `serde`/`serde_json` as dependencies — the HTTP client, JSON-RPC framing, LSP
server, and MCP server are all hand-rolled so the bootstrap stays small and fast to build.

## Pipeline

```
Compilation (per file, cached by content hash)      Linking (whole program)
  A1 parse sections        (md.rs, deterministic)      L1 load interfaces
  A2 extract entities      (compile.rs, LLM)           L2 resolve entities    (link.rs, deterministic)
  A3 extract requirements  (compile.rs, LLM)           L3 merge relationships (link.rs, deterministic)
  A4 consolidate edges     (compile.rs)                L4 synthesize defs     (link.rs, LLM)
  A5 local definitions     (folded into A2)            L5 semantic review     (link.rs, LLM)
                                                        L6 checks/reachability (link.rs)
```

## Source map

| File | Role |
| --- | --- |
| `model.rs` | Build-artifact data model (object / linked / reviewed artifacts, diagnostics) |
| `md.rs` | A1 Markdown section parser + position helpers for the LSP |
| `compile.rs` | Per-file compilation (A2–A5), provenance |
| `link.rs` | Linking (L1–L6) |
| `project.rs` | `jazyk.toml` discovery, settings, glob matching |
| `cache.rs` | Content-hash cache of object artifacts |
| `engine.rs` | Ties compile + cache + link together; shared by all frontends |
| `cli.rs` | `build` / `check` / `watch` / `codegen` / `testgen` |
| `lsp.rs` | Language server over stdio (diagnostics, definition, references, hover, completion) |
| `mcp.rs` | MCP server over stdio (compile, get_entity, requirements_for, …) |
| `benchmark.rs` | Grades whether a model is good enough to compile Jazyk |
| `jsonrpc.rs` | Content-Length framed JSON-RPC (shared by LSP/MCP) |
| `llm.rs` | OpenAI-compatible chat client |

## Build

```sh
cargo build --release   # produces target/release/jazyk
```

## Use

A Jazyk project is any directory containing a `jazyk.toml` (see
[`../docs/compiler/project-settings.md`](../docs/compiler/project-settings.md)). The CLI walks up
from the working directory to find it. You can also pass paths directly for an ad-hoc run.

```sh
jazyk build                 # compile + link, write artifacts to jazyk-out/
jazyk check                 # diagnostics only, non-zero exit on error (CI)
jazyk watch                 # rebuild on change
jazyk lsp --stdio           # language server for editors
jazyk mcp                   # MCP server for agents
jazyk benchmark             # grade the configured model
jazyk codegen               # generate code stubs per entity
jazyk testgen               # generate tests per requirement

# LLM overrides (default endpoint is a local Ollama):
jazyk build --llm-base-url http://localhost:11434/v1 --model llama3.1
```

Artifacts land in `jazyk-out/`: per-file `objects/*.json`, the global `linked.json` and
`reviewed.json`, and a combined `diagnostics.json`.

## Example

[`example/`](example) is a small project with a `jazyk.toml` and two docs. From there:

```sh
cd example
jazyk build --llm-base-url http://localhost:11434/v1 --model <your-model>
```

## Editor plugin

[`editors/vscode/`](editors/vscode) is a thin VS Code extension that launches `jazyk lsp`. See its
[README](editors/vscode/README.md) to build and run it.

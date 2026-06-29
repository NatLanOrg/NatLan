# Jazyk

Jazyk is a natural language compiler. It treats prose documentation as the source code of a program.
Instead of constraining English, the compiler reads loose docs and surfaces ambiguity,
open-endedness, and contradictions, producing machine-readable build artifacts. Downstream usages
(code generation, test generation, project management, documentation generation) consume those
artifacts. "Jazyk" means tongue/language in Slavic languages.

Status: early and exploratory. `PLAN.md` at the repo root is the canonical working design and runs
ahead of the published `docs/`. The `docs/` tree is the project's own documentation, and is also the
dogfood input the compiler runs on. `bootstrap/` is a working Rust implementation that already runs
the full pipeline plus CLI, LSP, MCP, and benchmark frontends.

## Architecture (compile + link)

The compiler follows separate compilation and linking, like a real toolchain. Each file compiles on
its own, then all files are linked together.

- Compilation (per file, parallel):
  - A1 parse sections (deterministic, markdown to a section tree)
  - A2 extract entities (LLM)
  - A3 extract requirements and the edges they imply (LLM, EARS)
  - A4 consolidate relationships (deterministic)
  - A5 local definitions (LLM; folded into A2 in the bootstrap)
  - produces one object artifact per file
- Linking (whole program, the linker):
  - resolve sub-stage: L1 load link interfaces, L2 resolve entities, L3 merge relationships
  - validate sub-stage: L4 synthesize definitions and validate merges, L5 cross-doc semantic review,
    L6 spec-lint / coverage / reachability
  - produces the linked artifact (after L3), the reviewed artifact (after L6), and diagnostics

Node types (the semantic graph):
- Section: document structure only (parent/child tree). Holds verbatim `raw` for reconstruction.
- Entity: a domain concept (a.k.a. symbol). Internal vs external, local vs global definition, scope,
  stable id.
- Requirement: an EARS statement attached to one or more entities. Behavior vs constraint is a
  derived facet of the EARS pattern.
- Relationship: a typed edge between entities, reified. Edges are a product of requirements (no orphan
  edges, minimum weak `reference`, UML types with promotion).
- Diagnostic: first-class, persisted, sticky. Severity `error|warning|info|none`, carries `reasoning`,
  reconciled across builds (never regenerated), human triage survives.

Key ideas (see `PLAN.md` and `docs/compiler/`):
- Compiler/linker analogy: entity = symbol, object artifact = `.o`, linker resolves entities across
  docs.
- Entity resolution is conservative and deterministic: explicit links, then exact name (`name-only`).
  No fuzzy matching or typo guessing. Unresolved is a `missing-link`.
- Semantic judgment is deferred to Linking: `false-merge` (split clashing names), `missing-link`,
  contradictions. Synthesizing one coherent definition is itself the merge-validity test (L4).
- Scopes prevent wrong merges. Stable identity matters (downstream binds to entity ids).
- Determinism via content-hash caching of LLM results. Reconcile diagnostics, do not regenerate.

## Repo layout

- `PLAN.md` — exploratory implementation plan, the working source of truth for the design.
- `docs/` — the project documentation (also the compiler's dogfood input).
  - top level: `main.md`, `compiler.md`, `cli.md`, `lsp.md`, `mcp.md`, `codegen.md`, `testgen.md`,
    `pm.md`, `docsgen.md`, `llm-test.md`, `site.md`, `benchmark.md`, `TODO.md`, `logo.{svg,png}`
  - `docs/compiler/`: `compilation.md`, `linking.md`, `model.md`, `artifacts.md`,
    `project-settings.md`, plus subdirs `compilation/`, `linking/`, `model/`, `artifacts/`,
    `concepts/` (one file per step, artifact, model node, and concept)
  - `docs/lsp/`: `lifecycle.md`, `transport.md`, `testing.md`, plus `capabilities/` and `editors/`
  - `docs/site/`: page specs for the public site (`page-home.md`, `page-compilation.md`, …)
  - `docs/benchmark/`: `cases.md`, `checks.md`, `scoring.md`, `case.schema.yaml`, plus `cases/`
  - schemas: `model.schema.yaml`, `compiler/artifacts/*.schema.yaml`, `project-settings.schema.yaml`,
    `benchmark/case.schema.yaml`
- `bootstrap/` — the Rust implementation (binary `jazyk`), plus `bootstrap/example/` (a sample
  project) and `bootstrap/editors/vscode/` (a thin VS Code extension that launches `jazyk lsp`).
- `jazyk.toml` — project settings at the repo root, the file that marks a Jazyk project.
- `jazyk-out/` — default build output directory (artifacts + cache). Not source.
- `.env` — local LLM credentials (gitignored).

Note: `docs/compiler/build-artifacts*` was renamed to `docs/compiler/artifacts*`. `PLAN.md` still
references the old path in a couple of places; the live tree is `artifacts/`.

## Docs writing style (match this exactly)

The owner is strict about voice. When writing or editing anything under `docs/`:
- Plain, to-the-point, short declarative sentences.
- Never use em dashes. Use commas, periods, parentheses, or colons.
- Bullet lists with `- `, nested where useful.
- Backticks for identifiers, filenames, field names, rule names.
- `e.g.` and `E.g.:` with fenced code blocks for examples. `→` for sequences.
- Sparing bold. One H1 per file. Headings `#`, `##`, `###`.
- No marketing language. State what it does.
- Cross-link with relative markdown links and anchors (GitHub slug of the heading).
- `PLAN.md` uses a heavier style (em dashes, bold). That is the working doc only. The `docs/` tree
  stays plain.

After editing docs, check that relative links and anchors resolve and that there are no em dashes.
Schemas are draft-07 JSON Schema written in YAML. `$id`s use `https://jazyk.org/schemas/*.json` and
cross-file `$ref`s use the `.json` filename. Shared definitions live in `model.schema.yaml`. Validate
the schemas and a sample artifact after changing them.

## Bootstrap (Rust)

- Docs-first workflow: any change requested to `bootstrap/` must first be reflected in `docs/` (and
  `PLAN.md` where it applies), then made in the code. Bootstrap is the special case where docs and
  code are updated together, in that order. Once the bootstrap is functional, future development
  happens in `docs/` only and codegen regenerates the code.
- Build: `cd bootstrap && cargo build` (debug binary at `bootstrap/target/debug/jazyk`) or
  `cargo build --release` (at `bootstrap/target/release/jazyk`). Dependencies are minimal
  (`serde`, `serde_json`, and a maintained `serde_yaml` fork for YAML artifacts); the LLM HTTP client,
  JSON-RPC framing, LSP server, and MCP server are all hand-rolled so the crate stays small and fast
  to build.
- Commands (all take optional `[path...]` and the LLM/output overrides below):
  - `jazyk build` — compile + link, write artifacts to the out dir.
  - `jazyk check` — compile, report diagnostics only, non-zero exit on error (CI).
  - `jazyk watch` — recompile on change.
  - `jazyk lsp [--stdio]` — language server over stdio (diagnostics, definition, references, hover,
    completion).
  - `jazyk mcp` — MCP server over stdio (compile, get_entity, requirements_for, …).
  - `jazyk benchmark` — grade whether the configured model is good enough to compile Jazyk.
  - `jazyk codegen` — generate a code stub per entity from its assembled requirements.
  - `jazyk testgen` — generate tests per requirement.
  - `jazyk gen <description.md> [--out FILE]` — generate an asset (e.g. an SVG logo) from a
    description.
  - Options: `--llm-base-url URL`, `--model M`, `--api-key K`, `--out DIR`.
- A project is any directory containing `jazyk.toml`; the CLI walks up from cwd to find it. Passing
  explicit paths runs ad-hoc without a `jazyk.toml`. Default out dir is `<root>/jazyk-out/`.
- Artifacts in the out dir are `YAML`. The source-mirrored build tree lives under `target/`: each
  source file gets a directory named after it (extension kept, e.g. `target/docs/cli.md/`) holding one
  file per compile stage: `sections.yaml` (A1), `entities.yaml` (A2/A5), `requirements.yaml` (A3),
  `object.yaml` (A4, the consolidated translation unit). Whole-program link stages live under
  `target/link/` (`<entity-slug>.synthesis.yaml` for L4, `<entity-slug>.review.yaml` for L5). The
  globals sit at the out-dir root: `linked.yaml`, `reviewed.yaml`, and the combined `diagnostics.yaml`.
  The files under `target/` are also the incremental cache: each begins with a `# jazyk:` key header
  comment, and a stage is skipped when the recomputed key matches. There is no separate hash-named
  `cache/` dir; reverting a change recomputes.
- Source map (`bootstrap/src/`): `model.rs` (data model), `md.rs` (A1 markdown parser + LSP position
  helpers), `compile.rs` (A2–A5), `link.rs` (L1–L6), `project.rs` (`jazyk.toml` discovery + settings
  + globs), `serialize.rs` (YAML format layer + cache-key header), `cache.rs` (source-mirrored stage
  store, the cache), `engine.rs` (compile + cache + link, shared by all frontends), `cli.rs`,
  `lsp.rs`, `mcp.rs`, `benchmark.rs`, `jsonrpc.rs` (Content-Length framed JSON-RPC), `llm.rs`
  (OpenAI-compatible chat client over raw TCP).
- LLM config precedence (highest first): CLI flag → env var (`JAZYK_LLM_BASE_URL`, `JAZYK_MODEL`,
  `JAZYK_API_KEY`) → global `~/.jazyk/config.toml` (or `~/.jazyk.toml`) `[llm]` → project `[llm]` →
  built-in default. The endpoint/model/auth are machine-level, so `~/.jazyk/config.toml` is their
  canonical home. `main.rs` also loads a `.env` (walking up from cwd, not overriding real env vars),
  used mainly to supply the API key env var. The local model is `gemma4:e4b-mlx` behind an
  OpenAI-compatible proxy at `http://127.0.0.1:3625`. Sends `Authorization: Bearer` when a key is set.
- The model is large and slow. A full `docs/` run takes a while, so test on a small subdirectory
  (e.g. `bootstrap/example/`) first.

## Working norms

- Do NOT stage or commit unless the owner explicitly asks. This is a hard rule.
- When asked to commit: commit as Matus Faro (`matus@matus.io`), signed, and never add Claude or AI
  co-author or attribution trailers.
- Keep secrets out of tracked files. `.env`, `*.env`, `bootstrap/target/`, and `bootstrap/jazyk-out/`
  are gitignored. (A root-level `jazyk-out/` from running `jazyk build` at the repo root is not yet
  gitignored; do not commit it.)
- Git remote: `git@github.com:JazykOrg/Jazyk.git`.

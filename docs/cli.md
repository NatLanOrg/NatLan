# CLI

The CLI is the command line frontend over the [compiler](./compiler/compiler.md#compiler). It runs Jazyk
against a project and drives the downstream usages.

## Definition

The CLI operates on a Jazyk project, the directory holding a `jazyk.toml`. It is found by walking up
from the working directory.

### Commands

- `jazyk build` runs [compilation](./compiler/compilation.md#compilation) and
  [linking](./compiler/linking.md#linking) and writes the
  [build artifacts](./compiler/artifacts.md#build-artifacts) to the target directory. Prints warnings
  and errors. Exits non-zero on error.
- `jazyk check` compiles without writing artifacts and reports
  [diagnostics](./compiler/model/diagnostic.md#diagnostic) only. Suitable for CI and pre-commit hooks.
- `jazyk watch` recompiles incrementally as files change. 
- `jazyk lsp` starts the [language server](./lsp/lsp.md#language-server) over stdio for editor
  integration. It recompiles incrementally and serves diagnostics, navigation, hover, and completion.
- `jazyk mcp` starts the [MCP server](./mcp.md#mcp) over stdio for agent integration. Like `lsp`, it
  embeds the compiler and serves the build graph; see the [MCP tools](./mcp.md#tools).
- `jazyk benchmark` runs the [LLM benchmark](./benchmark/benchmark.md#benchmark) against the configured model and
  reports whether it is good enough to compile Jazyk. Exits non-zero if the model fails the verdict.
- Consumer subcommands run against the latest build:
  - `jazyk codegen` for [code generation](./codegen.md#code-generation).
  - `jazyk testgen` for [test generation](./testgen.md#test-generation).

`build`, `check`, `watch`, `lsp`, `mcp`, and `benchmark` accept `--llm-base-url` and `--model` to
override the [LLM settings](./compiler/project-settings.md#llm), e.g. to point at a local Ollama. With
no flags, those settings come from the
[global config or environment](./compiler/project-settings.md#global-configuration).

`jazyk benchmark` follows the same rule. Pass `--model`, `--llm-base-url`, or `--api-key` (alone or in
combination) to grade a specific model or endpoint. With none of them, it grades the default model
resolved from the global config or environment.

### Help

Every command accepts `--help` and its short form `-h`, which print usage and exit `0` without running
the command.

- `jazyk --help` (or `jazyk -h`, or `jazyk help`) prints the list of commands and the shared options.
- `jazyk <command> --help` (or `-h`, or `jazyk help <command>`) prints help for that command: what it
  does, its usage line, and the options it accepts.

Run with no command, or with an unknown command, prints the same top-level usage to stderr and exits
non-zero.

### Exit codes

A clean build exits `0`. Warnings alone do not fail the build. Any error diagnostic, or a rule set to
`error` severity in the [linting rules](./compiler/project-settings.md#linting), causes a non-zero
exit. Output is human readable by default, with a machine readable format available for tooling.

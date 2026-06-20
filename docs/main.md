# Natlan
https://natlan.org

**Nat**ural **Lan**gauge as a programming language.

## Preamble

Natural language or ordinary language is any language that humans use to communicate
amongst each other. This project introduces a new higher-level programming language that
allows developers to define software in natural language.

Compared to common programming languages, Natural Language is flexible and allows for a
wide range of interpretations, making it difficult to define and construct software
out of it, if not properly constrained.

The syntax of Natural Languages such as English is already defined. Rather than constraining it,
we introduce a compiler that surfaces ambiguity, open-endedness, and contradictions in its usage.

### Read–eval–print loop

In current world, LLMs are invoked with short and well-defined prompts to produce more reliable outcomes.

An open-ended prompt becomes exponentially less reliable.
(e.g. "pelican on bicycle as SVG", "build me Facebook")

What if open-endedness is not the target we are aiming for?

Programming languages are constrained by their syntax and semantics. English language can describe
ambiguity. The prompts are unreliable because we are using natural language with ambiguity.

In a way, a "coding agent" (e.g. Claude Code) are a form of REPL, a way to interact with an LLM one
statement at a time to produce an incremental result.

If coding agent is a REPL, and a prompt is a single programming statement, than what does an entire
program look like?

Disregard the flexibility of natural language to produce ambiguous statements, there is no CPU instruction
to "build me a Facebook" or "draw me a pelican" so let's restrict our language to be well-defined.

Imagine requirements doc and UML diagrams as a programming language.

## Architecture

```mermaid
graph TD;
  User(["User"]);
  IDEs(["IDEs"]);
  Agents(["Coding Agents"]);

  subgraph Frontends
    CLI[CLI];
    LSP[Language Server];
    MCP[MCP Server];
  end

  Compiler[["Compiler<br/>(Rust library)"]];
  Docs[/"Documentation<br/>(Natural Language)"/];
  Artifacts[("Build Artifacts<br/>(machine-readable)")];

  subgraph Usages
    PM[Project Management];
    CodeGen[Code Generation];
    TestGen[Test Generation];
  end

  User --> CLI;
  IDEs --> LSP;
  Agents --> MCP;

  CLI --> Compiler;
  LSP --> Compiler;
  MCP --> Compiler;

  Docs --> Compiler;
  Compiler --> Artifacts;

  CLI --> Usages;
  MCP --> Usages;

  Artifacts --> CodeGen;
  Artifacts --> TestGen;
  Artifacts --> PM;
```

### Landing page

TODO natlan.org

### Compiler

[See More](./compiler.md#compiler)

### Frontends

- [CLI](./cli.md#cli)
- [Language Server](./lsp.md#language-server)
- [MCP Server](./mcp.md#mcp)

### Usages

### Frontends

- [Project Management](./pm.md#project-management)
- [Code Generation](./codegen.md#code-generation)
- [Test Generation](./testgen.md#test-generation)
- [Documentation Generation](./docsgen.md#documentation-generation)

---
# Scratch space

TODO Project Management:
- Tracking and planning of work
TODO Code generation:
- Code generation and updates to it (with migration)
TODO Testing:
- Unit/Integration/Cucumber tests tied to requirements
- LLM-driven tests (Run LLM given a requirement from doc and have it assert the code is correct)
TODO Others:
- Feed back implementation back into docs
  (e.g. background color was never defined, but the implementation made it blue so let's persist it)

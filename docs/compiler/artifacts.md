# Build Artifacts

Compilation and linking produce several artifacts. An artifact is the machine-readable form of the
documentation at a particular phase. Downstream usages read final artifacts, not the input docs.

There are two intermediate artifacts, one final artifact plus a diagnostics store.

## Object artifact

One per documentation file. It is the output of [compilation](./compilation.md). It holds the file's
sections, entities, requirements, and consolidated relationships, plus the entities it expects to
resolve in other files.

This is the translation unit of Jazyk, the equivalent of a `.o` object file.

[See more](./artifacts/object-artifact.md)

## Linked artifact

Whole program. It is the output of the resolve stage of [linking](./linking.md) (steps L1 to L3). It
holds the global entities (the symbol table), the global relationship graph, and a requirement index.

[See more](./artifacts/linked-artifact.md)

## Reviewed artifact

Whole program. It is the output of the validation stage of [linking](./linking.md) (steps L4 to L6).
It extends the linked artifact with synthesized entity definitions, semantic diagnostics, and
coverage.

[See more](./artifacts/reviewed-artifact.md)

## Diagnostics store

Warnings and errors are persisted in a store keyed by a stable id. They survive recompilation and are
consumed by IDEs and CI.

[See more](./artifacts/diagnostics-store.md)

## Reproducibility

Artifacts store the verbatim source, so the original documentation can be reconstructed from them.

[See more](./artifacts/reproducibility.md)

## Storage layout

Artifacts are written to the out dir (`jazyk-out/` by default) as `YAML`. The source-mirrored build
tree lives under `target/`: each documentation file gets one directory whose name keeps the source
file name and extension, and each compilation stage writes one file inside it. The whole-program
finals (`linked.yaml`, `reviewed.yaml`, `diagnostics.yaml`) sit at the out-dir root, so `target/`
holds only generated, source-shaped files.

The finals are rewritten whenever a whole-program build completes, not only by `jazyk build`. The
[LSP server](../lsp/lifecycle.md#persisted-output) and the [MCP](../mcp.md#mcp) `compile` tool also
write them, so any process reading the out dir sees the latest completed build.

E.g.: for `docs/cli.md`

```
jazyk-out/
  target/
    docs/
      cli.md/
        sections.yaml         # parse (A1)
        entities.yaml         # extract entities (A2, local definitions A5)
        requirements.yaml     # extract requirements (A3)
        object.yaml           # consolidated object artifact (A4), the linker input
    link/
      <entity-slug>.synthesis.yaml   # synthesize definitions (L4)
      <entity-slug>.review.yaml      # semantic review (L5)
  linked.yaml               # linked artifact
  reviewed.yaml             # reviewed artifact
  diagnostics.yaml          # diagnostics store
```

The files under `target/` are both the build output and the incremental cache. Each begins with a
`jazyk` header comment that records the stage's
[cache key](./concepts/determinism.md#process-only-changed). The object artifact (`object.yaml`) is
the consolidated translation unit; `sections.yaml`, `entities.yaml`, and `requirements.yaml` are the
per-stage slices that produce it. The whole-program link stages are not per file, so they live under
`target/link/`, keyed by entity.

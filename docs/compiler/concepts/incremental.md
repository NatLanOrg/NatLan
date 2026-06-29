# Incrementality

Jazyk recompiles only what changed and what depends on it.

## Compilation

[Compilation](../compilation.md#compilation) runs per file. Each stage is cached by its key (the file
content hash for the whole-file stages) in the per-stage [artifact files](../artifacts.md#storage-layout).
Only changed files recompile, and within a changed file only the stages whose inputs changed rerun.
Files compile in parallel.

## Linking

[Linking](../linking.md#linking) reloads the link interfaces. Incremental linking re-resolves only the
entities touched by changed files and their neighbors, not the whole project. The validation steps
re-review only the entities and relationships whose member set or requirements changed.

## Parallelism

Independent LLM work runs concurrently, not one call at a time. Files compile in parallel, and within
linking the per-entity validation ([L4 synthesize](../linking/synthesize-definitions.md#synthesize-definitions))
and review ([L5 semantic review](../linking/semantic-review.md#semantic-review)) run in parallel,
because each entity's judgment depends only on its own facts. A global cap bounds the number of
in-flight LLM requests so a large project does not overwhelm the backend (a local endpoint serializes
work and fails under heavy fan-out); it is configurable for slower or beefier endpoints. This turns the
wall-clock cost from the sum of every call into the slowest batch, which matters most when the model is
a slow remote endpoint.

## Change propagation

A change to one entity ripples to the entities related to it through the relationship graph, and stops
when nothing else changes. The [content-hash cache](./determinism.md#determinism) short-circuits any
entity whose result would not change, so propagation terminates.

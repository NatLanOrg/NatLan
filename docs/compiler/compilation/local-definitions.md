# Local definitions

Local definitions is step A5 of [compilation](../compilation.md#compilation). For each external
entity it writes what this file says about that entity. It uses an LLM.

A local definition is the matching surface the linker compares when it resolves entities across
files. It is partial, "what this file knows so far". The full definition is synthesized later, after
linking, from all files. See [Synthesize definitions](../linking/synthesize-definitions.md#synthesize-definitions).

In the bootstrap this step has no separate LLM call. The A2 prompt already returns a `definition`
field per entity, so local definitions are produced inside
[Extract entities](./extract-entities.md#prompt). The dedicated per-entity prompt described below is a
future refinement, not yet implemented.

## Consumes

For one external entity: the slices of this file that mention it (gathered from the entity's
provenance spans) and its requirements.

## Produces

A `localDefinition` string per external entity.

## LLM scope

One entity at a time. The model sees only the parts of this file that mention the entity, not the
whole file. Keep the definition concise: it is a matching surface, not the full spec.

## Cache key

The hash of the entity's contributing spans + model id + prompt version.

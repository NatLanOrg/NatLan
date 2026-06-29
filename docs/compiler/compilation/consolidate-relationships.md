# Consolidate relationships

Consolidate relationships is step A4 of [compilation](../compilation.md#compilation). It turns the
per-requirement `impliedEdges` from [Extract requirements](./extract-requirements.md#extract-requirements)
into [relationships](../model/relationship.md#relationship). It is mostly deterministic.

## Consumes

The file's requirements and their `impliedEdges`.

## Produces

A list of relationships. For each entity pair it:

- unions the requirements that tie the pair together,
- picks the strongest implied type (promotion from `reference` toward composition, dependency, and so
  on),
- derives cardinality where a requirement states it.

E.g.:
```yaml
relationships:
  - localId: "rel0"
    type: "dependency"     # strongest type across the pair's requirements
    members: ["e0", "e1"]
    requirements: ["r0"]   # never empty
    cardinality: { e0: "1", e1: "1..*" }
```

## LLM scope

None for the common case. An LLM runs only to reconcile a genuine type conflict for a pair, and then
it sees only that pair's candidate types and requirements.

In the bootstrap this step is fully deterministic. Type conflicts are resolved by ranking the implied
types and keeping the strongest (`rel_rank` in `bootstrap/src/compile.rs`); no LLM call is made. The
type-reconciliation prompt is a future refinement, not yet implemented.

## Diagnostics

- conflicting implied types for a pair: resolve to the strongest, or warn if genuinely contradictory.

## Cache key

The contributing requirements' hashes (plus model id if type reconciliation ran).

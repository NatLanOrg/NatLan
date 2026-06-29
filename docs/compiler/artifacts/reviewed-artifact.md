# Reviewed artifact

The reviewed artifact is the whole program output of the validation stage of
[linking](../linking.md#linking) (steps L4 to L6). It extends the
[linked artifact](./linked-artifact.md#linked-artifact).

## Contents

It adds to each global entity, and to the project as a whole:

- `globalDefinition` and `aliases` per [entity](../model/entity.md#entity). Synthesized once all facts
  are linked, by [synthesize definitions](../linking/synthesize-definitions.md#synthesize-definitions).
  Synthesis also validates the merge: a `name-only` merge that cannot form one coherent definition
  becomes a `false-merge` error and is split.
- semantic [diagnostics](../model/diagnostic.md#diagnostic): contradictions, redefinition, overlap,
  fragmentation, missing links. Produced by
  [semantic review](../linking/semantic-review.md#semantic-review).
- `coverage` and reachability results. Produced by [checks](../linking/checks.md#checks).

## Shape

It extends the linked artifact. E.g.:
```yaml
entities:
  - globalId: "ent:ABC"
    globalDefinition: "ABC is a 3-wheeled motorcycle component that ..."  # synthesized post-link
    aliases: ["ABC", "ABC component"]
diagnostics:
  - severity: "error"
    phase: "semantic"
    rule: "cross-doc-contradiction"
    message: "abc.md implies ABC is a tricycle (3 wheels); xyz.md states ABC is a 3-wheeled motorcycle."
    related: ["ent:ABC"]
    sources: ["abc.md#/abc/wheels", "xyz.md#/vehicles/abc"]
  - severity: "warning"
    phase: "semantic"
    rule: "entity-fragmented"
    message: "ABC is specified across 6 files; consider consolidating into abc.md."
    related: ["ent:ABC"]
coverage:
  - entity: "ent:ABC"
    behaviors: 4
    constraints: 2
    testsDerivable: 5
```

## Schema

The reviewed artifact follows [this schema](./reviewed-artifact.schema.yaml).

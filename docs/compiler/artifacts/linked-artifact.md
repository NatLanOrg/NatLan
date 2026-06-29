# Linked artifact

The linked artifact is the whole program output of the resolve stage of
[linking](../linking.md#linking) (steps L1 to L3). It combines all
[object artifacts](./object-artifact.md#object-artifact) into one graph.

## Contents

- `entities`: the global [entities](../model/entity.md#entity), the symbol table. Each global entity
  lists the object files that contribute to it (its members) and how it was resolved. Produced by
  [resolve entities](../linking/resolve-entities.md#resolve-entities).
- `relationships`: the global [relationship](../model/relationship.md#relationship) graph. Edges are
  re-keyed to global entities and merged across files. Produced by
  [merge relationships](../linking/merge-relationships.md#merge-relationships).
- `requirements`: a global requirement index. Ids are stable across rebuilds (see
  [stable identity](../concepts/stable-identity.md#stable-identity)).
- `diagnostics`: resolution diagnostics from L2, e.g. `name-only-link`, `link-section-mismatch`,
  `missing-link`.

The [reviewed artifact](./reviewed-artifact.md#reviewed-artifact) builds on this.

## Shape

E.g.:
```yaml
entities:
  - globalId: "ent:DEF"
    canonicalName: "DEF"
    aliases: ["Def service"]
    scope: "billing"
    members:                       # contributing object files
      - { object: "abc.md", localId: "e1", role: "reference" }
      - { object: "def.md", localId: "e0", role: "definition" }
    confidence: 0.95
    links: ["abc.md#/abc/behavior -> def.md#/def"]
relationships:
  - globalId: "rel:ABC~DEF"
    type: "dependency"
    members: ["ent:ABC", "ent:DEF"]
    cardinality: { "ent:ABC": "1", "ent:DEF": "1..*" }
    requirements: ["req:abc.md:r0"]
    confidence: 0.8
requirements:                      # global index (id-stable across rebuilds)
  - globalId: "req:abc.md:r0"
    earsText: "..."
    entities: ["ent:ABC", "ent:DEF"]
    relationship: "rel:ABC~DEF"
    sourceSection: "abc.md#/abc/behavior"
diagnostics:
  - severity: "warning"
    phase: "link"
    rule: "name-only-link"
    message: "'Customer' linked across abc.md and customer.md by name only; add an explicit link to confirm."
    related: ["ent:Customer"]
```

## Schema

The linked artifact follows [this schema](./linked-artifact.schema.yaml).

# Semantic review

L5 is the whole-program check. It runs on entities whose merges [L4](./synthesize-definitions.md)
already confirmed. Reading all requirements for an entity together, it finds issues that are invisible
to any single file.

It checks for:

- Missing link: entities left separate that are actually the same concept despite different names (e.g.
  `buyer` and `Customer`). Suggest a link. This is the inverse of L4's false merge.
- Contradiction: a confirmed-same entity whose requirements conflict. E.g. one doc says entity A has 3
  wheels (reads like a tricycle), another says A is a 3-wheeled motorcycle.
- Redefinition: an entity defined differently across files.
- Overlapping requirements: duplicate requirements across files.
- Fragmentation: an entity thinly spread across many files. Suggest consolidating it into its own file.
- Incompleteness: a requirement that only makes sense combined with another. If the combination is
  coherent, info. If confusing, warn or error.

This is where link-time feedback happens. A requirement may be incomplete or misleading until a
requirement from another doc is seen. Because L5 groups all requirements per entity after linking, that
cross-doc incoherence surfaces here.

- Consumes: per entity (and per relationship), all requirements grouped across docs, the global
  definition, and the entity's existing diagnostics.
- Produces: diagnostics on entities, relationships, and requirements, plus suggested links.
- Deterministic: no. LLM per entity, structured output, reconciled against prior diagnostics (see
  [sticky diagnostics](../concepts/sticky-diagnostics.md)).
- LLM scope: one entity (the symbol) with its grouped requirements, its definition, and its existing
  diagnostics. The model sees the symbol and its statements, not the source files.
- Cache key: grouped-requirements hash plus definition hash plus prior-diagnostics id-set plus model id
  plus prompt version. Stored in `target/link/<entity-slug>.review.yaml` (see
  [storage layout](../artifacts.md#storage-layout)).

## Prompt

The exact call the bootstrap makes (`bootstrap/src/link.rs`, `link`). One call per cross-doc entity, at
temperature 0, parsed as JSON. It runs only for entities whose members span more than one document and
that have at least one requirement.

System prompt (wrapped for readability, sent as one line):
```text
Given the requirements for one entity gathered across documents, find real problems.
Return ONLY JSON: {"diagnostics":[{"rule":string,"severity":"error"|"warning"|"info","message":string,"reasoning":string}]}.
Use rules like cross-doc-contradiction, redefinition, overlapping-requirements, incompleteness.
Return an empty array if there are no problems.
```

User message: the entity name and one line per requirement gathered across documents.
```text
Entity: <canonical name>
Requirements:
- <requirement ears text>
- <requirement ears text>
```

Expected reply:
```json
{
  "diagnostics": [
    { "rule": "cross-doc-contradiction", "severity": "error", "message": "...", "reasoning": "..." }
  ]
}
```

Each returned diagnostic becomes one of the diagnostics below (`rule` defaults to `semantic`,
`severity` to `warning` when missing or invalid). The bootstrap sends only the grouped requirements,
not the global definition or prior diagnostics.

The reply is constrained by this schema, sent as
[structured output](../concepts/determinism.md#structured-output) (`response_format`, name `review`):
```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["diagnostics"],
  "properties": {
    "diagnostics": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["rule", "severity", "message", "reasoning"],
        "properties": {
          "rule": { "type": "string" },
          "severity": { "type": "string", "enum": ["error", "warning", "info"] },
          "message": { "type": "string" },
          "reasoning": { "type": "string" }
        }
      }
    }
  }
}
```

## Diagnostics

- `missing-link` (warn)
- `cross-doc-contradiction` (warn or error)
- `redefinition` (warn or error)
- `overlapping-requirements` (warn)
- `entity-fragmented` (warn)
- `incomplete-when-combined` (info, warn, or error)

E.g.:
```yaml
# consumes: a validated entity + its grouped requirements
ent:ABC:
  globalDefinition: "..."
  requirements: [ ...grouped across abc.md, xyz.md... ]
# produces
diagnostics:
  - { rule: cross-doc-contradiction, severity: error,
      message: "abc.md implies ABC is a tricycle (3 wheels); xyz.md states ABC is a 3-wheeled motorcycle.",
      subjects: [ent:ABC] }
  - { rule: missing-link, severity: warning,
      message: "'buyer' (abc.md) and 'Customer' (customer.md) appear to be the same entity but were not linked.",
      subjects: [ent:buyer, ent:Customer] }
```

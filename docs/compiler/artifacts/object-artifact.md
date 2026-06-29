# Object artifact

The object artifact is the output of [compilation](../compilation.md#compilation) for one
documentation file. There is one object artifact per file. It is the Jazyk equivalent of a `.o`
object file: a self contained translation unit that the [linker](../linking.md#linking) later
combines with the others.

## Contents

- `doc`: file metadata (source file, format, content hash).
- `sections`: a map of [section](../model/section.md#section) keyed by internal reference (the
  fragment after `#`). Produced by [parse](../compilation/parse.md#parse).
- `entities`: a map of [entity](../model/entity.md#entity) keyed by local id. Produced by
  [extract entities](../compilation/extract-entities.md#extract-entities).
- `requirements`: the [requirements](../model/requirement.md#requirement) and the edges each one
  implies. Produced by [extract requirements](../compilation/extract-requirements.md#extract-requirements).
- `relationships`: the consolidated [relationships](../model/relationship.md#relationship). Produced
  by [consolidate relationships](../compilation/consolidate-relationships.md#consolidate-relationships).
- `externals`: entities referenced here but expected to resolve in another file, with any relocation
  link.

References inside the artifact are file internal (the fragment after `#`). The file is implicit. Cross
file references appear only under `externals` as relocations, which carry the full file path. The
[linker](../linking.md#linking) resolves them.

## Shape

E.g.:
```yaml
doc:
  sourceFile: "file:///proj/docs/abc.md"
  format: "markdown"
  contentHash: "<murmur3 of file>"
sections:                          # map: internal reference (after '#') -> section
  "/abc/overview":
    title: "Overview"
    kind: "heading"        # heading | list-item | code-block | blockquote | diagram | root
    order: 0
    parent: "/abc"         # internal reference
    raw: "..."             # verbatim, for reconstruction
entities:                          # map: localId -> entity
  "e0":
    name: "ABC"
    aliases: ["ABC component"]
    linkage: "external"    # internal | external
    role: "definition"     # definition | reference  (decl/def)
    scope: "public"        # public | private | a named context, from the docs
    localDefinition: "ABC is the component that ..."
    provenance: [{ section: "/abc/overview", span: [12, 48] }]   # internal ref
    confidence: 0.91
requirements:
  - id: "r0"
    earsText: "When a user submits ABC, the system shall validate DEF."
    pattern: { type: "event", trigger: "...", response: "..." }
    entityRefs: ["e0", "e1"]
    impliedEdges: [{ members: ["e0", "e1"], type: "dependency" }]  # edge(s) this req produces
    relationshipRef: "rel0"          # back-pointer to the consolidated edge
    sourceSection: "/abc/behavior"   # internal ref
    verificationMethod: "test"   # optional hint for testgen
    reasoning: "Email is the login identifier, so it must be unique."   # the "why"
    provenance: [{ span: [0, 63] }]
    confidence: 0.88
relationships:             # derived: consolidated from the requirements that tie each pair
  - localId: "rel0"
    type: "dependency"     # strongest type implied across its requirements ('reference' if none stronger)
    members: ["e0", "e1"]
    requirements: ["r0"]   # the requirements this edge is a product of (never empty)
    cardinality: { e0: "1", e1: "1..*" }   # optional, if a requirement states it
    provenance: [{ section: "/abc/behavior" }]   # internal ref
    confidence: 0.7
externals:                 # unresolved references / relocations (cross-file, so carry the file)
  - localId: "e1"
    name: "DEF"
    relocation: "file:///proj/docs/def.md#/def"   # explicit link, if any
```

## Notes

- Entity local ids (`e0`, `e1`) are stable across rebuilds. See
  [stable identity](../concepts/stable-identity.md#stable-identity).
- `linkage` marks an entity `internal` (private to this file) or `external` (part of the file's link
  interface). Only externals are loaded during linking.
- Relationships are never authored directly. They are a product of requirements. See
  [relationships](../model/relationship.md#relationship).
- Verbatim `raw` text supports [reconstruction](./reproducibility.md#reproducibility).

## Schema

The object artifact follows [this schema](./object-artifact.schema.yaml).

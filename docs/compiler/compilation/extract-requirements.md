# Extract requirements

Extract requirements is step A3 of [compilation](../compilation.md#compilation). For each section it
extracts [requirements](../model/requirement.md#requirement) written in
[EARS](../concepts/ears.md#ears), links each to one or more entities, and emits the relationship
edges those requirements imply. It uses an LLM.

## Edges come from requirements

A [relationship](../model/relationship.md#relationship) exists only because a requirement ties its
entities together. Any requirement that references two or more entities emits an edge between them, at
minimum a weak `reference`, stronger when the requirement warrants it. Nothing else creates edges. A
diagram arrow or a structural sentence (`A is part of B`) is captured as a requirement, so it flows
through the same path.

## Consumes

One section's text, plus the file's entity table (from [Extract entities](./extract-entities.md#extract-entities)).

## Produces

A list of requirements. Each has:

- `earsText` and the parsed EARS `pattern`.
- `entityRefs`: the entities it touches.
- `impliedEdges`: the edge(s) it produces, each with a type (default `reference`).
- `sourceSection`, `provenance`, `confidence`.
- optional `verificationMethod` (a hint for test generation) and `reasoning`
  (see [Reasoning](../concepts/reasoning.md#reasoning)).

E.g.:
```yaml
requirements:
  - id: "r0"
    earsText: "When a user submits ABC, the system shall validate DEF."
    pattern: { type: "event" }
    entityRefs: ["e0", "e1"]
    impliedEdges: [{ members: ["e0", "e1"], type: "dependency" }]
    sourceSection: "/abc/behavior"
```

## LLM scope

One section's text, plus the file's entity table (names and aliases, so requirements can link to
entities), plus the section's ancestor titles for context. Not the whole file. This keeps each prompt
small and lets sections run in parallel.

## Prompt

The exact call the bootstrap makes (`bootstrap/src/compile.rs`, `compile_text`). One call per section,
in parallel, at temperature 0, parsed as JSON. Cached per section (see [Cache key](#cache-key)).

System prompt (wrapped for readability, sent as one line):
```text
You extract requirements from one section of a software documentation file, given the file's entities.
A requirement is a single testable statement in EARS style (e.g. 'The system shall ...', 'When X, the system shall Y').
Return ONLY JSON, no prose, no fences.
Shape: {"requirements":[{"ears":string,"entities":[string],"evidence":string,"edges":[{"members":[string,string],"type":string}]}]}.
Extract only requirements stated in THIS section; return an empty array if it states none.
'entities' are names taken only from the provided list that the requirement is about.
'evidence' MUST be the exact, verbatim sentence or phrase copied character-for-character from the section that this requirement is based on (so it can be located in the text) — do not paraphrase it.
'edges' tie two of those entities together; 'type' is one of generalization, realization, composition, aggregation, association, dependency, reference (use reference if unsure).
Use only names from the provided list.
```

User message: the entity table (comma-separated entity names) followed by the section's raw markdown.
```text
Entities: <comma-separated entity names>

Section:
<section raw markdown>
```

Expected reply:
```json
{
  "requirements": [
    {
      "ears": "When a user submits ABC, the system shall validate DEF.",
      "entities": ["ABC", "DEF"],
      "evidence": "the system validates DEF on submit",
      "edges": [{ "members": ["ABC", "DEF"], "type": "dependency" }]
    }
  ]
}
```

`ears` becomes `earsText`, `evidence` is the verbatim span used for provenance, and each `edges` entry
becomes an `impliedEdge`. The bootstrap sends only the entity table and the section body, not ancestor
titles.

The reply is constrained by this schema, sent as
[structured output](../concepts/determinism.md#structured-output) (`response_format`, name
`requirements`):
```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["requirements"],
  "properties": {
    "requirements": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["ears", "entities", "evidence", "edges"],
        "properties": {
          "ears": { "type": "string" },
          "entities": { "type": "array", "items": { "type": "string" } },
          "evidence": { "type": "string" },
          "edges": {
            "type": "array",
            "items": {
              "type": "object",
              "additionalProperties": false,
              "required": ["members", "type"],
              "properties": {
                "members": { "type": "array", "items": { "type": "string" } },
                "type": {
                  "type": "string",
                  "enum": ["generalization", "realization", "composition", "aggregation", "association", "dependency", "reference"]
                }
              }
            }
          }
        }
      }
    }
  }
}
```

`members` is always a pair (the two entities the edge ties together).

## Diagnostics

- `non-ears` / `open-ended`: the statement cannot be cast as a verifiable requirement.
- `requirement-without-entity`: the requirement references no entity.
- `intra-section-contradiction`: a section contradicts itself.

## Cache key

Section content hash + entity-table hash + model id + prompt version. Cached per section inside
`target/<doc>/requirements.yaml`, so editing one section regenerates only that section (see
[storage layout](../artifacts.md#storage-layout)).

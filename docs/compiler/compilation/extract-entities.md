# Extract entities

Extract entities is step A2 of [compilation](../compilation.md#compilation). It finds the
[entities](../model/entity.md#entity) mentioned anywhere in one file. It uses an LLM.

## Consumes

The parsed sections of one file (from [Parse](./parse.md#parse)).

## Produces

An entities map (`localId` → entity). Each entry has:

- `name` and optional `aliases`.
- `linkage`: `internal` or `external`. Internal entities are private to this file. External entities
  are part of the file's link interface and are visible to the linker. See
  [Load interfaces](../linking/load-interfaces.md#load-link-interfaces).
- `role`: `definition` or `reference` (whether this file defines the entity or only mentions it).
- `scope`: `public`, `private`, or a named context, when the documentation indicates it. The model is
  prompted to capture an entity's [scope](../concepts/scopes.md#scopes) if the docs say it (e.g. "this
  entity is internal to the billing service"). Left as `public` (the default) when the docs say nothing.
- `provenance`: the sections and character spans it came from.
- `confidence`.

## LLM scope

The whole document. The model must see entities mentioned anywhere to dedup them and to judge
internal vs external. For files too large for the context window, chunk by top-level sections and add
a document-level merge pass that unifies duplicates across chunks. The model sees this file only,
never other files.

## Prompt

The exact call the bootstrap makes (`bootstrap/src/compile.rs`, `compile_text`). One call per file, at
temperature 0, parsed as JSON. The same call also produces each entity's local definition, so the
bootstrap folds [Local definitions](./local-definitions.md#local-definitions) (A5) into this step.

System prompt (wrapped for readability, sent as one line):
```text
You extract domain entities from a software documentation file.
An entity is any named concept: a component, a type, a field, a thing.
Return ONLY a JSON object, no prose, no markdown fences.
Shape: {"entities":[{"name":string,"linkage":"internal"|"external","role":"definition"|"reference","definition":string}]}.
'definition' is one short sentence describing the entity as this document presents it.
'role' is 'definition' if this document defines or specifies the entity, otherwise 'reference'.
'linkage' is 'external' if the entity is a shared concept other documents likely use, otherwise 'internal'.
```

User message: the file's full text, sent verbatim with no wrapper.

Expected reply:
```json
{
  "entities": [
    { "name": "Cart", "linkage": "external", "role": "definition", "definition": "Holds the products a customer intends to buy." }
  ]
}
```

`definition` is stored as the entity's `localDefinition`. `name`, `linkage`, and `role` populate the
fields above. `scope`, `aliases`, and `provenance` are not requested by this prompt in the bootstrap.

The reply is constrained by this schema, sent as
[structured output](../concepts/determinism.md#structured-output) (`response_format`, name `entities`):
```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["entities"],
  "properties": {
    "entities": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["name", "linkage", "role", "definition"],
        "properties": {
          "name": { "type": "string" },
          "linkage": { "type": "string", "enum": ["internal", "external"] },
          "role": { "type": "string", "enum": ["definition", "reference"] },
          "definition": { "type": "string" }
        }
      }
    }
  }
}
```

## Determinism

Run at low temperature with structured output so the entity set is stable run to run. Results are
reconciled against the prior object artifact so entity `localId`s stay stable. See
[Stable identity](../concepts/stable-identity.md#stable-identity).

## Diagnostics

- `low-confidence-entity`: extraction is uncertain.
- `intra-doc-name-clash`: one name is used for two different things in the same file.

## Cache key

File content hash + model id + prompt version. Stored in `target/<doc>/entities.yaml` (see
[storage layout](../artifacts.md#storage-layout)).

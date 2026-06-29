# Requirement

A requirement is an [EARS](../concepts/ears.md#ears) statement attached to one or more
[entities](./entity.md#entity), and optionally to a [relationship](./relationship.md#relationship).
EARS covers both behaviors and constraints.

E.g.:

```
The system shall ensure each User email is unique.
When a user submits a duplicate email, the system shall reject the registration.
```

## Fields

- `earsText`: the requirement in EARS form.
- `pattern`: the parsed EARS pattern (trigger, state, condition, response).
- `entityRefs`: the entities the requirement references.
- `impliedEdges`: the relationship edges the requirement produces (see below).
- `sourceSection`: the section it came from.
- `evidence`: the verbatim source snippet the requirement was extracted from. The model copies the
  exact text (rather than emitting fragile character offsets), so tooling can locate it to anchor
  [diagnostics](../../lsp/capabilities/diagnostics.md#mapping) to the precise span. The
  `sourceSection` is the section whose text contains this snippet.
- `provenance` and `confidence`.
- `verificationMethod`: optional. A hint for test generation.
- `reasoning`: optional. The why the docs give for it. See [Reasoning](../concepts/reasoning.md#reasoning).

## Behavior vs constraint

The behavior vs constraint distinction is a derived facet of the EARS pattern, not a separate type. A
`When ...` requirement is a behavior. A ubiquitous `shall ensure ...` is a constraint. The compiler
reads it off the pattern, so there is no separate requirement taxonomy to maintain.

## Requirements produce relationships

A requirement that references two or more entities produces a
[relationship](./relationship.md#relationship) between them, at minimum a weak `reference`. Edges are a
product of requirements, never extracted on their own.

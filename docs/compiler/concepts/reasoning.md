# Reasoning

`reasoning` is a single field that records the why behind a decision. It is stored next to whatever it
explains. It shows up in two places, but it is one idea:

- On a [requirement](../model/requirement.md#requirement), [entity](../model/entity.md#entity), or
  [relationship](../model/relationship.md#relationship): why it exists or is shaped that way, often
  taken from the docs' own explanation. E.g. "email must be unique because it is the login identifier."
  It carries provenance like any extracted fact.
- On a [diagnostic](../model/diagnostic.md#diagnostic): why the compiler emitted an error, a warning,
  or nothing at an ambiguity point. Because the compiler is LLM-backed, recording this makes its calls
  auditable.

## Disposition of an ambiguity

The outcome of an ambiguity is graded by how much ambiguity remains. The chosen disposition and its
reasoning are recorded:

| Ambiguity             | Disposition         | Recorded as                                                                    |
|-----------------------|---------------------|--------------------------------------------------------------------------------|
| none or trivial       | silent              | nothing                                                                        |
| small but real        | `none` (considered) | a diagnostic with severity `none` plus reasoning, hidden in the IDE by default |
| moderate              | `warning`           | a diagnostic plus reasoning                                                    |
| high or contradictory | `error`             | a diagnostic plus reasoning                                                    |

The `none` (considered) record is optional and threshold-gated to avoid noise. It is kept when the
ambiguity is worth revisiting later. It also gives continuity. If a later build raises the same case to
a warning, the earlier reasoning carries forward.

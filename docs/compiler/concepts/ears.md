# EARS

EARS (Easy Approach to Requirements Syntax) is the syntax Jazyk uses for
[requirements](../model/requirement.md#requirement). It is a small set of sentence patterns that keep a
requirement specific and testable while staying close to natural language.

EARS covers both behaviors and constraints, so Jazyk does not need a separate requirement taxonomy.
The pattern itself signals the kind.

## Patterns

- Ubiquitous: "The system shall `<response>`." E.g. "The system shall ensure each `User` email is
  unique." (a constraint)
- Event-driven: "When `<trigger>`, the system shall `<response>`."
- State-driven: "While `<state>`, the system shall `<response>`."
- Unwanted behavior: "If `<condition>`, then the system shall `<response>`."
- Optional feature: "Where `<feature>`, the system shall `<response>`."

A requirement stores the EARS text plus its parsed pattern (trigger, state, condition, response). The
behavior-vs-constraint distinction is derived from the pattern, not stored separately.

What EARS does not express is the entity itself. Entities are tracked as nodes that requirements point
at. See [Requirement](../model/requirement.md#requirement) and [Entity](../model/entity.md#entity).

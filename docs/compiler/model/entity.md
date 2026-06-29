# Entity

An entity is a domain concept of any granularity: a component, a field, a product, a shoe. Entities are
extracted by an LLM from the documentation. They are not authored in a fixed format, so the docs stay
flexible.

## Fields

- `name` and `aliases`: the handles used to resolve the entity across files.
- `linkage`: `internal` (private to one document) or `external` (visible to other documents). See
  [internal vs external](#internal-vs-external).
- `role`: `definition` (the doc specifies the entity) or `reference` (the doc only mentions it).
- `localDefinition`: what one document says about the entity. The matching surface used during linking.
- `globalDefinition`: the merged definition, produced after linking from all linked facts.
- `scope`: `public`, `private`, or a named context, captured from the documentation. Keeps distinct
  concepts with the same name apart and is a signal the linker uses when matching. See
  [Scopes](../concepts/scopes.md#scopes).
- `provenance` and `confidence`: source sections and the extraction confidence.
- `reasoning`: optional. The why behind the entity, when the docs explain it. See
  [Reasoning](../concepts/reasoning.md#reasoning).

## Declared vs defined

An entity is declared when a document merely mentions it, and defined when a document specifies it. The
distinction drives ownership and diagnostics. An entity referenced everywhere but defined nowhere is a
dangling reference.

## Internal vs external

Internal entities are private to one document and are never loaded by the linker. External entities are
part of a document's link interface: the linker loads only these to resolve entities across files. See
[Load link interfaces](../linking/load-interfaces.md#load-link-interfaces).

## Local vs global definition

Each document produces a local definition: what it knows about the entity so far. This is the matching
surface the linker compares. The global definition is synthesized after linking, once all facts are
available. See [Synthesize definitions](../linking/synthesize-definitions.md#synthesize-definitions).

## Identity

Entity ids must stay stable across builds so downstream artifacts do not churn. See
[Stable identity](../concepts/stable-identity.md#stable-identity).

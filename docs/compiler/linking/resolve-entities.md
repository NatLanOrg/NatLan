# Resolve entities

L2 decides which external entities across files are the same entity. It is conservative, and it is not
a pure name matcher: it weighs each entity's name, its [scope](../concepts/scopes.md#scopes), and its
local definition. Deeper judgment over the whole graph is still deferred to
[L4](./synthesize-definitions.md) and [L5](./semantic-review.md).

It assigns a stable global id to each resolved entity and records how each was resolved (the tier) and
which objects contribute to it.

## Resolution tiers

Strongest first. An in-document link (a relocation: a markdown link from a reference to a definition)
is the strongest signal, because the author stated it in the docs.

1. Direct link plus name match (`direct-link`). A relocation points at
   `customer.md#/customer-definition` and that section defines an entity of the same name. Confident
   link, no warning.
2. Direct link, section mismatch. The relocation points into `customer.md` but the same-named entity is
   under a different heading. Link, and warn (`link-section-mismatch`).
3. Name plus definition (`name-and-definition`). The same name is an entity in two or more docs, their
   scopes are compatible, and their local definitions describe the same thing. Link, no warning.
4. Name only (`name-only`). The same name, compatible scope, but the definitions are too thin to
   confirm. Link, and warn (`name-only-link`) suggesting the author add an in-document link or clarify
   the definition.

Kept apart: same name but different named scope, or same name and scope but definitions that describe
different things. These stay distinct, with no diagnostic, because the docs expressed the difference.

Unresolved: an external entity that matches nothing by link or name. Warn (`missing-link`). We do not
fuzzy-match and do not guess typos. `Cutsomer` does not auto-correct to `Customer`. It is a missing link
for a human (or L5) to resolve.

- Consumes: the link interfaces from [L1](./load-interfaces.md).
- Produces: the global entity table (the symbol table), each entity with members and resolution tier,
  plus resolution diagnostics.
- Deterministic: in-document links and exact (case and whitespace normalized) name comparison within
  scope are deterministic. Comparing local definitions to confirm or reject a borderline same-name match
  uses the LLM; it never fuzzy-matches names or guesses typos.
- Cache key: the link-interface hashes (which include the local definitions and scopes) plus the model
  id and prompt version.

Global ids are assigned with rename detection so they stay stable across builds. See
[stable identity](../concepts/stable-identity.md).

## Diagnostics

- `link-section-mismatch` (warn)
- `name-only-link` (warn)
- `missing-link` (warn)
- `dangling-relocation` (error): an explicit link target file or section does not exist.

E.g.:
```yaml
# consumes: the two link interfaces from L1
# produces: one global entity, resolved by tier 1
entities:
  "ent:Customer":
    canonicalName: Customer
    scope: "<scope>"
    members:
      - { object: customer.md, localId: e0, role: definition }
      - { object: abc.md,      localId: e1, role: reference }
    resolvedBy: direct-link
  "ent:Order":
    canonicalName: Order
    members: [{ object: abc.md, localId: e2, role: definition }]
diagnostics: []
```

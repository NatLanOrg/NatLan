# Completion

When authoring a cross-reference, the server suggests entities that already exist in the project, so a
reference resolves instead of becoming a
[missing link](../../compiler/linking/resolve-entities.md#resolve-entities).

LSP method: `textDocument/completion`.

## Candidates

Candidates are the [external entities](../../compiler/model/entity.md#internal-vs-external) known to the
project: those that are part of some document's link interface and are therefore visible to the linker.
Internal entities (private to one document) are not offered, because referencing them from another file
would not resolve.

The server matches the typed prefix against each entity's `name` and `aliases`, and filters to the
[scope](../../compiler/concepts/scopes.md#scopes) of the current file, since resolution only merges
within a scope.

## Items

Each completion item carries:

- The entity name as the insert text (using the document's cross-reference syntax where one applies).
- A short detail from the entity's definition, so near-duplicate names are distinguishable.
- The defining file, to disambiguate same-named entities in different scopes.

Completion is deliberately conservative: it offers entities that exist, mirroring how
[resolve entities](../../compiler/linking/resolve-entities.md#resolve-entities) does not fuzzy-match or
guess typos. It reduces missing links at authoring time rather than repairing them afterward.

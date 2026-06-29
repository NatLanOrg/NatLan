# Semantic tokens

Color every [entity](../../compiler/model/entity.md#entity) mention in a document, so the spans that
are command-clickable (see [definition](./definition.md#definition-and-references)) are visible.

LSP method: `textDocument/semanticTokens/full`.

## What is highlighted

The highlighted spans are exactly the entity-name occurrences the server uses for navigation: for
each global entity with a member in the file, every whole-word, case-insensitive occurrence of its
canonical name or an alias. Coloring and clickability come from the same enumeration, so what lights
up is what you can jump from.

Overlapping matches (e.g. `Product` inside `Product ID`) collapse to the longest match, the same rule
[definition](./definition.md#definition-and-references) uses, because semantic tokens may not overlap.

## Token type and modifiers

One token type, `entity`. Three modifiers carry the facets:

- `definition`: this file defines the entity (its member has `role: definition`).
- `external`: the entity is a shared concept (`linkage: external`), likely used by other documents.
- `unresolved`: the entity is referenced but defined nowhere (no member has `role: definition`). It
  also carries a [missing-link](../../compiler/linking/resolve-entities.md#resolve-entities)
  diagnostic.

The editor maps the type and modifiers to colors. The [VS Code](../editors/vscode.md#vs-code)
extension ships defaults: a base color for `entity`, underline for `definition`, italic for
`external`, and an error color for `unresolved`. Users can override these in their own settings.

## Timing

Tokens resolve after [linking](../../compiler/linking.md#linking) completes, the same point at which
[definition](./definition.md#definition-and-references) starts resolving, so highlighting and
navigation appear together. The editor re-requests tokens after each recompile, so they refresh as
the document changes.

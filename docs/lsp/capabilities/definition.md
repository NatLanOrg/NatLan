# Definition and references

Navigate the [entity](../../compiler/model/entity.md#entity) graph from the editor: jump from a mention
of an entity to where it is defined, and list everywhere it is used.

LSP methods: `textDocument/definition` and `textDocument/references`.

## Entities are the symbols

An entity is the unit of navigation. Each entity carries provenance: the
[sections](../../compiler/model/section.md#section) and character spans it was extracted from. A
section's location is `sourceFile + "#" + reference`, which the server turns into an LSP location.

The server maps the cursor position to the entity mentioned there (via the entity's provenance spans in
that file), then answers from the linked graph.

## Definition

Go to definition resolves to the entity's defining section: the section in the document where the
entity has `role: definition`. After [linking](../../compiler/linking.md#linking) an entity may be
defined in another file; the server follows the resolved global entity to that file and section. An
entity that is only referenced and defined nowhere is a dangling reference and has no definition target
(it will also carry a [missing-link](../../compiler/linking/resolve-entities.md#resolve-entities)
diagnostic).

## References

Find references lists every section that mentions the entity: all members of the resolved global
entity across files, plus the [requirements](../../compiler/model/requirement.md#requirement) and
[relationships](../../compiler/model/relationship.md#relationship) that involve it. This is the
"list everything that relates to an entity" view, backed by the global entity table produced by
[resolve entities](../../compiler/linking/resolve-entities.md#resolve-entities).

## Scopes

Resolution respects [scopes](../../compiler/concepts/scopes.md#scopes): two entities with the same name
in different bounded contexts are distinct, so navigation does not jump across a scope boundary to an
unrelated `Order`.

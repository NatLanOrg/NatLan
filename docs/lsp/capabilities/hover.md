# Hover

Hovering an entity mention shows what the compiler knows about it without leaving the document.

LSP method: `textDocument/hover`.

## Content

The server maps the hovered position to an [entity](../../compiler/model/entity.md#entity) and renders:

- Definition. The entity's `globalDefinition` if linking has synthesized one, otherwise its
  `localDefinition` for this document.
- Relationships. The [relationships](../../compiler/model/relationship.md#relationship) the entity
  participates in (type, the other member, and cardinality), so a reader sees how it connects without
  opening the graph.
- Requirements. The [requirements](../../compiler/model/requirement.md#requirement) that mention the
  entity, summarized.
- Diagnostics. Any open [diagnostics](./diagnostics.md#diagnostics) on the entity, with their
  [reasoning](../../compiler/concepts/reasoning.md#reasoning), so a hover explains why something is
  flagged.

## Notes

Hover reads the linked graph, so it reflects cross-file facts (the merged definition, relationships
contributed by other documents). It answers against the last completed build; see
[lifecycle](../lifecycle.md#incremental-recompile).

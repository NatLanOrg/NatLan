# Data model

The compiler turns documentation into a graph of typed nodes. There are five node types. One is
structural, the rest carry meaning.

Sections hold only the document structure (the parent/child tree). All semantic meaning lives in
entities, requirements, and relationships.

## Node types

- [Section](./model/section.md#section): a unit of document structure. Tree only.
- [Entity](./model/entity.md#entity): a domain concept (a component, a field, a product).
- [Requirement](./model/requirement.md#requirement): an EARS statement about one or more entities.
- [Relationship](./model/relationship.md#relationship): a typed edge between entities.
- [Diagnostic](./model/diagnostic.md#diagnostic): a warning or error the compiler recorded.

## How they relate

- A document is parsed into a tree of sections.
- Entities and requirements are extracted from the sections.
- A requirement references one or more entities. When it references two or more, it also produces a
  relationship between them.
- Diagnostics attach to any of the above and are persisted across builds.

Entities, requirements, and relationships are produced per file during
[compilation](./compilation.md#compilation), then resolved across files during
[linking](./linking.md#linking).

## Schema

The shared model definitions are in [this schema](./model.schema.yaml).

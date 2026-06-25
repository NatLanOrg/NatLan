# Stable identity

Downstream tools bind to entity ids. Generated code, tickets, and tests all reference an entity by id.
If the compiler re-ids or renames entities on every run, everything downstream churns. Entity identity
must stay stable across builds.

## Id scheme

An entity id is derived from stable inputs, not the raw extracted string:
- canonical name
- [scope](./scopes.md#scopes)
- alias table

## Rename detection

When an entity is renamed in the docs, the linker should keep its existing id rather than mint a new
one. This is the entity-level analog of section move detection in
[Compilation](../compilation.md#compilation). It lets downstream bindings survive a rename.

This is an open problem, and the hardest correctness issue in the linker. It depends on matching
entities across builds when their names or definitions shift.

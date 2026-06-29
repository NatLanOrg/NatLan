# Scopes

Entity resolution merges entities with the same name across files. Merging by name alone is unsafe.
Two different bounded contexts may both define `Order` and mean different things. A scope lets the
documentation say which same-named entities are the same and which are deliberately distinct.

Scope is a property of an [entity](../model/entity.md#entity), captured from the documentation during
[extract entities](../compilation/extract-entities.md#extract-entities), not a project setting. Three
kinds:

- `public` (the default): the entity participates in cross-file resolution across the whole project.
- `private`: the entity stays within its own document and is never merged with entities in other files.
- A named context (e.g. a microservice or bounded context like `billing` or `fulfillment`): the entity
  only resolves with same-named entities that share that scope.

Two `Order` entities marked with different named scopes stay separate, and that separation is
intentional and recorded in the docs, so it raises no diagnostic. To keep same-named concepts apart, an
author states their scope in the documentation (e.g. "this `Order` is internal to the billing service").
To force a merge, an author either makes the definitions agree or adds an in-document link from the
reference to the definition.

Scope is one of the signals [Resolve entities](../linking/resolve-entities.md#resolve-entities) uses;
the entity's definition is another. The linker is not a pure name matcher.

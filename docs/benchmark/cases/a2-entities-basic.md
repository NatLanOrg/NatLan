# a2-entities-basic

[A2 extract entities](../../compiler/compilation/extract-entities.md#extract-entities) on a small
document. The model must find the named domain concepts and return them in the entity shape. This is
the most basic check that a model can extract at all.

See [Cases](../cases.md#cases) for the format and [Checks](../checks.md#checks) for the checks.

```yaml
stage: A2
input:
  doc: |
    # Cart

    A Cart holds the Products a Customer intends to buy. Each Cart belongs to one Customer.
    A Product has a price and a name.
expect:
  schema: entities
  assertions:
    - kind: presence
      path: entities[name=Cart]
      note: Cart is extracted as an entity
    - kind: presence
      path: entities[name=Product]
      note: Product is extracted as an entity
    - kind: presence
      path: entities[name=Customer]
      note: Customer is extracted as an entity
    - kind: field
      path: entities[name=Cart].role
      value: definition
      note: the document defines Cart, so its role is definition
  judge:
    rubric: >-
      Each entity's definition is a single accurate sentence describing the concept as this document
      presents it, with no invented detail.
    threshold: 0.6
```

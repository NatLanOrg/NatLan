# a4-relationship-conflict

[A4 consolidate relationships](../../compiler/compilation/consolidate-relationships.md#consolidate-relationships)
when two requirements imply different [types](../../compiler/model.schema.yaml) for the same entity
pair. A4 is deterministic for the common case and only calls the model to reconcile a genuine type
conflict. The model must pick the strongest defensible type rather than average or drop the edge.

See [Cases](../cases.md#cases) and [Checks](../checks.md#checks).

```yaml
stage: A4
input:
  pair: [Order, Customer]
  edges:
    - type: association
      requirement: An Order is associated with a Customer.
    - type: aggregation
      requirement: A Customer has many Orders.
expect:
  schema: relationship
  assertions:
    - kind: field
      path: relationship.type
      value: aggregation
      note: aggregation is stronger than association, so it wins the consolidation
    - kind: presence
      path: relationship.requirements
      note: the consolidated relationship keeps both contributing requirements
  judge:
    rubric: >-
      The chosen type is the strongest one the two requirements jointly support, with reasoning that
      explains why the weaker candidate was subsumed rather than contradicted.
    threshold: 0.5
```

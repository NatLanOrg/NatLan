# a3-requirements-edges

[A3 extract requirements](../../compiler/compilation/extract-requirements.md#extract-requirements),
checking the implied edge between entities. A sentence that says one thing is composed of another must
yield an edge of the right [relationship type](../../compiler/model.schema.yaml). Edges are the only
source of [relationships](../../compiler/model/relationship.md#relationship), so the model must type
them correctly.

See [Cases](../cases.md#cases) and [Checks](../checks.md#checks).

```yaml
stage: A3
input:
  section: |
    An Order is composed of one or more OrderLines. The system shall reject an Order with no
    OrderLines.
  entities: [Order, OrderLine, System]
expect:
  schema: requirements
  assertions:
    - kind: presence
      path: requirements[0].edges[0]
      note: an edge between Order and OrderLine is implied
    - kind: field
      path: requirements[0].edges[members=Order,OrderLine].type
      value: composition
      note: an Order is composed of OrderLines, so the edge is a composition
  judge:
    rubric: >-
      The requirement captures that an Order must have at least one OrderLine, and the edge type
      reflects composition rather than a weaker association or reference.
    threshold: 0.6
```

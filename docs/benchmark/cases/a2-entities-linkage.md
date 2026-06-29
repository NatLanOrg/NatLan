# a2-entities-linkage

[A2 extract entities](../../compiler/compilation/extract-entities.md#extract-entities), checking the
[`linkage`](../../compiler/model/entity.md#internal-vs-external) classification. A shared concept other
documents would use must be `external`; a concept private to this document must be `internal`. Getting
linkage wrong breaks [linking](../../compiler/linking.md#linking) downstream, so this is a load-bearing
judgment, not cosmetic.

See [Cases](../cases.md#cases) and [Checks](../checks.md#checks).

```yaml
stage: A2
input:
  doc: |
    # Invoice

    An Invoice bills a Customer for an Order. Internally, the Invoice is rendered by a
    LayoutEngine that is specific to this document's billing module.
expect:
  schema: entities
  assertions:
    - kind: field
      path: entities[name=Customer].linkage
      value: external
      note: Customer is a shared concept other documents use
    - kind: field
      path: entities[name=Order].linkage
      value: external
      note: Order is a shared concept other documents use
    - kind: field
      path: entities[name=LayoutEngine].linkage
      value: internal
      note: LayoutEngine is private to this document
```

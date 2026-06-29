# l4-definition-false-merge

[L4 synthesize definitions](../../compiler/linking/synthesize-definitions.md#synthesize-definitions)
when two documents use the same name for different concepts. The model must judge the definitions
incoherent and emit a [false-merge](../../compiler/linking/synthesize-definitions.md#synthesize-definitions)
diagnostic instead of inventing a definition that papers over the conflict. This is the negative case:
the model must reject a bad match.

See [Cases](../cases.md#cases) and [Checks](../checks.md#checks).

```yaml
stage: L4
input:
  name: Order
  definitions:
    - source: sales.md
      text: An Order is a customer's request to purchase products.
    - source: kitchen.md
      text: An Order is the sequence in which dishes are plated and sent out.
expect:
  schema: synthesis
  assertions:
    - kind: field
      path: synthesis.coherent
      value: false
      note: the two documents mean different things by Order
  mustEmit:
    - false-merge
  judge:
    rubric: >-
      The reasoning identifies that the two Orders are distinct concepts that happen to share a name,
      and does not fabricate a merged definition.
    threshold: 0.5
```

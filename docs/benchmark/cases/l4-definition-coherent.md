# l4-definition-coherent

[L4 synthesize definitions](../../compiler/linking/synthesize-definitions.md#synthesize-definitions)
when two documents describe the same entity consistently. The model must judge the definitions coherent
and merge them into one global definition, without raising a
[false-merge](../../compiler/linking/synthesize-definitions.md#synthesize-definitions). This is the
positive case: the model should accept a true match.

See [Cases](../cases.md#cases) and [Checks](../checks.md#checks).

```yaml
stage: L4
input:
  name: Customer
  definitions:
    - source: billing.md
      text: A Customer is a person or organization that is billed for orders.
    - source: crm.md
      text: A Customer is an account the business sells to and invoices.
expect:
  schema: synthesis
  assertions:
    - kind: field
      path: synthesis.coherent
      value: true
      note: both documents describe the same concept
  mustNotEmit:
    - false-merge
  judge:
    rubric: >-
      The synthesized definition is one sentence that fairly combines both documents' views of Customer
      without contradicting either or inventing detail.
    threshold: 0.6
```

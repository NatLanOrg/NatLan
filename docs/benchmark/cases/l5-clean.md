# l5-clean

[L5 semantic review](../../compiler/linking/semantic-review.md#semantic-review) when the requirements
for an entity are consistent. The model must stay quiet and return no
[diagnostics](../../compiler/model/diagnostic.md#diagnostic). This is the negative case: it guards
against a model that invents problems, which would bury real diagnostics in noise and make the compiler
untrustworthy.

See [Cases](../cases.md#cases) and [Checks](../checks.md#checks).

```yaml
stage: L5
input:
  name: User
  requirements:
    - The system shall ensure each User email is unique.
    - When a User registers, the system shall send a confirmation email.
expect:
  schema: diagnostics
  assertions:
    - kind: field
      path: diagnostics.length
      value: 0
      note: consistent requirements produce no diagnostics
  mustNotEmit:
    - cross-doc-contradiction
    - redefinition
    - overlapping-requirements
```

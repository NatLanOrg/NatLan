# l5-contradiction

[L5 semantic review](../../compiler/linking/semantic-review.md#semantic-review) when two documents make
incompatible claims about one entity. The model must surface a `cross-doc-contradiction`
[diagnostic](../../compiler/model/diagnostic.md#diagnostic). This is the positive case: the model must
catch a real problem that only appears once requirements are gathered across documents.

See [Cases](../cases.md#cases) and [Checks](../checks.md#checks).

```yaml
stage: L5
input:
  name: ABC
  requirements:
    - The ABC is a tricycle and shall have three wheels.
    - The ABC is a motorcycle and shall have two wheels.
expect:
  schema: diagnostics
  assertions:
    - kind: field
      path: diagnostics[rule=cross-doc-contradiction].severity
      value: error
      note: an unreconciled contradiction is an error
  mustEmit:
    - cross-doc-contradiction
  judge:
    rubric: >-
      The message names the specific incompatible claims (wheel count and vehicle class) rather than
      vaguely asserting a conflict.
    threshold: 0.6
```

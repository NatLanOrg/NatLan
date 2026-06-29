# a3-requirements-ears

[A3 extract requirements](../../compiler/compilation/extract-requirements.md#extract-requirements) on a
section that states a clear rule. The model must produce a single testable
[EARS](../../compiler/concepts/ears.md#ears) requirement that references the right entities. This checks
that the model can turn prose into a structured, testable statement.

See [Cases](../cases.md#cases) and [Checks](../checks.md#checks).

```yaml
stage: A3
input:
  section: |
    The system shall ensure each User email is unique.
  entities: [User, System]
expect:
  schema: requirements
  assertions:
    - kind: presence
      path: requirements[0]
      note: at least one requirement is extracted
    - kind: substring
      path: requirements[0].ears
      value: shall
      note: the requirement is phrased in EARS style
    - kind: presence
      path: requirements[0].entities[=User]
      note: the requirement references the User entity
  judge:
    rubric: >-
      The requirement is a single, testable, ubiquitous EARS statement that captures the email
      uniqueness constraint and nothing more. It references only entities from the provided list.
    threshold: 0.6
```

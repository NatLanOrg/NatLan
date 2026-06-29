# Cases

A case is one predefined test: an input fed to a single LLM stage, and the expectations its output must
meet. Each case lives in its own file under [`cases/`](./cases), so a case states both what is sent to
the model and what counts as a good answer.

## Case format

A case file is Markdown: prose explaining the case's intent, followed by a fenced `yaml` block with the
structured case. The structure is defined by [case.schema.yaml](./case.schema.yaml).

- `stage`. The LLM stage under test: `A2`, `A3`, `A4`, `L4`, or `L5`.
- `input`. The predefined input for that stage, shaped to match what the stage receives during a real
  build (e.g. a whole document for A2; a section plus an entity list for A3; an entity's local
  definitions for L4).
- `expect`. The checks to run, see [Checks](./checks.md#checks):
  - `schema`. The output shape the stage must produce.
  - `assertions`. A list of deterministic expectations (presence, field value, substring, emission).
  - `judge`. An optional rubric and threshold for grading free-text output.
  - `mustEmit` / `mustNotEmit`. [Diagnostic](../compiler/model/diagnostic.md#diagnostic) rules that
    must, or must not, appear (for the diagnostic-producing stages).

## Index

Each stage has at least one positive case and, where it makes sense, a negative case, so the benchmark
catches both failure to produce good output and failure to stay quiet when the input is clean.

- A2 extract entities
  - [a2-entities-basic](./cases/a2-entities-basic.md)
  - [a2-entities-linkage](./cases/a2-entities-linkage.md)
- A3 extract requirements
  - [a3-requirements-ears](./cases/a3-requirements-ears.md)
  - [a3-requirements-edges](./cases/a3-requirements-edges.md)
- A4 consolidate relationships
  - [a4-relationship-conflict](./cases/a4-relationship-conflict.md)
- L4 synthesize definitions
  - [l4-definition-coherent](./cases/l4-definition-coherent.md)
  - [l4-definition-false-merge](./cases/l4-definition-false-merge.md)
- L5 semantic review
  - [l5-contradiction](./cases/l5-contradiction.md)
  - [l5-clean](./cases/l5-clean.md)

# Benchmark

The benchmark tests whether a given LLM is good enough to compile Jazyk. The
[compiler](../compiler/compiler.md#compiler) drives several of its stages with an LLM, and a weak or
mis-configured model silently produces broken artifacts. The benchmark runs predefined cases against
each of those stages and reports a score and a verdict.

It is run from the [CLI](../cli.md#cli) as `jazyk benchmark`. It is a development and operations tool,
not a [consumer](../main.md#consumers) of build artifacts: it grades the model, not a project.

## Definition

The benchmark runs a fixed set of [cases](./cases.md#cases). Each case feeds a predefined
input to one LLM stage and asserts that the output is good enough through three
[checks](./checks.md#checks): a schema check, deterministic assertions, and the model
[judging its own](./checks.md#judge-check) free-text output. Case results combine into per
stage and overall [scores](./scoring.md#scoring) and a final verdict: usable for compilation
or not.

Alongside the quality checks, the benchmark streams the responses to measure generation
[speed](./scoring.md#throughput) across all cases, splitting time to first token from the token decode
rate, since a correct but slow model is not usable for compilation. An output speed below the floor
fails the verdict the same way a failing stage does; time to first token is reported but does not gate.

The model is exercised exactly as it would be during a real build. The benchmark reads the
[`[llm]` settings](../compiler/project-settings.md#llm) from `jazyk.toml` and honors the same
`--llm-base-url` and `--model` overrides as [build and check](../cli.md#cli), so the score reflects the
model and endpoint that would actually compile the project.

### Stages covered

The benchmark targets the LLM-invoking stages of [compilation](../compiler/compilation.md#compilation)
and [linking](../compiler/linking.md#linking):

- A2 [extract entities](../compiler/compilation/extract-entities.md#extract-entities).
- A3 [extract requirements](../compiler/compilation/extract-requirements.md#extract-requirements).
- A4 [consolidate relationships](../compiler/compilation/consolidate-relationships.md#consolidate-relationships),
  the relationship-type conflict reconciliation.
- L4 [synthesize definitions](../compiler/linking/synthesize-definitions.md#synthesize-definitions).
- L5 [semantic review](../compiler/linking/semantic-review.md#semantic-review).

Deterministic stages (parsing, entity resolution) are not benchmarked: they do not depend on the model.

## Internals

- [Checks](./checks.md#checks). The schema, assertion, and judge checks a case can run.
- [Scoring](./scoring.md#scoring). How checks combine into scores and the final verdict.
- [Cases](./cases.md#cases). The case file format and the case index.

## Not LLM static analysis

The benchmark is distinct from [LLM static analysis](../llm-test.md), which uses the LLM to analyze the
*code* generated from a project. The benchmark tests the *model's* fitness to run the compiler.

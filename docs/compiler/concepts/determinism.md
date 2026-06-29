# Determinism

Jazyk is a compiler, and ideally the same project compiles to the same artifacts. It is mostly
driven by an LLM, which is not deterministic. There are several factors that attempt to stabilize
the output.

## Docs ambiguity is a compile error

When the documentation is ambiguous, the compiler attempts to surface the ambiguity as a compile
error.

*TODO link to ambiquity detection*

## Process only changed

A rebuild with unchanged inputs returns the previous output if it exists. Each step in
[Compilation](../compilation.md#compilation) and [Linking](../linking.md#linking) states its cache key that
determines whether the step can be skipped.

The cache is the per-stage artifact files under `target/` in the out dir (see
[storage layout](../artifacts.md#storage-layout)), not a separate hash-named store. Each file's
`jazyk` header comment records the key its result was computed from. Before running a step the
compiler recomputes that key from the current inputs: if it matches the stored header, the step is
skipped and the file is reused. Only the latest result per file and stage is kept, so reverting a
change recomputes rather than restoring an older cached result.

## Heavy prompting and scoped work

Each LLM task is narrowly scoped with clear expectations via prompting to minimize ambiguity.

## Structured output

Every LLM stage that returns data sends a JSON Schema with the request and constrains the model to it,
rather than relying on prompt wording alone. The schema is sent as the OpenAI-compatible
`response_format` field:

```json
"response_format": {
  "type": "json_schema",
  "json_schema": { "name": "<call name>", "strict": true, "schema": { ... } }
}
```

Each compilation and linking stage states its own schema. See
[Extract entities](../compilation/extract-entities.md#prompt),
[Extract requirements](../compilation/extract-requirements.md#prompt),
[Synthesize definitions](../linking/synthesize-definitions.md#prompt), and
[Semantic review](../linking/semantic-review.md#prompt).

The schema and the prompt's `Shape:` description are kept in sync. The schema constrains the model, the
prose explains the fields. When an endpoint does not support structured output, the call falls back to
prompt-only JSON (see [Retries](#retries)). The first rejection is sticky for the rest of the run, so
later calls skip `response_format` instead of paying a rejected request every time. Either way the
tolerant parser still extracts the first JSON object from the reply.

## Reasoning persisted

The results of each LLM task have a reasoning field to include additional information why certain decisions
have been made.

## Retries

LLM stages expect a structured JSON response. Because the model is probabilistic and the transport can
fail, each call is retried a bounded number of times before the step gives up:

- **Malformed output.** If the response is not valid JSON of the expected shape (truncated, wrapped in
  prose, wrong type), the call is retried. A successful result is [cached](#process-only-changed), so
  the retry cost is paid only once.
- **Transient transport errors.** Gateway and `5xx` responses and dropped connections are retried.

Retries run immediately, with no backoff delay. The local model serializes work behind a concurrency
cap, so spacing retries out adds latency without relieving load. Each retry logs the call's label (the
file and stage, or the linked entity) and the reason, so a failing call is identifiable in the output
rather than an anonymous counter.
- **Unsupported parameters.** A model that rejects a non-default temperature is retried once with the
  field omitted (see [project settings](../project-settings.md#global-configuration)). A model that
  rejects the [structured output](#structured-output) `response_format` is retried once without it,
  falling back to prompt-only JSON for the rest of the run.

The retry budget is bounded and configurable, so a persistently failing call surfaces an error rather
than looping forever. Retries do not change the [cache key](#process-only-changed).

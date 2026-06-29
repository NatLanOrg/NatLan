# Checks

A [case](./cases.md#cases) verifies a stage's output with up to three kinds of check. Each check
produces a sub-score from 0 to 1; [scoring](./scoring.md#scoring) combines them.

## Schema check

The stage output is requested the same way the [compiler](../compiler/compiler.md#compiler) requests it
during a real build: the stage's JSON schema is sent as a structured-output constraint
(`response_format: json_schema`, `strict`), so a conforming endpoint is steered to the right shape and
valid enum values at generation time. The benchmark therefore measures the model as the build uses it,
not a freeform prompt the build never issues. An endpoint that ignores the constraint (or rejects it and
falls back to prompt-only JSON) is still held to the same bar by the structural check below.

The check is **structural**, not a test that a key merely exists. It validates, in order:

- **Container.** The output is a JSON object holding the stage's top-level key (`entities`,
  `requirements`, `type`, `definition`, `coherent`, `diagnostics`).
- **Non-empty.** A list the stage must populate (entities, requirements, diagnostics) is a JSON array
  with at least one element. A stage that should produce items but returns `[]` fails.
- **Item fields.** Every element carries its required fields with the right type: text fields
  (`name`, `ears`, `rule`, `message`, `reasoning`) are non-empty strings, not empty or whitespace;
  reference fields (`entities`) are non-empty arrays of strings. An entity's `definition` is required
  non-empty only when its `role` is `definition`; a `reference`-role entity is only mentioned in the
  input, so its `definition` may be empty or absent (if present it must still be a string).
- **Enums.** A field constrained to an enum holds a value from the shared definitions in
  [model.schema.yaml](../compiler/model.schema.yaml), matched case-insensitively: `role` for entities,
  `relationshipType` for the relationship `type`, `severity` for diagnostics. An invented value (e.g.
  `role: entity`, `type: banana`, `severity: critical`) fails.
- **EARS shape.** For requirements, every `ears` string is EARS-shaped: it contains the mandatory
  `shall` keyword. A requirement whose `ears` is a bare entity name or prose without `shall` fails, even
  though the surrounding JSON parses.

This is what separates a model that returns conforming structure from one that returns a parseable blob
with the right keys but wrong contents.

The schema check is a **hard gate**. A model that cannot return parseable, conforming JSON cannot drive
the compiler, so a failed schema check fails the whole case regardless of the other checks. Its
sub-score is 1 (conforms) or 0 (does not). The first structural violation is reported with the offending
path and value (e.g. `requirements[0].ears missing 'shall'`), so a failure is debuggable.

## Assertion checks

Deterministic expectations about the content, stated by the case. They do not call the LLM, so they are
exact and repeatable. Assertions are **field-targeted**: each names the field or array it inspects, so a
value only counts when it appears where it carries meaning, not anywhere in the serialized output. A
case that wants `type: composition` is not satisfied by the model picking `association` and mentioning
the word "composition" in its `reasoning`. Each assertion is one of:

- **Presence.** A named node exists, e.g. an [entity](../compiler/model/entity.md#entity) named
  `Customer` with `linkage: external`, or a [requirement](../compiler/model/requirement.md#requirement)
  referencing two given entities. Matched against the relevant item field (an entity's `name`), not the
  whole blob.
- **Field value.** A field has an expected value, e.g. the relationship `type` equals `composition`, the
  `coherent` flag is `true`, or a [diagnostic](../compiler/model/diagnostic.md#diagnostic) has
  `severity: error`. Compared case-insensitively after trimming.
- **Substring.** A required string appears in a named free-text field. Two forms: `any`, the substring
  appears in that field of at least one array element (e.g. some requirement's `ears` mentions
  `unique`); and `each`, it appears in that field of every element (e.g. every requirement's `ears`
  contains `shall`).
- **Emission.** A diagnostic of a given `rule` is emitted (`mustEmit`) or is not (`mustNotEmit`), e.g.
  a contradiction case `mustEmit` `cross-doc-contradiction`, a coherent merge `mustNotEmit`
  `false-merge`.

The assertion sub-score is the fraction of the case's assertions that pass.

## Judge check

Some fields are free text where no exact match is right: an entity's synthesized `definition`, a
requirement's [EARS](../compiler/concepts/ears.md#ears) phrasing, a diagnostic's `message` and
`reasoning`. For these the case states a rubric, and the **model under test scores its own output**
from 0 to 1 against that rubric.

### Self-judging caveat

The judge is the same model being benchmarked, so the judge is only as good as the model. A weak model
is also a weak judge, and may rate its own poor output highly. The judge check is therefore a **soft
signal**, weighted below the others in [scoring](./scoring.md#scoring). The schema and assertion checks
are the reliable gates; the judge check adds resolution between models that both clear those gates.

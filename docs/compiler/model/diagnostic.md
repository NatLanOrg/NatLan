# Diagnostic

A diagnostic is a judgment the compiler made about the spec: a contradiction, an ambiguity, a
fragmentation, an unused entity, and so on. Diagnostics are first-class nodes. They are persisted and
reconciled across builds, so they stay stable. See
[Sticky diagnostics](../concepts/sticky-diagnostics.md#sticky-diagnostics).

## Fields

- `id` and `fingerprint`: a stable identity derived from what the diagnostic is about, not from the
  LLM's wording.
- `rule`: the rule that produced it. Each step lists the rules it can emit.
- `severity`: `error`, `warning`, `info`, or `none`. `none` is a considered judgment that was recorded
  but not surfaced.
- `subjects`: the nodes it concerns (entity, requirement, relationship, or section ids).
- `message`: the human facing text.
- `reasoning`: why this severity was chosen. See [Reasoning](../concepts/reasoning.md#reasoning).
- `lifecycle`: `open`, `resolved`, `superseded`, or `merged`.
- `triage`: `acknowledged`, `suppressed`, or `wontfix`, set by a human. Survives recompilation.
- `provenance` and `confidence`.
- `firstSeen` and `lastSeen`: build markers. Plus links to merged or related diagnostics.

## Why first class

Diagnostics are stored so IDEs and CI can read them, human triage survives across builds, and a
recompile reconciles them instead of regenerating them. See
[Sticky diagnostics](../concepts/sticky-diagnostics.md#sticky-diagnostics).

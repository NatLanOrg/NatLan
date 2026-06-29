# Stable diagnostics

If you ask an LLM to produce a warning or error, it will word it differently every time.
For this reason, diagnostics are updated across recompilations, not regenerated.

## Stable identity

Each diagnostic is stored in the [diagnostics store](../artifacts/diagnostics-store.md#diagnostics-store)
with a stable id and fingerprint. The fingerprint is derived from what the issue is about (rule,
subject node ids, and a coarse semantic fingerprint), not from the LLM's wording.

## Reconcile, not regenerate

On recompile, each diagnostic-emitting step is given the prior diagnostics for the scope it recomputes.
For each finding it decides:
- `keep`
- `update`
- `resolve`

New diagnostics are created only for genuinely new issues. Survivors keep their id and, unless
materially changed, their wording and [reasoning](./reasoning.md#reasoning).

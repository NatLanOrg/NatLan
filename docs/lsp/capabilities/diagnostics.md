# Diagnostics

The server publishes compiler [diagnostics](../../compiler/model/diagnostic.md#diagnostic) to the
editor so ambiguity, contradictions, and missing links show up inline as you write.

LSP method: `textDocument/publishDiagnostics` (server to client, push).

## Source

Diagnostics come from the [diagnostics store](../../compiler/artifacts/diagnostics-store.md#diagnostics-store),
which is built to be read by IDEs over LSP. The server does not invent diagnostics; it maps stored ones
onto the open documents.

## Mapping

- Location. A diagnostic's `subjects` are node ids (entities, requirements, relationships, sections).
  The server resolves each subject to a precise character span in the file by **locating text rather
  than trusting stored offsets**:
  - A **requirement** subject anchors to its
    [`evidence`](../../compiler/model/requirement.md#fields) snippet — the verbatim source text the
    model copied during extraction — found as a substring of the document.
  - An **entity** subject anchors to the first occurrence of the entity's
    [name](../../compiler/model/entity.md#entity) in the file (whole word, with a substring fallback so
    a plural like "Products" still matches the `Product` entity), in each file the entity belongs to.
  - When neither matches, it falls back to the relevant
    [section](../../compiler/model/section.md#section) heading.

  This quote-and-locate approach keeps the LLM stages from having to emit fragile character offsets:
  the model decides *which* text is relevant by quoting it, and the server computes the LSP range.
- Severity. `error` and `warning` map to the matching LSP severities; `info` maps to Information.
  Severity `none` is a considered judgment that is not surfaced, so it is not published (it remains
  visible to tooling that reads the store directly).
- Related information. `subjects` and related diagnostics in other files become LSP
  `relatedInformation`, so a cross-doc contradiction links to both sides.
- Message and reasoning. The `message` is the diagnostic text; the
  [reasoning](../../compiler/concepts/reasoning.md#reasoning) can be shown as related detail or on
  [hover](./hover.md#hover).

## Stickiness and triage

Diagnostics are [sticky](../../compiler/concepts/sticky-diagnostics.md#sticky-diagnostics). A recompile
reconciles against the stored diagnostics (`keep`, `update`, `resolve`, `merge`, `split`) instead of
regenerating them, so the editor's diagnostic list does not churn on every keystroke and ids stay
stable.

Human triage (`acknowledged`, `suppressed`, `wontfix`) is bound to the diagnostic id and survives
recompiles. The server hides `suppressed` and `wontfix` diagnostics by default and can offer a code
action to set triage, writing it back to the store so it persists across sessions and is shared with
CI.

## Republish

Diagnostics are published [progressively](../lifecycle.md#progressive-results) as a build proceeds:
per-file compilation diagnostics as each file finishes, then the cross-file linking diagnostics when
linking completes, so issues surface without waiting for the whole build. After each
[incremental recompile](../lifecycle.md#incremental-recompile) the server republishes diagnostics for
the affected files. Files whose diagnostics did not change are not republished.

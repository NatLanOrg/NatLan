# Diagnostics store

Warnings and errors are persisted in a diagnostics store. It survives recompilation and is keyed by a
stable diagnostic id. IDEs (over LSP) and CI read it directly.

Diagnostics are sticky. A rebuild does not recreate them from scratch. It reconciles against the
stored ones (keep, update, resolve, merge, split). Human triage (`acknowledged`, `suppressed`,
`wontfix`) is bound to the id and survives. See
[sticky diagnostics](../concepts/sticky-diagnostics.md#sticky-diagnostics).

Each diagnostic carries [reasoning](../concepts/reasoning.md#reasoning): why the compiler chose its
severity, or why it stayed silent. Severity can be `none`, a considered judgment that is recorded but
not surfaced.

See the [diagnostic](../model/diagnostic.md#diagnostic) node type for the full set of fields.

## Shape

E.g.:
```yaml
diagnostics:
  - id: "diag:cross-doc-contradiction:ent:ABC:7f3a"   # rule + subjects + fingerprint
    rule: "cross-doc-contradiction"
    severity: "error"            # error | warning | info | none (considered)
    subjects: ["ent:ABC"]
    message: "abc.md implies ABC is a tricycle (3 wheels); xyz.md states ABC is a 3-wheeled motorcycle."
    reasoning: "Both docs fix wheel count with incompatible vehicle classes; no explicit link reconciles them; high confidence both cannot hold."
    sources: ["abc.md#/abc/wheels", "xyz.md#/vehicles/abc"]
    confidence: 0.9
    lifecycle: "open"            # open | resolved | superseded | merged
    triage: "acknowledged"       # null | acknowledged | suppressed | wontfix  (human-set, sticky)
    firstSeen: "build:..."
    lastSeen:  "build:..."
    related: []                  # merged or superseded diagnostic ids
  - id: "diag:missing-link:ent:buyer~ent:Customer:0b21"
    rule: "missing-link"          # a judgment that was considered but not surfaced
    severity: "none"             # considered, not surfaced
    subjects: ["ent:buyer", "ent:Customer"]
    message: "'buyer' might be the same entity as 'Customer'."
    reasoning: "Their definitions overlap, but each doc uses the term in a distinct context; not confident enough to surface, recorded for continuity."
    lifecycle: "open"
    triage: null
```

## Schema

The diagnostics store follows [this schema](./diagnostics-store.schema.yaml).

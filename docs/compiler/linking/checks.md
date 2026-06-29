# Checks

L6 runs whole-program checks over the global graph: reachability, fitness rules, and coverage.

## Reachability and usage

This is the dead-code analog. It flags entities that are neither a declared root nor reachable from
one.

- `unused-entity` (warn): a component defined in a file but referenced nowhere (no incoming edge, no
  other requirement mentions it).
- `unreachable-entity` (warn): a disconnected island reachable from no root (e.g. A and B reference each
  other but nothing else points to them).

Roots are declared in `jazyk.toml` as a set of files: every entity defined in a root file is a root
entity. Roots can also be inferred (top-level or public components, entities referenced by external
systems). A pure "no incoming references" check over-flags legitimate top-level components, so the
conservative default is to flag only entities with zero incoming references as `unused-entity`.
Reachability from root entities is the stronger check once roots exist. See the roots setting in
[project settings](../project-settings.md#roots).

## Fitness rules (spec-lint)

Architectural rules written in plain English, evaluated over the graph. E.g.: "every persisted entity
declares a uniqueness constraint", or "no entity depends on more than N others".

## Coverage

Graph queries over the result. E.g.: requirements with no derivable test, entities with no behavior.

- Consumes: the global graph plus configured rules.
- Produces: configurable warnings and errors, plus a coverage report.
- Deterministic: reachability and coverage are deterministic graph queries. Fitness rules are evaluated
  by an LLM per matching graph slice.
- LLM scope (fitness rules only): the graph slice the rule targets (an entity plus its relationships and
  requirements). Not prose.
- Cache key: rule text plus targeted graph-slice hash plus model id.

## Diagnostics

- `unused-entity` (warn)
- `unreachable-entity` (warn)
- `<custom rule>` (configurable)
- `no-behavior`, `no-test` (warn or info)

E.g.:
```yaml
# consumes: the graph + configured rules
rule: "every persisted entity must declare a uniqueness constraint"
# produces
diagnostics:
  - { rule: unused-entity, severity: warning,
      message: "ent:LegacyWidget is defined in widgets.md but referenced by no requirement or relationship anywhere.",
      subjects: [ent:LegacyWidget] }
  - { rule: spec-lint/uniqueness, severity: warning,
      message: "ent:Order is persisted but declares no uniqueness constraint.", subjects: [ent:Order] }
coverage:
  - { entity: ent:Customer, behaviors: 3, constraints: 2, testsDerivable: 4 }
  - { entity: ent:Order, behaviors: 1, constraints: 0, testsDerivable: 1 }
```

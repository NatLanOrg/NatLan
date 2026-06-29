# Linking

Linking runs over all compiled object artifacts and resolves them into one global graph. It is the
linker. Where [compilation](./compilation.md) works on one file at a time, linking works on the whole
program.

Linking has two sub-stages:
- Resolve across files (L1 → L3): match shared entities and re-key relationships to global entities.
- Validate together (L4 → L6): synthesize definitions and check the merged graph.

## Steps

- [L1 Load link interfaces](./linking/load-interfaces.md): load each object's external entities, local
  definitions, external edges, and relocations.
- [L2 Resolve entities](./linking/resolve-entities.md): match external entities across files into
  global entities.
- [L3 Merge relationships](./linking/merge-relationships.md): re-key edges to global entities and merge
  duplicates.
- [L4 Synthesize definitions](./linking/synthesize-definitions.md): build each entity's global
  definition, and validate the merge.
- [L5 Cross-doc semantic review](./linking/semantic-review.md): find contradictions, missing links, and
  other cross-doc issues.
- [L6 Checks](./linking/checks.md): spec-lint, coverage, and reachability.

## Outputs

- The [linked artifact](./artifacts/linked-artifact.md) is produced after L3 (entities and
  relationships resolved).
- The [reviewed artifact](./artifacts/reviewed-artifact.md) is produced after L6 (validated and
  reviewed).

## Two-stage linking

A real linker resolves symbols by exact name. Jazyk cannot, because docs use synonyms, paraphrase, and
the same name for different things. Linking splits the work so resolution stays conservative and the
deeper validation stays auditable:

- L2 resolves entities using their name, their [scope](./concepts/scopes.md#scopes), and their
  definitions, plus in-document links. It is not a pure name matcher: same-named entities are merged
  only when their scope and definitions agree, and kept apart when they do not. It is conservative and
  never guesses typos.
- L4 and L5 apply deeper semantic judgment over the merged graph. L4 catches wrong merges (a
  `false-merge`) when an entity cannot form one coherent definition once all its facts are combined. L5
  catches the inverse (a `missing-link`) and reviews contradictions.

So L2 resolves by name, scope, and definition; L4 splits any merge that still fails to cohere; and L5
finds the links L2 missed. Conflicts are resolved by editing the documentation, not by project
settings: rename an entity, state its scope, or add an in-document link.

Diagnostics from linking are persisted and sticky. See
[sticky diagnostics](./concepts/sticky-diagnostics.md).

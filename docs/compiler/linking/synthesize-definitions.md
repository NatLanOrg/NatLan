# Synthesize definitions

L4 produces each entity's global definition. The act of synthesis is also the merge check: if the
members of a merged entity cannot form one coherent definition, the merge was wrong.

For each entity that [L2](./resolve-entities.md) merged, L4 attempts one coherent global definition from
all members' local definitions and requirements:

- Coherent: emit the global definition and aliases. The merge is confirmed.
- Incoherent: the merge is wrong, and how it is reported depends on how L2 merged it:
  - Merged by name (`name-and-definition` or `name-only`): a `false-merge` error. The members are
    distinct, split them. The message offers the fix in the docs: rename one if they are different, set
    their [scope](../concepts/scopes.md#scopes) to keep them apart, or add an in-document link to assert
    they are the same (which reclassifies the conflict as a contradiction for
    [L5](./semantic-review.md), not a merge error).
  - Merged by a direct link (the author linked them in the docs): trust the link and synthesize. Any
    incompatibility is left to L5 as a contradiction, not a false merge.

So L4 catches over-merges. L5 catches the inverse (missing links).

The local versus global definition split is described on [entity](../model/entity.md). Local
definitions are written per file during compilation. The global definition is built here, once all
facts are linked.

- Consumes: per entity, all members' local definitions and all requirements referencing it, plus how L2
  merged it (tier and members).
- Produces: per entity, either a global definition and aliases, or a `false-merge` diagnostic and a
  split into distinct entities.
- Deterministic: no. LLM per entity, structured output, sticky (see
  [sticky diagnostics](../concepts/sticky-diagnostics.md)).
- LLM scope: one entity. All its linked facts plus its merge info. Never the project, never raw files.
- Cache key: entity membership plus member (definition and requirement) hashes plus L2 tier plus model
  id plus prompt version. Stored in `target/link/<entity-slug>.synthesis.yaml` (see
  [storage layout](../artifacts.md#storage-layout)).

## Prompt

The exact call the bootstrap makes (`bootstrap/src/link.rs`, `link`). One call per merged entity, at
temperature 0, parsed as JSON. It runs only when an entity has more than one local definition. With a
single definition the bootstrap uses it directly and makes no call.

System prompt (wrapped for readability, sent as one line):
```text
Given several documents' definitions of the same named entity, decide if they describe one coherent thing.
Return ONLY JSON: {"coherent":true|false,"definition":string,"reasoning":string}.
'definition' is one coherent sentence if coherent, else an empty string.
```

User message: the entity name and one line per member, each tagged with the object (file) it came from.
```text
Entity: <canonical name>
Definitions:
- (<object>): <local definition>
- (<object>): <local definition>
```

Expected reply:
```json
{ "coherent": true, "definition": "A person or org that holds an account and places orders.", "reasoning": "..." }
```

If `coherent` is true, `definition` becomes the entity's `globalDefinition` (falling back to the first
member's definition when empty). If false, the bootstrap raises the `false-merge` diagnostic below and
carries `reasoning` into it. The bootstrap sends only the local definitions, not the entity's
requirements or its L2 tier.

The reply is constrained by this schema, sent as
[structured output](../concepts/determinism.md#structured-output) (`response_format`, name `synthesis`):
```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["coherent", "definition", "reasoning"],
  "properties": {
    "coherent": { "type": "boolean" },
    "definition": { "type": "string" },
    "reasoning": { "type": "string" }
  }
}
```

## Diagnostics

- `false-merge` (error): name-only-merged members cannot form one coherent definition.

E.g.:
```yaml
# consumes: a name-only-merged entity's facts + how L2 merged it
ent:Customer:
  resolvedBy: name-only
  members:
    - { customer.md: "a person or organization that holds an account" }
    - { crm.md: "a row in the CRM export table" }
  requirements: [ ... ]
# produces: incoherent + name-only -> false merge, with actionable resolutions
diagnostics:
  - { rule: false-merge, severity: error,
      message: "customer.md 'Customer' (account holder) and crm.md 'Customer' (CRM export row) cannot form one coherent definition. Rename one if they are different, set their scope to keep them apart, or add an in-document link to force them together.",
      subjects: [ent:Customer] }
splits:
  - { split: ent:Customer, into: [ent:Customer@customer.md, ent:Customer@crm.md] }
# a coherent entity instead yields:
# ent:Customer: { globalDefinition: "A person or org that holds an account and places orders.", aliases: [Customer, buyer] }
```

# Merge relationships

L3 resolves the endpoints of relationship edges. The edges produced during compilation (see
[consolidate relationships](../compilation/consolidate-relationships.md)) point at local entity ids
that are meaningful only inside one object. L3 re-keys them to the global ids assigned in
[L2](./resolve-entities.md), then merges edges that connect the same global pair.

This is the edge counterpart of L2. L2 resolves entity nodes, L3 resolves edge endpoints and merges
duplicates. Nothing new is extracted.

For each merged pair the requirements are unioned and the type is the strongest across the contributing
edges. See [relationship](../model/relationship.md) for edge types and how edges derive from
requirements.

- Consumes: the per-doc edges (local ids) plus the L2 localId to globalId map.
- Produces: the global relationship graph, edges keyed by global entity pair.
- Deterministic: mostly. Re-key, group by global pair, union requirements, pick the strongest type. An
  LLM runs only to reconcile a genuine type conflict.
- LLM scope (only on conflict): the conflicting edge's candidate types and its requirements. Not prose.
- Cache key: the resolved-entity-id set plus contributing edge hashes (plus model id if reconciliation
  ran).

E.g.:
```yaml
# consumes: per-doc edges (local ids) + L2's localId -> globalId map
abc.md.edges:      [{ members: [e1, e2], type: reference,   requirements: [r0] }]
customer.md.edges: [{ members: [e0, e5], type: association, requirements: [r3] }]
map: { abc.md/e1: ent:Customer, abc.md/e2: ent:Order, customer.md/e0: ent:Customer, customer.md/e5: ent:Order }
# produces: one merged global edge
relationships:
  "rel:Customer~Order":
    members: [ent:Customer, ent:Order]
    type: association
    requirements: [req:abc.md:r0, req:customer.md:r3]
```

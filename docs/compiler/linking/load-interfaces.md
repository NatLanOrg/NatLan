# Load link interfaces

L1 loads the inputs the linker needs from each [object artifact](../artifacts/object-artifact.md), and
nothing more.

To link at scale we do not load full documents. Each object exposes a small link interface:

- its external entities (the ones visible to other docs), with their local definitions and
  [scope](../concepts/scopes.md#scopes), the signals [L2](./resolve-entities.md#resolve-entities) uses
  to resolve them
- the relationships among those external entities
- relocations (in-document cross-file links, e.g. a markdown link to another file)

Internal entities (private to one doc) are never loaded. See the internal and external split on
[entity](../model/entity.md) and [scopes](../concepts/scopes.md).

- Consumes: the set of object artifacts.
- Produces: an in-memory link set of link interfaces.
- Deterministic: yes. No LLM.
- Cache key: the set of contributing object-artifact hashes.

E.g.:
```yaml
# consumes: the set of object artifacts
[ abc.md.object, customer.md.object ]
# produces: link interfaces only
linkInterfaces:
  abc.md:
    externals:
      - { localId: e1, name: Customer, localDefinition: "the buyer who places an order",
          relocation: "customer.md#/customer-definition" }
      - { localId: e2, name: Order, localDefinition: "a purchase a customer places" }
    edges: [{ members: [e1, e2], type: reference, requirements: [r0] }]
  customer.md:
    externals:
      - { localId: e0, name: Customer, role: definition, definedIn: "/customer-definition",
          localDefinition: "a person or organization that holds an account" }
```

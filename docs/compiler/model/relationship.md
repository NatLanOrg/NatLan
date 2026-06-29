# Relationship

A relationship is a typed edge between two (or more) [entities](./entity.md#entity). It is a real node
(reified), so [requirements](./requirement.md#requirement) and queries can attach to it.

This replaces the older idea of relationships between sections. Sections carry only the parent/child
tree. Semantic relationships are between entities.

## Edges are a product of requirements

A relationship exists only because one or more requirements tie its entities together. There are no
orphan edges. Any requirement that references two or more entities produces (or contributes to) an edge
between them, at minimum a weak `reference`. The entities share something, e.g. "all cars are blue"
ties `Car` and `Color` with nothing structural.

Consequences:

- The edge's type is the strongest one implied across all its requirements.
- The edge's requirements are exactly those that mention the pair, so "the requirements between A and
  B" is always well defined, and every edge carries provenance for free.
- Nothing creates edges except requirements. A diagram arrow or a structural sentence ("A is part of
  B") is captured as a requirement, which then yields the edge.

## Types

Relationship types, from strongest to weakest:

Generalization → Realization → Composition → Aggregation → Association → Dependency → Reference

`reference` is the weak default. The compiler promotes an edge to a stronger type as the requirements
warrant.

Meanings:

- Generalization: "is-a" (a Dog is an Animal). Inherits structure and behavior.
- Realization: a promise to fulfill a contract without inheriting implementation (an ArrayList
  realizes a List).
- Composition: part of, and owned by, the whole (a House is composed of Rooms).
- Aggregation: part of, but shared and independent (a Driver is part of a Car, but can exist without
  it).
- Association: a connection that persists, where one holds a reference to the other (a Student and a
  Course).
- Dependency: a connection that is temporary or optional (a CreditCard depends on FraudDetection).
- Reference: a cross reference whose semantics are not yet defined. The default until promoted.

## Cardinality

Some relationships carry cardinality (multiplicity) on each end, in UML notation: `1`, `0..1`, `1..*`,
`0..*`, `n..m`. Cardinality applies to the whole-part and link families (composition, aggregation,
association, dependency). The inheritance family does not carry cardinality.

## Binary and n-ary

Core relationships are binary. A requirement that covers three entities (A, B, C) contributes to the
pairwise edges (A-B, A-C, B-C) and stays attached to all three. True n-ary edges are not supported yet.

# Relationships

Any [section](./section.md#section-build-artifact) in documentation can be related to another section.
Relationships are the edges of the documentation graph.

## Types

Several relationship types are supported and their relative strength from strongest to weakest is:

Generalization → Realization → Composition → Aggregation → Association → Dependency → Reference

### Inheritance family

#### Generalization

Generalization is the classic "is-a" (e.g. A Dog is an Animal), so you inherit the parent's structure and behaviour.
This is typically implemented as a class inheritance.

#### Realization

A promise to fulfill a contract without inheriting any implementation. (e.g. ArrayList realizes/implements
List interface)

### Whole-part family

#### Composition

You are part of another and owned by that another. (e.g. a House is composed of a Room)

#### Aggregation

You are part of another and may be shared by others. (e.g. a Driver is part of a Car, but Driver can exist without it
or be part of multiple Cars)

### Link family

#### Association

Connection that tends to persist between two objects. One object holds a reference to the other. (e.g.
a student is associated with a course)

#### Dependency

Connection that tends to be temporary and/or optional. An object is given a temporary reference via method parameter
(e.g. CreditCard depends on FraudDetection)

#### Reference

A cross-reference whose semantics have not yet been defined. Simply a pointer from one section to another.

Reference is the weakest type and the default when documentation is first discovered. It is used when
a file references another file and a subsection is nested in a section.

The compiler then aims to resolve every reference into one of the stronger types above

## Direction

Every relationship is directional and stored on the source section. (e.g.
`Dog --generalization--> Animal`) Note that [cardinality](#cardinality) is also possible.

## Cardinality

Some relationship types support cardinality (multiplicity) constraints. Cardinality is applied on
both sides of a relationship and follows the UML multiplicity notation:

| Notation      | Meaning                    |
|---------------|----------------------------|
| `1`           | exactly one                |
| `0..1`        | optional, at most one      |
| `1..*`        | one or more                |
| `0..*` or `*` | any number, including none |
| `n..m`        | between `n` and `m`        |

Cardinality is meaningful for the whole-part and link families:
- composition
- aggregation
- association
- dependency

## Cyclical relationships

Cyclical relationships are allowed, although can cause trouble during downstream usage such as code
generation.

# Determinism

Jazyk is a compiler, and ideally the same project compiles to the same artifacts. It is mostly
driven by an LLM, which is not deterministic. There are several factors that attempt to stabilize
the output.

## Docs ambiguity is a compile error

When the documentation is ambiguous, the compiler attempts to surface the ambiguity as a compile
error.

*TODO link to ambiquity detection*

## Process only changed

A rebuild with unchanged inputs returns the previous output if it exists. Each step in
[Compilation](../compilation.md#compilation) and [Linking](../linking.md#linking) states its cache key that
determines whether the step can be skipped.

## Heavy prompting and scoped work

Each LLM task is narrowly scoped with clear expectations via prompting to minimize ambiguity.

## Reasoning persisted

The results of each LLM task have a reasoning field to include additional information why certain decisions
have been made.

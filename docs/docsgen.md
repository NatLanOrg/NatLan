# Documentation Generation

Documentation generation closes the loop. It feeds information discovered downstream back into the
source documentation, so the docs converge toward a complete and unambiguous spec over time.

Where [Code Generation](./codegen.md#code-generation) reads the docs to produce code, this usage
writes proposals back to the docs.

## Technical design

The compiler surfaces gaps. The [semantic review](./compiler/linking/semantic-review.md#semantic-review)
flags entities and requirements that are ambiguous or incomplete, and code generation is forced to
make decisions the docs never stated. Documentation generation harvests both:

- Resolving open-ended diagnostics. For a requirement flagged as ambiguous or incomplete, propose a
  concrete revision (e.g. turning "the page loads quickly" into a measurable statement).
- Persisting forced decisions. When generation had to choose an unspecified value (e.g. a background
  color that was never defined but was implemented as blue), propose adding that decision to the
  relevant section.

Each proposal is keyed to a section so it can be reviewed in context. When a parser provides the
optional [render](./compiler/compilation/docs-parser.md#parser-template) step, an accepted proposal
can be written back to the source file. A proposal is always reviewed by a human, never applied
silently.

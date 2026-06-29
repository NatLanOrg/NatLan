# Section

A section is a unit of a document's structure: a heading and its body, a list item, a code block, or a
diagram. Sections form a tree by parent/child nesting. They carry no semantic edges. All meaning is
captured as [entities](./entity.md#entity), [requirements](./requirement.md#requirement), and
[relationships](./relationship.md#relationship) extracted from the section text.

Sections exist for three reasons:

- Provenance. Every entity and requirement traces back to the section (and character span) it came
  from.
- Reconstruction. The verbatim text plus ordering lets the compiler rebuild the original document.
- Navigation. "Show the documentation context around an entity" resolves to its sections.

## Reference

A section is identified by its file internal reference: the path after `#`, e.g.
`/registration/required-fields/0`. The file is implicit (it is the document being compiled). The full
location is `sourceFile + "#" + reference`, assembled when needed.

Cross file references never appear inside a section. Those are relocations resolved during
[linking](../linking.md#linking).

## Fields

- `title`: the heading text.
- `raw`: the verbatim source text of the section.
- `sourceFile`: the URI of the file the section was parsed from.
- `format`: the format the section was parsed from (e.g. `markdown`).
- `order`: the section's ordinal among its siblings.
- `kind`: `heading`, `list-item`, `code-block`, `blockquote`, `diagram`, or `root`.
- `parent`: the internal reference of the parent section.

## Reconstruction

`raw`, `order`, and `parent` let the compiler reassemble a whole file from its sections. See
[Reproducibility](../artifacts/reproducibility.md#reproducibility).

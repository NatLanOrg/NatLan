# Reproducibility

Artifacts carry enough to reconstruct the original documentation from them. Reconstruction is best
effort verbatim. Formats that map cleanly to text (e.g. Markdown) reconstruct byte faithfully. Lossy
formats (e.g. PDF, DOCX) reconstruct approximately. The guarantee is that the content is stored and
the original format is recorded, so an approximate re-render is acceptable.

## Raw vs normalized content

Each [section](../model/section.md#section) stores its text twice:

- `raw`: the verbatim, un-normalized source exactly as it appeared in the file. This is the source of
  truth for reconstruction.
- `content`: the normalized form, with cross-references resolved. This is what extraction and
  downstream usages read. It is not used for reconstruction.

## Whole-file reassembly

A whole file is reconstructed by:

1. Selecting every section whose `sourceFile` matches the target file.
2. Ordering them by their internal reference path, then by `order` (the section's ordinal among its
   siblings).
3. Concatenating each section's `raw`.

For verbatim text formats this is pure string assembly and needs no help from the parser.

## Format and rendering

Every section records the `format` it was parsed from. Reconstruction uses it to choose how to render
the section back:

- Verbatim: concatenating `raw` reproduces the file directly. Always available.
- Rendered: for the normalized path, and for
  [documentation generation](../../docsgen.md#documentation-generation) writing proposals back, the
  parser's optional [`render`](../compilation/docs-parser.md#parser-template) step re-emits the format
  from structured content. Lossy formats may differ from the original here.

## Example

Given `registration.md`:

```markdown
# Registration
## Required fields
1. **Email**: Email from the company domain
```

Its sections reassemble in order:

| internal reference                  | order | raw                                           |
| ----------------------------------- | ----- | --------------------------------------------- |
| `/registration`                     | 0     | `# Registration`                              |
| `/registration/required-fields`     | 0     | `## Required fields`                          |
| `/registration/required-fields/0`   | 0     | `1. **Email**: Email from the company domain` |

Concatenating the `raw` column in this order reproduces the original file.

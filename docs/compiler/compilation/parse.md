# Parse

Parse is step A1 of [compilation](../compilation.md#compilation). It splits one documentation file
into a tree of sections. It is deterministic and uses no LLM.

The splitting is done by a format handler. See [Documentation parser](./docs-parser.md#documentation-parser)
for the parser template and [Markdown parser](./docs-parser/markdown.md#markdown-parser) for the
built-in handler.

## Consumes

One documentation file.

## Produces

A map of section reference → [section](../model/section.md#section). The reference is the
file-internal path only, the fragment after `#` (e.g. `/abc/required-fields/0`). The file is implicit
(it is the file being compiled). The full location URI is assembled later as
`sourceFile + "#" + reference`.

Each section carries `title`, `raw` (verbatim text, for reconstruction), `order` (ordinal among
siblings), `kind` (heading, list-item, code-block, ...), and `parent` (also an internal reference).
The step also computes a content hash per section and a hash for the whole file.

Cross-file references never appear here. Those are relocations resolved during
[linking](../linking.md#linking).

E.g.:
```yaml
sections:                          # map: internal reference (after '#') -> section
  "/abc/overview":
    title: "Overview"
    kind: "heading"
    order: 0
    parent: "/abc"
    raw: "..."
```

## Diagnostics

- `unsupported-format`: the file matched the project glob but no handler supports it.
- `parse-error`: the handler failed to parse the file.
- `empty-file`: the file has no content.

## Cache key

The file content hash. Stored in `target/<doc>/sections.yaml` (see
[storage layout](../artifacts.md#storage-layout)).

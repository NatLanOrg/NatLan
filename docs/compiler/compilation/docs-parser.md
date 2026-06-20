# Documentation Parser

A documentation parser is a tool that is able to read a specific format of documentation and convert
it into a common structured format. Parsers isolate format-specific concerns so the rest of the
[compiler](../../compiler.md#compiler) operates on a single, uniform representation.

The parser also abstracts away the capabilities of the underlying format including reference resolution,
functionality (defining relationships between sections), and diagram support.

## Parsed documentation format

Every parser produces the same output: a tree of [sections](../build-artifacts/section.md). The
tree mirrors the document's own nesting (e.g. a heading and its subheadings).

## Parser template

Each parser will read in its specific file and convert it into the common format. A parser implements
two responsibilities:

1. **Support detection**: given a file path, indicate whether this parser can be handled
   (Either by extension and/or by content inspection)
2. **Parse**: given a supported file, produce the tree of sections described above.

Parsers are either built in to the compiler or supplied by the project as
[custom handlers](../project-settings.md#handlers).

## Built-in formats

- [Markdown](./docs-parser/markdown.md#markdown-parser)

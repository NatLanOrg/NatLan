# Project Settings

NatLan Project settings are stored in a file named `natlan.toml` that indicates this folder is
a Natlan project.

## Docs

The `docs` section of the project settings is used to configure documentation.

### Glob

The pattern to match or exclude documentation files can be defined as:

```toml
[docs]
glob = [
    "!docs/LICENSE.md",
    "docs/**/*.md",
]
```

### Handlers

In addition to built-in handlers, you can define your own handlers for
custom documentation files here.

TODO path to docs
TODO matcher for docs (e.g. ignore certain files or include only certain files)
TODO plugin system on how to read certain types of files e.g.: markdown, docx, drawio, UML/XMI

### Linting

TODO:
- Define linting rules in plain english, e.g.:
  - Warn on grammatical errors
  - Error on wrong terminology, flow, formats etc...

## Schema

The JSON Schema for the project settings is [located here](./project-settings.schema.yaml).

# Compiler

## Overview

A natural language compiler produces a programmatically readable format of documentation. It resolves
ambiguity, open-endedness, and contradictions in the usage of natural language.

This compiler is exposed as a Rust module that can be used in a e.g. CLI or Language Server.

## Design

### Project settings

TODO need some kind of file that will tell us e.g. where the docs are (e.g. `./docs`), what to do with it (e.g. test code generation)

[See more](./compiler/project-settings.md#project-settings)

### Build Artifacts

The output of the compiler is a programmatically readable format.

[See more](./compiler/build-artifacts.md#build-artifacts)

### Compilation

[See more](./compiler/compilation.md#compilation)

# Transport

How the editor and the [language server](./lsp.md#language-server) exchange messages.

## stdio

The editor starts `jazyk lsp` as a child process and speaks LSP (JSON-RPC) over the process's stdin
and stdout. This is the default and the only transport an editor needs: the editor manages the
process's lifetime, and the connection closes when the process exits.

```
$ jazyk lsp --stdio
# the editor writes JSON-RPC requests to stdin and reads responses from stdout
```

`--stdio` is the default and may be omitted; it is accepted explicitly so editor configurations can be
unambiguous. Diagnostic and log output goes to stderr (and to the LSP `window/logMessage`), never to
stdout, so it cannot corrupt the protocol stream.

This is the same model used by [rust-analyzer](https://rust-analyzer.github.io/) and most language
servers, and it is what the [VS Code](./editors/vscode.md#vs-code) and
[IntelliJ](./editors/intellij.md#intellij) integrations expect.

## Other transports

A socket transport (the server listens on a port instead of using stdio) is sometimes useful for
debugging or for clients that cannot spawn a child process. It is not part of the first version. If
added, it would be a flag on `jazyk lsp` and would not change any of the [capabilities](./capabilities/diagnostics.md#diagnostics).

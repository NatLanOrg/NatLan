// VS Code extension: launches `jazyk lsp` and forwards LSP traffic. The extension does no
// analysis itself. Mirrors docs/lsp/editors/vscode.md.
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let extContext: vscode.ExtensionContext;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  extContext = context;
  await startClient();

  // The server launch args are built from settings at start time, so restart the server
  // whenever a jazyk.* setting changes (server path or LLM overrides).
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration('jazyk')) {
        await restartClient();
      }
    })
  );

  // A command to restart on demand.
  context.subscriptions.push(
    vscode.commands.registerCommand('jazyk.restartServer', restartClient)
  );
}

async function startClient(): Promise<void> {
  const config = vscode.workspace.getConfiguration('jazyk');
  const jazykPath = resolveBinary(config.get<string>('server.path'), extContext);

  // Pass LLM overrides through to the server when configured.
  const args = ['lsp', '--stdio'];
  const baseUrl = config.get<string>('llm.baseUrl');
  const model = config.get<string>('llm.model');
  if (baseUrl) {
    args.push('--llm-base-url', baseUrl);
  }
  if (model) {
    args.push('--model', model);
  }

  const serverOptions: ServerOptions = {
    run: { command: jazykPath, args, transport: TransportKind.stdio },
    debug: { command: jazykPath, args, transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'jazyk' },
      { scheme: 'file', language: 'markdown' },
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.md'),
    },
  };

  client = new LanguageClient('jazyk', 'Jazyk', serverOptions, clientOptions);
  await client.start();
}

async function restartClient(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
  await startClient();
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

// An explicit settings path, otherwise rely on PATH (and optionally a bundled binary).
function resolveBinary(
  configured: string | undefined,
  context: vscode.ExtensionContext
): string {
  if (configured && configured.trim().length > 0) {
    return configured;
  }
  const bundled = vscode.Uri.joinPath(context.extensionUri, 'bin', 'jazyk');
  try {
    // If a bundled binary exists, prefer it; otherwise fall through to PATH.
    require('fs').accessSync(bundled.fsPath);
    return bundled.fsPath;
  } catch {
    return 'jazyk';
  }
}

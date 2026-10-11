const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { pathToFileURL } = require("node:url");

const [sessionManagerPath, sessionSocket, filePath, diagnosticLogPath] =
  process.argv.slice(1);
let markerDirectory;
let waitMarkerFilePath;
let stage = "loadSessionManager";

function recordFailure(error) {
  if (!diagnosticLogPath) return;
  try {
    fs.appendFileSync(
      diagnosticLogPath,
      JSON.stringify({
        source: "ghostex-code-prompt-editor",
        event: "cli.code_prompt_editor_failure",
        timestamp: new Date().toISOString(),
        stage,
        errorName: String(error.name ?? "Error").slice(0, 128),
        errorCode: String(error.code ?? "").slice(0, 128),
        syscall: String(error.syscall ?? "").slice(0, 128),
        errorMessage: String(error.message ?? error).slice(0, 4096),
      }) + "\n",
    );
  } catch {
    // Diagnostics must not replace the original editor failure.
  }
}

function cleanup() {
  try {
    if (waitMarkerFilePath) fs.rmSync(waitMarkerFilePath, { force: true });
    if (markerDirectory) fs.rmdirSync(markerDirectory);
  } catch (error) {
    if (error.code !== "ENOENT") {
      console.error(
        `Could not remove prompt editor temporary files: ${error.message}`,
      );
    }
  }
}

for (const [signal, exitCode] of [
  ["SIGINT", 130],
  ["SIGTERM", 143],
]) {
  process.once(signal, () => {
    cleanup();
    process.exit(exitCode);
  });
}

/**
 * CDXC:PromptEditor 2026-09-23 WHY:
 * code-server's outer CLI rejects --wait. Use the workbench CLI's wait-marker contract through the existing exact-workspace open queue, so opening a Code view can finish before delivery and another project's window cannot consume the prompt.
 * VS Code removes the marker only after the editor closes, preserving Save and Don't Save behavior before the agent reads its prompt file again.
 * The explicit promptEditor request rejects older Code components instead of silently accepting their default auto-save behavior.
 * SEE-ALSO: .dependencies/code-server/src/node/vscodeSocket.ts; .dependencies/code-server/lib/vscode/src/vs/server/node/server.cli.ts.
 */
async function edit() {
  const { EditorSessionManagerClient } = require(sessionManagerPath);
  stage = "createWaitMarker";
  markerDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "ghostex-prompt-"));
  waitMarkerFilePath = path.join(markerDirectory, "wait");
  fs.writeFileSync(waitMarkerFilePath, "", { flag: "wx", mode: 0o600 });
  stage = "queueOpen";
  const status = await new EditorSessionManagerClient(sessionSocket).queueOpen({
    filePath,
    requestKey: path.basename(markerDirectory),
    workspaceFolder: process.cwd(),
    pipeArgs: {
      type: "promptEditor",
      folderURIs: [],
      fileURIs: [pathToFileURL(filePath).toString()],
      forceReuseWindow: true,
      waitMarkerFilePath,
    },
  });
  if (status !== "opened")
    throw new Error("The prompt editor request was replaced.");
  stage = "waitForEditorClose";
  for (;;) {
    try {
      fs.statSync(waitMarkerFilePath);
    } catch (error) {
      if (error.code === "ENOENT") break;
      throw error;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

edit()
  .catch((error) => {
    recordFailure(error);
    console.error(`Could not edit the prompt in Code: ${error.message}`);
    process.exitCode = 1;
  })
  .finally(cleanup);

const { spawn } = require("node:child_process");
const bindAddress = process.argv[process.argv.indexOf("--bind-addr") + 1];

// The SSH stdin pipe owns this launch, including abrupt client disconnects.
const child = spawn(process.execPath, process.argv.slice(1), {
  stdio: ["ignore", "pipe", "ignore"],
});
let stopping = false;
const stop = () => {
  if (stopping) return;
  stopping = true;
  child.kill("SIGTERM");
};
process.stdin.on("end", stop);
process.stdin.on("error", stop);
process.stdin.resume();
process.on("SIGHUP", stop);
process.on("SIGINT", stop);
process.on("SIGTERM", stop);

// Only this child's successful bind can acknowledge startup. Keep draining its
// logs after the receipt so closing the receipt pipe cannot terminate Code.
let output = "";
let acknowledged = false;
child.stdout.on("data", (chunk) => {
  if (acknowledged || stopping) return;
  output = (output + chunk.toString("utf8")).slice(-8192);
  if (output.includes(`HTTP server listening on http://${bindAddress}/`)) {
    acknowledged = true;
    process.stdout.end("GHOSTEX_CODE_READY\n");
  }
});
child.on("error", () => process.exit(1));
// The inherited runtime lock remains held until the owned process and its
// output pipes have closed, including code-server's inner server teardown.
child.on("close", (code) => process.exit(code ?? 1));

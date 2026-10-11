/**
 * The gxserver this build talks to, and the one HTTP call the React Native side makes itself (the
 * session list). Everything about a chat goes through the Rust chat host inside the GPUI library.
 *
 * `dev-endpoint.json` is written by `scripts/dev-endpoint.sh` (gitignored: it holds the daemon's
 * token) and bundled into the APK. On an emulator or a USB phone the same script runs
 * `adb reverse`, so `http://127.0.0.1:<port>` on the phone is the computer's gxserver.
 */
export type Endpoint = {
  baseUrl: string;
  authToken: string;
  /** Optional chat to open first (`projectId:sessionId`). */
  session?: string;
};

export type SessionRow = {
  projectId: string;
  sessionId: string;
  title: string;
  project: string;
  agent: string;
  lastActiveAt: string;
  kind: string;
};

// eslint-disable-next-line @typescript-eslint/no-require-imports
const endpoint = require('../../dev-endpoint.json') as Endpoint;

export function devEndpoint(): Endpoint {
  return endpoint;
}

const PROTOCOL_VERSION = 1;

async function rpc<T>(path: string, params: Record<string, unknown>): Promise<T> {
  const response = await fetch(`${endpoint.baseUrl}${path}`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${endpoint.authToken}`,
      'x-gxserver-protocol-version': String(PROTOCOL_VERSION),
    },
    body: JSON.stringify({ params, protocolVersion: PROTOCOL_VERSION }),
  });
  const body = (await response.json()) as { ok?: boolean; result?: T; error?: { message?: string } };
  if (!response.ok || body.ok === false || body.result === undefined) {
    throw new Error(body.error?.message ?? `gxserver answered ${response.status}`);
  }
  return body.result;
}

type RawSession = {
  projectId?: string;
  sessionId?: string;
  title?: string;
  cwd?: string;
  kind?: string;
  lastActiveAt?: string;
  agentId?: string;
  launchSettings?: { agentCommand?: string };
};

/** Live agent sessions, most recently active first. */
export async function listAgentSessions(): Promise<SessionRow[]> {
  const result = await rpc<{ sessions: RawSession[] }>('/api/listSessions', { includeStopped: false });
  return result.sessions
    .filter((session) => session.kind === 'agent' && session.projectId && session.sessionId)
    .map((session) => ({
      projectId: session.projectId!,
      sessionId: session.sessionId!,
      title: session.title?.trim() || 'Untitled session',
      project: (session.cwd ?? '').split('/').filter(Boolean).pop() ?? session.projectId!,
      agent: session.launchSettings?.agentCommand ?? session.agentId ?? 'agent',
      lastActiveAt: session.lastActiveAt ?? '',
      kind: session.kind ?? 'agent',
    }))
    .sort((a, b) => b.lastActiveAt.localeCompare(a.lastActiveAt));
}

//! The embedded GhosttyKit boundary: the native key target registry, the FFI function table, config
//! loading and surface config requests, the shared Ghostty app and its runtime callbacks, surface close
//! tokens, and the running and startup surface owners.

/*
CDXC:Terminal 2026-06-22-22:45:
Phase 2 crosses the real GhosttyKit/libghostty boundary for visible running Agents mount slots. The runtime owners may initialize Ghostty, create a finalized default config, share one Ghostty app, create/drop/update real surfaces from App-owned host NSViews, and mirror shell-derived terminal focus idempotently; they must not add command/cwd/env/session lifecycle, persistent IDs, terminal content, stdout/stderr, logs, fake handles, fallback success paths, overlays, hidden hit regions, broad hit-test routing, or synthetic input routing.

CDXC:SessionIdentity 2026-06-22-23:24:
Ghostty surface owners carry the private runtime session id separately from the pane/body mount slot. Mount slots remain layout attachments keyed by pane plus shell session, while runtime ids are process-local session identity and must not be persisted, logged, or shown as terminal titles.

CDXC:Terminal 2026-06-22-23:58:
Phase 3 startup may carry cwd, command, env vars, initial input, and wait-after-command only as runtime launch data on a prepared Ghostty surface request. Reject interior-NUL strings before FFI and keep CString/env-var storage scoped to ghostty_surface_new so private launch values never enter Debug output, logs, shell state, titles, or returned configs with dangling pointers.

CDXC:Terminal 2026-06-23-03:33:
Mounting startup surfaces need a startup-owned Ghostty boundary keyed by `AgentsTerminalStartupBodySlotId` plus process-local runtime id. This owner may create, resize, and free a hidden Ghostty surface from an already-prepared config request, but it must not require a Running mount slot, show or focus AppKit hosts, set Ghostty app/surface focus, apply Ready/Failed, persist, log, or expose launch/private terminal payloads.

CDXC:Terminal 2026-06-23-04:13:
Startup readiness may inspect Ghostty surface metadata only as redacted runtime facts: process-exited, foreground-process-id-present, and tty-name-present. Raw tty names and process ids must be freed or discarded at the FFI boundary and must not enter Debug output, shell state, logs, titles, launch payloads, or persistence.

CDXC:Terminal 2026-06-23-04:25:
Ready Mounting startup surfaces must be re-owned by the Running surface path instead of being dropped and recreated. The conversion consumes the startup owner without freeing the Ghostty surface, changes only the map key identity from startup body slot to Running body mount slot, and keeps raw process, tty, launch, and terminal content data out of logs, shell state, and Debug output.

CDXC:Terminal 2026-06-23-04:49:
Running Ghostty close parity must ask the embedded surface to close and wait for the runtime close callback before shell-tab removal. Each surface owner passes a process-memory close token as surface userdata, keeps the AppKit NSView available only through `platform.macos.nsview`, and records only confirmation-needed or confirmed-close state without logging, persistence, runtime ids, raw paths, command text, environment, stdout/stderr, tty names, process ids, or terminal content.

CDXC:Terminal 2026-06-23-05:03:
Running Ghostty surface ownership is generic over a typed body mount slot so command-pane terminals can use the same App-owned NSView and GhosttyKit surface pipeline without entering Agents workspace/startup maps. Command owners use command group/session ids, explicit launch-payload sources only, no title/status/path parsing, no logs, no persistence, and no input routing changes.

CDXC:Terminal 2026-06-27-01:25:
Plain command terminals may carry only the active project cwd supplied by the app's exact-slot command launch source, while Action terminals may carry their separate command payload. The Ghostty surface owner must remain ignorant of titles, shell state, terminal content, fallback cwd inference, and persisted project paths.

CDXC:Terminal 2026-06-23-05:30:
Mounted Running Agents and command terminals need a runtime-only process-exited query that returns only a redacted boolean. Callers that only need exit state must not use the richer metadata snapshot because that would unnecessarily cross the tty/pid FFI boundary and increase the chance of exposing raw terminal/process details.

CDXC:CommandPane 2026-06-23-05:39:
Close-confirm parity needs the Ghostty callback token to hand confirmation-needed events to GPUI exactly once so App-owned runtime state can hold the pending prompt identity. The token remains process memory only and must not expose terminal content, command text, paths, runtime ids, durable ids, logs, shell-state fields, or launch payload data.

CDXC:CommandPane 2026-06-23-05:47:
Canceling a pending close-confirm prompt must reset only the owner-local close-request latch after exact GPUI surface matching. That lets a later user close action ask Ghostty again without inventing a fallback close path or persisting prompt state.

CDXC:Terminal 2026-06-23-05:53:
Real terminal input parity begins with narrow owner wrappers over the existing embedded Ghostty input exports. These wrappers accept already-sanitized primitive values or borrowed byte slices, do not translate GPUI keyboard/mouse events yet, and must not store, log, persist, or expose terminal input text through Debug or shell-state JSON.

CDXC:Terminal 2026-06-23-05:58:
Zero-length text and preedit are distinct FFI edge cases. Text uses a stable non-null empty pointer because Ghostty slices the pointer unconditionally, while preedit clear follows Ghostty's AppKit path and passes a null pointer with length zero.

CDXC:CommandPane 2026-06-23-20:04:
Slice 237 binds GhosttyKit's real `ghostty_surface_needs_confirm_quit` query so close-confirm prompts can be backed by source-side ABI evidence. Surface owners may expose only a boolean for the current mounted surface; they must not log, persist, or reveal process ids, tty names, commands, paths, runtime ids, or terminal content.

CDXC:Terminal 2026-06-24-20:58:
Mounted GPUI terminal host NSViews own native AppKit key events because GPUI's root `KeyDownEvent` drops the macOS native keycode Ghostty needs for Return, Backspace, arrows, modifiers, and bindings. Register only the exact host-view to Ghostty-surface pairing while a real surface is mounted, keep the registry runtime-only, and never store typed text beyond the synchronous FFI call.

CDXC:Clipboard 2026-06-27-03:34:
Terminal file drops are transient text insertion to the exact mounted AppKit host view registered for native key forwarding. Dispatch the borrowed bytes only through that matched Ghostty surface, reject null, unregistered, or empty input, and do not add focused-surface fallback routing, logging, persistence, overlays, or hit-test routing.

CDXC:Terminal 2026-06-27-03:46:
AppKit IME committed text, marked preedit text, and candidate-window geometry must route only through the exact mounted terminal host view registered for Ghostty native input, including command-pane terminals. Borrow callback bytes only for the synchronous Ghostty call, reject empty committed text, allow empty preedit to clear via the null/zero Ghostty convention, and do not store raw IME text or fall back to focused surfaces.
*/

pub(crate) mod app_runtime;
pub(crate) mod close_token;
pub(crate) mod function_table;
pub(crate) mod native_key_targets;
pub(crate) mod runtime_config;
pub(crate) mod surface_config;
pub(crate) mod surface_owner;
pub(crate) mod surface_types;

pub(crate) use app_runtime::*;
pub(crate) use close_token::*;
pub(crate) use function_table::*;
pub(crate) use native_key_targets::*;
pub(crate) use runtime_config::*;
pub(crate) use surface_config::*;
pub(crate) use surface_owner::*;
pub(crate) use surface_types::*;

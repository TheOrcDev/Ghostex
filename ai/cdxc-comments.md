# CDXC comments: why the code exists, and what the user decided

Referenced from `AGENTS.md` ("CDXC comments"). The condensed rules in `AGENTS.md` are binding; this file keeps the full text and the examples, moved here verbatim on 2026-09-18. The canonical area list is `ai/AREAS.md`.

`CDXC:<Area> <yyyy-MM-dd> <KIND>:` comments are the codebase's memory of non-obvious reasons and of decisions the user made while prompting agents. They are greppable (`rg 'CDXC:RemotePairing'`), and they are the first thing to read before changing behaviour in an area. Read them first, write them sparingly, and keep them true.

They serve three purposes, and a comment that serves none of them should not be written:

1. **Explain WHY a piece of code was added**, so the same bug or regression is not reintroduced later: the symptom, the constraint, or the failed alternative that led to the current shape of the code. Not what the code does; the code already says that.
2. **Log decisions the user made** (product behaviour, UX, technical direction, deliberate exclusions), so a later agent that wants to go against one finds it and raises the conflict with the user instead of silently overriding it.
3. **Link functionality spread over several areas**, so one grep of the area tag finds every emitter, consumer, and contract of a feature across `apps/`, `packages/`, `server/`, and the edited parts of `.dependencies/`.

## Kinds

The kind marker says which purpose a comment serves, so decisions are distinguishable from agent reasoning and each kind is greppable on its own (`rg 'CDXC:.* DECISION:'` lists every user decision in the repo):

- `DECISION` — an instruction the user gave. Quote or closely paraphrase the user's words. Only the user creates decisions; an agent's own design choice is a `WHY`.
- `WHY` — a non-obvious reason, an external constraint, or an approach that was tried, failed, and must not be retried.
- `SEE-ALSO` — the other files, tags, or contracts that must stay in lockstep with this code. Use it only where a feature spans crates, apps, or the zmx/ghostty trees.

Comments written before the kind marker existed (2026-09-03) read as `WHY`. Do not retrofit them.

## When to write one

Write a CDXC comment only if at least one of these holds:

1. A reader of the diff would reasonably ask "why not the obvious way?"
2. The user gave an explicit instruction that this code implements.
3. An earlier approach was tried and removed, and must not come back.
4. This file is one of several that must change together.

Otherwise do not write one. Ordinary code, renames, mechanical refactors, obvious fixes, and anything the diff already explains get no CDXC comment.

Anti-patterns seen in this tree; do not write these:

- Lists of things the code "never touches" or "must not expose" (boundary or privacy disclaimers written for the agent's own reassurance).
- Restating the function signature, the control flow, or the type layout.
- A new dated entry for every iteration on the same day. Collapse them into one comment that states the final decision.
- A tag that names a single change, PR, task, or file instead of a feature.

## Areas: no sprawl

`<Area>` names a user-facing feature or a shared contract, never a single change, file, or task. The canonical list lives in **`ai/AREAS.md`**, one line per area. The list is deliberately small and grows rarely:

- Before writing a tag, look it up in `ai/AREAS.md` and use the existing area that covers the feature, even if your change is a sub-feature of it. Put the specific detail in the comment text, not in the tag.
- Never derive a tag from a file name, a struct name, a surface name (`GPUI…`, `React…`), or a task title.
- Create a new area only when no existing area covers the feature at all, the feature is user-visible or a cross-crate contract, and you expect several comments to share it. Add the line to `ai/AREAS.md` in the same commit and say so in your report. A new area for one comment is wrong; use the closest existing one.
- Do not create variants of an existing area (`SidebarSessions` next to `Sessions`, `CommandPaneActions` next to `CommandPane`). If a sub-feature needs its own grep handle, mention the sub-feature word in the comment text.

## Format and placement

- One line `CDXC:<Area> <yyyy-MM-dd> <KIND>:` followed by the reasoning in plain sentences. Date only, no time of day. No manual line wrapping inside the comment; break at sentence ends.
- Prefer a doc comment on the item that owns the behaviour: `///` or `//!` in Rust, JSDoc `/** */` in TS, `///` in Zig, a block comment in CSS. Add a short inline comment only for the specific line or variable the decision is about. In split module directories, put it in the per-concern file, never in `mod.rs` or `index.ts`.
- Keep them current: when requirements change, replace the comment (new date, new text, one sentence on what it supersedes) instead of stacking entries. Delete a CDXC comment when the code it explained is gone.
- Every explicit product or UX decision the user gives you gets one `DECISION` comment next to the code that implements it. Routine requests ("fix this bug", "rename this") do not.

## Conflicts with a DECISION

When your task would change behaviour covered by a `DECISION` comment, stop and tell the user: quote the comment, state what the task wants instead, and ask which one wins. If the user confirms the change, update the comment in the same commit. Never delete or weaken a `DECISION` comment without that exchange.

Good examples:

```
/// CDXC:ZmxWireGeneration 2026-09-03 WHY:
/// Cycling used to compare binary identity and restarted 57 live sessions after three additive rebuilds in one day.
/// Only a generation bump may cycle daemons.
/// SEE-ALSO: server/src/zmx/wire_cycle.rs, .dependencies/zmx/src/ipc.zig.
```

```
/**
 * CDXC:Settings 2026-05-13 DECISION:
 * User: the Settings modal is 20% wider than the first section-sidebar layout and uses a taller viewport so more settings stay visible without scrolling.
 */
```

Bad example (describes the code, records no decision, not worth a comment):

```
// CDXC:Sessions 2026-09-03 WHY: loop over sessions and render a row for each.
```

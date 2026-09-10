# Codex adapter

Claude is the primary development environment. `CLAUDE.md` is the shared policy;
read it once before project work. This file adds Codex-specific execution guidance,
not a second copy of the project rules. User instructions take precedence.

## Start with the relevant context

- On a fresh task or after a long gap, catch up from the latest relevant Claude
  session: locate by project/worktree and modification time, then extract the last
  user requests and assistant text. Start with one session and a bounded excerpt;
  expand only for missing decisions. Do not print raw JSONL, tool results, images,
  or entire histories. Compare historical claims against current Git state.
- Read `git status --short --branch` and the scoped diff before editing. Reuse
  already-read context during the same task; re-read when the source changes.
- Load `.claude/rules/structure.md` for `crates/**`, `core.md` for
  `crates/platitude-core/**`, and `app-ui.md` for `crates/platitude-app/**`.
  These paths are relative to `.claude/rules/`; Codex must load applicable rules
  explicitly rather than assume Claude's automatic loading ran.
- Search `.claude/rules-refs/` with concrete symbols and `rg -n -C 2`; read matching
  sections, never the entire directory. Follow `CLAUDE.md`'s task-specific document
  requirements. Use `.claude/skills/verify-ui/SKILL.md` for UI verification/launch;
  search its verb reference only for the chosen operation.
- The analysis in `internal-docs/Codex運用.md` is on-demand, not startup context.

## Seat identity before any claim

`xtask` currently reads Claude's session environment, not Codex's task ID.
In each fresh PowerShell invocation that acquires/releases a seat or runs another
identity-dependent xtask command, bridge the identity before running it:

```powershell
if ([string]::IsNullOrWhiteSpace($env:CODEX_THREAD_ID)) {
    throw 'Missing Codex task identity; do not run seat without an owner'
}
$env:CLAUDE_CODE_SESSION_ID = $env:CODEX_THREAD_ID
cargo xtask seat
```

Use the current task ID, never another session's ID. Do not invent `CLAUDE_PID`,
use a short-lived shell PID, or use the shared desktop PID. Do not set `CLAUDECODE`.
Keep `seat` argumentless; report its letter, branch, and absolute path immediately.
Work only in the assigned tree. Apply the same identity bridge for `seat release`.
Follow `CLAUDE.md` for release, rebase, GUI launch, and land authorization.

If assignment fails, do not equate its generic message with a full roster. Check
identity, `cargo xtask seats`, and `git worktree list --porcelain` read-only. If Git
metadata writes are sandbox-blocked, request tool escalation for the same properly
identified command. Do not retry without identity, choose a seat, or remove locks.
An anonymous `claude-seat` lock is not proof it is yours. Establish ownership before
any cleanup; unresolved ownership requires the user, not a guessed takeover.

## Keep evidence small and complete

- Prefer path discovery, then symbol search, then the needed section. Start shell
  results around 1,000-2,000 tokens. Batch independent reads with a combined output
  budget; do not combine many full files/diffs into a truncated result.
- Truncation means evidence is missing. Narrow the query or page the remaining
  section before deciding; never repeat the same oversized read. JSONL records and
  reference entries can be very long even when only a few lines are selected.
- For long-running commands, preserve full output in a task-local log and expose
  the exit status, log path, summary and relevant failure excerpts. Keep failure
  context available; an output limit must not turn a failed check into a pass.
- Wait on the original process/session handle for new output or completion. Avoid
  repeated tail/status/process-list calls when nothing changed. A wait timeout is
  not completion or a correctness assertion. Do independent work only when it
  does not invalidate the running check; keep progress updates concise.
- Reuse the gate's existing selection, logs and cache. Run required checks and the
  gate; do not add `--all`, `--fresh`, whole-repo audits or repeated passing tests
  without a relevant change or unresolved failure. Preserve failures and causal
  UI verification; no retry-to-green or elapsed-time substitutes.
- No subagents unless explicitly requested. Complete related review/fixes/checks
  in this task. Stop at the authorized outcome, with a compact handoff: branch/SHA,
  changed paths, verification result/log, unresolved issue and next allowed action.
  Do not copy full histories into handoffs or restart the same investigation.

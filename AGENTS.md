# Codex adapter

Claude is the primary development environment. `CLAUDE.md` is the shared policy;
read it once before project work. This file adds Codex-specific execution
guidance on top of it.

## Start with the relevant context

- On a fresh task or after a long gap, catch up from the latest relevant Claude
  session: locate by project/worktree and modification time, then extract the last
  user requests and assistant text. Start with one session and a bounded excerpt;
  expand only for missing decisions. Compare historical claims against current Git state.
- Read `git status --short --branch` and the scoped diff before editing. Reuse
  already-read context during the same task; re-read when the source changes.
- Load `.claude/rules/code.md` and `structure.md` for `crates/**`, `core.md` for
  `crates/platitude-core/**`, and `app-ui.md` for `crates/platitude-app/**`.
  These paths are relative to `.claude/rules/`; the automatic loading is Claude's alone.
- Claude's hooks (`.claude/settings.json`) do not run under Codex: nothing denies
  what they would, so keep the rules they enforce yourself.
- Use `.claude/skills/verify-ui/SKILL.md` for UI verification/launch;
  search its verb reference only for the chosen operation.

## Seat identity before any claim

`xtask` reads the hook session id, then `CLAUDE_CODE_SESSION_ID`, then
`CODEX_THREAD_ID`. Keep the bridge below in fresh PowerShell invocations for
older worktree runners:

```powershell
if ([string]::IsNullOrWhiteSpace($env:CODEX_THREAD_ID)) {
    throw 'Missing Codex task identity; do not run seat without an owner'
}
$env:CLAUDE_CODE_SESSION_ID = $env:CODEX_THREAD_ID
cargo xtask seat
```

The identity is the current task ID alone. `CLAUDE_PID` and `CLAUDECODE` stay
unset (a shell or desktop PID would name the wrong owner).
Report the seat's letter, branch, and absolute path immediately.
Work only in the assigned tree. Apply the same identity bridge for <!--call:seat.release-->`seat release`.
Follow `CLAUDE.md` for release, rebase, GUI launch, and land authorization.
Before reporting a seat released, verify the unlock result and the worktree's
lock state. Nothing lifts a claim but a landing, a release, and the takeover the
user asks for (<!--cmd:seat.takeover-->`PGG_ALLOW_TAKEOVER=1 cargo xtask seat takeover <letter>`). End without landing via
<!--call:seat.release-->`seat release`; committed work remains on its branch — and asking for a seat
from that tree takes it back, work and all.

If assignment fails, find the cause. Check
identity, `cargo xtask seats`, and `git worktree list --porcelain` read-only. If Git
metadata writes are sandbox-blocked, request tool escalation for the same properly
identified command. An anonymous `claude-seat` lock is not proof it is yours.
Establish ownership before any cleanup; unresolved ownership goes to the user.

## Keep evidence small and complete

- Prefer path discovery, then symbol search, then the needed section. Start shell
  results around 1,000-2,000 tokens. Batch independent reads with a combined output
  budget sized so every batched file/diff arrives whole.
- Truncation means evidence is missing. Narrow the query or page the remaining
  section before deciding. JSONL records and reference entries can be very long
  even when only a few lines are selected.
- For long-running commands, preserve full output in a task-local log and expose
  the exit status, log path, summary and relevant failure excerpts. A failed check
  stays failed through any output limit.
- Wait on the original process/session handle for new output or completion; the
  next tail/status/process-list call follows a change. A wait timeout is
  not completion or a correctness assertion. Do independent work only when it
  leaves the running check valid.
- Reuse the gate's existing selection, logs and cache; `--fresh`,
  whole-repo audits and repeats of passing tests need a relevant change or an
  unresolved failure.
- Subagents only on explicit request. Stop at the authorized outcome, with a
  compact handoff: branch/SHA, changed paths, verification result/log, unresolved
  issue and next allowed action.

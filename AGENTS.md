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
  what they would, so keep the rules they enforce yourself. Codex's activity hooks
  in `.codex/hooks.json` share Claude's Windows sleep holder; setup and fallback
  commands are in `internal-docs/Codexハーネス.md`.
- Use `.claude/skills/verify-ui/SKILL.md` for UI verification/launch;
  search its verb reference only for the chosen operation.
- Use `.claude/skills/gate/SKILL.md` for the check tiers and when to run gate,
  before running checks and before reporting work complete.

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
unset (a shell or desktop PID would name the wrong owner); a
`CLAUDE_CODE_HOST_SESSION_ID` the shell carries is not read for a task's seat.
Report the seat's letter, branch, and absolute path immediately.
Work only in the assigned tree. Apply the same identity bridge for <!--call:seat.release-->`seat release`.
Follow `CLAUDE.md` for release, rebase, GUI launch, and land authorization.
Before reporting a seat released, verify the unlock result and the worktree's
lock state. Nothing lifts a claim but a landing, a release, and the takeover the
user asks for (<!--cmd:seat.takeover-->`PGG_ALLOW_TAKEOVER=1 cargo xtask seat takeover <letter>`).
Keep the claim across replies, completed requests, user-input waits, and resumed
turns in this chat, just as a Claude conversation does. Release only when the user
asks to hand it back; a landing releases it through the shared workflow. Continue
in the same assigned tree. Rebase only on the user's current instruction.

While working in a seat, use the shared sleep holder. Trusted native activity
hooks own Codex activity; the holder supplements them with Desktop's debug log
to observe actual approval/question requests and answers. Configure logging once
with `cargo xtask` <!--call:awake.codex.setup-->`awake codex setup`, fully quit
Desktop, then launch its normal icon. Review the project's activity hooks in
Desktop Settings > Hooks (CLI: `/hooks`). See
[Codex harness](internal-docs/Codexハーネス.md) for activation and experiments.
If trusted activity hooks are not active, run `cargo xtask`
with <!--call:awake.codex.work-->`awake codex work`
at the start of work and refresh it before its one-hour activity TTL (including
long-running checks). Run `cargo xtask` with <!--call:awake.codex.idle-->`awake codex idle`
before yielding to the user or ending the turn. Idling sleep control does not
release the seat. Do not set `CLAUDE_PID` or `CLAUDECODE` to enable it.
Native hooks do not mark a mere PermissionRequest as a human wait. If the log
supplement is unavailable or cannot match a request, keep their activity verdict
and inspect the diagnostic from `cargo xtask` <!--call:awake.status-->`awake`.
Do not guess a human wait or release the seat to enable sleep.

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

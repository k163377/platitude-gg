//! The usage page's session machinery: the shared sleep hold and the hook
//! handlers.

// No `"\` continuation on the opening line (see `demo_repo`).
pub(super) const ENTRIES: &str = "  awake [log on|off]
      Which Claude and Codex sessions keep this machine from idle-sleeping
      (Windows only). Each session's hooks, and each subagent's apart,
      keep a claim under .awake/ in the primary checkout, in one protocol
      every compatible build's hooks update: working from a prompt, idle
      once the turn stops, and each tool call from its start to its end,
      with whether a person is asked about it. A holder with no window,
      one per revision of its script, reads while any claim stands and
      asks the system, never the display, to stay up while any counts —
      and asks again whenever a reading finds the request no longer
      counted, which can happen unsaid. A working claim counts while one
      of its own calls runs (open, no result yet, no person asked —
      however long it runs), or for an hour after its last hook while none
      is open — never while its open calls all wait on a person, nor once
      the session's transcript ends on the user interrupting the turn. A
      subagent stopped from outside runs no SubagentStop: its claim goes
      once its session's transcript records its stop (a background task's
      notice naming it, or the result of the call it runs under); Claude
      Code's own, run between turns with neither a transcript nor a
      .meta.json of its own, stops as a reading finds it. A notice (a
      background task's end, another session's message) written into a
      session's transcript since its last hook starts a turn: the holder
      marks the claim working from it, until the turn's Stop. A turn
      counts past its Stop until Claude Code records that the Stop hooks
      have run, when it takes what was queued for the next turn (a minute
      at most). An MCP server's pending request for input holds up one of
      the session's running calls on that server, and its answer frees
      that one request alone.
      No hook says when a person lets a call through, nor when a tool the
      desktop app runs asks a person itself: each holder reads those,
      best-effort, off the desktop app's own log
      (%LOCALAPPDATA%\\Claude\\logs\\main.log) — a permission request it
      ties to one open call alone holds that call up until its answer, and
      a grant sets it running again. The log is read after the fact, so
      neither is protected before the reading that sees it; without a line
      it can tie (no log, a format it does not know, calls it cannot tell
      apart) a call nothing shows running (an MCP tool, a fetch) is not
      seen from its grant to its end, and one the app asks about itself
      counts as running while it waits. `log off` makes every holder go by
      the hooks alone from its next reading, `log on` reads the log again.
      Any claim counts while a process of one of its shell calls runs —
      the session's or a subagent's, sent to the background at the start
      or halfway — or until the wake-up it scheduled is due; a process the
      Claude process already had, or started during another tool's call,
      is no work. A holder whose build's hooks have been quiet for ten
      minutes steps down while a holder of a build in use — or, when every
      build is quiet, of a higher revision — stands; the last one standing
      never does. A holder lets go, whatever still counts, once nothing
      has happened for two hours awake — no hook, no claim's transcript
      written, no counted process started or taking more than a tenth of a
      processor — and holds again at the next thing that does (released
      quiet:<minutes>m in the history). The holders keep in .awake/history
      what each asked for, when and on whose account — a line each time a
      verdict or its claims change, and one for each hold found no longer
      counted (lost) — to set a sleep, or the want of one, against.
      Prints the claims as written, each holder's last verdict, when its
      build's hooks last ran, the last line it wrote in the history, and
      what it last made of the desktop app's log. The transcript,
      Desktop-log and child-process rules above are Claude's; Codex uses
      its own lifecycle hooks, below.

      Codex manual fallback (.codex/hooks.json): awake codex work|idle
      CODEX_THREAD_ID names the chat; its claimed seat is read from Git
      even when the session cwd is the primary checkout. `work` refreshes
      working activity for one hour — refresh before it expires, during
      long checks too; `idle` ends its calls and lets sleep resume if
      nothing else works. Neither changes the seat claim. The actual Codex
      ancestor owns activity; never set CLAUDE_PID for this. Native hooks
      mark prompt/tool starts, blocking questions, results, Stop,
      Interrupt and SessionEnd. Unified exec stays open until its
      completion hook; unrelated children of Codex's shared process are
      not attributed by time spans. New or changed hooks require Codex's
      trust review (/hooks).

      Codex Desktop setup: awake codex setup [--undo]
      One-time Desktop logging setup for the shared sleep holder: saves
      the user's previous CODEX_MAX_LOG_LEVEL, enables debug logging and
      broadcasts the environment change; then fully quit Desktop and start
      it from its normal icon (no launch wrapper or CLI override). --undo
      restores the previous level if it is still this setup's debug
      setting, keeps a later user edit, and turns the supplement off. The
      log supplies the actual approval and question waits and their
      answers — a PermissionRequest hook alone is no evidence of a person
      waiting; an unmatched or missing log leaves native activity
      unchanged. `awake` reports the observed debug age, unmatched IDs and
      fallback diagnostics.

  hook <event>
      Claude Code hook handler (wired from .claude/settings.json; reads
      the hook payload from stdin). Events: pre-write, post-write,
      pre-shell, pre-tool (which also guards messages to other sessions,
      the board, the wrap-up's language and chips), post-tool,
      permission-request, permission-denied, elicitation,
      elicitation-result, subagent-start, subagent-stop, stop,
      stop-failure, pre-worktree, post-worktree, prompt-submit (which also
      tells a side session to detach itself, and stop holds it once until
      it has), session-start (which also installs the gate's git hook),
      session-end. `codex-session` (or `codex-awake`) takes Codex's
      lifecycle payload (.codex/hooks.json) apart from Claude's handlers:
      its sleep activity and landing permit, the gate's git hook at
      SessionStart, and a refusal of a shell line that writes main by hand
      or takes its git out from under the gate.
";

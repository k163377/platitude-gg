//! What `cargo xtask` prints when it is given no command it knows.
//!
//! One page, in four pieces. The two that grow are the ones that grow with
//! the app rather than with this runner: the demo presets, one line per
//! shape a repository can be stopped in, and the verify-ui verbs, one
//! paragraph per thing a headless run has to be told about itself. Both are
//! written where they are added to, and neither can push the other over a
//! ceiling.

mod demo_repo;
mod verify_ui;

/// Writes the page as one text: head, the two lists, then everything whose
/// entry is a paragraph the runner itself owns.
pub(crate) fn print() {
    print!("{HEAD}{}{}{TAIL}", demo_repo::PRESETS, verify_ui::VERBS);
}

const HEAD: &str = "\
cargo xtask <command>

commands:
  check [--verb <v>]...
      Stage-2 verification (CLAUDE.md 確認は 3 段), with the host and the
      container running in parallel: structure, fmt, clippy and the
      workspace tests here, while the container runs test -p
      platitude-core, that same clippy for Linux, verify-ui for each
      --verb, and bare. Clippy runs on both sides because the host's
      cannot answer for the other one: a name reachable only under
      #[cfg(not(windows))] is not compiled here at all, so an unused
      import or an orphaned fn behind that cfg would reach CI unseen.
      The two sides write to different build trees (target/ vs the
      docker volume), so the wall clock is whichever side finishes
      last. Each --verb also runs verify-ui on the host, so one flag
      covers the verb on both OSes; its value is a whole verify-ui
      argument line, quoted when the verb needs its preset or argument
      beside it (--verb 'co-authors 4 --preset co-authors'). Without
      --verb the summary says the touched verbs still have to run — it
      never passes for the whole of stage 2 on its own.

  gate [--host-only] [--all] [--fresh] [--dry-run] [--verb <line>]...
      The pre-merge tests, chosen by machine (CLAUDE.md 確認は 3 段;
      internal-docs/反映前テストの機械化.md). Reads the branch's diff
      against main, follows every file that reads a changed file — a
      `use` path, a re-export, a string naming the file, a QML type
      name, a QML mention of a #[qobject] model — and runs the tests in
      that reach: unit tests by module path, the integration binary by
      module, clippy per crate entered, shipped when the app's QML or
      entry point moved, the verify-ui verbs whose census names a
      reached component (crates/xtask/verb-census.txt, written by the
      runs themselves), bare when the app moved. structure, waits and
      fmt run every time, and a build input that changed (Cargo.toml,
      Cargo.lock, the toolchain, the Dockerfile) makes the reach the
      whole tree. Host and container sides run in parallel;
      each step is stamped by the object ids of what it reads, so a
      second run of one commit runs nothing and a rebase reruns only
      what main's move touched. A commit whose every step is green is
      stamped, and the reference-transaction hook lets main move onto
      stamped commits only. A component no verb's census names stops
      the gate by name: run a verb that shows it once, and the gate
      picks that verb from then on.
      options:
        --host-only   the daily tier: no container; stamps the host half
        --all         every file counts as changed — stage 2 in full
        --fresh       ignore the stamps and run everything owed
        --dry-run     print the reach and the steps, run nothing
        --verb <l>    a verify-ui line to run besides the census's
        --dir <tree>  gate that tree instead of this one
      gate verdict <old> <new>   the hook's question (exit 0 = may move)
      gate install               copy .githooks/reference-transaction
                                 beside .git (pg-gate/hooks/), note the
                                 tree whose xtask answers, point
                                 core.hooksPath there; every session
                                 start and every land does this
      gate deps [--file <p>]... [--why <p>]... [--history <n>] [--show]
                                 the graph itself: what a change reaches,
                                 how a file got there, what the last n
                                 commits would have owed

  structure
      The per-file length backstop of .claude/rules/structure.md (1000
      code lines — blank and comment-only lines do not count) over
      crates/**/*.rs and *.qml, and what the tree spends on comments —
      first step of `check`, and a second or two on its own. Three
      standings: a file the ledger (.claude/rules-refs/structure.md
      分割しない判断) gives a written reason not to split has no backstop
      at all, and the run asks only that the entry still names a file
      that is there — the entry goes when the file is split away; a file
      already over when this went in is pinned by
      crates/xtask/structure-baseline.txt at the length it had, free to
      shrink (the pin follows it down) and not to grow; everything else
      meets the backstop as written, so a file that crosses it for the
      first time fails the run that sees it. The other half of that § —
      fn 100 lines — is clippy's too_many_lines, raised to warn in the
      workspace lints, and an existing long function carries
      #[expect(clippy::too_many_lines)] until it is cut up.

  waits
      Every await in tests/it that resolves through a channel no Patience
      watches — a tracked outcome(), a session boundary wait, a ticker
      step — must sit under support::wait::bounded or an explicit
      timeout, so a silent hang fails by test name instead of sitting
      until the CI kill. Second step of `check`; subsecond on its own.

";

// No `"\` continuation on the opening line (see `demo_repo`).
const TAIL: &str = "  shipped [--no-build]
      Starts the build nobody else here makes: `cargo build --release`
      with no features, which is the one without the verification
      harness. Offscreen, bounded, reaped — a shipped build has no
      watchdog of its own, because that is a harness knob.
      What it is for is the failure only this build has: a QML file in
      `platitude.ui` that reaches into `platitude.auto` resolves in every
      build this runner drives and loads nothing in the shipped one.
      `structure` catches the type names statically; this catches the
      rest, an import of the absent module included.

  perf --repo <path> [--label <name>] [--runs <n>] [--breakdown]
      The measurement behind ci/baseline/perf-windows-x64.md, run the
      same way every time: release build, a real window (offscreen
      reports neither memory nor fps honestly), the PG_AUTO_* hooks,
      WorkingSet and private bytes sampled every 100ms for their
      maximum, and a deadline with a kill guard. The first run is
      discarded — the record is a warm-cache number.
      --breakdown builds with the `memprobe` feature and adds
      PG_MEM_REPORT=1, then prints the largest `mem report` line the run
      produced: live Rust heap, the models and the session parts holding
      it, and what none of them account for. Process memory minus Rust
      live bytes is not a measurement of Qt's live heap.
      options:
        --runs <n>        kept runs after the discarded first (default 3)
        --watchdog-ms <n> outer hang ceiling (default 300000)
        --settle-ms <n>   hold the app idle this long after it reports,
                          then read the memory once more and print it
                          beside the peak. The peak says what the work
                          cost while it ran; the difference says how much
                          of that the process handed back once it had
                          nothing to do (default 0 = read at once)
        --no-scroll       leave the scroll benchmark out
        --no-select       do not select a row or open a diff
        --selection <s>   none, first (default), or head
        --select-oid <id>  select this full OID from the loaded graph
        --file <path>     open this changed file (default: first)
        --no-diff         select and show details without opening a diff
        --output <path>   new directory for metadata and raw per-run logs
        --trace-frames    diagnostic app-clock frame series in app.log;
                          flushed after scrolling, not a budget run
        --no-open         start with no repository at all — the window and
                          nothing in it. Subtracting this from a run that
                          opened an empty repository leaves the cost of
                          putting the page up, which is otherwise
                          indistinguishable from the toolkit's own floor.
        --no-build        use the release binary already built

  linux [--rebuild] [--shell] [--stage core|app] <command…>
      Run a command against this checkout on Ubuntu, in a container built
      from ci/linux/Dockerfile. On Linux it skips the container and runs
      the command where it stands.
        cargo xtask linux test -p platitude-core --test it
        cargo xtask linux verify-ui commit --preset basic
        cargo xtask linux bare
        cargo xtask linux offline
      A cargo command goes to cargo; an xtask verb goes to cargo xtask.
      Two are neither, and both stay in the container on Linux too.
      `bare` builds the release workspace-wide and starts it on an Ubuntu
      carrying only what a package would declare, which is the only check
      that the thing runs somewhere it was not built. `offline` is CI's
      offline-test job run here: it builds the app with the harness and
      the test binaries, then runs ci/offline-test.sh — the suite plus the
      offscreen smoke — in a container with no network at all, which is
      the property CI reaches with `unshare -n`. That script has no other
      caller until CI first runs, so this is what keeps it from drifting.
      Both are worth a place in a pre-merge sweep, not in a daily one.
      Which image it runs in follows what the command needs: core is
      Ubuntu and the toolchain, app adds Qt, a software GL stack and the
      fonts デザイン規約 names for Ubuntu. The build directory is a docker
      volume, so this target/ is untouched, and a verify-ui run is handed
      a host directory to leave its screenshot in.
      options:
        --rebuild        build the image again even if one already matches
        --shell          open a shell in the container instead
        --stage <name>   core or app, when the guess is not the one wanted

  land [<branch>]
      Put a branch on main — the one sanctioned way (CLAUDE.md Git 運用;
      the pre-shell hook asks for PG_ALLOW_MAIN=1 in front, which is how
      the transcript records that the user asked). In the branch's own
      worktree: rebases it onto main when it is behind (a rebase that
      stops is walked back), runs `gate` there — cached steps are not
      paid twice — and only then fast-forwards main, in the primary
      checkout when it sits on main or on the bare ref (reattaching a
      detached primary) when main is checked out nowhere; the
      reference-transaction hook checks the stamp on the way. History
      stays linear and a landed seat stands at main's tip. Refuses a
      branch checked out nowhere, a dirty seat, and a seat that holds
      main. Bare `land` from a seat lands the seat's own branch. A
      landed branch's seat is handed back: its claude-seat claim is
      released once the commits are on main, and the next edit there
      claims it back (a lock written by hand stays).

  kill
      Reap this tree's app processes — the ones holding this tree's exe
      against the next link, or its store lock against the next window —
      and nobody else's. The pre-shell hook points image-name kills
      (taskkill /IM, Stop-Process -Name), which reach every seat and the
      user's own window, at this instead.

  launch [--no-build]
      Real-window start for 「起動して」 asks (the verify-ui skill's fast
      path): reap this tree's stale runs, build release, start detached,
      and confirm it outlived its first second. Run it with
      PG_ALLOW_GUI=1 in front — a real window is the user's ask. The app
      keeps a per-tree settings store on its own, so seats never fight
      over one instance lock.

  seats
      Where the six worktree seats a-f stand right now, one line each:
      branch, whether HEAD sits at main's tip, commits ahead of main
      (main..HEAD), uncommitted changes (status --porcelain lines), and
      how long since the seat's own index was written. The session
      greeting reads the same survey, but only once, when the session
      starts — a seat that looked free then can hold another session's
      work minutes later, so this is the line to read before entering
      one (CLAUDE.md ビルド・テスト). Ends with how to read the columns,
      and a locked seat carries the mark past them.

  shots <add|prune|open [--again]|list|path>
      The shot board: pictures land on a page that opens in a window of
      its own, magnifies to exact integer ratios without smoothing, and
      is still there after it is closed. A chat pane fits a picture to
      its width and cannot zoom it, so a 1440x900 window is unreadable
      there and shooting it larger changes nothing. `verify-ui` puts its
      own pair on the board as it goes, so the usual reason to reach for
      `add` is a crop or a comparison sheet made by hand. All six seats
      write to one board beside the primary checkout's .git (.shots/,
      gitignored) and every run carries the seat that took it — a
      picture from another tree proves nothing about this one, which is
      also why `prune` reaches this seat's runs and no others unless it
      is told to.

      One window, and it is `open` that hands it out: the page sits at a
      fixed path and is rewritten in place, so the window already showing
      the board is one F5 away from the run just taken. `open` says so
      rather than opening a second, whichever seat or session asks, and
      `--again` is the reader's own word that they closed theirs. Six
      windows of one board are six answers to which one is current, so
      the count is kept in the board rather than left to whoever
      remembers (shots/window.rs).

      A before/after goes on with `add --before <png> --after <png>`,
      which puts the two on one view side by side under one magnifier.
      Never as two runs: read one after the other, the difference is
      whatever the reader remembered rather than whatever changed.

      The board keeps itself to what is being looked at now, and the
      three rules that do it are in shots/sweep.rs: a picture retaken
      replaces the one before it, a seat's runs go when its branch
      lands, and a session's runs go when it ends. `prune` is what
      reaches the rest — an approach abandoned under a name nobody
      retakes (--label), or a seat whose session is long gone.

  hook <event>
      Claude Code hook handler (wired from .claude/settings.json; reads
      the hook payload from stdin). Events: pre-write, post-write,
      pre-shell, pre-chip, post-chip, stop, pre-worktree,
      post-worktree, session-start (which also installs the gate's git
      hook), session-end.
";

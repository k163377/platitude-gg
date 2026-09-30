//! What `cargo xtask` prints when it is given no command it knows.
//!
//! The two sections that grow with the app — demo presets and verify-ui
//! verbs — have files of their own, so neither pushes the other over the
//! length ceiling.

mod demo_repo;
mod verify_ui;

pub(crate) fn print() {
    print!("{HEAD}{}{}{TAIL}", demo_repo::PRESETS, verify_ui::VERBS);
}

const HEAD: &str = "\
cargo xtask <command>

commands:
  check [--verb <v>]...
      Stage-2 verification (CLAUDE.md 確認は 3 段), with the host and the
      container running in parallel: structure, waits, docs, qmltest,
      fmt, clippy, the workspace tests and the shipped build here, while
      the container runs test -p platitude-core, qmltest, that same
      clippy for Linux, verify-ui for each --verb, and bare. Clippy runs
      on both sides because the host's cannot answer for the other one: a
      name reachable only under
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

  gate [--host-only] [--all] [--fresh] [--keep-going] [--dry-run]
       [--verb <line>]... [--jobs <n>] [--dir <tree>] [--main <ref>]
      The pre-merge tests, chosen by machine (CLAUDE.md 確認は 3 段;
      internal-docs/反映前テストの機械化.md). Reads the branch's diff
      against main, follows every file that reads a changed file — a
      `use` path, a re-export, a string naming the file, a QML type
      name, a QML mention of a #[qobject] model — and runs the tests in
      that reach: unit tests by module path, the integration binary by
      module, clippy per crate entered, cargo-deny when deny.toml or a
      manifest moved, qmltest when the product's QML module, the QtTest
      files that read it, or the runner that stages it moved, shipped when
      the app's QML or entry point
      moved, the verify-ui verbs whose census names a reached component
      (crates/xtask/verb-census.txt, written by the
      runs themselves), bare when the app moved. The always-steps
      (structure, waits, docs, verbs, fmt) run every time and first, and
      a build input that changed (Cargo.toml, Cargo.lock, the toolchain,
      the Dockerfile) makes the reach the whole tree. Then host and
      container sides run in parallel;
      each step is stamped by the object ids of what it reads, so a
      second run of one commit runs nothing and a rebase reruns only
      what main's move touched. The verify-ui verbs of a side share
      nothing but the release the first of them builds, so they run
      several at a time; every other step of a side runs one at a time.
      The first red stops the run on both sides: nothing more starts,
      what is running is ended (a container step of its own is left to
      finish), and each step kept from running or ended is filed as
      halted, owed again by the next run. A commit whose every step is green is
      stamped, and the reference-transaction hook lets a session's git
      move main onto stamped commits only — a git the user runs carries
      no CLAUDECODE mark, and the hook does not answer for it. A
      component no verb's census names stops the gate by name: run a
      verb that shows it once, and the gate picks that verb from then
      on. The verbs rewrite their own census
      lines as they run, so a gate that finds the file rewritten stops
      before stamping — commit the generated file and run it again.
      options:
        --host-only   the daily tier: no container; stamps the host half
        --all         every file counts as changed — stage 2 in full, and the
                      `periodic` tests the daily tiers leave out
        --fresh       ignore the stamps and run everything owed
        --keep-going  run the rest after a red (--all does): a red verb
                      stops none of the others, the first red of a checks
                      group stops that group, and what passed is stamped.
                      A red always-step stops the run all the same
        --dry-run     print the reach and the steps, run nothing
        --verb <l>    a verify-ui line to run besides the census's
        --jobs <n>    this gate's verbs at a time per side (default: a
                      third of the logical CPUs, 1 to 8). The lanes are
                      the machine's: every gate on it shares that many
                      per side, so two gates at once run that many verbs
                      between them. A tree holds one gate
                      at a time — a second one there is refused with the
                      first's pid (a dry run holds nothing)
        --dir <tree>  gate that tree
        --main <ref>  read the diff and the stamp against this ref
                      (the tests' sandboxes; default main)
      gate verdict <old> <new>   the hook's question (exit 0 = may move)
      gate install               copy .githooks/reference-transaction
                                 beside .git (pgg-gate/hooks/), note the
                                 tree whose xtask answers, point
                                 core.hooksPath there; every session
                                 start and every land does this
      gate deps [--file <p>]... [--why <p>]... [--history <n>] [--show]
                                 the graph itself: what a change reaches,
                                 how a file got there, what the last n
                                 commits would have owed

  sweep [--dry-run | --if-the-generation-moved]
      Takes away every build product in target/ but the generation the
      canonical cargo lines stand on (internal-docs/反映前テストの
      機械化.md §世代の掃除). cargo never removes what it has replaced,
      so a Cargo.lock, a toolchain or a profile setting that moves leaves
      the whole of the last generation behind, named by nothing. What is
      live is asked of cargo, never of the clock: a written-down list of
      lines — the gate's, the ones a session types between gates, and the
      two builds the runner makes for itself — is run with
      --message-format=json, and what their artifacts and build-script
      directories name is what stays. A unit this tree has not built is
      compiled to be read; the count is printed, and it is once per
      generation. Everything written since the last sweep here stays
      whatever the lines said. Under debug/, release/ and shipped/ only,
      and within them deps/, build/ and incremental/: .fingerprint/ is
      kilobytes and the hook's own target/hooks is in use at
      unpredictable moments, so neither is touched, and nothing under
      target/ that is not cargo's is either. A profile cargo is building
      in is left whole, and a run that left one alone writes no stamp, so
      the next tail does it again. Every removal is printed with its
      size, and so is anything that would not go. The tail of a gate and
      of a landing runs this by itself when the generation key has moved;
      `gate --all` runs it whatever the key says. The container's build
      volume is swept through the same verb, by a tier that has a Linux
      side — never by --host-only, which starts no container at all. The
      volume is a build directory of its own, with its own rustc, key and
      stamp, so a tail starts the verb in there whatever this machine's
      key says and the run in there decides.
      options:
        --dry-run     print what would go, and take nothing
        --if-the-generation-moved
                      sweep only if this build directory's own key has
                      moved since its own last sweep — what a tail runs
                      in the container

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
      timeout, so a silent hang fails by test name, ahead of the CI
      kill. Second step of `check`; subsecond on its own.

  docs [--sync]
      The three ways an edit to internal-docs, .claude/rules,
      .claude/rules-refs or CLAUDE.md tears a block off the list it
      belonged to: a line left at a bullet's indent under a block that
      closed the list above it (ORPHAN-INDENT), a heading with no blank
      line over it (HEAD-NO-BLANK), and a table written at column 0
      inside a list, which ends the list and leaves the text after it
      hanging (TABLE-IN-LIST). All three survive review because nothing
      about them looks wrong in the source — every word is still there,
      in the order it was written — and only the renderer knows a line
      stopped belonging to the thing above it. A list opens on a bullet
      at any indent and is closed only by a block at column 0, which is
      what lets a table or a heading indented inside a list stay part of
      it; fenced code and YAML front matter are skipped whole. Structure
      only: how wide a line runs and how a sentence is built are the
      writer's. Third step of `check` and an always-step of `gate` —
      sixteen files in twenty milliseconds is under the cost of asking a
      stamp whether it still answers. The Write hook says the same thing
      about the one file an edit just landed in, which is where a tear is
      cheapest to undo: the turn that made it is the only one that still
      knows what the line meant to say.

      It also holds what those documents quote: the value cells of
      デザイン規約 against Theme.qml and Metrics.qml, and the command
      lines against the catalogue the modules declare
      (.claude/rules-refs/structure.md §コマンドの正本).
      options:
        --sync        write what is generated. Everything it writes is
                      also said, so the drift reaches the person who has
                      to decide which side was wrong

  qmltest
      The QtTest files under crates/platitude-app/tests/qml, run through
      Qt's own qmltestrunner (offscreen). They hold what only QML can be
      asked — whether a Canvas that owes the screenshot a paint has
      painted — and no Rust test reaches it. The product's whole QML
      module is staged under target/qmltest with its shipped qmldir,
      because `import platitude.ui` resolves by directory name and the
      product's directory is called `ui`. Nothing of the app is
      compiled, so a file answers in a fraction of a second.
      They run in one process. Every TestCase every file declares has to
      be in the log finished — a file may declare several, an
      initTestCase without its cleanup is a run that went down inside
      that case, and nothing here is optional, so a case simply absent is
      a run that fell short rather than one that was allowed to. A run
      that is not green is then run one file per process, which is what
      says which file; the verdict stays the first run's either way, so
      files that pass apart and fail together are not independent of
      each other, and that is a failure of its own. `gate` runs it, on
      both sides, when the change reaches that module or the tests
      themselves; `cargo xtask linux qmltest` is the container.

  deny
      cargo-deny over deny.toml: the bans that keep networking crates and
      git implementations out of the closure (CLAUDE.md 絶対制約), the
      registries, and the license allow list. Three checks —
      advisories fetches the RustSec database from github.com, and stage
      2 stays offline; CI runs the full set. Needs cargo-deny installed
      (`cargo install --locked cargo-deny`), and says so when it is
      missing. `gate` runs this when deny.toml or a manifest
      moved, which is every way the closure can change.

";

// No `"\` continuation on the opening line (see `demo_repo`).
const TAIL: &str = "  shipped [--no-build]
      Starts the build nobody else here makes: `cargo build --profile
      shipped` (target/shipped/) with no features, which is the one
      without the verification harness. Offscreen, bounded, reaped — a
      shipped build has no watchdog of its own, because that is a
      harness knob.
      What it is for is the failure only this build has: a QML file in
      `platitude.ui` that reaches into `platitude.auto` resolves in every
      build this runner drives and loads nothing in the shipped one.
      `structure` catches the type names statically; this catches the
      rest, an import of the absent module included.

  verbs
      Which verbs the gate never runs. The gate runs the argument lines
      the census holds, so a verb with no line there is one no change
      re-photographs — however it would be judged if it ran. Counts both
      sides off the tree: the names the harness compares PGG_AUTO_ACT
      against (crates/platitude-app/src/auto), and the first word of each
      census line. Prints the verbs with no line, and fails on a line
      whose verb the harness does not name anywhere — that one the gate
      still runs, and the run waits out its whole ceiling saying nothing.
      The two halves read the harness differently on purpose: the count
      is of dispatches, the failure is of every name it holds, since a
      verb can be reached through a list too.
      Recording a missing verb is one `verify-ui <line>` without
      --no-census.

  wedge-check
      Stops three runs on purpose and reads back what the parent could
      say about each. A run reaped at a ceiling is the one red nobody
      can arrange by waiting for it, and the account it leaves is worth
      only what it says when one happens: so a run is held past its own
      exit with no deadline thread (which leaves no wedge.txt, the
      shape observed), one is held with the thread up (both records,
      agreeing), and one is given a ceiling no verb can finish inside,
      which answers its watchdog on the event loop. Each is checked for the
      sentences the next occurrence will be read from — the stations it
      reached above all, which is the half that does not need the
      process to still be answering.
      Each run must fail and leave the expected record. The held cases
      must have completed the act and saved its picture first; a watchdog
      abort reaching the same exit station is not accepted.

  corpus [--force] [--path <dir>] [--against <repo>]
      Builds the repository `perf` measures against, and says where it
      is. 200,000 commits, 50,000 refs and a hundred thousand tracked
      files through fast-import — about six minutes and seven and a
      half gigabytes, once. A run that finds it already there does
      nothing, and prints what it found: the counts, the token, the
      working tree, the remotes, the graph the window would draw, the
      text its rows carry and what its diffs cost.
      --against takes those same readings of another repository and
      writes nothing to it, which is how the distance table in
      ci/baseline/perf-windows-x64.md is taken: both of its columns
      have to come from one implementation, or they are two
      definitions.
      The six minutes is nine and a half million objects through
      fast-import, which reads one stream on one thread: the blobs go
      through four of them at once and the commits through one, and
      the build prints its phases so that stays visible.
      It is generated because a clone of somebody's
      working repository is fetched behind the measurement's back, and a
      fetch changes the rows the graph draws, the ref tables the memory
      is mostly made of, and the commit whose diff is timed, all while
      HEAD holds still. It is not in git because the generator is the
      thing worth keeping: the dates and the strings are fixed, so a
      corpus deleted and built again is the same corpus, down to the
      object ids.
      It lives in .pgg-perf-corpus/ beside the primary checkout's .git —
      ignored, like the shot board — so all six seats measure one corpus
      and nothing is written outside the project. --path puts it
      somewhere else; --force builds over one already there.

  perf --repo <path> [--at <rev>] [--label <name>] [--runs <n>] [--breakdown] [--shipped]
      The measurement behind ci/baseline/perf-windows-x64.md, run the
      same way every time: release build, a real window (offscreen
      reports neither memory nor fps honestly) on one named screen, the
      PGG_AUTO_* hooks, WorkingSet and private bytes sampled every 100ms
      for their maximum, and a deadline with a kill guard. The first run
      is discarded — the record is a warm-cache number — unless an
      invocation minutes ago warmed the same exe and corpus, which the
      last one leaves a note of under target/perf.
      The machine is measured beside the process, and a run it spoiled is
      taken again: the session locking, the window
      going down or moving screens, frames not arriving at the rate the
      screen could show, and the share of the machine that went to
      something else — the git the app runs counted as the app's own,
      through a job object. The screen is kept awake for the whole invocation,
      builds and waits included, because a screen that went dark between
      two runs is as unmeasurable as one that went dark during one.
      --breakdown builds with the `memprobe` feature and adds
      PGG_MEM_REPORT=1, then prints the largest `mem report` line the run
      produced: live Rust heap, the models and the session parts holding
      it, and what none of them account for. Process memory minus Rust
      live bytes is not a measurement of Qt's live heap.
      --at <rev> measures the rig's build of that commit: the commit is
      checked out in .claude/worktrees/rig —
      the one worktree there that is no seat, entered and edited by
      nobody — built there with the feature set asked for, and the exe
      is shelved under the rig's target/ by commit and feature set, so
      measuring the same commit again (the other side of an A/B, the
      next stage table of a record) builds nothing. The seat that asked
      keeps its target/ and its uncommitted edits, and those edits are
      not what is measured: the commit is. Evidence still lands under
      this tree's target/perf, and the manifest names the commit.
      options:
        --at <rev>        the commit to measure, built on the rig (above)
        --runs <n>        kept runs after the discarded first (default 3)
        --watchdog-ms <n> outer hang ceiling (default 300000)
        --settle-ms <n>   hold the app idle this long after it reports,
                          then read the memory once more and print it
                          beside the peak. The peak says what the work
                          cost while it ran; the difference says how much
                          of that the process handed back once it had
                          nothing to do (default 0 = read at once)
        --attribute       at the end of that wait, read the settled
                          process from outside it, into attribution.txt in
                          each run and a summary at the end of the report:
                          VirtualQueryEx for what is committed as private
                          / mapped / image and the private allocations by
                          size class (where the heap segments line up),
                          QueryWorkingSetEx for what is resident by file
                          (the fonts summed on one line — a CJK fallback
                          font costs its resident pages),
                          and the process heaps block by block, busy and
                          free by size class (RtlQueryProcessDebugInformation).
                          This is the side the Rust counter of --breakdown
                          cannot see: the C++ objects QML builds and
                          tree-sitter's trees. Needs --settle-ms; Windows only
        --no-scroll       leave the scroll benchmark out
        --no-select       selection empty, diff closed
        --selection <s>   none, first (default), or head
        --select-oid <id>  select this full OID from the loaded graph
        --file <path>     open this changed file (default: first)
        --cases <tsv>     fixed name/OID/path/raw-or-coloured cases (tab-separated)
        --cycles <n>      repeat the case list; needs two distinct OIDs
        --completion <s> raw (default) or final coloured frame
        --diff-scroll    separately measure each visible overflowing diff
        --cache <s>      warm (default), first (--runs 1), or cold
        --cold-prepare <exe>  explicit cache preparation; receives repo and binary
        --compare <rev>  ABBA against --at; --runs counts blocks (two samples each)
                          See ci/baseline/perf-local.md for contracts and OS baselines.
        --no-diff         select and show details without opening a diff
        --output <path>   new directory for metadata and raw per-run logs
        --trace-frames    diagnostic app-clock frame series in app.log;
                          flushed after scrolling, diagnostic only
        --no-open         start with no repository at all — the window and
                          nothing in it. Subtracting this from a run that
                          opened an empty repository leaves the cost of
                          putting the page up, which is otherwise
                          indistinguishable from the toolkit's own floor.
        --no-build        use the binary already built (target/release/,
                          or target/shipped/ with --shipped; with --at,
                          the rig's shelf or nothing). Runs with
                          and without --breakdown build to the same
                          path, so this measures whichever ran last — the
                          evidence names the feature set that was asked
                          for, whatever is on disk
        --shipped         measure `cargo build --profile shipped` (no features)
                          — the build a person installs. It answers no
                          knob and reports no frame, so what it gives is
                          memory and the time to a finished graph; the
                          repository comes from the saved session, the way
                          a person opens one. Refuses the flags it cannot
                          answer. Run it beside an ordinary run to say
                          what carrying the harness costs.
        --screen <name>   OS device name to put the window on (default:
                          the primary). Every frame number is downstream
                          of this where monitors differ in refresh rate
        --corpus <token>  refuse unless the benchmark repository is the
                          one this token names — `perf` prints the token,
                          and a fetch changes it while HEAD holds still
        --retries <n>     how many times a run the machine spoiled is
                          taken again (default 3)
        --allow-noisy     publish what a busy, covered or locked machine
                          produced. The conditions are printed either way
        --software        draw with the software scene graph, whose
                          frames need no display: the display may be
                          on, off, or turned on and off while the runs
                          go, nothing holds it awake or pokes the input
                          timer, and the window is not raised over
                          whatever a person has in front. D3D presents
                          nothing to a display that is off and its
                          frames never swap, which is what this is for.
                          The working set is that renderer's, read
                          beside a D3D reading of the same commit, and
                          every frame number is labelled as not the
                          display path's

  still
      Whether a measurement is holding the machine still, and which
      builds are under way. `perf` holds the machine for the length of
      its runs; every app build here (`app_build::app_exe`), every cargo step
      of check and gate, the linux container, verify-ui and a corpus
      being made announce themselves and wait for a hold to lift before
      they begin, and a hold waits for the builds already announced — a
      build beside a measurement is counted as load that was not the
      application, and the run is refused and taken again. A cargo a
      session types while a hold stands is refused by the pre-shell hook
      unless it is the task runner or compiles nothing, because it runs
      outside any verb that could wait. Both are notes beside .git with a
      lock file each, held open by the process: a lock nobody holds is a
      note whoever meets it clears, whatever became of the process.

  footprint [--settle <seconds>] <command…>
      Runs the command and records, on one time axis, what the host has
      left, what the WSL VM says it is using, which containers stood
      before and after, and which processes inside the VM did the
      reading. The samplers stay up for --settle seconds after the
      command (60 by default), because what a run leaves behind is not
      the last tick of it.
        cargo xtask footprint --settle 300 gate --all --fresh
      For the question repeating a gate raises: whether the VM grows and
      what kind of memory grew. Neither side answers it alone —
      `vmmemWSL`'s working set is what Windows has lost, /proc/meminfo
      inside the distro is what Linux thinks it holds, and memory is not
      namespaced, so that file is the whole VM and a sum over docker
      stats is not a substitute. Judge by MemAvailable: MemFree alone
      cannot tell a cache that would be given back from memory that is
      gone. The rows are kept under target/footprint/<run>/ and nothing
      sweeps them.

  budget [--dir <tree>]
      What the machine's one budget is doing: how much of it the running
      units hold, the queue behind them in the order they will be handed
      it, and the landings in line. Every gate on the machine draws on
      this — one pool over both sides, every seat and every step, sized
      at one gate's widest moment (internal-docs/反映前テストの機械化.md
      §機械の予算と優先キュー) — so two gates at once share one gate's
      worth. The order is what
      moves main, then a window somebody is waiting at (`launch`), then
      every test; while a unit of one rank is short of room, nothing
      below it is admitted into what it is waiting for. A ticket whose
      process is gone holds nothing: liveness is the lock beside it, and
      the next reader clears it.

  budget hold --until <file> [--say <file>] [--weight <n>]
              [--landing | --launch] [--turn] [--dir <tree>]
              [--seat <name>] [--what <text>]
      One ticket, taken from a process of its own and held until the file
      appears. What the suite drives the priority, the exclusion and a
      killed holder with (crates/xtask/tests/gate/budget.rs) — both edges
      are the caller's, so nothing there waits on a clock.

  linux [--rebuild] [--shell] [--stage core|app] [--runner <name>
        [--container <name> --step <mark>]] <command…>
      Run a command against this checkout on Ubuntu, in a container built
      from ci/linux/Dockerfile. On Linux it skips the container and runs
      the command where it stands.
        cargo xtask linux test -p platitude-core --test it
        cargo xtask linux verify-ui commit --preset basic
        cargo xtask linux bare
        cargo xtask linux offline
        cargo xtask linux --container <name> --step <mark> stop
      A cargo command goes to cargo; an xtask verb goes to cargo xtask.
      `--runner <name>` starts an xtask verb from the copy a gate
      prepared; with `--container <name> --step <mark>` beside it the
      verb goes into the gate's own container by docker exec, every
      process of it carrying the mark. `stop` ends what still carries a
      mark in that container — what the gate does at a ceiling, by hand —
      and says what went and what would not.
      `runner <name> --gate <pid>` is the gate's own step and not one to
      type: it builds in the checkout's volume, so it runs only when the
      pid it names is the live gate of this tree. What it clears under
      gate-runner it clears out of one reading taken up front, sparing
      whichever pid the gate note carries at the moment of each
      decision — so one of these held up past its gate cannot reach a
      later run's copy.
      Two more are neither, and both stay in the container on Linux too.
      `bare` builds the release workspace-wide and starts it on an Ubuntu
      carrying only what a package would declare, which is the only check
      that the thing runs somewhere it was not built; `bare --discover`
      works that package list out afresh, whatever the one that
      stands says. `offline` is CI's
      offline-test job run here: it builds the app with the harness and
      the test binaries, then runs ci/offline-test.sh — the suite plus the
      offscreen smoke — in a container with no network at all, which is
      the property CI reaches with `unshare -n`. That script has no other
      caller until CI first runs, so this is what keeps it from drifting.
      Both are worth a place in a pre-merge sweep.
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
      the pre-shell hook lets it through only on the permit the user's
      latest message opened by saying 反映, and the fast-forward is what
      spends that permit — one message, one landing that moved main; a
      landing that stops short spends nothing). In the branch's own
      worktree: commits a census a gate or a verb run there left dirty
      (that one generated file, nothing else), rebases the branch onto
      main when it is behind (a rebase that stops is walked back), runs
      `gate` there — cached steps are not paid twice, and a census the
      verbs rewrote is committed as
      `chore(xtask): the verb census as the land's gate rewrote it` and
      gated once more (a second rewrite stops the landing for a person
      to read) — and only then fast-forwards main, in the primary
      checkout when it sits on main or on the bare ref (reattaching a
      detached primary) when main is checked out nowhere; the
      reference-transaction hook checks the stamp on the way. History
      stays linear and a landed seat stands at main's tip. Refuses a
      branch checked out nowhere, a dirty seat, and a seat that holds
      main. Bare `land` from a seat lands the seat's own branch. The
      seat's claim goes back to the roster with it: the tree is empty
      at main's tip, so the letter is free for whoever asks next, and a
      session that goes on working there claims it back at its next
      edit. A live claim of another session's stays, and so does a lock
      written by hand.

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
      PGG_ALLOW_GUI=1 in front — a real window is the user's ask. The app
      keeps a per-tree settings store on its own, so seats never fight
      over one instance lock. Somebody is waiting at the screen for this,
      so its build takes the machine ahead of every test and behind a
      landing (`budget`, and it says so when it waited); the window
      itself holds none of it.

  seat [release [<letter>]|takeover <letter>]
      Hands this session a worktree seat, and takes no argument: the
      letter is the answer (CLAUDE.md ビルド・テスト).
      Letters are tried until `git worktree lock` takes one, so the seat
      comes back already claimed — the lock is the only step that ever
      decided which of two sessions got a seat, and a session that picks
      a letter off a survey first is deciding from something the other
      session can read the same way. A letter taken from the roster comes
      back ready for a fresh stretch: claimed, with nothing uncommitted,
      on its own branch at main's tip, its board runs swept, created if
      the roster never made it. The tree this session is already standing
      in comes back instead, and comes back as it stands — emptiness is
      asked of a letter before it is handed to somebody, and the session
      in the tree is not somebody, so its own work stays where it is,
      branch and pictures alike, whether the claim came off by landing or
      by a release. A letter somebody else's claim is on is passed over
      either way. Asking twice gives the same seat. It prints the path
      EnterWorktree wants, and that is the only path the entry hook will
      let through, because the claim behind it is this session's. When
      every letter is held or carries work it fails, names what stands in
      the way, and that is where the session stops — seats are not added
      past f, and a letter changes hands only on the user's word.
      A claim is a conversation's, and no process is asked about it: the
      app restarting under a session, the machine sleeping (the SessionEnd
      every open conversation is handed), a tab left open — none of it
      lifts a claim. Three things do. `land` hands the seat back when the
      branch reaches main, whoever's claim was on it; a session that goes
      on working in the tree takes it again at its next edit or picture.
      `seat release [<letter>]` hands a seat back without landing and
      names what it still carries; the letter is needed only when the
      session holds more than one. And `seat takeover <letter>`, behind
      PGG_ALLOW_TAKEOVER=1, moves the letter to this session whoever
      holds it — a live session, one that is gone, nobody — with the tree
      as it stands: nothing moved to main's tip, the board's pictures
      kept, a tree that went away grown back on the letter's own branch.
      The pre-shell hook lets that verb through only behind the flag,
      which a session writes for an instruction that asked and for
      nothing else. Freeing a letter somebody else holds is a takeover
      followed by a release; a person's own `git worktree lock` is
      nobody's to take, and comes off by hand.

  seats
      Where the six worktree seats a-f stand right now, one line each:
      branch, whether HEAD sits at main's tip, commits ahead of main
      (main..HEAD), uncommitted changes (status --porcelain lines), and
      how long since the seat's own index was written. For reading how
      the roster stands — `seat` is what sits down, and nothing here is
      a letter to choose from. A claimed seat says whose it is past the
      columns, session and pid, and that is all a reader is told: whether
      the conversation behind it goes on is the user's to know, and a
      letter the user wants back changes hands with `seat takeover`. A
      letter with a branch but no tree says what that branch still
      carries — work nothing can reach until a takeover grows the tree
      back. Ends with how to read the columns.

  shots <add|crop|prune|open [--again]|list|path>
      The shot board: pictures land on a page that opens in a window of
      its own at 1:1, magnifies to exact whole ratios without smoothing,
      and is still there after it is closed. A chat pane fits a picture to
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
      whichever seat or session asks, and
      `--again` is the reader's own word that they closed theirs. Six
      windows of one board are six answers to which one is current, so
      the count is kept in the board itself
      (shots/window.rs).

      `crop <png> --at <x>:<y>:<width>:<height> [--scale <n>]` cuts a
      region out and magnifies it a whole number of times (three by
      default) without smoothing, beside the picture it came from. The
      window does this for a person; this is for reading the file
      itself, where there is nothing to zoom with. Both halves matter:
      a fractional ratio has to interpolate, and what a crop is read for
      is a pixel.

      A before/after goes on with `add --before <png> --after <png>`,
      which puts the two on one view side by side under one magnifier.
      As one run: read one after the other, the difference is
      whatever the reader remembered.

      The board keeps itself to what is being looked at now, and the
      rules that do it are in shots/sweep.rs: a picture retaken replaces
      the one before it, and a seat's runs go when its work does (its
      branch lands, or the seat is handed to fresh work). A session
      ending leaves the board standing. `prune` is what
      reaches the rest — an approach abandoned under a name nobody
      retakes (--label), or a seat whose session is long gone.

  awake [log on|off]
      Which sessions keep this machine from idle-sleeping (Windows
      only): each session's hooks, and each subagent's apart, write a
      claim under .awake/ in the primary checkout — in one activity
      protocol, which every compatible build's hooks update, so a Stop
      reaches every holder reading it — working from a prompt, idle once
      the turn stops, and each tool call from its start to its end, with
      whether a person is asked about it; a holder with no window, one
      per revision of its script, reading while any claim stands, asks
      the system, never the display, to stay up while any counts. A
      working claim counts while one of its own calls runs (open, no
      result yet, no person asked — however long it runs), or for an
      hour after its last hook while none is open — never while its open
      calls all wait on a person, nor once the session's transcript ends
      on the user interrupting the turn. A subagent stopped from outside
      runs no SubagentStop: its claim goes once its session's transcript
      records its stop (a background task's notice naming it, or the
      result of the call it runs under). A notice (a background task's
      end, another session's message) written into a session's
      transcript since its last hook starts a turn: the holder marks the
      claim working from it, until the turn's Stop. An MCP server's
      pending request for input holds up one of the session's running
      calls on that server, and its answer frees that one request alone.
      No hook says when a person lets a call through, nor when a tool
      the desktop app runs asks a person itself: each holder reads
      those, best-effort, off the desktop app's own log
      (%LOCALAPPDATA%\\Claude\\logs\\main.log) — a permission request it
      ties to one open call alone holds that call up until its answer,
      and a grant sets it running again. The log is read after the
      fact, so neither is protected before the reading that sees it;
      without a line it can tie (no log, a format it does not know,
      calls it cannot tell apart) a call nothing shows running (an MCP
      tool, a fetch) is not seen from its grant to its end, and one the
      app asks about itself counts as running while it waits. `log off`
      makes every holder go by the hooks alone from its next reading,
      `log on` reads the log again. Any claim counts while a process of
      one of its shell calls runs — the session's or a subagent's, sent
      to the background at the start or halfway — or until the wake-up
      it scheduled is due. A process the Claude process already had, or
      started during another tool's call, is no work. The directory's
      lock is held for its files alone. Prints the claims as written,
      each holder's last verdict, and what it last made of the desktop
      app's log.

  hook <event>
      Claude Code hook handler (wired from .claude/settings.json; reads
      the hook payload from stdin). Events: pre-write, post-write,
      pre-shell, pre-tool, pre-chip, post-chip, post-tool,
      permission-request, permission-denied, elicitation,
      elicitation-result, subagent-start, subagent-stop, stop,
      stop-failure, pre-worktree, post-worktree, prompt-submit,
      session-start (which also installs the gate's git hook),
      session-end.
";

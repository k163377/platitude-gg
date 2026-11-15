//! The usage page's measurement: the corpus, perf, and what this machine
//! holds and spends while a command runs.

// No `"\` continuation on the opening line (see `demo_repo`).
pub(super) const ENTRIES: &str =
    "  corpus [--force] [--path <dir>] [--against <repo>] [--worktrees <n>]
         [--probe]
      Builds the repository `perf` measures against, and says where it is:
      200,000 commits, 50,000 refs and a hundred thousand tracked files
      through fast-import — about six minutes and seven and a half
      gigabytes, once. A run that finds it already there does nothing but
      print what it found: the counts, the token, the working tree, the
      remotes, the graph the window would draw, the text its rows carry
      and what its diffs cost. --against takes those same readings of
      another repository and writes nothing to it, which is how the
      distance table in ci/baseline/perf-windows-x64.md is taken: both of
      its columns have to come from one implementation, or they are two
      definitions. For ci/baseline/git-slots-windows-x64.md,
      --worktrees <n> stands exactly n worktrees beside it (0 takes them
      down; each is a full checkout and a ref that moves the token), and
      --probe measures, in the first, what the reads behind a click cost
      as processes and what git's own speed-ups give back.
      The six minutes is nine and a half million objects through
      fast-import, one stream on one thread (the blobs through four at
      once, the commits through one); the build prints its phases. It is
      generated because a fetch changes a clone of somebody's working
      repository behind the measurement's back — the rows the graph draws,
      the ref tables the memory is mostly made of, and the commit whose
      diff is timed, all while HEAD holds still. It is not in git because
      the generator is the thing worth keeping: the dates and the strings
      are fixed, so a corpus deleted and built again is the same corpus,
      down to the object ids.
      It lives in .pgg-perf-corpus/ beside the primary checkout's .git —
      ignored, like the shot board — so all six seats measure one corpus
      and nothing is written outside the project. --path puts it somewhere
      else; --force builds over one already there.

  perf --repo <path> [--at <rev>] [--label <name>] [--runs <n>] [--breakdown] [--shipped]
      The measurement behind ci/baseline/perf-windows-x64.md, run the same
      way every time: release build, a real window (offscreen reports
      neither memory nor fps honestly) on one named screen, the _PGG_AUTO_*
      hooks, WorkingSet and private bytes sampled every 100ms for their
      maximum, and a deadline with a kill guard. The first run is
      discarded — the record is a warm-cache number — unless an invocation
      minutes ago warmed the same exe and corpus, which the last one
      leaves a note of under target/perf. A warm-cache invocation with the
      harness starts, before that, with a calibration run that weighs the
      font database's population, which the record reads the budget line
      net of.
      The machine is measured beside the process, and a run it spoiled is
      taken again: the session locking, the window going down or moving
      screens, frames not arriving at the rate the screen could show, and
      the share of the machine that went to something else — the git the
      app runs counted as the app's own, through a job object. The screen
      is kept awake for the whole invocation, builds and waits included: a
      screen that went dark between two runs is as unmeasurable as one
      that went dark during one.
      --breakdown builds with the `memprobe` feature and adds
      _PGG_MEM_REPORT=1, then prints the largest `mem report` line the run
      produced: live Rust heap, the models and the session parts holding
      it, and what none of them account for. Process memory minus Rust
      live bytes is not a measurement of Qt's live heap.
      --at <rev> measures the rig's build of that commit: the commit is
      checked out in .claude/worktrees/rig — the one worktree there that
      is no seat, entered and edited by nobody — built there with the
      feature set asked for, and the exe is shelved under the rig's
      target/ by commit and feature set, so measuring the same commit
      again (the other side of an A/B) builds nothing. The seat that asked
      keeps its target/ and its uncommitted edits, and those edits are not
      what is measured: the commit is. Evidence still lands under this
      tree's target/perf, and the manifest names the commit.
      Each build is made and run with the Qt its own commit pins (its
      .qt-version, as qmake answers for it), so --at across a Qt update
      compares the whole dependency set; --qt <version> builds and runs
      with that Qt instead, to tell Qt apart from the rest, and the
      manifest says which (qt_source). A shelved exe is taken again only
      when the commit, the feature set, the Qt (qmake's version and
      prefix), rustc, the C++ compiler, cargo's configuration files and
      the build flags in the environment all match — build.txt beside it,
      and in the evidence, lists them — and a run is refused if it would
      load another Qt than its build linked. The rig's build does not
      inherit what this tree's cargo configuration put into the
      environment (its [env]): the commit's own configuration sets that.
      cargo still reads the configuration of every directory above the
      rig, the primary checkout's included, and build.txt names each one
      it read.
      See internal-docs/性能.md §計測 for contracts and OS baselines.
      options:
        --at <rev>        the commit to measure, built on the rig (above)
        --qt <version>    build and run with this Qt, not the pinned one
        --label <name>    the report's heading (default: the repository's
                          name)
        --runs <n>        kept runs after the discarded first (default 3)
        --watchdog-ms <n> outer hang ceiling (default 300000)
        --settle-ms <n>   hold the app idle this long after it reports,
                          then read the memory once more and print it
                          beside the peak, whose difference is what the
                          process handed back once idle (default 0 = read
                          at once)
        --attribute       at the end of that wait, read the settled
                          process from outside, into attribution.txt in
                          each run and a summary in the report: what is
                          committed as private / mapped / image, the
                          private allocations and the heaps' blocks (busy
                          and free) by size class, and what is resident by
                          file (the fonts summed on one line) — the side
                          --breakdown's Rust counter cannot see: the C++
                          objects QML builds and tree-sitter's trees.
                          Needs --settle-ms; Windows only
        --no-font-walk    no calibration run: the working set is published
                          walk included
        --no-scroll       leave the scroll benchmark out
        --no-select       selection empty, diff closed
        --selection <s>   none, first (default), or head
        --select-oid <id> select this full OID from the loaded graph
        --file <path>     open this changed file (default: first)
        --cases <tsv>     fixed name/OID/path/raw-or-coloured cases
                          (tab-separated)
        --cycles <n>      repeat the case list; needs two distinct OIDs
        --completion <s>  raw (default) or final coloured frame
        --diff-scroll     separately measure each visible overflowing diff
        --back-to-back    start each operation where the last one left the
                          page (default: on a page that owes nothing, no
                          git running)
        --cache <s>       warm (default), first (--runs 1), or cold
        --cold-prepare <exe>
                          cache preparation; receives repo and binary
        --compare <rev>   ABBA against --at; --runs counts blocks (two
                          samples each)
        --no-diff         select and show details without opening a diff
        --output <path>   new directory for metadata and raw per-run logs
        --trace-frames    diagnostic app-clock frames in app.log, flushed
                          after scrolling
        --no-open         start with no repository — the window and
                          nothing in it; subtracted from a run on an empty
                          repository, it leaves the cost of putting the
                          page up, which is otherwise indistinguishable
                          from the toolkit's own floor
        --no-build        use the binary already built (target/release/,
                          or target/shipped/ with --shipped; with --at,
                          the rig's shelf or nothing). Runs with and
                          without --breakdown build to the same path, so
                          this measures whichever ran last — the evidence
                          names the feature set that was asked for,
                          whatever is on disk
        --shipped         measure `cargo build --profile shipped` (no
                          features) — the build a person installs. It
                          answers no knob and reports no frame, so what it
                          gives is memory and the time to a finished
                          graph; the repository comes from the saved
                          session, the way a person opens one. Refuses the
                          flags it cannot answer. Run it beside an
                          ordinary run to say what carrying the harness
                          costs.
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
        --software        draw with the software scene graph, whose frames
                          need no display — D3D presents nothing to a
                          display that is off: the display may be on, off,
                          or switched while the runs go, nothing holds it
                          awake or pokes the input timer, and the window
                          is not raised over what a person has in front.
                          The working set is that renderer's, read beside
                          a D3D reading of the same commit, and every
                          frame number is labelled as not the display
                          path's
        --setting <k=v>   a [defaults] key for the run's settings.toml,
                          for an A/B over a setting; repeatable, copied
                          verbatim (a string carries its own quotes)
        --log <level>     the app's PGG_LOG: error, warn, info (default),
                          debug or trace; debug prints the per-command
                          breakdown, so its run is no budget run

  still
      Whether a measurement is holding the machine still, and which builds
      are under way. `perf` holds the machine for the length of its runs;
      every app build here (`app_build::app_exe`), every cargo step of the
      gate, the linux container, verify-ui and a corpus being made
      announce themselves and wait for a hold to lift before they begin,
      and a hold waits for the builds already announced — a build beside a
      measurement is counted as load that was not the application, and the
      run is refused and taken again. A cargo a session types while a hold
      stands is refused by the pre-shell hook unless it is the task runner
      or compiles nothing, because it runs outside any verb that could
      wait. Both are notes beside .git with a lock file each, held open by
      the process: a lock nobody holds is a note whoever meets it clears,
      whatever became of the process.

  footprint [--settle <seconds>] <command…>
      Runs the command and records, on one time axis, what the host has
      left, what the WSL VM says it is using, which containers stood
      before and after, and which processes inside the VM did the reading.
      The samplers stay up for --settle seconds after the command (60 by
      default), because what a run leaves behind is not the last tick of
      it.
        cargo xtask footprint --settle 300 gate --whole
      For the question repeating a gate raises: whether the VM grows and
      what kind of memory grew. Neither side answers it alone — the VM
      process's working set (wslc's `vmmemwslc-…`) is what Windows has
      lost, /proc/meminfo inside the VM is what Linux thinks it holds, and
      memory is not namespaced, so that file is the whole VM and a sum
      over the engine's stats is not a substitute. Judge by MemAvailable:
      MemFree alone cannot tell a cache that would be given back from
      memory that is gone. The rows are kept under target/footprint/<run>/
      and nothing sweeps them.

  budget [--dir <tree>]
      What the machine's one budget is doing: how much of it the running
      units hold, the queue behind them in the order they will be handed
      it, and the landings in line. Every gate on the machine draws on
      this — one pool over both sides, every seat and every step, sized at
      one gate's widest moment (internal-docs/反映前テストの機械化.md
      §機械の予算と優先キュー) — so two gates at once share one gate's
      worth. The order is what moves main, then a window somebody is
      waiting at (`launch`), then every test; while a unit of one rank is
      short of room, nothing below it is admitted into what it is waiting
      for. A ticket whose process is gone holds nothing: liveness is the
      lock beside it, and the next reader clears it.

  budget hold --until <file> [--say <file>] [--weight <n>]
              [--landing | --launch] [--turn] [--dir <tree>]
              [--seat <name>] [--what <text>] [--jobs <n>]
              [--queued <file>] [--child-until <file>]
      One ticket, taken from a process of its own and held until the file
      appears. What the suite drives the priority, the exclusion and a
      killed holder with (crates/xtask/tests/gate/budget.rs) — both edges
      are the caller's, so nothing there waits on a clock: its pid goes to
      --queued on joining the queue and to --say on admission, and
      --child-until holds a child under the ticket.

";

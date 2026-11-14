//! What `cargo xtask` prints when it is given no command it knows.
//!
//! The two sections that grow with the app — demo presets and verify-ui
//! verbs — have files of their own, so neither pushes this one over the
//! length ceiling; the commands are what divide next, a file to a stage
//! of the work.

mod demo_repo;
mod verify_ui;

pub(crate) fn print() {
    print!("{HEAD}{}{}{TAIL}", demo_repo::PRESETS, verify_ui::VERBS);
}

const HEAD: &str = "\
cargo xtask <command>

commands:
  gate [--host-only] [--whole] [--fresh] [--keep-going] [--dry-run]
       [--verb <line>]... [--jobs <n>] [--dir <tree>] [--main <ref>]
      The pre-merge tests, chosen by machine (gate skill 確認は 3 段;
      internal-docs/反映前テストの機械化.md). Reads the branch's diff
      against main, follows every file that reads a changed file — a `use`
      path, a re-export, a string naming the file, a QML type name, a QML
      mention of a #[qobject] model — and runs the tests in that reach:
      unit tests by module path, the integration binary by module, clippy
      per crate entered, cargo-deny when deny.toml or a build input moved,
      qmltest when a file under what a QtTest run reads moved (the runner
      as built, the staged module, the tests and the QML they import; on
      the container side its image too), shipped when the app's QML or
      entry point moved, every registered verify-ui line
      (crates/xtask/verb-lines.txt) when the app's QML or the harness that
      drives it is reached, bare when either product crate is reached. A
      file read as it stands — named in a string, a bin a test shoots —
      moves no code: it owes its readers' tests and the registered lines,
      not clippy; taken out, it owes what still names it. A module one
      system alone compiles owes the other side nothing. The always-steps
      (structure, waits, versions, docs, relied-messages check, verbs,
      fmt) run every time and first; a build input that changed
      (Cargo.toml, Cargo.lock, the toolchain, cargo's configuration, a
      vendored crate) makes the reach the whole tree, the Dockerfile the
      container's side of it, clippy.toml every crate's clippy, and a TOML
      change to comments alone nothing. Then host and container sides run
      in parallel; each step is stamped by the object ids of what it
      reads, so a second run of one commit runs nothing and a rebase
      reruns only what main's move touched. The verify-ui verbs of a side
      share nothing but the release the first of them builds, so they run
      several at a time; other steps run one at a time per group, groups
      side by side. The first red stops the run on both sides: nothing
      more starts, what is running is ended (a container step of its own
      is left to finish), and each step kept from running or ended is
      filed as halted, owed again by the next run. A commit whose every
      step is green is stamped, and the reference-transaction hook lets a
      session's git move main onto stamped commits only; a git the user
      runs carries no session's mark, and the hook lets it pass
      (gate::hooks). A component no registered line shows stops the gate
      by name: name it on the row of the line whose run brings it up. No
      step writes the tree, so the stamp names the commit as it stands. A
      tree holds one gate at a time: a second one there is refused with
      the first's pid (a dry run holds nothing).
      options:
        --host-only   the daily tier: no container; stamps the host half
        --whole       stage 3: no diff is read, every file counts as
                      changed; every registered line on both sides (a twin
                      is registered nowhere) and the `periodic` tests;
                      implies --fresh and --keep-going
        --fresh       ignore the stamps and run everything owed
        --keep-going  run the rest after a red: a red verb stops none of
                      the others, the first red of a checks group stops
                      that group, and what passed is stamped. A red
                      always-step stops the run all the same
        --dry-run     print the reach and the steps, run nothing
        --verb <l>    a registered verify-ui line to run besides the
                      reach's, on both sides
        --jobs <n>    this gate's verbs at a time per side (default: a
                      third of the logical CPUs, 1 to 8), from the budget
                      every gate shares
        --dir <tree>  gate that tree
        --main <ref>  read the diff and the stamp against this ref (the
                      tests' sandboxes; default main)
      gate verdict <old> <new>   the hook's question (exit 0 = may move)
      gate install               copy .githooks/reference-transaction
                                 beside .git (pgg-gate/hooks/), note the
                                 tree whose xtask answers, point
                                 core.hooksPath there; every session start
                                 and every land does this
      gate deps [--file <p>]... [--why <p>]... [--history <n>] [--show]
                                 the graph itself: what a change reaches,
                                 how a file got there (--why), what the
                                 last n commits would have owed
                                 (--history); --show adds each file's
                                 readers and reads

  sweep [--dry-run | --if-the-generation-moved]
      Takes away every build product in target/ but the generation the
      canonical cargo lines stand on
      (internal-docs/反映前テストの機械化.md §世代の掃除): cargo never
      removes what it has replaced, so a Cargo.lock, a toolchain or a
      profile setting that moves leaves the whole last generation behind,
      named by nothing. What is live is asked of cargo, never of the
      clock: a written-down list of lines — the gate's, the ones a session
      types between gates, and the two builds the runner makes for itself
      — is run with --message-format=json, and what their artifacts and
      build-script directories name stays. A unit this tree has not built
      is compiled to be read, once per generation, and the count is
      printed. Everything written since this tree's last sweep stays
      whatever the lines said. Only deps/, build/ and incremental/ under
      debug/, release/ and shipped/ are swept — not .fingerprint/
      (kilobytes), the hook's own target/hooks (in use at unpredictable
      moments) or anything under target/ that is not cargo's. A profile
      cargo is building in is left whole, and a run that left one alone
      writes no stamp, so the next tail does it again. Every removal is
      printed with its size, and so is anything that would not go. The
      tail of a gate and of a landing runs this when the generation key
      has moved, and `gate --whole` whatever the key says. A tier with a
      Linux side (never --host-only) also starts the verb in the
      container's build volume, a build directory of its own with its own
      rustc, key and stamp, where the run decides for itself.
      options:
        --dry-run     print what would go, and take nothing
        --if-the-generation-moved
                      sweep only if this build directory's own key has
                      moved since its own last sweep — what a tail runs in
                      the container

  structure
      The per-file length backstop of .claude/rules/structure.md (1000
      code lines — blank and comment-only lines do not count) over
      crates/**/*.rs and *.qml, and what the tree spends on comments; an
      always-step of `gate`. A file the ledger
      (.claude/rules-refs/structure.md 分割しない判断) lists by path, its
      reason not to be split written in its own header comment, has no
      backstop, and the run asks only that the entry still names a file
      that is there; a file pinned by crates/xtask/structure-baseline.txt
      is held at its length, free to shrink (the pin follows it down) and
      not to grow; any other file that crosses the backstop fails. The
      other half of that § — fn 100 lines — is clippy's too_many_lines,
      raised to warn in the workspace lints, and an existing long function
      carries #[expect(clippy::too_many_lines)] until it is cut up. Eight
      more checks of how the tree is divided run here too; the list heads
      crates/xtask/src/structure.rs.

  waits
      No test takes a stretch of clock for an answer, and no verb of this
      runner takes a wait from anywhere but crate::wait
      (.claude/rules/core.md §非同期・並行テスト). Reads every statement
      of test code in every crate, the QtTest files and the app's
      automation harness included, and this runner's own tool bodies, and
      names each shape it refuses (the list heads
      crates/xtask/src/waits/mod.rs): a sleep, a deadline of its own, an
      answer thrown away, a tracked outcome() awaited with no
      support::wait::bounded or timeout under it — so a silent hang fails
      by test name — and the harness's rules for its stages and reach.
      What a test or a verb may still do it says on the line, in a
      waits(<purpose>) comment with its reason; a marker that covers
      nothing is a finding of its own. An always-step of `gate`.

  versions
      The versions the workflows cannot name once, held to their source:
      every runner name of an OS across CI's workflows and local actions
      is one release — Ubuntu's the one the container's Dockerfile names
      in UBUNTU, since the container stands for CI's Linux machine — no
      runner name is -latest, and every action is at one commit under one
      release comment. What a file names once (the crates' lock, the
      toolchain file, Qt's pin, the Dockerfile's AQT_SOURCE that CI's Qt
      action reads) is not read. An always-step of `gate`.

  relied-messages check | audit | watch | master
      git's words the product reads
      (platitude-core/src/relied_messages.rs), held to git's sources. Each
      form names the file of git's tree it is written in and words on that
      line; the ledger (crates/xtask/relied-messages.txt) holds every such
      line of every tree read — each release from the minimum on and
      master, Git for Windows and its main, Apple Git, the container's
      Ubuntu, OpenSSH — grouped where they are the same. `check` (an
      always-step of `gate`, offline) holds the table to the ledger (that
      each witness is a test by its full name is the suite's own
      relied_witnesses test); `audit` fetches every tree into the primary
      checkout's target/relied-messages and writes the ledger again;
      `watch` (weekly in CI) holds the heads and any release the ledger
      has not read to it, red only where a line changed, naming the trees
      that carry the change; `master` (Linux —
      `cargo xtask linux relied-messages master` elsewhere) builds
      master's git and runs the witnesses under it, red too where one of
      them did not run.

  docs [--sync]
      The rule documents — internal-docs, .claude/rules,
      .claude/rules-refs and CLAUDE.md — read for the three ways an edit
      tears a block off the list it belonged to: a line left at a bullet's
      indent under a block that closed the list above it (ORPHAN-INDENT),
      a heading with no blank line over it (HEAD-NO-BLANK), and a table
      written at column 0 inside a list, which ends the list and leaves
      the text after it hanging (TABLE-IN-LIST). A tear keeps every word
      in order, so review does not see it; only the renderer does. A list
      opens on a bullet at any indent and is closed only by a block at
      column 0, which is what lets a table or a heading indented inside a
      list stay part of it; fenced code and YAML front matter are skipped
      whole. Structure only: how wide a line runs and how a sentence is
      built are the writer's. The Write hook says the same about the one
      file an edit just landed in, where a tear is cheapest to undo.

      It also holds the caps of the always-loaded documents (CLAUDE.md
      規約の置き場所), and what the documents quote: the value cells of
      デザイン規約 against Theme.qml and Metrics.qml, and the command
      lines of the documents and skills, this page and xtask's sources
      against the catalogue the modules declare
      (.claude/rules-refs/structure.md §コマンドの正本). An always-step of
      `gate` — reading every file costs less than asking a stamp whether
      it still answers.
      options:
        --sync        write what is generated. Everything it writes is
                      also said, so the drift reaches the person who has
                      to decide which side was wrong

  qmltest [<tst_name>...] | qmltest --together
      The QtTest files under crates/platitude-app/tests/qml, run through
      Qt's own qmltestrunner (offscreen): what only QML can be asked, such
      as whether a Canvas that owes the screenshot a paint has painted.
      The product's whole QML module is staged under target/qmltest with
      its shipped qmldir, because `import platitude.ui` resolves by
      directory name and the product's directory is called `ui`; the
      Rust-backed `platitude` module is staged all but empty, with the
      stand-ins under tests/qml/stubs (`GitFacts`, `Harness`, `PerfProbe`)
      as its singletons. Nothing of the app is compiled.
      Every file runs in a process of its own — the files named (by stem,
      `tst_ink`) or, with none named, all of them — and a red file does
      not stop the rest. Every TestCase a file declares has to be in the
      log finished: an initTestCase without its cleanup is a run that went
      down inside that case, and a case absent is a run that fell short.
      `--together` runs every file in one process instead: a red there
      that the files' own processes do not show says the files are not
      independent of each other, and the verdict names the last file that
      finished green and the first that failed, the cause undetermined. A
      run that is not green keeps its staging and a manifest under
      target/qmltest/failed, in the group of its kind (suite, selected,
      together; the newest five of each). `gate` runs the files apart, on
      both sides, when a changed file lies under what a run reads, and the
      one-process run in the whole gate alone; `cargo xtask linux qmltest`
      is the container.

  deny
      cargo-deny over deny.toml: the bans that keep networking crates and
      git implementations out of the closure (CLAUDE.md 絶対制約), the
      registries, and the license allow list. Runs bans, sources and
      licenses: advisories fetches the RustSec database from github.com
      and stage 2 stays offline, so only CI's deny job
      (.github/workflows/ci.yml) runs all four. Needs cargo-deny installed
      (`cargo install --locked cargo-deny`), and says so when it is
      missing. `gate` runs this when deny.toml, a manifest or another
      build input (the lock, the toolchain, cargo's configuration, a
      vendored crate) moved — every way the closure can change.

";

// No `"\` continuation on the opening line (see `demo_repo`).
const TAIL: &str = "  replay [--tier <fonts|premerge|whole>] [--line '<verify-ui line>']...
         [--out <dir>] [--no-build]
      Runs verify-ui lines again on the machine this stands on, one after
      another, and gathers every run's pictures and log under one
      directory with a summary.md beside them — what CI's macOS job runs
      (.github/workflows/shots.yml), where no gate stands and the pictures
      have to travel to be looked at. Each line is one verify-ui, judged
      as a typed run is; the lines build the release until one has gone
      green, and the rest reuse it. A red line stops none of the others,
      unless what went red is the build they all read: the lines left are
      then named in the summary, not run. Nothing here writes the board.
      options:
        --tier <t>    the registered lines to run: fonts (the tier table's
                      rows judged on what the fonts decide), premerge
                      (every line a gate owes before a merge), whole
                      (every line the whole gate owes); none takes no
                      registered line
        --line <l>    a line to run besides, as it would follow verify-ui;
                      one value may hold several with ; between them
        --out <dir>   where the pictures and logs go: a directory that
                      holds nothing yet, which replay never empties. Left
                      out, this tree's target/replay, taken down and stood
                      up again by every run, one run at a time
        --no-build    reuse the release already built

  shipped [--no-build]
      Starts the build nobody else here makes:
      `cargo build --profile shipped` (target/shipped/) with no features,
      the one without the verification harness — offscreen, bounded and
      reaped here, since a shipped build has no watchdog (a harness knob).
      Green is the app's own line once Main.qml loaded, `window loaded`,
      waited for as long as the load takes, and the window still up a
      short stand after it. Anything short of that is red, worded by where
      it fell short: Qt's line that the QML would not load, no
      `build tree` (the app never got going), an exit before the stand is
      out, or the ceiling, which only names a load that never ends — how
      long a load takes is the machine's, so no stretch of clock stands in
      for the line.
      The window that stood is then stopped as an IDE stops it — Ctrl+C
      through its console on Windows, SIGINT elsewhere — and must close
      normally through its own close. On Windows two more windows take a
      Ctrl+C that must not reach them, once cargo left and under a parent
      that is not cargo, and must stand; elsewhere a second window must
      stand through a SIGHUP its launcher ignored, then close normally on
      SIGTERM.
      What it is for is the failure only this build has: a QML file in
      `platitude.ui` that reaches into `platitude.auto` resolves in every
      build this runner drives and loads nothing in the shipped one.
      `structure` catches the type names statically; this catches the
      rest, an import of the absent module included.

  package [--out <dir>] [--password-from <variable>] [--no-build]
      A snapshot of the app as a macOS bundle, on a Mac only, holding of
      Qt only what the lists in crates/xtask/src/package/contents.rs name.
      First, qmlimportscanner reads ui/ twice. As it is written (no
      -importPath), each of Qt's modules ui/ imports must be one the
      bundle carries; followed through Qt's modules (-importPath at Qt's
      qml), each module reached must be in a directory of Qt's qml and
      either carried or listed as reached and left out, with why, and each
      module carried or so listed must be reached. Each miss is red, by
      name, and so is anything the scanner says on stderr: a file it could
      not read drops out of its answer, imports and all. Then the shipped
      build and pgg-todo-editor go in Platitude GG.app with the icon and
      the Info.plist (packaging/macos — the workspace version, and the
      snapshot's identifier, which is not the release's). xtask puts in
      the Qt plugins and the QML modules the lists name — a module's
      plugin in Contents/PlugIns/quick, linked from its directory — and
      strips each library (-x) as macdeployqt strips its own. macdeployqt
      then runs inside the bundle with -no-plugins and no -qmldir, and
      brings only the frameworks the app and those libraries load. The
      app's binary and each library xtask put in keep one run path, inside
      the bundle, and pgg-todo-editor none; qt.conf says where the plugins
      and the QML modules are (Plugins = PlugIns, QmlImports =
      Resources/qml). Before the signature, the bundle's Frameworks,
      PlugIns and QML modules are held to the lists exactly, and each
      module's plugin to its qmldir: one more or one fewer is red, by
      name. Signed ad hoc and verified, it is started offscreen from
      inside the bundle and judged as `shipped` judges a build — its
      `window loaded` waited for, then the window standing — and then by
      dyld's own account (DYLD_PRINT_LIBRARIES): an image from neither the
      bundle nor the system is red. Last, ditto zips it as
      platitude-gg-snapshot-<day>-<commit>-macos-arm64.zip — or, with a
      password, hdiutil puts it in an AES-256 .dmg of that name, which
      must then call itself encrypted and open under the password onto a
      bundle whose signature still verifies.
      Not a release: no notices of the bundled licences travel with it,
      and no Apple identity signs it.
      options:
        --out <dir>   where the bundle and the archive go (the bundle is
                      replaced); left out, this tree's target/package
        --password-from <variable>
                      the .dmg instead of the zip, under the password the
                      variable holds (CI: SNAPSHOT_DMG_PASSWORD, the
                      repository secret). Never a password on the command
                      line: hdiutil reads it off its stdin
        --no-build    package the shipped build already built

  verbs
      The registered lines (crates/xtask/verb-lines.txt) held against the
      harness, the judgment tables and the sources, before any gate
      chooses off them. Fails on: a verb the harness answers
      (crates/platitude-app/src/auto, the names it compares PGG_AUTO_ACT
      against) with no registered line — one no change re-photographs; a
      registered line whose verb the harness no longer names anywhere —
      the gate would run it and the app ignore the name; a line spelled
      otherwise than verify-ui reads it back; a judgment row of
      verify/verbs no registered line reaches; a row naming what is no
      component, or what every line shows already; the tier table
      disagreeing with the lines; a harness file nothing loads; a verb the
      window answers that the page's completion ledger
      (AutoActCompletion.defersCompletion) does not defer. The two halves
      read the harness differently on purpose: the answered verbs are its
      dispatches, the gone check reads every name it holds, since a verb
      can be reached through a list too.
      Registering a line is one row: the line as typed, a tab, and what
      its run brings up beyond the window's core (or -). A run says what
      it met beside its row (`shown:` in verify-ui's output).
      options:
        --core        print the window's core: what Main and the `*` row
                      compose, which every line shows
        --shown <l>   print a registered line's subjects and what they
                      bring up beyond the core
        --why <c>     print whether a component is in the core, who
                      composes it outright and whose popup opens onto it
        --ledger      print every registered line as a row: line, tier,
                      must-say, claims, promises (tab-separated)

  wedge-check
      Stops four runs on purpose and reads back what the parent could say
      about each. A run reaped at a ceiling is the one red nobody can
      arrange by waiting for it, and the account it leaves is worth only
      what it says when one happens. So one run is held past its own exit
      with no deadline thread (which leaves no wedge.txt, the shape
      observed), one is held with the thread up (both records, agreeing),
      one has its verb's completion withheld (its loop answers its
      watchdog), and one is held as the first while the parent's own look
      at its threads stalls (the look is ended at its own ceiling, the app
      still reaped and the run reported). Each must fail, and is checked
      for the sentences the next occurrence will be read from — the
      stations it reached above all, the half that does not need the
      process to still be answering. The held cases must have completed
      the act and saved its picture first; a watchdog abort reaching the
      same exit station is not accepted.
      options:
        --no-build    reuse the release the gate's block already built

  shot-check
      Lands a read in a picture on purpose and reads the saved pictures
      back. A picture is one claim — the page settled as the grab answered
      — and a read landing in it is retaken from its source, not from the
      texture the read was drawn into; whether it is cannot be told from
      the run's own lines. So commit-menu is driven with
      `--fault-owed-at-shot` seven times: the window's picture and the
      scene's (a mirror's texture), each with the read before the grab, as
      the grab answers, and never, a mark drawn into the content while it
      stands; and the scene's again under the name of the page's own
      `records`, retaken only while a picture waits that read out. Each
      run must pass, say its attempts' lines (`picture … attempt=`), and
      leave the mark only in pictures of the two runs with no read at all.
      options:
        --no-build    reuse the release the gate's block already built

  records-check
      Holds a write's records back from the command log and reads what the
      completion waited for. A write's answer reaches the tab and its
      commands reach the log on two feeds that drain in no order of their
      own, and a run that passed cannot say which came first; so the log's
      records are held from the write's first asked command
      (`--fault-hold-records`) until a wait stands on them alone. Three
      registered lines — a push that lands, one refused into a report, and
      a tag push refused in git's words after three commands, which raises
      the log — must each pass, say the release came while held and before
      the completion, complete on the write's records, and stand the log's
      rows in the completion's walk where the log is up.
      options:
        --no-build    reuse the release the gate's block already built

  corpus [--force] [--path <dir>] [--against <repo>] [--worktrees <n>]
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
      neither memory nor fps honestly) on one named screen, the PGG_AUTO_*
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
      PGG_MEM_REPORT=1, then prints the largest `mem report` line the run
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

  linux [--rebuild] [--shell] [--stage core|app] [--runner <name>
        [--container <name> --step <mark>]] <command…>
      Run a command against this checkout on Ubuntu, in a container built
      from ci/linux/Dockerfile — by WSL's own wslc on Windows (WSL 3.0 or
      later, no Docker Desktop), by docker elsewhere. On Linux it skips
      the container and runs the command where it stands.
        cargo xtask linux test -p platitude-core --test it
        cargo xtask linux verify-ui commit --preset basic
        cargo xtask linux bare
        cargo xtask linux --container <name> --step <mark> stop
      A cargo command goes to cargo; an xtask verb goes to cargo xtask.
      `--runner <name>` starts an xtask verb from the copy a gate
      prepared; with `--container <name> --step <mark>` beside it the verb
      goes into the gate's own container by the engine's exec, every
      process of it carrying the mark. `stop` ends what still carries a
      mark in that container — what the gate does at a ceiling, by hand —
      and says what went and what would not.
      `runner <name> --gate <pid>` is the gate's own step and not one to
      type: it builds in the checkout's volume, so it runs only when the
      pid it names is the live gate of this tree, and one held up past its
      gate cannot reach a later run's copy.
      `bare` is neither a cargo command nor a verb, and stays in the
      container on Linux too: it builds the release workspace-wide and
      starts it on an Ubuntu carrying only what a package would declare,
      the only check that the thing runs somewhere it was not built;
      `bare --discover` works that package list out afresh, whatever the
      one that stands says.
      An engine that gives no answer is not an answer: what is asked of it
      is asked under a ceiling, and a line that cannot learn whether its
      image is built stops there and builds nothing. The first line to
      meet a silence keeps what stood on the host — the engine's
      processes, its session process's threads and memory, the disks' last
      events — under pgg-engine in the primary checkout's target/tmp, and
      says where; `witness` takes the same by hand, for a wslc seen
      hanging anywhere else. Take it BEFORE anything ends the session
      (`wslc system session terminate`, ending wslcsession.exe, a reboot):
      the README it leaves says what an administrator can add.
        cargo xtask linux witness
      Which image it runs in follows what the command needs: core is
      Ubuntu and the toolchain, app adds Qt, a software GL stack and the
      fonts デザイン規約 names for Ubuntu. The build directory is a volume
      of the engine's, so this target/ is untouched, and a verify-ui run
      is handed a host directory to leave its screenshot in.
      options:
        --rebuild        build the image again even if one already matches
        --shell          open a shell in the container instead
        --stage <name>   core or app, when the guess is not the one wanted

  land [<branch>]
      Put a branch on main — the one sanctioned way (CLAUDE.md Git 運用;
      the land skill). The pre-shell hook lets it through only on the
      permit the user's latest message opened by typing /land, and the
      fast-forward is what spends it: one message, one landing that moved
      main; a landing that stops short spends nothing. In the branch's own
      worktree: rebases the branch onto main when it is behind (a rebase
      that stops is walked back), runs `gate` there (cached steps are not
      paid twice), and only then fast-forwards main — in the primary
      checkout when it sits on main, or on the bare ref (reattaching a
      detached primary) when main is checked out nowhere; the
      reference-transaction hook checks the stamp on the way. History
      stays linear and a landed seat stands at main's tip. Refuses a
      branch checked out nowhere, a dirty seat, a seat that holds main, a
      seat that is not the head of the line of landings (`land line`
      below), and — before anything moves, and again just before main does
      — a seat that put runs on the board after the user's latest message
      (the one the session's permit records): what went up after the ask
      is UI the user has not approved, so the landing takes them off and
      leaves the permit waiting for the user's next message, and the
      session asks for that approval and lands on the next /land. Reads
      the permit again before the rebase and just before main moves: a
      landing run in the background outlives the user's next message,
      which closes the permit, so the landing stops there with nothing
      moved. A landing that stops once it has taken its turn — a rebase
      that stopped, a red gate, runs awaiting approval found after the
      gate, a closed permit, a refused fast-forward — leaves the line of
      landings, and the user's next /land puts the seat back at the end;
      so does a branch with nothing to land, whose /land ends there with
      the permit unspent. A refusal ahead of the turn (a dirty seat, a
      branch checked out nowhere, a seat behind the head) is the session's
      to put right and land again.
      A landing that moved a version the shipped build is made from (Qt's
      pin, the toolchain, a crate the app's binary resolves) tags main
      before and after it (deps/<date>/before, …/after) and pushes the
      pair where main goes (internal-docs/反映前テストの機械化.md
      §版を動かした land のタグ).
      Bare `land` from a seat lands the seat's own branch. Once main has
      moved it clears what the session running it made: the branches its
      git created (the reference-transaction hook writes each one's maker
      down), the branch of the tree the app made for the session to start
      in (written down as the session starts), and the trees, seats aside,
      that have one of them out — each when main, or the branch as it
      stood before its rebase, holds all it holds. A tree with uncommitted
      files, a lock, work of its own or a process in it stays, and is
      named with every branch of the session's that stays. A tree the
      session started in that a process still works in is left behind —
      the terminal tab the app opens there outlasts the session's work —
      and any later landing judges it the same way and takes it once it
      can. Nothing else another session, the app or the user made is
      judged or named.
      The seat's claim goes back to the roster with it, whoever's it was:
      the tree is empty at main's tip, so the letter is free for whoever
      asks next (`seat`). A lock written by hand stays.

  land line
      The line of landings and this session's place in it. The prompt hook
      puts a seat in the line when the user's message is /land
      (`.permits/<session>.line` beside the permit) and takes it out on
      any other message; a seat stands in the line while its permit is
      open, it has not left, and no other conversation's claim is on its
      seat (its own is known by session id or by the desktop app's
      session; a seat nobody claims contradicts nothing). A conversation
      that goes on under a new session id (an interrupted prompt edited,
      /clear) takes its old id's place away at its first message, and
      `seat release` ends the place with the claim. Oldest /land first, to
      the millisecond, never overtaken: the head lands, the next prepares,
      and the rest wait. For the next, it says what the head's landing
      changes among the rule and design documents — a stop (CLAUDE.md, the
      land / review-points / verify-ui / gate SKILL.md: prepare nothing),
      paths to read under .claude/ and internal-docs/, or that the diff
      could not be read — as the land skill's stages go.

  land wait next
  land wait head
      Wait until this seat is next in the line (or already the head), or
      until it is the head, then say the place and what that stage does.
      Spends nothing meanwhile — a look at the line every few seconds — so
      the session runs it in the background and ends its turn; the
      process's end is the wake-up. Ends with an error the moment the seat
      is not in the line: its permit closed, it left, or its /land was
      typed outside a seat; a line it cannot read (git not answering) is
      read again for a minute before that ends it too. A head the awake
      holder found nothing happening for, for two hours after the seat
      reached the head, is let go by whoever reads the line next.

  land leave <why>
      This seat leaves the line of landings, for what stopped it: a
      landing it found it cannot make without the user's word, at any
      stage — the next seat's own code needing more than plain fixes, the
      head's review raising a question, a red unrelated to the diff,
      pictures awaiting approval — before the user is asked. A landing
      that stops past its turn, or has nothing to land, leaves on its own.
      The user's next /land puts the seat back at the end; nothing else
      does.

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
      landing (`budget`, and it says so when it waited); the window itself
      holds none of it.

  seat [release [<letter>]|takeover <letter>]
      Hands this session a worktree seat, and takes no argument: the
      letter is the answer (CLAUDE.md ビルド・テスト). Letters are tried
      until a lock is created on one (git's own lock file, made only where
      none stands), so the seat comes back already claimed: the lock alone
      decides between two sessions, and a letter picked off a survey is
      one the other session can read the same way. A letter taken from the
      roster comes back ready for a fresh stretch: claimed, with nothing
      uncommitted, on its own branch at main's tip, its board runs swept,
      created if the roster never made it. The tree this session already
      stands in comes back instead, as it stands, branch and pictures
      alike, whether its claim came off by landing or by a release. Asked
      from the checkout, the letter the session's transcript last put it
      in is asked for first, and the other free letters go quietest first;
      a letter somebody else's claim is on is passed over either way.
      Asking twice gives the same seat. It prints the path EnterWorktree
      wants, the only one the entry hook lets through. When every letter
      is held or carries work it fails, names what stands in the way, and
      that is where the session stops — seats are not added past f, and a
      letter changes hands only on the user's word.
      A claim is a conversation's, and no process is asked about it: the
      app restarting under a session, the machine sleeping (the SessionEnd
      every open conversation is handed), a tab left open — none of it
      lifts a claim. Nor does a new session id: a claim names the desktop
      app's session too, under which a prompt edited after an interrupt,
      or a /clear, goes on with a new id that renames the claim at its
      next touch. Three things do lift one. `land` hands the seat back
      when the branch reaches main, whoever's claim was on it; a session
      that goes on working in the tree takes it again at its next edit — a
      picture takes nothing, and none goes up from a seat the session does
      not hold, nor does a compaction or a resume, which carry a
      conversation on rather than begin one there.
      `seat release [<letter>]` hands a seat back without landing and
      names what it still carries; the letter is needed only when the
      session holds more than one. And `seat takeover <letter>`, behind
      PGG_ALLOW_TAKEOVER=1, moves the letter to this session whoever holds
      it — a live session, one that is gone, nobody — with the tree as it
      stands: nothing moved to main's tip, the board's pictures kept, a
      tree that went away grown back on the letter's own branch. The
      pre-shell hook lets that verb through only behind the flag, which a
      session writes for an instruction that asked and for nothing else.
      Freeing a letter somebody else holds is a takeover followed by a
      release; a person's own `git worktree lock` is nobody's to take, and
      comes off by hand.

  seats
      Where the six worktree seats a-f stand right now, one line each:
      branch, whether HEAD sits at main's tip, commits ahead of main
      (main..HEAD), uncommitted changes (status --porcelain lines), and
      how long since the seat's own index was written. For reading how the
      roster stands — `seat` is what sits down, and nothing here is a
      letter to choose from. A claimed seat says whose it is past the
      columns, session and pid, and that is all a reader is told: whether
      the conversation behind it goes on is the user's to know, and a
      letter the user wants back changes hands with `seat takeover`. A
      letter with a branch but no tree says what that branch still carries
      — work nothing can reach until a takeover grows the tree back. Ends
      with how to read the columns.

  shots <add|crop|diff|prune|open|list|path>
      The shot board: pictures land on a page that magnifies them to exact
      whole ratios without smoothing. A chat pane fits a picture to its
      width and cannot zoom it, so a 1440x900 window is unreadable there
      whatever size it is shot at. `verify-ui` puts its own pair on the
      board as it goes, so `add` is for a crop or a comparison sheet made
      by hand. All six seats write to one board beside the primary
      checkout's .git (.shots/, gitignored), and every run carries the
      seat that took it — a picture from another tree proves nothing about
      this one, which is also why `prune` reaches this seat's runs and no
      others unless told to. `open` shows the board, `list` its runs and
      `path` where the page is; `shots` alone prints every subcommand's
      options.

      `crop <png> --at <x>:<y>:<width>:<height> [--scale <n>]` cuts a
      region out and magnifies it a whole number of times (three by
      default) without smoothing, beside the picture it came from — for
      reading the file itself, where there is nothing to zoom with; a
      fractional ratio would interpolate the pixel a crop is read for.

      `diff <before.png> <after.png>` draws where two pictures of one
      place differ — the after in grey, lighter pixels green, darker ones
      red, each place framed in yellow — for the move of a pixel or two
      that two pictures side by side do not show.

      A before/after goes on with `add --before <png> --after <png>`,
      which puts the two on one view side by side under one magnifier, as
      one run: read one after the other, the difference is whatever the
      reader remembered.

      The board keeps itself to what is being looked at now
      (shots/sweep.rs): a picture retaken replaces the one before it, and
      a seat's runs go when its work does (its branch lands, or the seat
      is handed to fresh work). A session ending removes none of the runs.
      `prune` reaches the rest — an approach abandoned under a name nobody
      retakes (--label), or a seat whose session is long gone.

  film <in|out|shoot|bake|changes>
      Behaviour as an animation, for the board (film skill). A scene file
      says what the frames show and what the hand does; `shoot` plays it
      into the headless window with real input events — hover is Qt's own
      — and grabs the views, the hand and a caption band every tick into
      target/film/<scene>/<side>/. Each side of a comparison is shot from
      its own tree (main, or the commit before a fix, against the branch)
      from one repository the scene builds once and copies, since a preset
      can differ between two trees. The verb that plays it is not the
      harness's own: `in` puts it into this tree and `out` takes it out by
      the same substitutions backwards, and a commit is refused while it
      stands. `shoot` takes --side (now unless told), --no-build and
      --fresh-base. `bake` (--sides to choose) puts the sides on one clock
      — the setup cut, each side counted from its first frame after it,
      each as long as the longest — as one APNG each, made of the frames
      themselves, never compressed again, and names the last frame under
      each caption to read. `changes` (--side) says when a side's picture
      changed and where.

  awake [log on|off]
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

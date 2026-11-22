//! The usage page's pre-merge checks: the gate and the always-steps it runs, the
//! build-directory sweep, and the checks a session runs between gates.

// No `"\` continuation on the opening line (see `demo_repo`).
pub(super) const ENTRIES: &str =
    "  gate [--related | --host-only] [--whole] [--fresh] [--keep-going] [--dry-run]
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
      (structure, waits, versions, docs, wrap, relied-messages check,
      verbs, fmt) run every time and first; a build input that changed
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
        --related     the daily tier, before a change is reported: the
                      host side, and of the verbs only the lines that show
                      a QML component the branch changed (a singleton, by
                      the components that read it), the rows the tables
                      owe and --verb; a Rust or harness change runs its
                      tests and no verb. Stamps its steps, never the
                      commit: the pre-merge gate pays only the rest
        --host-only   no container; stamps the host half
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
                [--main <ref>]
                                 the graph itself: what a change reaches,
                                 how a file got there (--why), what the
                                 last n commits would have owed
                                 (--history); --show adds each file's
                                 readers and reads; --main is the ref the
                                 branch's change is read against
                                 (default main)

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
      whatever the lines said. Only build/<package>/<hash>/ (one unit
      each; a lifted bin's is every unit its link stood on as a line
      ended) and incremental/ under debug/, release/ and shipped/ are
      swept, and the flat deps/, .fingerprint/ and build/<unit>-<hash>/
      an older cargo writes go whole — not the hook's own target/hooks
      (in use at unpredictable moments) or anything under target/ that
      is not cargo's. A profile
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
      carries #[expect(clippy::too_many_lines)] until it is cut up. Nine
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
      whole. Structure only: how wide a line runs is `wrap`'s, and how a
      sentence is built the writer's. The Write hook says the same about the one
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

  wrap [--fix] [<file>...]
      The prose of every markdown file git tracks or would add filled to
      120 columns, an East Asian wide character counted as two, and every
      file ending on a line end. Left as they stand: vendor/ (the
      patched-in crates' own documents) and a file with a carriage return
      or a tab in an indent, whole; .claude/rules-refs and verify-ui's
      verbs.md, line by line (one entry to a line, for the Grep that
      returns it whole), which still end on a line end. Only paragraphs
      are prose — blocks are read as CommonMark and GFM's tables read
      them — and only their line ends move, where what is drawn stays the
      same: the CSS drafts' segment break rule drops a line end between
      two Japanese characters and draws any other as a space, judged by
      the characters drawn either side of it. Where those cannot be told
      (an emphasis delimiter that may be literal, a tag, a bracket) the
      line end stays where it is. Code spans, tags, comments, links,
      bracketed text and bare URLs are never split, and no line opens on
      a block's marker. Code, display math, tables, headings, HTML, block
      quotes, front matter and a paragraph that could hold a hard break
      stay byte for byte. An always-step of `gate`; the Write hook says
      the same about the file an edit just landed in. Without --fix it
      names the paragraphs that are not filled and the files that do not
      end on a line end, and fails; files named narrow it to them.
      options:
        --fix         write the filled paragraphs and the line ends

  qmltest [<tst_name>...] | qmltest --together
      The QtTest files under crates/platitude-app/tests/qml, run through
      Qt's own qmltestrunner (offscreen): what only QML can be asked, such
      as whether a Canvas that owes the screenshot a paint has painted.
      The product's QML is staged whole under target/qmltest with its
      shipped qmldirs, ui/ and its modules of singletons in the
      directories under it, because `import platitude.ui` and
      `import platitude.ui.<name>` resolve by directory name; the
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
      (`cargo install cargo-deny`), and says so when it is
      missing. `gate` runs this when deny.toml, a manifest or another
      build input (the lock, the toolchain, cargo's configuration, a
      vendored crate) moved — every way the closure can change.

";

//! The usage page's runs of the app outside a verb: replays of recorded
//! lines, the shipped build and its package, and the checks over the
//! verify-ui harness itself.

// No `"\` continuation on the opening line (see `demo_repo`).
pub(super) const ENTRIES: &str =
    "  replay [--tier <fonts|premerge|whole>] [--line '<verify-ui line>']...
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
      (crates/platitude-app/src/auto, the names it compares _PGG_AUTO_ACT
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

";

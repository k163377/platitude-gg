//! The `verify-ui` section: the options every run takes, and a paragraph for
//! each verb that cannot be judged the ordinary way — by its screenshot and
//! by git having refused nothing. Kept apart from the runner's own paragraphs
//! for the reason the preset list is.

// No `"\` continuation on the opening line (see `demo_repo`).
pub(super) const VERBS: &str = "  verify-ui <verb> [arg] [options]
      perf none|details|diff|scroll|scroll-none|font-walk --preset perf checks
      the performance driver's visible state; offscreen timings are not
      benchmarks.
      Build the app (release), run it headless (offscreen QPA) against a
      repository, fire PGG_AUTO_ACT=<verb> / PGG_AUTO_ACT_ARG=<arg>, and
      judge the run by its 'screenshot saved=true' stderr line and by
      whether git refused any of the writes it made.
      The verb `solo` is run with this process holding the config
      directory's lock, so the app it starts is a second instance; that
      run is judged on reporting that it was turned away as well.
      The verb `details-fit` is judged on its own report too: a details
      pane whose column runs off the right of the window frames exactly
      like one that fits, so the picture cannot answer it. So is
      `window-fill`: a maximised window fills the screen, leaving no
      desktop beside it for an unpainted edge to show against. And so is
      `commands-clear`, where the panel the reader emptied goes down with
      the press and what is judged is what it left behind: a window with
      no panel in it, and a quiet mark in a corner of the band. Its
      neighbour `commands-copy` is judged on its report for the opposite
      reason: Ctrl+C changes nothing on screen at all, so what left for
      the clipboard is read back and counted against the rows.
      `details-grow` and `wip-grow` (each with a `-squeeze` twin) pull the
      description box's grip past everything and are judged the same way,
      on what sits under the box still being inside the pane. The details
      argument is the row, and the row with a short message — where the
      grip is not offered — is the other half of that pair; the commit
      editor's argument is the description to type, and a long one is
      written for it when the run brings none, since that box starts
      empty.
      options:
        --repo <dir>      run against this repository
        --preset <name>   or against a fresh demo repo (default: basic)
                          Both repeat: one tab per repository, in order.
        --no-build        reuse the existing release binary
        --select          also set PGG_AUTO_SELECT=1
        --scroll-to <where>
                          also set PGG_SCROLL_TO=top|bottom|nav-bottom: park
                          the graph — or the sidebar's branch list — at one
                          end once the page has stopped arriving
        --system-title-bar
                          also set PGG_SYSTEM_TITLE_BAR=1: the window shape
                          mac and Linux come up in, asked for from a
                          machine that folds the band into the title bar
        --watchdog-ms <n> the backstop under a run whose completion never
                          arrives; never chooses the shot, and never an
                          assertion that a verb is quick (default 600000).
                          Lower it only to diagnose the wait itself
        --fault-hang <station>
                          hold the app at that station for good, by the
                          word the trail names it with: starting,
                          event-loop, left-event-loop, settings-flush,
                          tabs-closing, writes-joining, runtime-stopping,
                          run-dir-clearing, hub-down, qt-tearing-down,
                          exiting. A run that cannot pass; what
                          `wedge-check` drives.
        --fault-no-deadline
                          start it with no deadline thread, so it leaves
                          no wedge.txt however it is stopped — the shape a
                          run held past `exiting` has anyway, since the
                          exit ends every other thread before the loaded
                          libraries are given their detach. With
                          `--fault-hang`, the run is reaped the moment the
                          trail says it reached that station (the verdict
                          reads HELD AT); the ceiling is only the backstop
                          behind that
        --fault-hold-act  swallow the verb's completion, so the act runs,
                          the loop goes on turning and only the ceiling
                          ends the run. The third shape `wedge-check`
                          drives, and the one a short ceiling cannot make
                          on its own: a ceiling that outruns a completion
                          here loses to it on a busier machine
        --fault-stall-look
                          stall the runner's own look at a run it reaps:
                          the thread listing is a process that never
                          answers, ended at a short ceiling of its own.
                          The fourth shape `wedge-check` drives — the
                          diagnostic ran out of time and the app was still
                          reaped and reported
        --shot-dir <dir>  screenshot directory (default: temp, kept)
        --config-dir <d>  settings.toml / state.toml directory. Fresh per
                          run by default; name one to carry what a run
                          wrote into the next one.
        --restore         open the tabs the config directory remembers
                          instead of a named repository
        --allow-write-failure
                          a write git refused is what this verb shows, so
                          it does not sink the run (delete-branch-refused,
                          delete-branch-go — whose first `--delete` is the
                          one git turns down —, commands-fail,
                          commands-fail-shut, commands-clear, fetch-fail,
                          fetch-recover, fetch-recover-held, fetch-resume,
                          push-retry, remote-refused, tag-refused,
                          push-outdated,
                          commit-refused, notice-over-diff, stale-part —
                          which refuses a write of its own, without git —,
                          rename-taken, fold-across-merge, fold-off-branch,
                          fold-first-commit, fold-unfetched-base,
                          drop-last-commit, band-actions-alert, push
                          --preset unreachable, and publish-new-go given
                          a URL nothing can reach — that preset cannot
                          stand in for it, since a branch already pushed
                          with an upstream has no first push left to
                          publish). A replay that stops part-way is not
                          among them any more: git left it standing,
                          which is a landing rather than a failed write.
        --old-git <ver>   run the app against a git that answers --version
                          with <ver> and passes everything else to the real
                          one, so an installation older than the supported
                          minimum can be photographed working. Implied by
                          the verbs old-git and old-git-card.
        --other-git <ver> stand a second git beside the pictures, answering
                          <ver>, and tell the app where it is — a git that
                          answers and is *not* the one the run is on, which
                          is the one state the settings screen's git
                          chapter grows a button for and the one no path
                          names on both a desk and a container. Takes the
                          place of --old-git rather than joining it, and
                          the runs about it (settings-git-leave,
                          settings-git-path other) imply one.
      The open-refused verbs (open-not-a-repo, open-bare, the -retry /
      -cancel ways out, and open-fail-tab for the road that keeps its
      tab) make their own folder when no argument names one: what they
      are about is a folder no repository can be.
      The verb `tab-widths` makes its own repositories too — its argument
      is how many (1..=16, default 8), and it names them at a spread of
      lengths, because a strip of equally-named tabs cannot show which
      of them gave way and which were left at their own width.
      `tab-mark` opens the same strip and puts the pointer on the tab its
      argument names (default 1), which is the only way a headless run
      reaches the half of the `✕` rule that hover answers.
      `tab-drag` opens four of them and carries one across the strip,
      which is the other gesture no headless run has a pointer for. Its
      argument is `<from>:<to>` (default `3:1`), the two places by the
      order the tabs came up in. `tab-hold` opens the same four and takes
      one up without setting it down (argument: which tab, default 0),
      because a tab drawn away from its own row is what the settled strip
      cannot show. `tab-edge` opens twelve, puts the window down on its
      floor so the strip has to overflow, and holds a tab past the end of
      it until the strip has travelled the whole way.
      `tab-pin` and `tab-pin-go` put the window on the same floor with
      the same twelve tabs and are read as a pair: the first sends the
      strip to the end furthest from the tab in front and photographs the
      stand-in that rides the edge that tab went out of, the second
      presses that stand-in and waits out the travel it starts. Their
      argument is which tab is in front (default 0, which puts the
      stand-in on the left edge). Twelve rather than eight because the
      run has to exist at all on both machines — eight tabs stand at
      whatever the run divides into and fill the wider Linux band
      exactly, leaving nothing to travel in.
      `tab-open-go` stages that same strip and then opens a thirteenth
      repository into it, which is the other road the band travels by: a
      repository asked for is one the reader means to see, so the strip
      goes to the seat it lands in. It builds that repository itself when
      no argument names one — the twelve handed to the run are opened at
      startup, and a tab already in the strip cannot arrive in it.
      `badges`, `badges-hover` and `badges-hover-early` take a window
      width — a number, or `floor` — and may take a strip off that same
      ladder after it, as `<width>:<tabs>`. The band's shortfall is
      shared between the tab strip and the state group, so where the
      group gives its words up moves with how many tabs stand beside it;
      a run against one repository reaches the floor a hand can drag to
      with the words still on. The `--preset` goes on the tab in front,
      which is the only one the band shows a state for, so only one may
      be named. The last of the three differs from the second in one
      thing: it puts its stand-in pointer down before the band has
      placed the group, which is the order the card is only reached in
      by being asked for a second time.
      `tab-name` opens a strip of a different shape: four repositories
      called `repo` — two of them under a parent they also share — and
      one nobody shares at all, which is what a name that has to grow to
      tell itself apart needs standing beside it. It points at the tab
      its argument names (default 0) and reads the hover as well.
      `identity` and `identity-half` are read as a pair, and are the only
      verbs whose write would land outside a demo repository. Every run is
      started on a git configuration of its own (GIT_CONFIG_GLOBAL in the
      shot directory, a working directory outside every checkout to read
      it in) and never sees the one on this machine; what these two seed
      into it is the state the screen begins from, which for everyone else
      is a fixture identity that keeps the screen from opening at all. The
      argument is what to type, as `<name>|<email>`. `identity` fills the
      screen and leaves it; `identity-half` saves against a seed whose
      user.email holds two values, which git refuses to overwrite with one
      — so the name lands and the address does not, the same way a lost
      configuration lock leaves it. Both are judged on their own report:
      a screen still standing because the save did not take frames exactly
      like one nobody has answered yet.

";

//! The `verify-ui` section: the options every run takes, and a paragraph for
//! each verb that cannot be judged the ordinary way — by its screenshot and
//! by git having refused nothing. Kept apart from the runner's own paragraphs
//! for the reason the preset list is.

// No `"\` continuation on the opening line (see `demo_repo`).
pub(super) const VERBS: &str = "  verify-ui <verb> [arg] [options]
      Build the app (release), run it headless (offscreen QPA) against a
      repository, fire PG_AUTO_ACT=<verb> / PG_AUTO_ACT_ARG=<arg>, and
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
        --select          also set PG_AUTO_SELECT=1
        --watchdog-ms <n> maximum run time; never chooses the shot (default 120000)
        --shot-dir <dir>  screenshot directory (default: temp, kept)
        --config-dir <d>  settings.toml / state.toml directory. Fresh per
                          run by default; name one to carry what a run
                          wrote into the next one.
        --restore         open the tabs the config directory remembers
                          instead of a named repository
        --allow-write-failure
                          a write git refused is what this verb shows, so
                          it does not sink the run (delete-branch-refused,
                          commands-fail, commands-clear, fetch-fail,
                          fetch-recover, fetch-resume, push-retry,
                          remote-refused, tag-refused, push-outdated,
                          commit-refused, notice-over-diff,
                          band-actions-alert, and push / publish-new-go
                          against an unreachable
                          remote). A replay that stops part-way is not
                          among them any more: git left it standing,
                          which is a landing rather than a failed write.
        --old-git <ver>   run the app against a git that answers --version
                          with <ver> and passes everything else to the real
                          one, so an installation older than the supported
                          minimum can be photographed working. Implied by
                          the verbs old-git and old-git-card.
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
      cannot show. `tab-edge` opens eight, puts the window down on its
      floor so the strip has to overflow, and holds a tab past the end of
      it until the strip has travelled the whole way.
      `tab-name` opens a strip of a different shape: four repositories
      called `repo` — two of them under a parent they also share — and
      one nobody shares at all, which is what a name that has to grow to
      tell itself apart needs standing beside it. It points at the tab
      its argument names (default 0) and reads the hover as well.
      `identity` and `identity-half` are read as a pair, and are the only
      verbs whose write would land outside a demo repository — so they are
      handed a git configuration of their own (GIT_CONFIG_GLOBAL in the
      shot directory) and never see the one on this machine. The argument
      is what to type, as `<name>|<email>`. `identity` fills the screen and
      leaves it; `identity-half` saves against a configuration whose
      user.email holds two values, which git refuses to overwrite with one
      — so the name lands and the address does not, the same way a lost
      configuration lock leaves it. Both are judged on their own report:
      a screen still standing because the save did not take frames exactly
      like one nobody has answered yet.

";

//! The window and its strip of tabs: what it fills, the floor it stops
//! at, which tab was carried where, and the band that gives way when it
//! is crowded.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // `edge=` rides along in the report as diagnosis: whether an edge
    // lands off the screen is a question for a real monitor, which the
    // offscreen platform lacks.
    Verb {
        name: "window-fill",
        when: &[],
        plain: "window_fill fills=true",
    },
    // A window let past its floor frames like one held at it — the pane
    // that went past is outside the frame.
    Verb {
        name: "window-floor",
        when: &[],
        plain: "window_floor fits=true",
    },
    // Which tab went: every demo tree is called `repo`, so the strip
    // photographs the same either way. `gone=` is the pressed tab's own
    // path, asked of the strip once it caught up with the model — it also
    // catches a report sent before any tab opened. How many are left is
    // the run's: the verb takes as many repositories as it is given.
    Verb {
        name: "middle-close",
        when: &[],
        plain: "middle_close gone=true",
    },
    // The close that arrived while git was writing, turned away and said
    // out loud. `busy=true` is the write actually in flight under the
    // photograph — the dialog could as well stand over an idle repository.
    Verb {
        name: "quit-waits",
        when: &[],
        plain: "quit_wait dialog=true window=true busy=true",
    },
    // The lock that takes no answer, and the write the quit was asked over
    // landing anyway. `vetoes=2` is the door held a second time;
    // `escape=false` is the policy Qt itself consults, so this goes red if
    // `CloseOnEscape` comes back. `landed=true`: the hook held the commit,
    // the close was pressed, and nothing was killed.
    Verb {
        name: "quit-locked",
        when: &[],
        plain: "quit_lock dialog=true window=true escape=false vetoes=2 landed=true",
    },
    // The same close over a save the application itself asked for (the
    // identity the dialog took), held by the run until shutdown joins it.
    // `held=1` is the save standing at the hold when the close landed,
    // `vetoes=1` the gate turning it away for that save with no session
    // writing. That the exit then waited for the save is read off the
    // run's own gitconfig after the process ends
    // (`super::shim::held_save_landed`).
    Verb {
        name: "quit-save-held",
        when: &[],
        plain: "quit_save dialog=true window=true vetoes=1 busy=true held=1",
    },
    // Another copy's uncommitted row standing this tab in that copy.
    // Whether the strip grew a second tab (`grew=`) or the tab it had
    // moved in (`stood=`) are the two answers this verb tells apart
    // (デザイン規約 §タブの所作). `kept=`: the page that asked is the page
    // that arrived, its graph never emptied — the copy switch swaps the
    // session under a standing page and replaces the rows in one go
    // (`Hub::restand_tab`). `drew=`: another copy's uncommitted row becomes
    // this tree's own, so the graph is swapped, where a copy with the same
    // picture swaps nothing (`worktree-stand`); both are read against the
    // record of the graph the new session is handed (`session::DrawnGraph`).
    Verb {
        name: "carried-open",
        when: &[],
        plain: "carried_open tabs=1 grew=false stood=true drew=true kept=true",
    },
    // The same landing by the left menu's door: one tab before and after,
    // and the copy the row named is the one the session opened. `log=` is
    // the second thing the page keeps — the command log is this window's
    // record, and the arrived copy numbers its commands from one without
    // touching the rows the left copy put there (`CommandMsg::run`).
    Verb {
        name: "worktree-stand",
        when: &[],
        plain: "worktree_stand tabs=1 grew=false stood=true drew=false kept=true log=true",
    },
    // Away and back: `empty=` is the arrived copy not wearing the words
    // typed in the one left, `back=` those words in the box again, `wip=`
    // the pane holding them being the one on screen.
    Verb {
        name: "copy-draft",
        when: &[],
        plain: "copy_draft empty=true back=true wip=true kept=true",
    },
    // The path under the hand on such a tab. `copy=`: the words name the
    // copy the tab stands in, not the repository it is named after — they
    // differ only on a tab that has been stood somewhere. `tip=` is the
    // hover having opened.
    Verb {
        name: "worktree-tip",
        when: &[],
        plain: "worktree_tip tip=true copy=true native=false",
    },
    // The same landing, left and come back to. `kept=` is that copy still
    // drawn on the tab, read off the strip because the tab left has no page
    // to ask. `front=` is which of the two landings the picture is of;
    // `stood=` the copy come back to, which only the return can answer.
    Verb {
        name: "worktree-kept",
        when: &[(
            Arg::Has("away"),
            "worktree_kept tabs=2 kept=true front=false",
        )],
        plain: "worktree_kept tabs=2 kept=true front=true stood=true",
    },
    // What a tab switch carries and drops, in one line because neither
    // half means anything alone. `sessions=1` with two tabs open is the
    // release itself — the tab left behind holds no repository. `folded=` /
    // `log=` are the layout left on the last tab, found on the next;
    // `empty=` / `back=` / `wip=` are `copy-draft`'s, with the page thrown
    // away and read again in between.
    Verb {
        name: "tab-carry",
        when: &[],
        plain: "sessions=1 folded=true log=true empty=true back=true wip=true",
    },
    // A strip of namesakes and the path the hand asks for. `unique=`: no
    // two tabs read alike. `native=false`: a path on screen uses `/`
    // (規約 §パスの区切り), so a backslash in a name or its hover is
    // Windows spelling one its own way. `tip=` is the hover, which comes
    // out on the overlay.
    Verb {
        name: "tab-name",
        when: &[],
        plain: "unique=true native=false tip=true",
    },
    // Which tab was carried and where it rested: a check that the order
    // merely changed passes with any two tabs swapped, and a carry that
    // never took hold photographs the order it started in. `moved=` is
    // the carried tab's own path, found where it was asked for.
    Verb {
        name: "tab-drag",
        when: &[],
        plain: "tab_drag moved=true",
    },
    // The other half of the gesture: a tab never drawn away from its row
    // settles like one whose rows only jumped. `lifted=` is the offset the
    // transform carries, read off the tab itself.
    Verb {
        name: "tab-hold",
        when: &[],
        plain: "tab_hold lifted=true",
    },
    // The strip travelling under a tab held past its end: judged is that
    // the tab reached the far end of an order it could not see when the
    // hand took hold — a scrolled strip frames the same either way.
    Verb {
        name: "tab-edge",
        when: &[],
        plain: "tab_edge landed=true",
    },
    // A real font's widths going in, real tabs coming out. The widths are
    // read by eye against the verb's notes; `crushed=0` is what they
    // cannot carry — a tab is as wide as its name, so a name cut to the
    // mark alone leaves every number in place and looks like a short name.
    //
    // Two runs, because there are two states and no count names either.
    // The arithmetic between them is `tests/qml/tst_tabwidths.qml`'s; only
    // a window answers that this platform's font reached the state, so
    // each run says which it is in and is judged on it — a count that
    // stopped crowding the band then fails here.
    Verb {
        name: "tab-widths",
        when: &[
            (Arg::OneOf(&["", "ample"]), "tab_widths crushed=0 cut=false"),
            (
                Arg::Is("short"),
                "tab_widths crushed=0 cut=true folded=true",
            ),
        ],
        plain: "tab_widths crushed=0",
    },
    // The stand-in for the tab in front, and the press that takes it away
    // (`tab-pin-go`), read as a pair. A front tab genuinely at the run's
    // edge frames like a stand-in for a tab nowhere on it, so `onScreen=`
    // carries both: the stand-in is wanted exactly while its row is out of
    // sight, and a pair that agrees has the rule backwards.
    //
    // `kept=true crushed=0`: the stand-in is drawn at its tab's width, so
    // what is left to get wrong is what it says in that width.
    //
    // Which edge it took is judged only where the run named the tab — the
    // first one, sent to the far end, rides the left edge. Any other
    // argument still has to have stood one up.
    Verb {
        name: "tab-pin",
        when: &[(
            Arg::OneOf(&["", "0"]),
            "tab_pin kept=true crushed=0 stood=true onScreen=false left=true",
        )],
        plain: "tab_pin kept=true crushed=0 stood=true onScreen=false",
    },
    // And the travel: `travelled=` is the strip having moved under the
    // press, `gone=` the stand-in's answer to having arrived. `crushed=0`
    // is what this picture does hold — the tab in front at the width the
    // crowded strip left it — and cannot be read for.
    Verb {
        name: "tab-pin-go",
        when: &[],
        plain: "tab_pin_go crushed=0 gone=true onScreen=true travelled=true",
    },
    // The same arrival asked for by opening a repository: the band moved,
    // the tab it moved for is whole in the run, and the stand-in stepped
    // aside.
    Verb {
        name: "tab-open-go",
        when: &[],
        plain: "tab_open_go arrived=true gone=true travelled=true",
    },
    // The reader doing the next thing after that arrival: a press anywhere
    // ends the ask, so the run narrowing under the new tab leaves the strip
    // where it was with the stand-in up. A strip that travelled after the
    // press holds the tab whole — which the reader never asked for.
    Verb {
        name: "tab-open-moved-on",
        when: &[],
        plain: "tab_open_moved_on travelled=false stood=true ask=false",
    },
    // The ☰'s card, standing. `yield=true`: the band's empty run is the
    // platform's caption, and while the card is up it has to stop being
    // that or every press there is swallowed and the card never goes down.
    Verb {
        name: "app-menu",
        when: &[],
        plain: "app_menu open=true yield=true",
    },
    // The panel standing — the verb the state matrix is photographed
    // with. `settled=true` (repository landed, nothing running) is what the
    // picture needs behind it; the rest is what it cannot be read for.
    //
    // `lit=false`: no hand is on the panel.
    //
    // `boxed=true`: every button at the panel's right end is as deep as its
    // own two lines. The branch's name writes one line for a branch
    // following nothing (`worktrees`) and for a page still reading, and a
    // button held to that depth stands its word over its frame and its
    // mark under it.
    //
    // `rule=true`: the line between the three and the find stands in the
    // step between them, clear of both frames (a pixel onto either reads as
    // that frame's edge), and ends on the frames' own top and bottom,
    // folded or not.
    //
    // `track=` is where the branch's counts came out: at the `end` of an
    // upstream longer than the name and its counts (`panel` — `main` over
    // `origin/main`), or `after` the name at their own step where the
    // name's line is the longer (`longnames`).
    Verb {
        name: "ops-panel",
        when: &[
            (
                Arg::WithPreset("panel"),
                "ops_panel settled=true lit=false boxed=true rule=true track=end",
            ),
            (
                Arg::WithPreset("longnames"),
                "ops_panel settled=true lit=false boxed=true rule=true track=after",
            ),
        ],
        plain: "ops_panel settled=true lit=false boxed=true rule=true",
    },
    // The panel's own doors. The counts ride along: a card is built from
    // listings that arrive after the tab, and an empty one frames like one
    // whose rows were left out — and `open=false` is right for a repository
    // with nowhere else to stand, a preset fault that must read as one.
    //
    // `lit=true turned=true` on every door: the name the card hangs off
    // stays washed with its chevron turned down while the card stands.
    // `yield=true` is the band's grab runs handed back to the scene while
    // the card stands (`WindowChrome.captionYielded`) — otherwise the
    // platform takes a press on them and the card stands through the
    // click meant to close it.
    Verb {
        name: "ops-stand",
        when: &[],
        plain: "ops_door open=true tier=stand lit=true turned=true yield=true",
    },
    // …and each tier, opened the way resting on the row opens it. Which
    // card stands is judged, not that one does: a tier asked for and not
    // opened leaves the card above it on screen, alike once cropped.
    Verb {
        name: "ops-stand-repos",
        when: &[],
        plain: "ops_door open=true tier=repos lit=true turned=true yield=true",
    },
    // With a filter typed into the left menu first, the tier still opens
    // while the section lists nothing: the card is a way to move, not the
    // list being read (`NavSectionModel.copyCard`).
    Verb {
        name: "ops-stand-copies",
        when: &[(
            Arg::Is("zzz"),
            "ops_door open=true tier=copies lit=true turned=true yield=true folder= repos=0 listed=0 copies=",
        )],
        plain: "ops_door open=true tier=copies lit=true turned=true yield=true",
    },
    // The branch door, which offers every local branch this repository
    // has bar the one the window is on, filed the way the left menu files
    // them (`OpsBranchMenu`).
    Verb {
        name: "ops-branch",
        when: &[],
        plain: "ops_door open=true tier=branch lit=true turned=true yield=true",
    },
    // …and a folder of it, opened the way resting on its row opens it.
    // Which folder stands is judged, for the same reason, one row per
    // folder the census runs; an argument with no row of its own is judged
    // on the tier alone.
    Verb {
        name: "ops-branch-folder",
        when: &[
            (
                Arg::Is("feature"),
                "ops_door open=true tier=folder lit=true turned=true yield=true folder=feature repos=",
            ),
            (
                Arg::Is("topic"),
                "ops_door open=true tier=folder lit=true turned=true yield=true folder=topic repos=",
            ),
        ],
        plain: "ops_door open=true tier=folder lit=true turned=true yield=true",
    },
    // A row of each card pressed, judged where the press lands — the
    // branch the panel names, the copy the tab reads, the tab in front.
    // `landed=` is the claim: a row that closes its card and reaches
    // nothing looks the same. `card=none` is the card gone with the press.
    Verb {
        name: "ops-branch-pick",
        when: &[],
        plain: "ops_pick door=branch found=true landed=true card=none",
    },
    Verb {
        name: "ops-copy-pick",
        when: &[],
        plain: "ops_pick door=copy found=true landed=true card=none",
    },
    // `named=true`: the panel names the tab it moved to in the press itself,
    // before that tab has read a thing — the names were the page's answer,
    // which waited on git, and a late name looks the same in the landing.
    Verb {
        name: "ops-repo-pick",
        when: &[],
        plain: "ops_pick door=repo found=true landed=true card=none named=true",
    },
    // The mark pressed a second time: a card reopened by that press and one
    // never closed are the same photograph, so judged is the card down with
    // the run back in the platform's hands.
    Verb {
        name: "app-menu-reclick",
        when: &[],
        plain: "app_menu open=false yield=false",
    },
    // The three states themselves. A band never crowded looks like one that
    // gave the crowd room, so the crowd is said out loud — and so is the
    // shape, which a width does not name: the same number lands in a
    // different shape on a different font. The arithmetic behind the
    // shapes is `BandStateShare`'s (`tst_bandshare.qml`); the runs here are
    // a group the band had room for and the same group narrowed by a real
    // crowd of tabs.
    //
    // The width is the run's, calibrated to the band: the strip is first
    // in the queue for what the band has (`TopBar`), so whatever else
    // stands on the row moves the width at which the group narrows.
    //
    // The claim stops before the fit: the widths showing the last shape
    // are below the floor a hand can drag the window to. The folded mark is
    // reached through the floor instead — it folds the group whatever the
    // share-out says — here in red, and in yellow by `old-git-fold`. The
    // floor itself is `window-floor`'s question.
    //
    // `floor`: `fitted=true` is the group cut to its mark's cell — a folded
    // group that kept its words' width stands empty band beside the mark,
    // read as a wider grab run. `narrowed=` is left out: the share-out
    // under the fold is the fonts' answer, not this row's.
    //
    // `shadow=true` on every row: the shape is decided on a second
    // laying-out of the band with the words kept (`TopBar`'s `bandAsked`),
    // and a cell that shadow does not mirror (the window's own buttons set
    // their width rather than asking for it) hands the words room the
    // group's cell lacks, so they run over the strip's grab run. Only a
    // band with those buttons (a Windows run) can tell.
    Verb {
        name: "badges",
        when: &[
            (
                Arg::Is("1000:6"),
                "op=true conflicts=true identity=true oldGit=false narrowed=true words=true \
                 mark=false shadow=true",
            ),
            (
                Arg::Starts("floor"),
                "words=false mark=true shadow=true tint=danger fitted=true",
            ),
        ],
        plain: "op=true conflicts=true identity=true oldGit=false narrowed=false words=true \
                mark=false shadow=true",
    },
    // The card, opened. `rows=` is which rows arrived — a card with one row
    // and one with three frame alike once cropped to the band.
    //
    // One stopped operation stands for all of them: the rows are the same
    // three and only the op row's word differs, which is the model's
    // (`worktree::tests::every_operation_that_can_stand_here_…`).
    Verb {
        name: "badges-hover",
        when: &[],
        plain: "card=true rows=op,conflicts,identity",
    },
    // The same card opened from the other order: the stand-in pointer goes
    // down before the band has placed the group, so the group arrives under
    // a hand already there. A pointer that cannot move raises the beat
    // once, so the card is owed a second asking when the row answers —
    // `badges-hover` puts the pointer on a group placed long before and
    // passes either way.
    Verb {
        name: "badges-hover-early",
        when: &[],
        plain: "card=true rows=op,conflicts,identity",
    },
    // The badge for a graph that is not this repository's history, and the
    // card line that parts its two states: an out-of-date graph is pixel for
    // pixel the current one, an emptied column looks like an opening, and
    // the badge alone cannot say which put it up. So the model's two are
    // read beside it, and the card is required open because that line is
    // where the difference is written. `tint=`: both states are `danger`
    // (規約 §状態); a warning badge is the rule not reaching the paint.
    Verb {
        name: "graph-stale",
        when: &[],
        plain: "graph_stale badge=true stopped=false stale=true tint=danger card=true",
    },
    Verb {
        name: "graph-stopped",
        when: &[],
        plain: "graph_stale badge=true stopped=true stale=false tint=danger card=true",
    },
    // Where the page lands when the pass it opened on carries every other
    // copy's row and none of its own — the arrangement the run is started
    // into, since which of the walk and the first status arrives first is
    // the scheduler's.
    //
    // `early=` / `earlyCopy=` are the page while the row was held back
    // (nothing opened, nothing stood on a copy); `wip=` is the pass
    // carrying ours landing on this window's tree. `held=true` is the hold
    // answering for itself, so a run in the ordinary order cannot pass as
    // this one. `otherTop=true`: the held-back pass led with a neighbour
    // copy's row, the row the misreading takes — without it a graph with
    // no all-zero row on top passes without asking the question. `row=` is
    // where the landing put the reader.
    Verb {
        name: "wip-landing",
        when: &[],
        plain: "wip_landing held=true otherTop=true early=false earlyCopy=false wip=true copy=false row=0",
    },
    // The landing a stopped operation owes, in the same arrangement.
    // `owed=true` is the move decided and held while the row was out; the
    // press is a replay this window started from a clean tree, so the
    // conflicts it stopped in are what the landed row is made of.
    Verb {
        name: "wip-landing-stopped",
        when: &[],
        plain: "wip_stop_landing held=true otherTop=true owed=true early=false earlyCopy=false wip=true copy=false op=REBASING lit=0",
    },
    // The panel's three actions at the last cell that still has a word in
    // it: a band shot at one width says nothing about where its shape was
    // to change.
    //
    // Run where the names are long (`--preset longnames`): what the panel
    // runs short of comes off the names first, so with `repo` on `main` the
    // actions keep every word down to the window's floor.
    // `cut=` rides along as diagnosis: how long the cut stretch lasts is
    // the installed fonts' question, and for a four-letter command it can
    // be nothing (`…` and two characters measure what `push` does).
    Verb {
        name: "band-actions",
        when: &[],
        plain: "band_actions fits=true folded=false deep=true",
    },
    // The end of that road: the words given up for the marks at the floor
    // the window has with its left list open — the width they must be
    // done by. `deep=` is every frame still two lines deep: the fold gives
    // up the word, not the depth.
    Verb {
        name: "band-actions-fold",
        when: &[],
        plain: "band_actions fits=true folded=true deep=true",
    },
    // The marks with one saying the last go failed. A `!` on a button with
    // no word has only the mark's corner to stand in, so judged is that it
    // survived the fold. The refusal is the far side's: a plain `push` into
    // a remote that moved on (`--preset longnames`, with
    // `--allow-write-failure`) — the origin has to be reachable for the go
    // to get far enough to be refused.
    Verb {
        name: "band-actions-alert",
        when: &[],
        plain: "band_actions fits=true folded=true deep=true cut=true alert=true",
    },
    // The same corner on the other mark, put there by the run of failures
    // that stops the timer. Its own line because the band's `alert=` is the
    // pair or-ed together and cannot say which button carries it.
    Verb {
        name: "band-actions-stopped",
        when: &[],
        plain: "band_stopped suspended=true turned=true alert=true",
    },
    Verb {
        name: "band-actions-none",
        when: &[],
        plain: "band_actions fits=true folded=false deep=true cut=false",
    },
    // The fourth badge. A run whose shim never reached PATH reads this
    // machine's git and wears no badge; `badge=` is the band's own reading,
    // so the whole path from `git --version` to the row is judged.
    Verb {
        name: "old-git",
        when: &[],
        plain: "old-git badge=true",
    },
    // The card it opens. `rows=` rides along unjudged: the other three rows
    // come and go with the machine (a container with no identity stands
    // one of them).
    Verb {
        name: "old-git-card",
        when: &[],
        plain: "old-git badge=true card=true",
    },
    // The band folded with nothing red standing in it: a mark that never
    // came up frames as a band with room to spare, and three dots' colour
    // is not for a cropped screenshot to settle. `tint=` names which rule
    // painted it (デザイン規約「色は最も重い状態が決める」); a red mark over a
    // lone warning is that rule failing silently. `fitted=true` as in
    // `badges`' `floor`.
    Verb {
        name: "old-git-fold",
        when: &[],
        plain: "mark=true tint=warning fitted=true",
    },
    // The picker is the platform's own window and stands in neither
    // photograph (`popups=0`), so the line is the whole evidence. It comes
    // up beside the repository already open (rules-refs/app-ui.md).
    //
    // `beside_copy=` reads the folder without knowing the machine's paths:
    // whether the picker came up in the folder the copy sits in. The plain
    // run stands in the repository's own copy, where the two folders are
    // one; `copy` stands in a linked one kept below the root
    // (`Route::NestedCopy`), where the picker leaves the copy's folder.
    Verb {
        name: "open-picker",
        when: &[(Arg::Is("copy"), "picker beside_copy=false")],
        plain: "picker beside_copy=true",
    },
];

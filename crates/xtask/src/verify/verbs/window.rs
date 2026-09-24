//! The window and its strip of tabs: what it fills, the floor it stops
//! at, which tab was carried where, and the band that gives way when it
//! is crowded.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // `edge=` rides along in that report as diagnosis: whether an
    // edge would land off the screen is a question about a real
    // monitor, and the offscreen platform has none to answer with.
    Verb {
        name: "window-fill",
        when: &[],
        plain: "window_fill fills=true",
    },
    // A window held at its floor and one let past it frame alike — the
    // picture is of the panes either way, and the one that went past
    // simply has a pane outside the frame, where a screenshot cannot
    // follow. What the floor came to is a number or it is nothing.
    Verb {
        name: "window-floor",
        when: &[],
        plain: "window_floor fits=true",
    },
    // Which tab went is the whole question, and every demo working
    // tree is called `repo`, so the strip photographs the same either
    // way. `gone=` is the pressed tab's own path, asked of the strip
    // after it caught up with the model — a run that closed the
    // neighbour, or one that reported before a tab had ever opened,
    // is caught by nothing else. How many are left is the run's: the
    // verb takes as many repositories as it is given.
    Verb {
        name: "middle-close",
        when: &[],
        plain: "middle_close gone=true",
    },
    // The close that arrived while git was writing, turned away and
    // said out loud. None of it is a picture on its own: a vetoed
    // window frames like one nobody asked to close, and the dialog in
    // the overlay could equally be standing over a repository with
    // nothing running — `busy=true` is the write actually in flight
    // under the photograph.
    Verb {
        name: "quit-waits",
        when: &[],
        plain: "quit_wait dialog=true window=true busy=true",
    },
    // The lock that takes no answer, and the write the quit was asked
    // over landing anyway. `vetoes=2` is the door tried a second time
    // and held a second time; `escape=false` is the policy Qt itself
    // consults, so the day `CloseOnEscape` comes back this goes red.
    // `landed=true` is the whole point of the wait — the hook held the
    // commit, the close was pressed, and nothing was killed — and no
    // picture can say it (a history with the commit looks like a
    // history that was always going to get it).
    Verb {
        name: "quit-locked",
        when: &[],
        plain: "quit_lock dialog=true window=true escape=false vetoes=2 landed=true",
    },
    // The same close over a save the application itself asked for — the
    // identity the dialog took — held by the run until the shutdown
    // joins it. `held=1` is the save standing at the hold when the close
    // landed, `vetoes=1` the gate turning it away for that one save with
    // no session writing. The other half of the claim, that the exit
    // then waited for the save, is not a line the app can print: it is
    // read off the run's own gitconfig once the process has ended
    // (`super::shim::held_save_landed`).
    Verb {
        name: "quit-save-held",
        when: &[],
        plain: "quit_save dialog=true window=true vetoes=1 busy=true held=1",
    },
    // What a tab switch carries and what it drops, in one line
    // because neither half means anything alone. `sessions=1` with
    // two tabs open is the release itself — the tab left behind is
    // holding no repository — and it is the one thing here no picture
    // can say. `folded=` / `log=` are the layout the reader left the
    // last tab in, found on the next one; `empty=` is that tab's own
    // commit editor, which the words stayed out of; `back=`
    // is those words still standing where they were typed, after
    // everything else on that page was thrown away and read again;
    // `wip=` is the pane holding them being the one on screen, which
    // is the half of "kept" that a report about text alone misses.
    // Another working copy's uncommitted row standing this tab in that
    // copy. **The two halves are what the picture cannot hold**: a
    // window showing one repository frames the same whether the strip
    // grew a second tab for the copy (`grew=`) or the tab it already had
    // moved into it (`stood=`), and those are the two answers this verb
    // exists to tell apart (デザイン規約 §タブの所作).
    // `kept=` is the third of them and the one the other two cannot
    // reach: the page that asked is the page that arrived, and its
    // graph was never emptied on the way — the copy switch swaps the
    // session under a standing page and replaces the rows in one go
    // (`Hub::restand_tab`), where a rebuilt page photographs exactly
    // the same.
    // `drew=` is the pair's other half, and the two presets answer it
    // the two ways there are: another copy's uncommitted row becomes
    // this tree's own, so the rows differ and the graph is swapped
    // (`drew=true`), where standing in a copy whose picture is the same
    // swaps nothing at all (`worktree-stand`). Both are read against
    // the record of the graph the session taking over is handed
    // (`session::DrawnGraph`), which is what lets a pass say "this is
    // already on screen".
    Verb {
        name: "carried-open",
        when: &[],
        plain: "carried_open tabs=1 grew=false stood=true drew=true kept=true",
    },
    // The same landing by the left menu's door. One tab before and one
    // after, and the copy the row named is the one the session opened.
    // `log=` is the second thing the page keeps: the command log is the
    // record of what this window ran, and the window is the same one —
    // the copy arrived at numbers its own commands from one without
    // touching the rows the copy left behind put there
    // (`CommandMsg::run`).
    Verb {
        name: "worktree-stand",
        when: &[],
        plain: "worktree_stand tabs=1 grew=false stood=true drew=false kept=true log=true",
    },
    // Away and back: `empty=` is the copy arrived at not wearing the
    // words typed in the one left, `back=` those words standing in the
    // box again, and `wip=` the pane holding them being the one on
    // screen. None of the three is a picture — the last frame of this
    // verb is one page with a commit message in it, which is what the
    // first frame was.
    Verb {
        name: "copy-draft",
        when: &[],
        plain: "copy_draft empty=true back=true wip=true kept=true",
    },
    // And the path that comes out under the hand on such a tab.
    // `copy=` is the whole of it: the words are the copy the tab is
    // standing in, not the repository it is named after, and the two
    // differ only on a tab that has been stood somewhere. `tip=` is the
    // hover itself, which the strip's own picture cannot hold — a run
    // whose tip never opened photographs an ordinary band.
    Verb {
        name: "worktree-tip",
        when: &[],
        plain: "worktree_tip tip=true copy=true native=false",
    },
    // The same landing, left and come back to. `kept=` is the run
    // naming that copy still drawn on the tab — read off the strip,
    // because the tab that was left has no page to ask. `front=` is
    // which of the two landings the picture is of, and it is the whole
    // difference between them: a strip of two tabs frames the same
    // either way. `stood=` is the copy the reader came back to, which
    // only the returning landing can answer.
    Verb {
        name: "worktree-kept",
        when: &[(
            Arg::Has("away"),
            "worktree_kept tabs=2 kept=true front=false",
        )],
        plain: "worktree_kept tabs=2 kept=true front=true stood=true",
    },
    Verb {
        name: "tab-carry",
        when: &[],
        plain: "sessions=1 folded=true log=true empty=true back=true wip=true",
    },
    // A strip of namesakes, and the path the hand asks for on top of
    // it. `unique=` is what both machines have to answer: no two tabs
    // read alike. `native=false` is the other half — a path on screen is
    // punctuated with `/` wherever it runs (規約 §パスの区切り), so a
    // backslash in a name or in the hover beside it is Windows having
    // spelled one its own way. `tip=` is the hover, which the strip's own
    // picture cannot hold (it comes out on the overlay), and a run whose
    // tip never opened photographs a perfectly ordinary band.
    Verb {
        name: "tab-name",
        when: &[],
        plain: "unique=true native=false tip=true",
    },
    // Which tab was carried and where it came to rest. A strip whose
    // order merely changed passes with any two tabs swapped, and one
    // where the carry never took hold photographs the order it
    // started in — which is a strip that looks like every other one.
    // `moved=` is the carried tab's own path, found at the place it
    // was asked for.
    Verb {
        name: "tab-drag",
        when: &[],
        plain: "tab_drag moved=true",
    },
    // The other half of the same gesture, and the one the settled
    // strip cannot hold: a strip whose tab was never drawn away from
    // its own row photographs exactly like one whose rows only ever
    // jumped. `lifted=` is the offset the transform is carrying, read
    // off the tab itself.
    Verb {
        name: "tab-hold",
        when: &[],
        plain: "tab_hold lifted=true",
    },
    // And the strip travelling under a tab held past its end. The
    // picture is a scrolled strip either way — the one that travelled
    // and the one that was already there frame alike — so what is
    // judged is that the tab reached the far end of an order it could
    // not see when the hand took hold.
    Verb {
        name: "tab-edge",
        when: &[],
        plain: "tab_edge landed=true",
    },
    // What a strip made of the run it was handed, with a real font's
    // widths going in and real tabs coming out. The widths are read by
    // eye against the shape the verb's own notes give, and `crushed=0`
    // is the one reading they cannot carry: a tab is drawn as wide as
    // its name, so a name cut away to the mark alone leaves every number
    // here exactly where it should be, and the picture of it is a narrow
    // tab with a mark in it — which is what a short name in this strip
    // looks like as well.
    //
    // **Two runs, because there are two states and no count names
    // either.** What the arithmetic does between them is a table asked
    // without a window (`tests/qml/tst_tabwidths.qml`); what only a
    // window answers is that this platform's font reached the state at
    // all, so each run says which one it is in and is judged on it. A
    // count that stopped crowding the band — a wider window, a narrower
    // font — then fails here instead of going on passing against a strip
    // with room to spare.
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
    // The stand-in for the tab in front, and the press that takes it
    // away again — read as a pair. A strip whose front tab is genuinely
    // at the edge of the run frames exactly like one standing in for a
    // tab that is nowhere on it, so `onScreen=` carries both: the
    // stand-in is wanted exactly while its row is out of sight, and a
    // run where both halves agree has the rule backwards.
    //
    // `kept=true crushed=0` is in front of that pair: the stand-in is
    // drawn at the width of the tab it stands for, so what is left for
    // it to get wrong is what it says in that width — and the rows it
    // stands in front of are off the run, where no picture reaches them
    // at all.
    //
    // Which edge it took is judged only where the run said which tab to
    // stand behind — the first one, sent away to the far end, rides the
    // left edge. Any other argument still has to have stood one up.
    Verb {
        name: "tab-pin",
        when: &[(
            Arg::OneOf(&["", "0"]),
            "tab_pin kept=true crushed=0 stood=true onScreen=false left=true",
        )],
        plain: "tab_pin kept=true crushed=0 stood=true onScreen=false",
    },
    // And the travel, which no picture holds: a settled strip with its
    // front tab in view is the same photograph whether it travelled
    // there or was never sent away. `travelled=` is the strip having
    // moved under the press, `gone=` the stand-in's own answer to
    // having arrived. `crushed=0` is what this verb's picture does
    // hold — the tab in front at the width the crowded strip left it —
    // and cannot be read for.
    Verb {
        name: "tab-pin-go",
        when: &[],
        plain: "tab_pin_go crushed=0 gone=true onScreen=true travelled=true",
    },
    // And the same arrival asked for by opening a repository.
    // No picture holds this one either: a strip
    // standing on its newest tab frames the same whether the band went
    // there or was sitting on that end all along, so the three that are
    // judged say the band moved, the tab it moved for is whole in the run,
    // and the stand-in that was up has stepped aside.
    Verb {
        name: "tab-open-go",
        when: &[],
        plain: "tab_open_go arrived=true gone=true travelled=true",
    },
    // And the reader doing the next thing after that arrival: a press
    // anywhere ends the ask, so the run narrowing under the tab it
    // brought in leaves the strip where it was with the stand-in up. The
    // picture of a strip that travelled after the press holds the tab
    // whole — the one a reader never asked for — so the three judged are
    // the strip not moving, the stand-in standing, and the ask gone.
    Verb {
        name: "tab-open-moved-on",
        when: &[],
        plain: "tab_open_moved_on travelled=false stood=true ask=false",
    },
    // The ☰'s card, standing. `yield=true` is the half no picture holds:
    // the band's empty run is the platform's caption, and while the card
    // is up it has to stop being that or every press landing there is
    // swallowed and the card never goes down. A run where the run was
    // never handed back frames the same open card.
    Verb {
        name: "app-menu",
        when: &[],
        plain: "app_menu open=true yield=true",
    },
    // The panel standing, which is the verb the state matrix is
    // photographed with. `settled=true` is the whole claim a picture
    // needs behind it — the repository landed, nothing running — and
    // everything beside it is what the picture cannot be read for.
    //
    // **No name is lit**: no hand is on the panel, and a lit name is a
    // shade in the picture.
    //
    // `boxed=true` is every button at the panel's right end as deep as its
    // own two lines. The branch's name writes one line for a branch
    // following nothing (`worktrees`) and for a page still reading its
    // repository, and a button held to that depth stands its word over
    // its frame and its mark under it — a shape a picture of that end
    // alone cannot tell from a style.
    //
    // `rule=true` is the line between the three and the find standing in
    // the step between them, clear of both frames — a pixel onto either
    // reads as that frame's edge — and ending on the frames' own top and
    // bottom, folded or not, which a 1px line in a picture cannot be read
    // for without counting.
    //
    // `track=` is where the branch's counts came out: carried to the `end`
    // of an upstream longer than the name and its counts (`panel` — `main`
    // over `origin/main`), or `after` the name at their own step where the
    // name's line is the longer (`longnames`). A pixel either way reads
    // as set against the upstream in the picture.
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
    // The panel's own doors. The counts ride along because a card is
    // assembled from listings that arrive after the tab does, and a card
    // holding nothing photographs exactly like one whose rows were all
    // left out — and because `open=false` is the right answer for a
    // repository with nowhere else to stand, which is a preset fault
    // rather than a wiring one and has to be legible as such.
    //
    // `lit=true turned=true` on every door: the name the card hangs off
    // stays washed with its chevron turned down while the card stands —
    // and a name under the hand that pressed it frames the same whether
    // the card is what keeps it lit or not. `yield=true` is the band's
    // grab runs handed back to the scene while the card stands
    // (`WindowChrome.captionYielded`): the platform takes a press on them
    // otherwise, and the card stands through the click meant to close it.
    Verb {
        name: "ops-stand",
        when: &[],
        plain: "ops_door open=true tier=stand lit=true turned=true yield=true",
    },
    // …and each tier of it, opened the way resting on the row opens it.
    // **Which card is standing is judged**, not that one is: a tier
    // asked for and not opened leaves the card above it on screen, and
    // the two frame alike once the picture is cropped to the panel.
    Verb {
        name: "ops-stand-repos",
        when: &[],
        plain: "ops_door open=true tier=repos lit=true turned=true yield=true",
    },
    // With a filter typed into the left menu first, **the tier still
    // opens while the section lists nothing**: the card is a way to move,
    // not the list being read (`NavSectionModel.copyCard`).
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
    // **Which folder stands is judged**, one row per folder the census
    // runs: a folder asked for and not opened leaves the card above it
    // standing, and the two frame alike once the picture is cropped to
    // the panel. An argument with no row of its own is judged on the tier
    // alone.
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
    // **`landed=` is the claim**: a row that closes its card and reaches
    // nothing is the same picture as one that landed. `card=none` is the
    // card gone with the press.
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
    // `named=true` is the panel naming the tab it moved to in the press
    // itself, before that tab has read a thing: the names were the page's
    // answer and waited on git for it, and in the picture of the landing
    // a name that arrived late and one that was there all along are the
    // same name.
    Verb {
        name: "ops-repo-pick",
        when: &[],
        plain: "ops_pick door=repo found=true landed=true card=none named=true",
    },
    // And the mark pressed a second time. The card that reopened on that
    // press and the card that was never closed are the same photograph —
    // which is how this went unnoticed — so what is judged is the card
    // being down with the run back in the platform's hands.
    Verb {
        name: "app-menu-reclick",
        when: &[],
        plain: "app_menu open=false yield=false",
    },
    // The three states themselves. A run where one of them never stood
    // photographs a band that was never crowded, and that picture
    // cannot be told from a band that gave the crowd room — so the
    // crowd has to be said out loud.
    //
    // **And so does the shape**, which a width does not name: the same
    // number lands in a different one on a different font, so a run that
    // says only which badges stood goes on passing in a shape nobody
    // asked for (measured 2026-09-18: one width folded the mark on
    // Windows and kept its words on Linux). The arithmetic behind the
    // shapes is `BandStateShare`'s and is asked in one process
    // (`tst_bandshare.qml`); the two runs here are the two sides of what
    // a window is for — a group the band had the room for, and the
    // same group narrowed by a real crowd of tabs.
    //
    // **The width is the run's, and it is calibrated to the band.** The
    // strip is at the front of the queue for what the band has
    // (`TopBar`), so whatever else stands on the row moves the window
    // width at which the group starts narrowing. What the run is for is
    // the shape, so the number moves with the band.
    //
    // The claim stops before the fit: the widths that show the last
    // shape are below the floor a hand can drag the window to
    // (`fits=false` is what was asked for there). The folded mark is
    // reached through the floor instead of through the arithmetic —
    // the floor folds the group whatever the share-out says, so it lands
    // the same on both sides — here in red, and in yellow by
    // `old-git-fold`. The floor itself is `window-floor`'s question.
    //
    // `floor`'s claim is the fold and its width: `fitted=true` is the
    // group cut to its mark's cell. A folded group that kept the width
    // its words asked for stands empty band beside the mark, which the
    // picture reads as a wider grab run behind the tabs and nothing in
    // the frame names. `narrowed=` is left out — what the share-out came
    // to under the fold is the fonts' answer, not this row's.
    //
    // `shadow=true` is on every row: the shape is decided on a second
    // laying-out of the band with the words kept (`TopBar`'s `bandAsked`),
    // and a cell that shadow does not mirror — the window's own buttons,
    // which set their width rather than asking for it, measured — hands
    // the words room the group's cell does not have, and they run out of
    // it over the strip's grab run. Only a band with those buttons (a
    // Windows run) can tell; elsewhere it holds trivially.
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
    // The card, opened. `rows=` is the half the picture cannot carry
    // on its own: a card with one row and a card with three frame the
    // same way once it is cropped to the band, and which rows arrived
    // is the whole question the group raises when it gives way.
    //
    // **One stopped operation is every stopped operation here.** The
    // card ran a second time over a stopped cherry-pick, and the rows
    // are the same three — the only thing that differed was the word
    // inside the op row, which is the model's
    // (`worktree::tests::every_operation_that_can_stand_here_…`, where
    // all four are walked without a window).
    Verb {
        name: "badges-hover",
        when: &[],
        plain: "card=true rows=op,conflicts,identity",
    },
    // The same card judged the same way, opened from the other order:
    // the stand-in pointer goes down before the band has placed the
    // group, so the group arrives under a hand that is already there. A
    // pointer that cannot move raises the beat once and never again, so
    // the card is owed a second asking when the row answers — and
    // nothing else photographs whether it gets one. `badges-hover` puts
    // the pointer down on a group the band placed long before, and
    // passes either way (measured: the group's second asking taken back
    // out leaves that verb green and this one at the watchdog).
    Verb {
        name: "badges-hover-early",
        when: &[],
        plain: "card=true rows=op,conflicts,identity",
    },
    // The badge for a graph that is not this repository's history, and
    // the card line that parts its two states. **Neither is a picture's
    // to judge**: a whole graph that has gone out of date is pixel for
    // pixel the graph that was current a moment ago, and an emptied
    // column is what an opening looks like — the badge is the only mark
    // either leaves, and the badge alone cannot say which of the two put
    // it up. So the model's own two are read beside it, one of them true
    // and the other false, and the card is required open because that
    // line is where the difference is written. `tint=` rides in the
    // judged run: both states are `danger` (規約
    // §状態), and a badge that came up warning is the rule not having
    // reached the paint.
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
    // working copy's row and none of its own — the arrangement the run is
    // started into, since which of the walk and the first status gets
    // there first is the scheduler's.
    //
    // Both moments are in the one line because neither means anything
    // alone: `early=` and `earlyCopy=` are the page while the row was
    // held back — nothing opened, and nothing stood on a copy — and
    // `wip=` is the pass that carries ours landing on this window's own
    // tree. `held=true` is the hold answering for itself, so a run that
    // read the ordinary order cannot pass as one that read this.
    // `otherTop=true` is the rest of that arrangement: the held-back pass
    // led with a neighbour copy's row, which is the row the misreading
    // takes. Without it a run whose graph simply had no all-zero row on
    // top would pass while never asking the question. The row at the end
    // is judged too — where the landing put the reader is the half a
    // picture of a working-tree pane cannot tell from any other.
    Verb {
        name: "wip-landing",
        when: &[],
        plain: "wip_landing held=true otherTop=true early=false earlyCopy=false wip=true copy=false row=0",
    },
    // The other landing on the working tree, read in the same
    // arrangement: the one a stopped operation owes. `owed=true` is the
    // move decided and still held while the row was out, and the press
    // that made it is a replay this window started from a clean tree —
    // the conflicts it stopped in are what the row it lands on is made
    // of.
    Verb {
        name: "wip-landing-stopped",
        when: &[],
        plain: "wip_stop_landing held=true otherTop=true owed=true early=false earlyCopy=false wip=true copy=false op=REBASING lit=0",
    },
    // The panel's three actions at the last cell that still has a word
    // in it. A band shot at one width says nothing about the width its
    // shape was supposed to change at, so what is judged is that the
    // words were still there at the narrowest cell that holds one.
    //
    // **Run where the names are long** (`--preset longnames`). What the
    // panel runs short of comes off the names first, so with a
    // repository called `repo` on a branch called `main` the actions
    // are still saying every word at the floor a hand can drag the
    // window to — the shapes below exist, and no width this window
    // takes reaches them.
    // `cut=` rides along as diagnosis: how long the stretch
    // where the wordings are cut lasts is a question about the
    // installed fonts, and for a four-letter command it can be nothing
    // at all (`…` and two characters measure what `push` does).
    Verb {
        name: "band-actions",
        when: &[],
        plain: "band_actions fits=true folded=false",
    },
    // And the end of that road: the words given up for the marks, at
    // the floor the window has with its left list open — which is the
    // width they have to be finished by. A band
    // whose actions never folded frames as three ordinary buttons,
    // which is what an ordinary band looks like.
    Verb {
        name: "band-actions-fold",
        when: &[],
        plain: "band_actions fits=true folded=true",
    },
    // The same marks with one of them saying the last go did not work.
    // A `!` on a button with no word after it has nowhere to stand but
    // the mark's own corner, and that corner is also where a signature
    // and an avatar's badge sit elsewhere — so what is judged is that
    // the mark survived the fold at all. The refusal is the far side's:
    // a plain `push` into a remote that moved on (`--preset diverged`,
    // the run carrying `--allow-write-failure`), which is turned down
    // for not fast-forwarding. The reachable origin is the point — the
    // go has to get far enough to be refused for a `!` to stand on.
    Verb {
        name: "band-actions-alert",
        when: &[],
        plain: "band_actions fits=true folded=true cut=true alert=true",
    },
    // The same corner on the other mark. What puts it there is the
    // run of failures that stops the timer, so this one is judged on
    // its own line: the band's `alert=` is the pair or-ed together
    // and cannot say which of the two buttons is carrying it, while
    // the frame at this width belongs to the stopped fetch alone.
    Verb {
        name: "band-actions-stopped",
        when: &[],
        plain: "band_stopped suspended=true turned=true alert=true",
    },
    // Kept beside its pair so the two read together.
    Verb {
        name: "band-actions-none",
        when: &[],
        plain: "band_actions fits=true folded=false cut=false",
    },
    // The fourth badge. A run whose shim never reached PATH reads the
    // git this machine has, wears no badge, and photographs an
    // ordinary window — which is exactly what an ordinary window looks
    // like. `badge=` is the band's own reading, so the whole path from
    // `git --version` to the row is what passes or fails here.
    Verb {
        name: "old-git",
        when: &[],
        plain: "old-git badge=true",
    },
    // And the card it opens. `rows=` rides along: the other three
    // rows come and go with the machine (a container with no identity
    // configured stands one of them), and only this row is the verb's.
    Verb {
        name: "old-git-card",
        when: &[],
        plain: "old-git badge=true card=true",
    },
    // The band folded with nothing red standing in it. Two halves, and
    // the picture holds neither on its own: a mark that never came up
    // frames as a band with room to spare, and the colour of three
    // dots is not something a cropped screenshot settles an argument
    // about. `tint=` is named — what is being judged is which rule
    // painted it (規約 §状態: 最も重い状態が
    // 決める), and a red mark over a lone warning is the way that rule
    // fails silently. `fitted=true` is the band's cell cut to the mark:
    // a folded group that kept the words' allotment reads in the picture
    // as a wider grab run, which nothing else in the frame names.
    Verb {
        name: "old-git-fold",
        when: &[],
        plain: "mark=true tint=warning fitted=true",
    },
    // The picker is the platform's own window and stands in neither
    // photograph — the overlay never holds it (`popups=0`) — so a run
    // that opened it and one that never did are the same pair of
    // pictures. The line is the whole of the evidence, and the folder
    // it names is the verb's own subject: the picker comes up beside
    // the repository that is already open (rules-refs/app-ui.md).
    //
    // `beside_copy=` is what makes the folder readable without knowing
    // the machine's paths: whether the picker came up in the folder the
    // **copy** sits in. The plain run stands in the repository's own
    // copy, where the two folders are one, and `copy` stands in a linked
    // one kept below the root (`Route::NestedCopy`) — there the picker
    // leaves the copy's folder, which is the whole of the difference
    // between opening beside the repository and opening beside the copy.
    Verb {
        name: "open-picker",
        when: &[(Arg::Is("copy"), "picker beside_copy=false")],
        plain: "picker beside_copy=true",
    },
];

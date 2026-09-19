//! The left list: the name boxes it opens, the rename gesture, what a
//! fold takes away, and the tooltips that spell out an elided row.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // Naming a branch off the top row, held here for what the run
    // waited on: the barrier every write verb passes through says
    // whether it waited on **its own write**
    // (`AutoActDriver.writeBarrier`).
    //
    // **`mine=false` is a barrier that went back to counting.** A count
    // moves for the fetch a repository makes on its way open and for the
    // interval's own, and between the press and git having the write
    // there is a stretch where nothing is running for a quiet condition
    // to catch — so a run could pass both halves on somebody else's
    // answer and photograph the page before its own write ran. `settled=`
    // is the other half: everything that write invalidated had been read
    // again and published before anything was drawn (P3-確認事項).
    Verb {
        name: "name-branch",
        when: &[],
        plain: "write_barrier mine=true settled=true",
    },
    // A name git will not take — the branch beside this one already
    // carries it. **The box stays open holding what was typed** and git's
    // own words go under it, where closing it first would have thrown the
    // typing away and left the answer nowhere but the log
    // (デザイン規約 §答えの要らない報せ, by design).
    //
    // Nothing here is a picture: a box that closed and a box that stayed
    // open are a frame's colour apart, and the words are in a tooltip.
    // `tone=warning` is the other half — the gesture is still going, which
    // is the whole reason the box is still there.
    Verb {
        name: "rename-taken",
        when: &[],
        plain: "rename_taken open=true refused=true bar=true tone=warning why=",
    },
    // A tag renamed to its own name in other letters. **git writes a ref
    // as a file**, so on a case-insensitive disk both names would be gone
    // — core refuses it outright, and the box is where that answer belongs
    // (デザイン規約 §答えの要らない報せ, by design). Two runs make the
    // pair: the one that has to be turned down, and the one beside it
    // that goes through, since a box refusing everything frames the same.
    //
    // Every field is the output side. `box=` is the field drawn and
    // holding the keyboard, because the ask that opens it lands before
    // the row is laid out and a refusal on a field nobody can type into
    // is not the state this is about. `tip=` is the refusal reaching the
    // reader — the box's own mark says it turned the name down, while the
    // sentence saying why is raised on the shared tooltip by a binding
    // Qt can drop, and it is read through the box so that somebody
    // else's tip cannot answer for it. `text=` is that sentence where the
    // reader gets it, which is why the reason the model worked out is not
    // on the line at all.
    Verb {
        name: "rename-tag-box",
        when: &[(
            Arg::Is("v1.0"),
            "tag_name_box was=v0.3-local typed=v1.0 box=true refused=false tip=false text=",
        )],
        plain: "tag_name_box was=v0.3-local typed=V0.3-LOCAL box=true refused=true tip=true \
                text=Only the letter case differs — on this disk that deletes both names",
    },
    // Every door onto the history while a rebase replays behind the
    // screen — the graph's and the left pane's. **None of it is a
    // picture**: a switch the road turned away, a box that never opened, a
    // `+` that greys and a menu row that greys all frame exactly like a
    // window nobody touched, and the picture answers the other half —
    // that the rows around them did not go out with them. `frozen=false`
    // is in the line because the plan's freeze is the other state and
    // takes the pane whole: a run that photographed that one instead
    // would leave a picture nothing in the report could tell apart.
    Verb {
        name: "doors-held",
        when: &[],
        plain: "doors_held held=true frozen=false road=true rowmenu=true drop=true \
                box=false plus=true menu=true switchrow=true why=true",
    },
    // The row taken away before git answered for it, held open for
    // the picture. `row=-1` is the sidebar without it and `chips=true`
    // the graph without its chip — **both halves at once** is the whole
    // of the rule, and a run that only ever reached the state after
    // the refs landed would say the same `row=-1` for a different
    // reason (デザイン規約 §消す操作は先に画面から消す).
    Verb {
        name: "delete-gone",
        when: &[],
        plain: "gone_row tag=v0.3-local row=-1 total=2 chips=true",
    },
    // …and the same row let go of again, which **no picture can answer
    // at all**: a row gone because the list no longer carries it draws
    // exactly like a row the window is drawing without. `stood=true` is
    // the press having taken it away and `chips=false` the standing in
    // being over, and the claim is the pair — either alone passes a
    // window that never stood the row in, or one that never let it go.
    //
    // It is the other half of `delete-gone` and fails the other way: a
    // delete whose rows are put down by counting listings
    // (`ops::StandIn`) can be answered by a reading that never saw the
    // write, and one measured against a number no reading reaches is
    // never answered at all. The second of those leaves this run
    // waiting out the ceiling.
    Verb {
        name: "delete-stood-down",
        when: &[],
        plain: "stood_down tag=v0.3-local stood=true row=-1 total=2 chips=false",
    },
    // The delete row dressed with git's answer before any click. **The
    // picture cannot answer this**: a row nobody asked about wears
    // `branch --delete`, and so does one whose answer came back merged
    // — the line is the only place the two are told apart, and without
    // a row here the verb was judged on the shot alone (verify-ui
    // §PGG_AUTO_ACT 動詞表). It is also what fails the run that opened
    // its card over a delete that was out, which is otherwise 120
    // silent seconds of watchdog (`AutoActRefVerbs.earlyDeleteTimer`).
    //
    // `from=` is left outside the run on purpose: an in-window branch
    // answers off the drawn rows and a tip off git, and which of the
    // two a run gets is the refs' timing.
    // `merged=` is one word for both sides for the same reason — the
    // third of them, `unknown`, is the reads having fallen over, and it
    // fails this claim on a line.
    Verb {
        name: "delete-branch-early",
        when: &[(
            Arg::Is("feature/topic-a"),
            "merged=no code=branch -D held=true note=not merged",
        )],
        plain: "delete_early asked=true",
    },
    // The same card, over the one branch whose tip the drawn rows cannot
    // answer for — so `from=` is in the claim here, where the row above
    // leaves it out: this run takes git's road every time. What settles
    // it is the repository: `--preset deep-parked` forks `parked` below
    // the window's cut and leaves it there, and a tip on no drawn row
    // is what sends the question to `merge-base`
    // (`GraphModel::branch_delete_merged` empty →
    // `RepoSession::check_branch_delete`).
    //
    // **`merged=no` is the half nothing else reaches.** The string git's
    // answer arrives as is compared in one place
    // (`RefBranchMenu.refusedRow`), and that comparison is on this road
    // alone: with no run that takes it, a wrong word there costs nothing
    // that shows — the row would offer the plain delete, and every
    // picture in the suite would frame the same. The rest of the line is
    // the row it dresses, which is the whole point of asking early.
    //
    // The claim is `plain`: the verb has one repository and one branch
    // in it, so a run given anything else is a mistake that should fail
    // on a line.
    Verb {
        name: "delete-branch-early-far",
        when: &[],
        plain: "delete_early asked=true from=git merged=no code=branch -D held=true \
                note=not merged",
    },
    // The three deletes asked of the left row's card, each judged first
    // on the press having left the page at all. **The picture cannot
    // answer that**: the page drops a delete asked while a write is
    // running (`RepoPage.deleteRow`), and a card standing over a row
    // nobody acted on frames exactly like one whose write is still out —
    // so the run waits on a write nobody made, and 120 silent seconds of
    // watchdog is the whole of what it says (`AutoActRefVerbs`, the same
    // hole `delete-branch-early` had).
    //
    // One line for all three because it is one claim, and it is the only
    // one they share: what each does after the press is its own — `-go`
    // holds the `-D` the refusal left, `-refused` stands still and reads
    // the row it turned into, and the plain one stops at the write.
    Verb {
        name: "delete-branch",
        when: &[],
        plain: "delete_row asked=true",
    },
    Verb {
        name: "delete-branch-go",
        when: &[],
        plain: "delete_row asked=true",
    },
    Verb {
        name: "delete-branch-refused",
        when: &[],
        plain: "delete_row asked=true",
    },
    // The delete table's greyed rows, each saying why it is out. **The
    // picture cannot answer this**: a tooltip is words, and a row that
    // greyed for the wrong reason frames exactly like one that greyed
    // for the right one. The two runs are a pair — the local row is out
    // because of where the working tree is standing, the remote row
    // because the two names are standing on different commits
    // (デザイン規約 §左メニューの所作 の削除の表), and either alone
    // would pass an implementation that gave both rows one reason.
    //
    // The local row's own line is not in here: it is git's refusal
    // written out with an em dash, and a non-ASCII `must_say` never
    // matches on Windows (verify-ui §Windows での実行・デバッグの罠).
    Verb {
        name: "delete-blocked-tip",
        when: &[(
            Arg::Ends(":remote"),
            "delete_blocked code=push --delete tip=true \
             reason=The remote is on another commit",
        )],
        plain: "delete_blocked code=branch --delete tip=true",
    },
    // The fourth: an elided row whose hover says the whole name.
    // One verb serves both panes (the argument picks one), so the
    // wanted line names neither — `tree=` echoes which view the
    // argument asked for (`-tree` keeps the tree, where row 0 is
    // the elided folder chain), `tip=` the shared instance's own
    // visible.
    // The third pane is claimed by its words as well as its tip, and
    // it is the only one of the three that needs to be. The rows there
    // come from the model the sidebar shares, which folds by
    // `<run>:<path>` and keeps the path itself beside it — one field in
    // the list this row was written for, two in that one — so a folder
    // row handed the fold key says `whole:src` everywhere it says a
    // name, and the hover is the only place a path is spelled out at
    // all (observed). `topic` is the copy whose one file is nested.
    Verb {
        name: "path-tip",
        when: &[
            (
                Arg::Is("carried:topic-tree"),
                "path_tip pane=carried tree=true tip=true text=src",
            ),
            (
                Arg::Is("carried:topic"),
                "path_tip pane=carried tree=false tip=true text=src/topic.txt",
            ),
            (Arg::Ends("-tree"), "tree=true tip=true"),
        ],
        plain: "tree=false tip=true",
    },
    // And the same tooltip taken away: `copied=` is the whole of the
    // tip read back out of a selection made on it, so it is false the
    // moment the words stop being a field the reader can drag over.
    // **A picture cannot answer this** — a selected line and an
    // unselected one differ by a wash the shot's own scaling can lose,
    // and a `Text` put back in place of the field would frame
    // identically.
    Verb {
        name: "tip-copy",
        when: &[],
        plain: "tip=true copied=true",
    },
    // ...and taken from the band around the words — every place in a
    // tip that nobody else takes is a start
    // (規約 §hover のツールチップ). The starts are the air
    // itself, sampled: a grid over the tip with the points standing on
    // the sentence dropped, which is exactly what a real press could
    // reach the pad at.
    //
    // **`all=` is the claim.** How many places a tip's air has
    // depends on the length of the path in it and on the machine that
    // drew it, so the number rides along as diagnosis and the
    // judgement is "every one of them, and there was at least one".
    // `caret=` is beside it because a selection with no keyboard on it
    // is not one `Ctrl+C` can take (規約: 選択できることと持ち帰れる
    // ことは別).
    // `hand=` is the half a sweep cannot say for itself: a run enters
    // the pad's own functions, because a pointer cannot be injected, so
    // a pad whose `MouseArea` had been taken out would answer every
    // sweep it was asked and never see a press.
    Verb {
        name: "tip-sweep",
        when: &[],
        plain: "tip_sweep all=true caret=true hand=true",
    },
    // A row's tooltip. `lit=` is the control — the first row of the
    // working copies' listing, which answered the same pointer on the
    // way in, so an empty overlay in the picture is the row's own
    // answer, from a pointer that landed. **It answers by opening**:
    // that section's rows open under themselves now, and it is still
    // the one section every repository has a row in.
    // `wants=` is the row's own words and `tip=` the shared instance
    // carrying them: every row that answers with a tooltip spells its
    // name in full, so both are judged.
    //
    // **The rows left on this side are few**: the remote's own row,
    // stashes, tags and the working tree's files. A row of BRANCHES,
    // REMOTES or WORKTREES is read by `nav-open` instead.
    // **A fold parent is judged on the words**, and only on a name the
    // row itself does not already show: `--preset nested` row 2 is the
    // folder `backend`, whose path is `team/backend`, so a tooltip built
    // from the row's shown segment instead of its fold key parts from
    // the claim here. Every folder at the top of a tree spells the two
    // the same, which is why the claim cannot live on one of those.
    Verb {
        name: "nav-tip",
        when: &[(
            Arg::Is("branch:2"),
            "lit=true wants=true tip=true open=false text=team/backend",
        )],
        plain: "lit=true wants=true tip=true",
    },
    // Where one click on a left-panel row leads. **The picture cannot
    // answer it**: the window opens with a row already lit, so a graph
    // standing on the commit the click asked for and a graph that never
    // moved frame the same — and a wiring that dropped the click
    // silently is exactly the second one. `same=` is the graph's own row
    // carrying the commit the row named, `lit=` the light on it, and the
    // two together are what the press produced. `wip=false` is beside
    // them because the history has to be the face on screen for either
    // to mean anything.
    //
    // `at=` and `name=` stay out of the claim: they are the repository's
    // numbers — where that commit happened to sort and what the folder
    // was called — and a preset with one more commit in it would fail a
    // claim that reached them while nothing about the jump had changed.
    Verb {
        name: "nav-jump",
        when: &[],
        plain: "same=true lit=true marked=true wip=false",
    },
    // A branch row opens its facts under itself rather than raising the
    // shared tooltip (`NavRowFacts`), so the pair judged here is the
    // other way round: nothing in the tooltip, the row open.
    //
    // **The words it carries stay out of the claim.** The name, the
    // upstream and the working copy holding the branch are the
    // repository's and the preset's, not the rule's — they go on the
    // line (`says=`) for the reader of the report, and the picture is
    // where a row that opened on nothing is caught.
    // A REMOTES row opens on the same lines, and two of them are its
    // own: the branch measured against the reading, and the copy
    // holding **that** branch. Neither is anything the picture settles —
    // a line drawn from the row's own name instead of from the join
    // frames the same — so the words are judged.
    //
    // A WORKTREES row opens on the same lines, and the one it fills is
    // the state git noted on the checkout — the answer its own mark
    // stands for, which goes from the row's seat while the row is open.
    // **The words are judged there too**: a padlock drawn with no
    // sentence beside it and one drawn with the wrong sentence frame the
    // same to a claim that stopped at `open=`, and the reason a lock was
    // taken for is the half that comes off the row's other slot.
    //
    // **`open=` rides inside them**: the sentence is empty until a row
    // is open, so `says=` carrying a name is the row having opened.
    Verb {
        name: "nav-open",
        when: &[
            (
                Arg::Is("remote:0:main"),
                "says=origin/main local=main track=1/0 held=",
            ),
            (
                Arg::Is("remote:0:topic-a"),
                "says=origin/feature/topic-a local=feature/topic-a track=1/0 held=topic",
            ),
            // The two shapes of a state, on the rows of the `worktrees`
            // preset — the one repository that holds every annotation
            // `git worktree list` can print. **The name leads the
            // claim**: a working copy's row goes on showing the folder
            // it is named by while it is open, and the path it stands at
            // rides at the tail of the sentence, since it is this
            // machine's. **The state is claimed and the words are not**:
            // a lock draws no line at all — the padlock on the row is
            // the whole of it, and what it was taken for belongs to
            // whoever took it — while a folder git can no longer find
            // draws one naming the warning, which the picture reads.
            // A copy holding a branch that reads a remote: the lines say
            // of that branch what its own row would — the reading it is
            // measured against and how far it stands from it — and the
            // counts are the branch's, which no role of this row carries.
            (
                Arg::Is("worktree:1"),
                "says=topic local= track=1/0 held= up=origin/feature/topic-a gone=false \
                 branch=feature/topic-a",
            ),
            (
                Arg::Is("worktree:2"),
                "says=gone local= track=0/0 held= up= gone=false branch=gone/branch state=PRUNABLE",
            ),
            (
                Arg::Is("worktree:3"),
                "says=hotfix local= track=0/0 held= up= gone=false branch=hotfix/urgent state=LOCKED",
            ),
        ],
        plain: "wants=false tip=false open=true",
    },
    // The rest on what a row opened, and the one thing those lines
    // keep for it: where the working copy stands. **`open=` rides with
    // it** — a tip that came up over a row that never opened is the
    // shared instance answering for something else, and the overlay
    // frames the same either way.
    Verb {
        name: "nav-open-tip",
        when: &[],
        plain: "open=true tip=true",
    },
    // The reader who does not wait out the rest: a press on a closed
    // row's own line that starts to move. **Three answers in one line** —
    // the lines came out at once (`open=`), the drag reached them
    // (`copied=`), and the row did not take the gesture for a click
    // (`clicked=false`). None of the three is anything the picture
    // settles: a row that opened and then dropped the drag frames
    // exactly like one that carried it.
    Verb {
        name: "nav-drag-open",
        when: &[],
        plain: "open=true caret=true copied=true clicked=false",
    },
    // The same rest, taken on the last row of a section holding more
    // rows than it has height for (`--preset stack`: 30 branches, 10 of
    // them on screen). **The list is what has to move**: the lines open
    // below the row's own line, and at the foot of the list there is
    // nothing below — so a list that stood still opens them under the
    // bottom edge, where the picture shows the row and none of them.
    //
    // `shown=` is the two geometries weighed against each other — where
    // the row landed and what the list is showing — rather than the
    // scroll that was asked for: a list keeping a note of a move it
    // never made would otherwise vouch for itself. It rides with
    // `open=` because a row that never opened shows whole as well.
    //
    // `away` is the other half of the same rule: what the list gave up
    // to show them it takes back when the hand leaves. `back=` is the
    // run's own note of where the list stood before the hand arrived,
    // weighed against where it is standing now — the list's memory of
    // the move is the thing being tested, so it cannot be the witness.
    Verb {
        name: "nav-open-foot",
        when: &[
            (Arg::Is(""), "open=true shown=true"),
            (Arg::Is("away"), "open=false shown=false back=true"),
        ],
        plain: "open=",
    },
    // The open row, and then the next thing the hand does. **Each order
    // has its own line**, because what the row must do differs: it
    // closes when the hand leaves, closes when a name box opens on it,
    // closes when a menu takes over the list, goes with the row a filter
    // takes away, and stays under the menu it raised itself.
    //
    // `lit=` rides with them: the row wears the wash over the whole of
    // itself while it is open, and one that went dark under its own
    // lines reads as a row nothing is on.
    Verb {
        name: "nav-open-then",
        // **Each claim is one run of the line**, since that is how a must_say
        // is read: the words in between are part of what is being claimed.
        when: &[
            (Arg::Is("away"), "open=false lit=false box=false"),
            (Arg::Is("edit"), "open=false lit=true box=true"),
            (Arg::Is("menu"), "open=false lit=true box=false menu=true"),
            (
                Arg::Is("filter"),
                "open=false lit=false box=false menu=false rows=0",
            ),
            // A menu asked for from the open lines themselves. **The row
            // stays open** — it is what that menu is about, and lines
            // that went out from under it would take the context with
            // them. The order beside this one (`menu`) is a menu raised
            // anywhere else, which does close the row: the pair is the
            // rule.
            (
                Arg::Is("rightclick"),
                "open=true lit=true box=false menu=true",
            ),
            // The hand on those lines. A drag takes the words away and
            // **the row hears nothing** — the row's second click opens a
            // name box, and a reader dragging over a name to copy it
            // must not find it in one. A press that never moved is the
            // row's click, which is the other half: lines that answered
            // only one of the two would pass a claim written for the
            // other.
            (Arg::Is("sweep"), "caret=true copied=true clicked=false"),
            (Arg::Is("tap"), "copied=false clicked=true"),
        ],
        plain: "open=",
    },
    // The other order: a name box first, then a hand on the row it is
    // open on. **Nothing may open.** A row that opens takes the rows
    // under it down with it, and one of them carries the box — the only
    // thing on screen saying what mode the reader is in. `lit=true`
    // rides along because the row is still the one under the hand: the
    // pointer landed, and what answered it is the row staying shut
    // rather than the hand never arriving.
    Verb {
        name: "nav-open-held",
        when: &[],
        plain: "box=true open=false lit=true",
    },
    // The same opening, on a row of the section the folded rail has
    // open. **`peek=true` is the point**: the row that grows is inside
    // that popup, and a section that cannot make room for it takes
    // itself — and the row — down as the rows under it move.
    Verb {
        name: "nav-peek-open",
        when: &[],
        plain: "peek=true open=true",
    },
    // The current branch's stand-in riding the edge its own row went out
    // of, with the list scrolled out from under it. **The picture cannot
    // answer this**: on the top edge the stand-in draws the same whether
    // the row left upwards or was never in the list to begin with — the
    // second is a filter's doing and the state `nav-tip head`
    // photographs — and `rowshown=false` next to `above=true` is the only
    // place the two are told apart. `y=0` fixes *where* the top edge is,
    // which `above=true` does not: the stand-in's place is bound straight
    // off that answer, so the number moves only if the binding is given a
    // margin or an inset to start from, and that is the change it holds.
    //
    // `row=` and `rested=` are left outside the claim on purpose. They
    // are the repository's numbers — where the current branch happened
    // to sort, and how tall the section came out
    // — and a preset with one more branch in it would fail a claim that
    // reached them while nothing about the stand-in had changed.
    Verb {
        name: "nav-pin-edge",
        when: &[],
        plain: "nav_pin_edge pin=true above=true rowshown=false name=main y=0",
    },
    // The left menu's rename gesture, and the absence that is the
    // whole of its bug: a click landing in the folded list's section
    // after that section went away and came back is an ordinary
    // click. A picture cannot carry it — the run that armed nothing
    // frames as a folded rail with a section beside it, which is
    // what `nav-peek` frames as too. So the pair is read out loud:
    // `armed=` is the row's own answer to
    // the click it was given, `collapsed=`/`box=` what came of it
    // (the box puts the whole list back over the diff the fold was
    // made for — observed).
    Verb {
        name: "nav-reclick",
        when: &[],
        plain: "away=false armed=true collapsed=true box=true focused=true",
    },
    Verb {
        name: "nav-reclick-away",
        when: &[],
        plain: "away=true armed=false collapsed=true box=false",
    },
    // The same claim by the other door — the menu's `beginRename` —
    // and the rule it answers to: a box opens where its row is, so a
    // peeked row keeps both the fold and whatever the fold was made
    // for. `diff=` is the half that only the second one has a file
    // to lose.
    Verb {
        name: "nav-peek-rename",
        when: &[],
        plain: "collapsed=true diff=false editing=branch:",
    },
    Verb {
        name: "diff-fold-by-rename",
        when: &[],
        plain: "collapsed=true diff=true editing=branch:",
    },
    // The two ways the box is walked away from with nothing typed
    // into it. Same blind spot: a pane with no box in it frames like
    // a pane that never opened one.
    // And the row the box lands on, brought back from wherever the
    // list had scrolled to. A picture cannot carry it either: a
    // section showing its last rows and one showing its first frame
    // the same, and 2,000 tags all read alike.
    Verb {
        name: "nav-rename-far",
        when: &[],
        plain: "shown=true box=true",
    },
    Verb {
        name: "nav-rename-drop",
        when: &[(
            Arg::Is("fold"),
            "nav_drop how=fold collapsed=true box=false",
        )],
        plain: "nav_drop how=away collapsed=false box=false",
    },
    // The run that walks the list on past the row a box is standing
    // on: the box has to go with it, which is the whole of what a box
    // drawn outside its list owes. A sidebar with no box in it is the
    // same picture whether the box went or never opened.
    // The two name boxes a sidebar row opens, held at the width the
    // argument dragged the pane to. Whether what is in the box is cut
    // is in the picture; the other two are not — a box drawn where
    // nothing can be typed frames exactly like one waiting for a
    // name, and a box drawn outside its list has one more way to be
    // missing than a box in a row has.
    Verb {
        name: "nav-branch-box",
        when: &[(Arg::Ends(":away"), "open=true focused=true shown=false")],
        plain: "open=true focused=true shown=true",
    },
    Verb {
        name: "nav-rename-box",
        when: &[(Arg::Ends(":away"), "open=true focused=true shown=false")],
        plain: "open=true focused=true shown=true",
    },
    // A tag that was made. The write answers before the read that puts
    // the row in TAGS, so a run judged at the write barrier photographs
    // a sidebar with nothing new in it and passes — and the count alone
    // would pass for a tag left on whatever HEAD happened to be, which
    // is why the commit it was asked for is echoed too.
    Verb {
        name: "create-tag",
        when: &[(Arg::Ends(":box"), "create_tag box=true mode=tag")],
        plain: "create_tag tag=v9.9-new row=1 total=4 at=true",
    },
    // The third mode of that same field, judged on the same line for
    // the same reasons. Which mode it came up in is the one thing
    // here the picture *does* answer: the question is written in the
    // box (`Create tag here?`), and it is all an empty box holds.
    Verb {
        name: "nav-tag-box",
        when: &[],
        plain: "open=true focused=true shown=true",
    },
    // The same walking away, over the graph: a press that landed
    // somewhere else takes an empty box with it and leaves one with
    // something typed in it standing. Both halves are read off the
    // report because the picture holds only one of the two states,
    // and the card the run is about fades — so `shown=` says which
    // end of that fade the picture was taken at.
    Verb {
        name: "find-drop",
        when: &[(Arg::Is(""), "find_drop open=false shown=false")],
        plain: "find_drop open=true shown=true",
    },
    Verb {
        name: "name-box-drop",
        when: &[(Arg::Has(":"), "name_drop box=true")],
        plain: "name_drop box=false",
    },
];

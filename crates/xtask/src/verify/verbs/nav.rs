//! The left list: the name boxes it opens, the rename gesture, what a
//! fold takes away, and the tooltips that spell out an elided row.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // Naming a branch off the top row, claimed for the barrier every write
    // verb passes through (`AutoActDriver.writeBarrier`): whether it waited
    // on its own write. `mine=false` is a barrier that went back to
    // counting writes — a count also moves for the opening and interval
    // fetches, so a run could pass on somebody else's answer and photograph
    // the page before its own write ran. `settled=`: everything that write
    // invalidated was read again and published before drawing.
    Verb {
        name: "name-branch",
        when: &[],
        plain: "write_barrier mine=true settled=true",
    },
    // A name git will not take (a sibling branch carries it): the box stays
    // open holding what was typed, git's words under it
    // (デザイン規約 §答えの要らない報せ). A closed box and an open one are a
    // frame's colour apart and the words are in a tooltip, so none of it is
    // a picture. `tone=warning`: the gesture is still going.
    Verb {
        name: "rename-taken",
        when: &[],
        plain: "rename_taken open=true refused=true bar=true tone=warning why=",
    },
    // A tag renamed to its own name in other letters: git writes a ref as
    // a file, so on a case-insensitive disk both names would go — core
    // refuses it and the box says so (デザイン規約 §答えの要らない報せ).
    // The pair is the refused run and one that goes through, since a box
    // refusing everything frames the same. Every field is the output side
    // (rules-refs/app-ui.md「断りのツールチップ」): `box=` drawn and focused,
    // `tip=` read through the box, `text=` the sentence the reader gets.
    Verb {
        name: "rename-tag-box",
        when: &[(
            Arg::Is("v1.0"),
            "tag_name_box was=v0.3-local typed=v1.0 box=true refused=false tip=false text=",
        )],
        plain: "tag_name_box was=v0.3-local typed=V0.3-LOCAL box=true refused=true tip=true \
                text=Only the letter case differs — on this disk that deletes both names",
    },
    // A tag both sides hold, renamed here, and the bar asking what the
    // remote does about it (デザイン規約 §手元の改名の後のリモート).
    // `rows=3` / `shown=` are read off the chooser itself, so a pick that
    // never reached the form says so where the flow's own number would
    // call it taken. `hold=` / `neutral=` are the bar dressed by the pick:
    // only the answer taking a name off the remote is held, only the one
    // writing nothing stays plain (§状態). `here=true`: the local rename
    // landed, so the bar is this road's; `mark=true`: the row under it.
    Verb {
        name: "rename-tag-remote",
        when: &[
            (
                Arg::Ends(":replace"),
                "rename_carry kind=tag rows=3 pick=replace shown=true answerable=true hold=true \
                 neutral=false pill=Replace here=true mark=true",
            ),
            (
                Arg::Ends(":add"),
                "rename_carry kind=tag rows=3 pick=add shown=true answerable=true hold=false \
                 neutral=true pill=Create here=true mark=true",
            ),
            (
                Arg::Ends(":leave"),
                "rename_carry kind=tag rows=3 pick=leave shown=true answerable=true hold=false \
                 neutral=true pill=Leave here=true mark=true",
            ),
        ],
        plain: "rename_carry kind=tag rows=3 pick=none shown=true answerable=true hold=false \
                neutral=true pill=Create here=true mark=true",
    },
    // The same road with the answer given: the lines above plus the write
    // (judged by the run's `write-failures`); no argument means `replace`.
    Verb {
        name: "rename-tag-remote-go",
        when: &[
            (
                Arg::Ends(":add"),
                "rename_carry kind=tag rows=3 pick=add shown=true answerable=true hold=false \
                 neutral=true pill=Create here=true mark=true",
            ),
            (
                Arg::Ends(":leave"),
                "rename_carry kind=tag rows=3 pick=leave shown=true answerable=true hold=false \
                 neutral=true pill=Leave here=true mark=true",
            ),
        ],
        plain: "rename_carry kind=tag rows=3 pick=replace shown=true answerable=true hold=true \
                neutral=false pill=Replace here=true mark=true",
    },
    // …with a hand on the held pill, the one answer whose tip says the
    // running order. `tip=true`: the words opened.
    Verb {
        name: "rename-tag-remote-tip",
        when: &[],
        plain: "rename_carry kind=tag rows=3 pick=replace shown=true answerable=true hold=true \
                neutral=false pill=Replace here=true mark=true tip=true",
    },
    // Every door onto the history, graph and left pane, while a rebase
    // replays behind the screen. A refused door frames like an untouched
    // window, so the doors are the line and the picture is the rows around
    // them staying. `frozen=false` tells this apart from the plan's freeze,
    // which takes the pane whole.
    Verb {
        name: "doors-held",
        when: &[],
        plain: "doors_held held=true frozen=false road=true rowmenu=true drop=true \
                box=false plus=true menu=true switchrow=true why=true",
    },
    // The row taken away before git answered
    // (デザイン規約 §消す操作は先に画面から消す). `row=-1` is the sidebar
    // without it and `chips=true` the graph without its chip; the pair is
    // the rule, since a run reaching the state after the refs landed says
    // the same `row=-1`.
    Verb {
        name: "delete-gone",
        when: &[],
        plain: "gone_row tag=v0.3-local row=-1 total=2 chips=true",
    },
    // …and the same row let go of again: a row gone from the list draws
    // like a row the window is drawing without. `stood=true` is the press
    // having taken it away and `chips=false` the stand-in over; either
    // alone passes a window that never stood the row in or never let go.
    // Put down by counting listings (`ops::StandIn`), a stand-in can end
    // on a reading that never saw the write, or never — the second waits
    // out the ceiling here.
    Verb {
        name: "delete-stood-down",
        when: &[],
        plain: "stood_down tag=v0.3-local stood=true row=-1 total=2 chips=false",
    },
    // The delete row dressed with git's answer before any click: an
    // unasked row and one answered merged both wear `branch --delete`, so
    // the line is the only witness. `asked=` also fails a card opened while
    // a delete was out, otherwise a silent watchdog
    // (`AutoActRefVerbs.earlyDeleteTimer`). `from=` stays out of the claim:
    // an in-window branch answers off the drawn rows, others off git, by
    // the refs' timing. `merged=` is one word on both roads; `unknown` (the
    // reads fell over) fails here.
    Verb {
        name: "delete-branch-early",
        when: &[(
            Arg::Is("feature/topic-a"),
            "merged=no code=branch -D held=true note=not merged",
        )],
        plain: "delete_early asked=true",
    },
    // The same card over a branch whose tip no drawn row holds, so
    // `from=git` is claimed: `--preset deep-parked` forks `parked` below the
    // window's cut, which sends the question to `merge-base`
    // (`GraphModel::branch_delete_merged` empty →
    // `RepoSession::check_branch_delete`). `merged=no` is the half nothing
    // else reaches — git's answer is compared as a string in one place
    // (`RefBranchMenu.refusedRow`), on this road alone, and a wrong word
    // there silently offers the plain delete. `plain` because the verb has
    // one repository and one branch; anything else should fail.
    Verb {
        name: "delete-branch-early-far",
        when: &[],
        plain: "delete_early asked=true from=git merged=no code=branch -D held=true \
                note=not merged",
    },
    // The three deletes of the left row's card, each judged first on the
    // press having left the page: the page drops a delete asked while a
    // write runs (`RepoPage.deleteRow`), and the run would then wait on a
    // write nobody made, silent until the watchdog. It is the one claim the
    // three share; what each does after the press is its own.
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
    // The delete table's greyed rows with their tooltip up. The sentences
    // are read by `tests/qml/tst_branchcard.qml`; only `:remote` claims its
    // own here. A window adds the two lookups the rows are told apart by,
    // one run each: `:remote` that `upstreamDrifted` arrived, `worktrees`
    // that the copy holding the branch did. No run on the current branch:
    // `current` is core's own word, so that row stands on no lookup.
    Verb {
        name: "delete-blocked-tip",
        when: &[
            // The current branch of `basic`, whose reading has moved: the
            // sentence names that reading, so the row is this one run.
            (
                Arg::Is(":remote"),
                "delete_blocked code=push --delete tip=true aside=true holder= \
                 reason=Differs from origin/main",
            ),
            (
                Arg::Ends(":remote"),
                "delete_blocked code=push --delete tip=true aside=true holder= reason=Differs from ",
            ),
            // `holder=` is the folder of the copy holding this branch —
            // the witness that the card looked the copy up.
            (
                Arg::Is("feature/topic-a"),
                "delete_blocked code=branch --delete tip=true aside=true holder=topic",
            ),
        ],
        plain: "delete_blocked code=branch --delete tip=true aside=true",
    },
    // A working copy's WORKTREE card, raised from its WORKTREES row. The
    // picture shows a greyed row; only the line says it is out rather
    // than pale, which sentence its hover gives, and whether the menu
    // stands on the branch the copy has out (`branch=`) — the two locks of
    // `--preset worktrees` (with and without a reason), a copy on a branch
    // and one on none. `held=false`: the row is a click; `cut=false`: the
    // folder came out whole, which a Linux font once missed by a pixel.
    // `longnames`' copy is cut on purpose: its forced hover gives the whole
    // line back. `says=` is the tip's text.
    Verb {
        name: "worktree-menu",
        when: &[
            (
                Arg::Is("a-very-long-working-copy-folder-name"),
                "worktree_menu copy=a-very-long-working-copy-folder-name card=true offered=true \
                 blocked=false tip=true code=worktree remove branch=true held=false cut=true \
                 says=worktree remove a-very-long-working-copy-folder-name",
            ),
            (
                Arg::Is("hotfix"),
                "worktree_menu copy=hotfix card=true offered=true blocked=true tip=true \
                 code=worktree remove branch=true held=false cut=false \
                 says=Locked — release run is using this checkout",
            ),
            (
                Arg::Is("spike"),
                "worktree_menu copy=spike card=true offered=true blocked=true tip=true \
                 code=worktree remove branch=true held=false cut=false \
                 says=This working copy is locked",
            ),
            (
                Arg::Is("detached"),
                "worktree_menu copy=detached card=true offered=true blocked=false tip=false \
                 code=worktree remove branch=false held=false cut=false",
            ),
            // The repository's own copy: its branch's menu, and a card that
            // makes copies but has no `worktree remove` — git never removes it.
            (
                Arg::Is("repo"),
                "worktree_menu copy=repo card=true offered=false blocked=false tip=false \
                 code=worktree remove branch=true",
            ),
            (
                Arg::WithPreset("worktrees"),
                "card=true offered=true blocked=false tip=false code=worktree remove",
            ),
        ],
        // No preset: a copy of `demo-repo worktrees` opened with `--repo`
        // is the one this tab stands in (verbs.md).
        plain: "card=true offered=true blocked=true tip=true code=worktree remove branch=true \
                held=false cut=false says=This tab is showing this working copy",
    },
    // The same card from the graph: the row the copy stands on, its menu
    // aimed at the copy's folder chip (`detached`) or at the branch chip
    // of the copy that has it out (`topic`). `:go` presses it, and the
    // copy leaves the list as from the sidebar.
    Verb {
        name: "worktree-graph",
        when: &[
            (Arg::Ends(":go"), "worktree_gone row=-1 log=false"),
            (
                Arg::Is("detached"),
                "worktree_menu copy=detached card=true offered=true blocked=false tip=false \
                 code=worktree remove branch=false held=false cut=false",
            ),
            (
                Arg::Is("topic"),
                "worktree_menu copy=topic card=true offered=true blocked=false tip=false \
                 code=worktree remove branch=true held=false cut=false",
            ),
        ],
        plain: "card=true offered=true code=worktree remove",
    },
    // The copy leaves the list for good (`row=-1` once the listing that
    // saw the removal is drawn), and a landing raises no log.
    Verb {
        name: "worktree-remove",
        when: &[],
        plain: "worktree_gone row=-1 log=false",
    },
    // git keeps a copy holding work: the report bar in the gesture's
    // colour, the log left shut. `why=true` is the screen's own sentence
    // — git's ends in advice to force it; `mark=true` the tree mark in
    // front of the copy's name in the heading.
    Verb {
        name: "worktree-remove-refused",
        when: &[(
            Arg::Is("topic"),
            "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
             said=topic was not removed mark=true",
        )],
        plain: "write_notice open=true clears=true why=true tone=warning log=false wrong=false",
    },
    // The two rows that make a working copy, on the WORKTREE card
    // (デザイン規約 §作業コピーを作る — `card=` is that card opened).
    // `here=` is `Create worktree here…`, on every row with a commit;
    // `add=` is `worktree add`, on a branch no copy has out — not on the
    // tree's own branch (`main`) nor one another copy holds (`worktrees`'
    // `feature/topic-a`, whose `switch` row opens that copy instead).
    // `folder=` is where it would go, and `copy-places` puts something in
    // the way of two branches: the row is greyed and its forced hover says
    // which — a greyed row and a pale one are a picture apart, the
    // sentence none. Its one long branch is cut on the row (`cut=`), and
    // the forced hover gives the whole line back. The tip stands beside the
    // row (`aside=`) with the tree mark in front of the folder (`mark=`,
    // デザイン規約 §ref の種別); `says=` is the tip's text.
    Verb {
        name: "worktree-add-menu",
        when: &[
            (
                Arg::Is("branch:feature/blocked"),
                "copy_menu card=true here=true add=true folder=feature-blocked blocked=true tip=true \
                 aside=true mark=feature-blocked cut=false \
                 says=The folder is not empty — feature-blocked",
            ),
            (
                Arg::Is("branch:fix/listed"),
                "copy_menu card=true here=true add=true folder=fix-listed blocked=true tip=true \
                 aside=true mark=fix-listed cut=false \
                 says=Taken by another working copy — fix-listed",
            ),
            (
                Arg::Has("long-enough"),
                "copy_menu card=true here=true add=true \
                 folder=feature-a-name-long-enough-to-cut-its-folder blocked=false tip=true aside=true \
                 mark=feature-a-name-long-enough-to-cut-its-folder cut=true \
                 says=worktree add feature-a-name-long-enough-to-cut-its-folder",
            ),
            (
                Arg::WithPreset("worktrees"),
                "copy_menu card=true here=true add=false folder= blocked=false",
            ),
            (
                Arg::Is("branch:main"),
                "copy_menu card=true here=true add=false folder= blocked=false",
            ),
            // A tag has no branch to take out: only the new one.
            (
                Arg::Starts("tag:"),
                "copy_menu card=true here=true add=false folder= blocked=false",
            ),
            (
                Arg::Has("remote-only"),
                "copy_menu card=true here=true add=true folder=feature-remote-only blocked=false tip=false \
                 aside=false mark= cut=false",
            ),
        ],
        plain: "copy_menu card=true here=true add=true folder=feature-topic-a blocked=false tip=false \
                aside=false mark= cut=false",
    },
    // The box `Create worktree here…` opens, and what it turns down before
    // the press: a name git would not take, one a branch already has, and
    // one whose folder holds something (`feature-blocked` is where the
    // branch `feature/blocked` would go as well). `tip=` the reason up in
    // the box's own tip, `mark=` the folder its tree mark stands before —
    // a sentence about a branch has none.
    Verb {
        name: "worktree-box",
        when: &[
            (
                Arg::Ends("=feature-blocked"),
                "copy_box where=graph open=true mode=worktree refused=true tip=true \
                 mark=feature-blocked why=The folder is not empty — feature-blocked",
            ),
            (
                Arg::Ends("=feature/free"),
                "copy_box where=graph open=true mode=worktree refused=true tip=true mark= \
                 why=A branch called that already exists",
            ),
            // A name opening with `-`, which git would run as an option of
            // its own `git branch` (`branch::is_valid_name`).
            (
                Arg::Ends("=-m"),
                "copy_box where=graph open=true mode=worktree refused=true tip=true mark= \
                 why=git will not take this as a name",
            ),
            (
                Arg::Ends("@graph"),
                "copy_box where=graph open=true mode=worktree refused=false tip=false mark= why=",
            ),
        ],
        plain: "copy_box where=nav open=true mode=worktree refused=false tip=false mark= why=",
    },
    // A copy made and the tab stood in it: the folder the rule names, the
    // branch out in it, one tab (the tab moved, not grew), the page kept,
    // and no log raised by the landing.
    Verb {
        name: "worktree-new",
        when: &[(
            Arg::Ends("=feature/next"),
            "copy_made stood=true copy=feature-next branch=feature/next tabs=1 kept=true \
             log=false",
        )],
        plain: "copy_made stood=true copy=hotfix-patch branch=hotfix/patch tabs=1 kept=true \
                log=false",
    },
    // The row's own branch out in a new copy: a local branch as it is, a
    // remote one with no local branch made local and following it.
    Verb {
        name: "worktree-add",
        when: &[(
            Arg::Has("remote-only"),
            "copy_made stood=true copy=feature-remote-only branch=feature/remote-only tabs=1 \
             kept=true log=false",
        )],
        plain: "copy_made stood=true copy=feature-topic-a branch=feature/topic-a tabs=1 \
                kept=true log=false",
    },
    // A name taken between the box and git: the copy was not made, said
    // over the graph in the gesture's colour with the log left shut
    // (デザイン規約 §作業コピーを作る), the heading naming the folder behind its tree mark.
    Verb {
        name: "worktree-new-refused",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                said=hotfix-patch was not created mark=true",
    },
    // An elided row whose hover says the whole name. The wider lines name
    // no pane: `tree=` echoes the view asked for (`-tree`, where row 0 is
    // the elided folder chain), `tip=` the shared instance's visible. The
    // carried pane is also claimed by its words: its model folds by
    // `<run>:<path>` and keeps the path beside it, so a folder row handed
    // the fold key says `whole:src`, and the hover is the only place the
    // path is spelled out. `topic` is the copy whose one file is nested.
    Verb {
        name: "path-tip",
        when: &[
            (
                Arg::Is("carried:topic-tree"),
                "path_tip pane=carried tree=true tip=true aside=true text=src",
            ),
            (
                Arg::Is("carried:topic"),
                "path_tip pane=carried tree=false tip=true aside=true text=src/topic.txt",
            ),
            (Arg::Ends("-tree"), "tree=true tip=true aside=true"),
        ],
        plain: "tree=false tip=true aside=true",
    },
    // The same tooltip copied: `copied=` is the whole tip read back out of
    // a selection on it, false the moment the words stop being a field (a
    // `Text` in place of the field would frame identically).
    Verb {
        name: "tip-copy",
        when: &[],
        plain: "tip=true copied=true",
    },
    // …and swept from the air around the words — every place in a tip
    // nobody else takes is a start (規約 §hover のツールチップ); the starts
    // are a grid over the tip minus the points on the sentence. `all=` /
    // `hand=` as in verbs.md §面の掃き. `caret=`: a selection with no
    // keyboard on it is not one `Ctrl+C` can take
    // (デザイン規約「選択できることと持ち帰れることは別」).
    Verb {
        name: "tip-sweep",
        when: &[],
        plain: "tip_sweep all=true caret=true hand=true",
    },
    // A row's tooltip. `lit=` is the control: the first WORKTREES row
    // answered the same pointer on the way in (by opening — every
    // repository has a row there), so an empty overlay is the row's own
    // answer. `wants=` / `tip=` are the row's words and the shared instance
    // carrying them. Rows left on this side: the remote's own row, stashes,
    // the working tree's files (BRANCHES / REMOTES / WORKTREES go through
    // `nav-open`, tags `nav-open-tag`). `branch:2 --preset nested` is the
    // folder `backend` at `team/backend`: a tip built from the shown
    // segment instead of the fold key fails there, where a top-level folder
    // would spell the two the same.
    Verb {
        name: "nav-tip",
        when: &[(
            Arg::Is("branch:2"),
            "lit=true wants=true tip=true open=false aside=true text=team/backend",
        )],
        plain: "lit=true wants=true tip=true open=false aside=true",
    },
    // Where one click on a left-panel row leads. The window opens with a
    // row already lit, so a graph that jumped and one that never moved
    // frame the same. `same=` is the graph's row carrying the commit the
    // row named, `lit=` the light on it; `wip=false` because the history
    // must be the face on screen. `at=` / `name=` are the repository's
    // numbers and stay out of the claim.
    Verb {
        name: "nav-jump",
        when: &[],
        plain: "same=true lit=true marked=true wip=false",
    },
    // A left-panel row opens its facts under itself rather than raising
    // the shared tooltip (`NavRowFacts`): nothing in the tooltip, the row
    // open. What the facts come out as is `tests/qml/tst_navfacts.qml`'s,
    // over stood-in models; left for a window is the lookups — the real
    // model answering, for the row that asked — which `says=` claims, one
    // run per lookup a section makes. `says=` carrying a name is also the
    // row having opened (the sentence is empty until then).
    Verb {
        name: "nav-open",
        when: &[
            // The ordinary branch row: the only claim reaching the
            // section's own lookup (`upstreamOf`) — the row below reads a
            // gone upstream, which is a role.
            (
                Arg::Is("branch:2"),
                "says=main local= track=1/0 held= up=origin/main gone=false",
            ),
            // A reading git can no longer reach: `gone` is a role and
            // `upstreamOf` answers only refs that exist, so a row reading
            // just one of the two loses this state (`up=` empty,
            // `gone=false`).
            (
                Arg::Is("branch:1"),
                "says=release-1.2 local= track=0/0 held= up=origin/release-1.2 gone=true",
            ),
            // The current branch's stand-in builds its own row's worth of
            // answers (`HeadPinRow.gatherFacts`) — a second way in, and the
            // only one whose counts are the section's `headAhead` /
            // `headBehind`.
            (
                Arg::Is("head:topic"),
                "says=main local= track=1/0 held= up=origin/main gone=false",
            ),
            // The stand-in a fold makes, with a hand on it (`nav-pin-seat`
            // is the seat with none): opening pushes the rows below, so a
            // stand-in placed by counting whole rows leaves its seat here.
            (Arg::Is("head::1"), "open=true under=1 sat=true"),
            (
                Arg::Is("remote:1:topic-a"),
                "says=origin/feature/topic-a local=feature/topic-a track=1/0 held=topic",
            ),
            // WORKTREES rows on `--preset worktrees` (every annotation
            // `git worktree list` prints). The folder name leads; the path
            // is this machine's and rides at the tail. The state is
            // claimed, not its words (a lock draws no line — the padlock
            // is all of it). `worktree:1` holds a branch reading a remote:
            // the lines and counts are that branch's, which no role of
            // this row carries.
            (
                Arg::Is("worktree:1"),
                "says=topic local= track=1/0 held= up=origin/feature/topic-a gone=false \
                 branch=feature/topic-a",
            ),
            (
                Arg::Is("worktree:2"),
                "says=gone local= track=0/0 held= up= gone=false branch=gone/branch state=PRUNABLE",
            ),
        ],
        plain: "wants=false tip=false open=true",
    },
    // A TAGS row, which opens on the remotes carrying its name; one, two
    // and no carriers all frame as a row with lines under it. `remotes=`
    // is what the lines name; `against=` / `apart=` which reading the
    // others are read against and which stands elsewhere, both off the
    // index alone (`RemoteTagIndex::carriers_against`); `here_apart=`
    // the copy here weighed by the same test — what the row's own name
    // wears (`RemoteTagIndex::apart_at`). The empty list means something
    // only because the run fetches and waits for the reading first
    // (`AutoActNavBoxVerbs.navOpenTagTimer`).
    Verb {
        name: "nav-open-tag",
        // `says=` carrying the name is the row having opened; `open=` is
        // left out because a claim is one run of the line and the geometry
        // stands between them.
        when: &[
            // The marked line's supplement, the only place its colour is
            // explained; it names the reading it is apart from. The run
            // rests on the first line, which here is the one apart.
            (
                Arg::Is("v1.5:drift:tip"),
                "apart=fork here_apart=true by=origin why= path= tip=true text=Differs from origin",
            ),
            // The same for the row's own name: the copy here stands where
            // the fork does, off origin's commit, and says so in the same
            // words.
            (
                Arg::Is("v1.5:drift:name"),
                "apart=fork here_apart=true by=origin why= path= tip=true text=Differs from origin",
            ),
            // Origin silent, fork and mirror agreeing: they decide, and the
            // copy here — moved off their commit — is the one apart, named
            // against both of them.
            (
                Arg::Is("v3.1-moved:name"),
                "remotes=fork,mirror against=origin apart= here_apart=true by=fork,mirror \
                 why= path= tip=true text=Differs from fork, mirror",
            ),
            // Origin silent, fork and mirror disagreeing: nobody decides,
            // and every holder — the copy here too — stands apart.
            (
                Arg::Is("v3.2-split:tip"),
                "remotes=fork,mirror against=origin apart=fork,mirror here_apart=true by= \
                 why= path= tip=true text=Remotes disagree",
            ),
            // Only fork and mirror, agreeing, and nothing here: nobody apart.
            (
                Arg::Is("v3.0-pair"),
                "says=v3.0-pair local= track=0/0 held= up= gone=false branch= state= \
                 remotes=fork,mirror against=origin apart= here_apart=false by=fork,mirror",
            ),
            // Carried by both remotes, on one commit.
            (
                Arg::Is("v1.0:remote"),
                "says=v1.0 local= track=0/0 held= up= gone=false branch= state= \
                 remotes=fork,origin against=origin apart= here_apart=false",
            ),
            // The two remotes hold the name on different commits; the ones
            // marked are those apart from the reading this window acts on —
            // the fork, and the local tag standing where the fork is.
            (
                Arg::Is("v1.5:drift"),
                "says=v1.5 local= track=0/0 held= up= gone=false branch= state= \
                 remotes=fork,origin against=origin apart=fork here_apart=true",
            ),
            // A name only `fork` has (no local ref): nobody stands apart,
            // since the reference remote has no reading of it.
            (
                Arg::Is("v0.9-theirs:remote"),
                "says=v0.9-theirs local= track=0/0 held= up= gone=false branch= state= \
                 remotes=fork against=origin apart= here_apart=false",
            ),
            // And the row that opens on nothing at all.
            (
                Arg::Is("v2.0-local"),
                "says=v2.0-local local= track=0/0 held= up= gone=false branch= state= \
                 remotes= against=origin apart= here_apart=false",
            ),
        ],
        plain: "open=true",
    },
    // A menu asked for on one carrier's line of an open TAGS row: that
    // remote, named on its own, is the one every row reaching over there
    // acts on — even where nobody could be picked unasked (`v3.0-pair`,
    // carried by fork and mirror, origin silent). `reach=` is what no
    // picture of a row that names no remote could say.
    Verb {
        name: "tag-line-menu",
        when: &[(
            Arg::Is("v3.0-pair:1"),
            "tag_reach reach=mirror offered=true blocked=false push=to mirror \
             del=v3.0-pair from mirror why=",
        )],
        plain: "tag_reach reach=",
    },
    // The rest that opened a row, held, with the working copy's path as
    // the tip. `open=`: a tip over a row that never opened is the shared
    // instance answering for something else. `same=` is the box's words
    // against what the row asks for (`NavRowFacts.said`) — an empty box
    // answers `tip=true` too, and the path is this machine's, so it cannot
    // be a literal.
    Verb {
        name: "nav-open-tip",
        when: &[],
        plain: "open=true tip=true same=true",
    },
    // A press on a closed row's line that starts to move without waiting
    // out the rest: the lines came out at once (`open=`), the drag reached
    // them (`copied=`), and the row took no click (`clicked=false`).
    Verb {
        name: "nav-drag-open",
        when: &[],
        plain: "open=true caret=true copied=true clicked=false",
    },
    // The same rest on the last row of an overfull section
    // (`--preset stack`): the list has to scroll, or the lines open under
    // its bottom edge. `shown=` weighs where the row landed against what
    // the list shows, not the scroll asked for (a list noting a move it
    // never made would vouch for itself); with `open=`, since an unopened
    // row shows whole too. `away`: the list scrolls back when the hand
    // leaves, and `back=` is the run's own note of where it stood — the
    // list's memory is what is tested, so it cannot be the witness. `bar`:
    // taking the list's bar closes the row at the grab (`shut=`) and gives
    // the list back the same way, and the row asked for again while the bar
    // is held stays shut.
    Verb {
        name: "nav-open-foot",
        when: &[
            (Arg::Is(""), "open=true shown=true"),
            (Arg::Is("away"), "open=false shown=false back=true"),
            (Arg::Is("bar"), "open=false shown=false back=true shut=true"),
        ],
        plain: "open=",
    },
    // The open row, then the hand's next move, one line each: it closes on
    // leaving, on a name box, on a menu over the list, goes with the row a
    // filter removes, and stays under a menu it raised itself. `lit=`: an
    // open row wears the wash whole, and one dark under its own lines
    // reads as a row nothing is on.
    Verb {
        name: "nav-open-then",
        // Each claim is one run of the line: the words between are claimed.
        when: &[
            (Arg::Is("away"), "open=false lit=false box=false"),
            (Arg::Is("edit"), "open=false lit=true box=true"),
            (Arg::Is("menu"), "open=false lit=true box=false menu=true"),
            (
                Arg::Is("filter"),
                "open=false lit=false box=false menu=false rows=0",
            ),
            // A menu raised on the open row — its lines, or its own line —
            // keeps the row open: it is what the menu is about; `menu`
            // above is the pair that closes.
            (
                Arg::Is("rightclick"),
                "open=true lit=true box=false menu=true",
            ),
            (
                Arg::Is("rightclick-name"),
                "open=true lit=true box=false menu=true",
            ),
            // A drag over the lines copies and the row hears nothing (its
            // second click would open a name box); a press that never moved
            // is the row's click. `own=`: the graph went to the row's own
            // commit — the tap lands on the lines' foot, which no band
            // covers, and a tap caught by a band would go where that line
            // names while leaving the same click mark.
            (Arg::Is("sweep"), "caret=true copied=true clicked=false"),
            (Arg::Is("tap"), "copied=false clicked=true own=true"),
        ],
        plain: "open=",
    },
    // A name box first, then a hand on its row: nothing may open, since
    // opening moves the rows below and the box with them. `lit=true` says
    // the hand landed, so `open=false` is the row staying shut.
    Verb {
        name: "nav-open-held",
        when: &[],
        plain: "box=true open=false lit=true",
    },
    // A line of an open row that names another commit, pressed: the graph
    // goes there and the panel stays. `followed=` the line had somewhere
    // to go, `landed=` the graph's row for it is lit, `marked=` the panel's
    // click is still the open row's; `box=false` because a click that armed
    // the name gesture would open a box a moment later.
    Verb {
        name: "nav-follow",
        when: &[],
        plain: "followed=true landed=true marked=true box=false",
    },
    // The pointer resting on that line: the band drawn there (`aimed=`)
    // and the hand cursor (`hand=`), from two parts; the row still open.
    Verb {
        name: "nav-follow-lit",
        when: &[],
        plain: "aimed=true hand=true open=true",
    },
    // The same opening inside the folded rail's popup: `peek=true` is the
    // popup surviving the rows under it moving.
    Verb {
        name: "nav-peek-open",
        when: &[],
        plain: "peek=true open=true",
    },
    // The four ways the pointer leaves the folded rail's open section:
    // `peek=` empty is the section gone, `peek=<kind>` standing, and only
    // `-into` (the hand inside what it opened) must stand
    // (verbs.md の `nav-peek-shut` の行). A section on its way out
    // (`HoverCardHost` waits a beat) frames like one standing, so the eye
    // passes the state these verbs disprove. `top=` follows because
    // `peek=` alone is a prefix of `peek=branch`; `width=` is the rail's
    // and stays out of the claim.
    Verb {
        name: "nav-peek-away",
        when: &[],
        plain: "peek= top=",
    },
    Verb {
        name: "nav-peek-out",
        when: &[],
        plain: "peek= top=",
    },
    Verb {
        name: "nav-peek-shut",
        when: &[],
        plain: "peek= top=",
    },
    Verb {
        name: "nav-peek-into",
        when: &[],
        plain: "peek=branch top=",
    },
    // The folded rail's section and a standing menu: under a graph row's
    // menu it still opens at a rest and goes once the hand has left
    // (`behind`, read after the beat that closes a section left alone;
    // `opened=` is the rest's answer, or a section that never came out
    // would read as closed), and a graph row's menu raised over it takes it
    // down (`under`); the menu raised on its own row holds it with the hand
    // gone (`own`), and holds only that section — TAGS, rested on under
    // that menu, goes as a hover does (`own-swap`). `shown=` is the popup
    // itself — `peek=` clears as it starts to close.
    Verb {
        name: "menu-peek",
        when: &[
            (
                Arg::Is("own"),
                "menu=true opened=true peek=branch shown=true raised=sidebar",
            ),
            (
                Arg::Is("own-swap"),
                "menu=true opened=true peek= shown=false raised=sidebar",
            ),
        ],
        plain: "menu=true opened=true peek= shown=false raised=graph",
    },
    // The current branch's stand-in on the top edge its row scrolled out
    // of. A row filtered away (`nav-open head:<filter>`) draws the same,
    // and `rowshown=false` beside `above=true` tells them apart. `y=0`
    // pins where the edge is: the place is bound straight off `above`, so
    // only a margin or inset given to that binding moves it. `row=` /
    // `rested=` are the repository's numbers and stay out of the claim.
    Verb {
        name: "nav-pin-edge",
        when: &[],
        plain: "nav_pin_edge pin=true above=true rowshown=false name=main y=0",
    },
    // The stand-in with no row because a fold shut over it, no hand on it
    // (the hand is `nav-open head::<row>`). `sat=` reads both ends — the
    // stand-in's own `y` against the row holding the seat — so other
    // arithmetic parts from it. `says=` is claimed per fold: folders still
    // on screen are left out of the name, the ones the fold shut are not;
    // `depth=` is that same folder's column.
    Verb {
        name: "nav-pin-seat",
        when: &[
            (
                Arg::Is("1"),
                "under=1 sat=true lit=false depth=0 says=team/backend/api/fix-auth",
            ),
            (
                Arg::Is("2"),
                "under=2 sat=true lit=false depth=1 says=backend/api/fix-auth",
            ),
            (
                Arg::Is("3"),
                "under=3 sat=true lit=false depth=2 says=api/fix-auth",
            ),
            // A row above opens and pushes the seat down; a stand-in
            // placed by counting whole rows stays behind over the opened
            // lines. `open=true` is the push, `sat=` the stand-in following.
            (Arg::Is("2:0"), "open=true under=2 sat=true"),
        ],
        // One run of the line (`pin=` sits between `under=` and these);
        // `sat=` cannot be true with nothing standing.
        plain: "sat=true lit=false",
    },
    // The left menu's rename gesture in the folded rail's section: a click
    // after the section went away and came back is an ordinary click
    // (`-away`). The run that armed nothing frames like `nav-peek`, so
    // `armed=` is the row's answer to the click and `collapsed=` / `box=`
    // what came of it.
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
    // The same by the menu's `beginRename`: a box opens where its row is,
    // so a peeked row keeps the fold and what it was made for. `diff=`:
    // only the second verb has a file open to lose.
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
    // The row the box lands on, scrolled back into view (a section's last
    // rows and its first frame alike).
    Verb {
        name: "nav-rename-far",
        when: &[],
        plain: "shown=true box=true",
    },
    // The box walked away from with nothing typed, two ways — a pane with
    // no box frames like one that never opened it.
    Verb {
        name: "nav-rename-drop",
        when: &[(
            Arg::Is("fold"),
            "nav_drop how=fold collapsed=true box=false",
        )],
        plain: "nav_drop how=away collapsed=false box=false",
    },
    // The two name boxes a sidebar row opens, at the width the argument
    // dragged the pane to: the cut is the picture's; `focused=` / `shown=`
    // are not (a box nobody can type into frames like a waiting one).
    // `:away` scrolls the list past the box's row, and the box — drawn
    // outside its list — has to leave with it (`shown=false`).
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
    // the row in TAGS, so the barrier alone passes on an unchanged
    // sidebar; `at=` is the commit asked for, since a count alone passes
    // a tag left on HEAD.
    Verb {
        name: "create-tag",
        when: &[(Arg::Ends(":box"), "create_tag box=true mode=tag")],
        plain: "create_tag tag=v9.9-new row=1 total=4 at=true",
    },
    // The field's third mode, on the same line; which mode it came up in
    // is the picture's (the box reads `Create tag here?`).
    Verb {
        name: "nav-tag-box",
        when: &[],
        plain: "open=true focused=true shown=true",
    },
    // A jump to a match far down sends HEAD's row off, and its stand-in
    // must dim with the rows the search passed over — lit, it reads as a
    // match. `150` makes that jump on `deep` (HEAD row 0, first match row
    // 591); other runs still reach the line after the jump. No spaces in
    // the query: the census replays a line word by word.
    Verb {
        name: "find",
        when: &[(Arg::Is("150"), "pin=dim")],
        plain: "find_settled shift=",
    },
    // The search drawn, then scrolled: the scroll waits for the frame that
    // drew the search, or the run is `find` again. On `deep`, `5` scrolls
    // HEAD's row (no 5 in it) off the top.
    Verb {
        name: "find-scroll",
        when: &[(Arg::Is("5"), "find_scroll grabbed=true moved=true pin=dim")],
        plain: "find_scroll grabbed=true moved=true",
    },
    // A press elsewhere on the graph takes an empty box away and leaves a
    // typed one standing; the picture holds one state, and `shown=` says
    // which end of the card's fade it caught.
    Verb {
        name: "find-drop",
        when: &[(Arg::Is(""), "find_drop open=false shown=false")],
        plain: "find_drop open=true shown=true",
    },
    // The note under the find card once the typing stops: hex under the
    // hash floor has one, a line with any other character or four hex
    // characters none (`head:4` types HEAD's first four).
    Verb {
        name: "find-hint",
        when: &[
            (
                Arg::Is("head:4"),
                "find_hint stopped=true shown=false matches=1",
            ),
            (
                Arg::OneOf(&["", "fix", "feat", "zzz"]),
                "find_hint stopped=true shown=false",
            ),
        ],
        plain: "find_hint stopped=true shown=true",
    },
    // The keystroke after the stop takes the note down, though the line is
    // still short of a hash.
    Verb {
        name: "find-hint-key",
        when: &[],
        plain: "find_hint_key before=true after=false stopped=false",
    },
    // The panel's own mark opening the find bar; the key opens the same
    // picture, so the press reaching the page from the mark is the line.
    Verb {
        name: "band-find",
        when: &[],
        plain: "band_find open=true",
    },
    Verb {
        name: "name-box-drop",
        when: &[(Arg::Has(":"), "name_drop box=true")],
        plain: "name_drop box=false",
    },
];

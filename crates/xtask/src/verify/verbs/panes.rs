//! What a pane is judged against: the room it was given, and the grip
//! that asks for more of it.
//!
//! `solo` is here as the same kind of claim about the run itself: the
//! window had the machine to itself, which no picture shows.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    Verb {
        name: "solo",
        when: &[],
        plain: "solo blocked=true",
    },
    // The same gate screen, swept (verbs.md §面の掃き): it stands before
    // any repository opens, so no command log carries its words.
    // `held=true`: the path is on screen only while another build holds
    // the store.
    Verb {
        name: "gate-sweep",
        when: &[],
        plain: "gate_sweep all=true caret=true hand=true held=true",
    },
    // The two failures a folder that will not open lands on: the modal
    // dialog has no seat for the log, and the tab's screen has one only
    // for git's half — the folder is written nowhere else. `kind=` says
    // which of the three screens was swept (only one has a line from git).
    Verb {
        name: "open-dialog-sweep",
        when: &[],
        plain: "open_dialog_sweep all=true caret=true hand=true kind=plain",
    },
    Verb {
        name: "open-fail-sweep",
        when: &[],
        plain: "open_fail_sweep all=true caret=true hand=true kind=plain",
    },
    // `fits`: a child that will not shrink floors the column, and every
    // box paints past the window's edge — framing like a pane that fits.
    // `fills`: the author row got the whole block, so the right-aligned
    // hash plate sits at the edge rather than against the end of the name.
    Verb {
        name: "details-fit",
        when: &[],
        plain: "details_fit fills=true fits=true",
    },
    // The parent line cut to the room the row leaves it: nothing on the
    // name or the date line gave way to it, and whatever it left out is
    // counted. How many digits and links stand is the font's, so only the
    // merge of two (`--preset octopus` row 8) names the count — and the
    // stash filed with its untracked files (`--preset stashes` row 0),
    // whose three parents in git are one in the graph.
    Verb {
        name: "details-parents",
        when: &[
            (
                Arg::Starts("8"),
                "details_parents floor=true said=true order=true even=true crowd=false fits=true \
                 of=2 drawn=2 more=0",
            ),
            (
                Arg::WithPreset("stashes"),
                "details_parents floor=true said=true order=true even=true crowd=false fits=true \
                 of=1 drawn=1 more=0 git=3",
            ),
        ],
        plain: "details_parents floor=true said=true order=true even=true crowd=false fits=true",
    },
    // The count's card: its first hash on a single parent's seat, the
    // arrow left outside it, nothing else of the line beside it, every hash
    // whole, and the plate as wide as before, so the name beside it holds
    // still. The eight rows are `--preset octopus` row 0's.
    Verb {
        name: "details-parents-card",
        when: &[(
            Arg::WithPreset("octopus"),
            "parents_card open=true seat=true clear=true bare=true whole=true still=true rows=8 of=8",
        )],
        plain: "parents_card open=true seat=true clear=true bare=true whole=true still=true",
    },
    // The author row's two lines, each on one baseline whatever the faces
    // (the name's and the hashes' mono): a pixel off is what the eye reads
    // as crooked, and no picture is asked for a pixel.
    Verb {
        name: "details-align",
        when: &[],
        plain: "details_align line1=true line2=true sizes=true apart=true face=true",
    },
    // The credit on the date row: whole where the row has room for it
    // (`--preset co-authors` row 4), folded at the pane's floor beside a
    // merge's parents (`--preset octopus-mates`) — the face for the first
    // and the count unchanged, or the face alone for one; no name cut to a
    // sliver.
    Verb {
        name: "co-authors",
        when: &[
            (
                Arg::Is("0:min"),
                "co_authors count=3 cut=true first=Claude Opus 5 open=false folded=true said=+2",
            ),
            (
                Arg::Is("8:min"),
                "co_authors count=1 cut=true first=Claude Opus 5 open=false folded=true said=-",
            ),
        ],
        plain: "co_authors count=3 cut=false first=Claude Opus 5 open=false folded=false said=+2",
    },
    // A press on the card's row: the page lands on that parent and the
    // card goes down with the press.
    Verb {
        name: "details-parents-go",
        when: &[],
        plain: "parents_go landed=true shut=true",
    },
    // `Ctrl+A` on a value field reaches the whole of it, cut tail
    // included; the claim is that what came out is what the model says the
    // pane shows (a `Text` in place of the field draws the identical row).
    Verb {
        name: "details-select",
        when: &[],
        plain: "details_select match=true",
    },
    // One selection in the window: sweeping a second value drops the
    // first (`dropped=`) and keeps the second (`held=`) — a pane that
    // cleared everything would pass on the first alone.
    Verb {
        name: "details-select-away",
        when: &[],
        plain: "details_select_away dropped=true held=true",
    },
    // A summary box's right-click menu, read off the card: the rows on the
    // table and the ones that can be pressed, in order. A greyed row and a
    // lit one differ by a shade the picture leaves to the eye.
    Verb {
        name: "field-menu",
        when: &[
            (
                Arg::Is("typed"),
                "field_menu box=typed opened=true \
                 rows=Undo,Redo,Cut,Copy,Paste,Delete,Select all live=Undo,Paste,Select all",
            ),
            (
                Arg::Is("selected"),
                "field_menu box=selected opened=true \
                 rows=Undo,Redo,Cut,Copy,Paste,Delete,Select all \
                 live=Cut,Copy,Paste,Delete,Select all",
            ),
            (
                Arg::Is("readonly"),
                "field_menu box=readonly opened=true rows=Copy,Select all live=Copy,Select all",
            ),
        ],
        // `empty`, and no argument.
        plain: "field_menu box=empty opened=true \
                rows=Undo,Redo,Cut,Copy,Paste,Delete,Select all live=Paste",
    },
    // The menu key on a surface holding the keyboard: the card stood, and
    // under the left end of the row the keyboard is on (the diff: under its
    // row, right of the row's head). `pointer` is Qt's request with a place
    // of its own — xcb's menu key at the pointer — which the graph turns
    // away: a card there is the pointer's place answered for the keyboard.
    Verb {
        name: "key-menu",
        when: &[
            (
                Arg::Is("graph:request"),
                "key_menu on=graph via=request opened=true under=true",
            ),
            (
                Arg::Is("graph:pointer"),
                "key_menu on=graph via=pointer opened=false",
            ),
            (
                Arg::Starts("files:"),
                "key_menu on=files via=key opened=true under=true",
            ),
            (
                Arg::Starts("diff:"),
                "key_menu on=diff via=key opened=true under=true",
            ),
            (
                Arg::Starts("nav:"),
                "key_menu on=nav via=key opened=true under=true",
            ),
        ],
        // `graph`.
        plain: "key_menu on=graph via=key opened=true under=true",
    },
    // A range selection is swept from the room beside the values (a few
    // characters, one line — aiming at them misses). Claimed together: the
    // sweep reaches the value, and a press on the value's box is still the
    // control there (`grabs=` / `ours=`), which on the plate's two is
    // `false` — the hit areas did not shrink. A sweep reaches what is on
    // screen; a cut tail is `details-select`'s.
    Verb {
        name: "details-sweep",
        when: &[
            (
                Arg::Starts("hash"),
                "details_sweep reach=9/9 caret=true grabs=false ours=false",
            ),
            (
                Arg::Starts("parent"),
                "details_sweep reach=9/9 caret=true grabs=false ours=false",
            ),
        ],
        plain: "details_sweep reach=9/9 caret=true grabs=true ours=true",
    },
    // The plate's gesture: a click copies the whole hash, a drag picks out
    // the shown one — neither is on screen. `quiet=` is the half `held=`
    // does not give; `let=` the press releasing the drag before it, or the
    // plate copies from under a stale wash; `hand=`: the run calls the
    // hand's functions, so a hand removed, disabled or shrunk would still
    // answer. Whether Qt delivers a real press is `qmltest`'s
    // (tst_hashplate).
    Verb {
        name: "details-hand",
        when: &[],
        plain: "details_hand acted=true held=true quiet=true let=true hand=true",
    },
    // The press's word, said on the tip the reader is already looking at
    // — a tip does not go down to change its mind. The words are not
    // spelled, so a translation stays green and a tip keeping the old
    // sentence goes red.
    Verb {
        name: "hash-tip",
        when: &[],
        plain: "hash_tip offered=true said=true",
    },
    // The order that breaks: the press lands before the tip arrives. The
    // attached tooltip takes its words when `visible` rises, so a tip
    // still counting comes up with the offer unless the answer re-arms it.
    // `counting=`: nothing was up when the press went in.
    Verb {
        name: "hash-tip-counting",
        when: &[],
        plain: "hash_tip_counting counting=true said=true",
    },
    // The author's name, capped at its own width so the signature's word
    // stays against it. A ceiling a fraction of a pixel under that width
    // cuts the last glyph, which frames like the whole name. At the verbs'
    // window the row has 200px of slack — nothing is meant to give.
    Verb {
        name: "author-card",
        when: &[],
        plain: "author_card cut=false",
    },
    Verb {
        name: "author-card-open",
        when: &[],
        plain: "author_card cut=false",
    },
    // A pull past the pane's room draws what sits under the box over the
    // window's footer, framing like a pane that fits (as `details-fit`).
    // Only that is judged: whether the grip is offered depends on the
    // message.
    Verb {
        name: "details-grow",
        when: &[],
        plain: "description_grow keeps=true",
    },
    Verb {
        name: "details-grow-squeeze",
        when: &[],
        plain: "description_grow keeps=true",
    },
    Verb {
        name: "wip-grow",
        when: &[],
        plain: "description_grow keeps=true",
    },
    Verb {
        name: "wip-grow-squeeze",
        when: &[],
        plain: "description_grow keeps=true",
    },
    // Whether the corner text gives the pane's foot back. The working
    // tree's foot is the commit button's, so `wip` is always
    // `shown=false room=0`; the details pane goes either way by preset.
    // Judged on `shown=` (the label's own visibility), not `room=` (what
    // was asked): an instance writing its own `visible` over the corner's
    // stays green on `room=`. The pane is named, never defaulted
    // (`verify::options::name_the_verb`). `basic` leaves the corner bare
    // and `long` runs the list into it; keyed on the preset, a run that
    // read the room before the list arrived fails instead of passing as
    // both.
    Verb {
        name: "corner",
        when: &[
            (Arg::Is("wip"), "git_corner pane=wip shown=false room=0"),
            (
                Arg::WithPreset("long"),
                "git_corner pane=details shown=false room=0",
            ),
            // `room=` is left off: how much the one-file list leaves bare
            // is the platform's font.
            (
                Arg::WithPreset("basic"),
                "git_corner pane=details shown=true",
            ),
        ],
        plain: "git_corner pane=details",
    },
    // The two bars a panel carries, each judged as a pair: each says its
    // state in brightness alone, a few pixels one step apart. The panel's
    // slab is opaque, so the named ink is the report — a frame's ink while
    // the list is being sent, one step off the pane's ground once the
    // reader has left (デザイン規約 §QML 実装ルール のバーの明るさ). It is
    // the paint that landed; the asked-for `bright` stays green with it
    // cut off.
    Verb {
        name: "pane-bar",
        when: &[],
        plain: "pane_bar ink=#334155",
    },
    Verb {
        name: "pane-bar-away",
        when: &[],
        plain: "pane_bar ink=#0f172a",
    },
    // The description box's bar is the style's see-through one, reaching
    // the same two looks by carrying all or three tenths of its single ink.
    // It is the one bar handed to a `ScrollView`, whose flickable never
    // reports moving, so the lit half says the bar still watches the text.
    Verb {
        name: "text-bar",
        when: &[],
        plain: "text_bar ink=1",
    },
    Verb {
        name: "text-bar-away",
        when: &[],
        plain: "text_bar ink=0.3",
    },
    // The arrows at a bar's ends: a click is one step, a hold runs on and
    // holds the bar the way a drag does, sliding off stops the run with the
    // hand still down, and letting go stops it and gives the bar back. Each
    // is read off the view having moved or not, never off a clock.
    Verb {
        name: "bar-arrow",
        when: &[],
        plain: "bar_arrow click=40 held=true left=true stopped=true",
    },
    // The track between them: a click is one page, a hold runs on and
    // stops with the thumb's far end on the hand, a hand slid off keeps
    // the run from starting, and letting go gives the bar back.
    Verb {
        name: "bar-track",
        when: &[],
        plain: "bar_track page=true reached=true held=true left=true stopped=true",
    },
    // The middle button's hand left running on a surface: the anchor ring
    // stands the same over a list that moved and one that did not, so
    // `answered=` is read off the surface. In the working tree's two boxes
    // the platform decides — where it pastes on middle, `answered=` is the
    // hand stepping aside (`MiddleAutoScroll.claimedAt`).
    Verb {
        name: "middle-hand",
        when: &[(
            Arg::OneOf(&["wip-description", "wip-summary"]),
            "answered=true",
        )],
        plain: "answered=true took=true",
    },
    Verb {
        name: "settings-hand",
        when: &[],
        plain: "answered=true took=true",
    },
    // A commit git wrote, and its boxes let go. git answers before the
    // reading that redraws the pane (`session::write::run_write`), and the
    // press is two writes (stage, then commit), so the barrier may answer
    // for the first. `empty=true`: the answer found the editor that sent
    // it among what the notify carried (`ops::Press`) — a property would
    // be rewritten by the fetch behind it. `amend=false` for both verbs:
    // `amend` turns the row on first, and it goes down with the boxes.
    Verb {
        name: "commit",
        when: &[],
        plain: "commit_answered landed=true empty=true amend=false",
    },
    Verb {
        name: "amend",
        when: &[],
        plain: "commit_answered landed=true empty=true amend=false",
    },
    // A stopped merge takes the amend row away (git will not amend in the
    // middle of one, and the button is the merge's `--continue`); any other
    // tree with a commit keeps it. Neither shows in a picture of a row
    // that is simply not there. With a branch to merge (`--preset
    // clashing`) the box is ticked first: the merge's stop takes the tick
    // away with the row (`ticked=false amending=false`).
    Verb {
        name: "amend-row",
        when: &[
            (
                Arg::WithPreset("conflict-staged"),
                "amend_row shown=false barred=true",
            ),
            (
                Arg::Is("feature/clash"),
                "amend_row shown=false barred=true op=MERGING ticked=false amending=false",
            ),
        ],
        plain: "amend_row shown=true barred=false",
    },
    // The amend box ticked over a stage that takes back all HEAD changed
    // (`--preset undone`): git's answer arrived, and the button refuses
    // with the tip that says why. The run waits on the tip alone, so
    // `empties=` is what the answer was; a button refusing for an empty
    // summary looks the same, so the words are read too.
    Verb {
        name: "amend-tip",
        when: &[],
        plain: "amend_tip empties=true enabled=false tip=true why=Amending would leave the last commit empty",
    },
    // The offer to take HEAD's authorship over, standing only where there
    // is something to take. `differs=` is why the row is there (a window
    // that never read HEAD's author also leaves it off); the name, because
    // `differs=true` against an author that never arrived is the wrong
    // reason. What git records is `commit_integration.rs`'s
    // (`amending_keeps_the_author_until_reset_author_is_asked_for`).
    Verb {
        name: "amend-author",
        when: &[],
        plain: "amend_author differs=true name=Yuki Tanaka",
    },
    // The box ticked and sent, judged after the page is back: the amend
    // answers before the rebuild starts, so a run stopped at the write
    // photographs the replaced commit under the old author. Both names,
    // because a plain amend already moves the committer.
    Verb {
        name: "amend-reset-author",
        when: &[],
        plain: "author=Demo User committer=Demo User",
    },
    // A commit a hook would not have, read as `remote-refused`: `said=` is
    // built from what core classified, `says=` the line naming the hook and
    // sending the reader to the log for its words, `log=false wrong=false`
    // nothing raised as an error (デザイン規約 §答えの要らない報せ). The same
    // under 2.55's warning for `core.commentChar=auto`, and a commit git
    // could not sign.
    Verb {
        name: "commit-refused",
        when: &[(
            Arg::WithPreset("unsigned"),
            "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
             said=The commit was not made place=GIT COMMANDS \
             says=The commit could not be signed — see GIT COMMANDS",
        )],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The commit was not made place=GIT COMMANDS \
                says=A commit hook turned it down — see GIT COMMANDS",
    },
    // The same refusal with a file open in the middle, where a bar living
    // in the graph pane would be hidden. `clears=true`: the middle stands
    // under the bar.
    Verb {
        name: "notice-over-diff",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The commit was not made place=GIT COMMANDS \
                says=A commit hook turned it down — see GIT COMMANDS",
    },
    // A part of a file that is not that file any more, refused here before
    // git ran: there is no log row, so raising the panel would put it empty
    // over the diff. `why=true` is our own second line
    // (`Words.writeReportedWhy`).
    Verb {
        name: "stale-part",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=Nothing was staged",
    },
    // The door, not the table: each kind's dress (heading, whose second
    // line, colour) is `tests/qml/tst_reportdress.qml`'s; this is the
    // page's door (`RepoPage.showReport`) handing it to the bar the reader
    // sees. Half-rename is nobody's default — `warning` with our own
    // second line — so a door dropping either reads as another arm.
    // `log=false`: the kind is handed in with nothing spawned.
    Verb {
        name: "report-tone",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                said=The rename did not finish",
    },
    // The one report whose own line names the command log
    // (`Words.reportPlace`), raised through the same door: its place word
    // pressed, the panel coming up marked. Nothing ran, so the panel was
    // down before the press and `open=` is the press's own doing; `bar=true`
    // is the report still standing over the graph.
    Verb {
        name: "notice-link",
        when: &[],
        plain: "notice_link pressed=true before=false open=true lit=true bar=true",
    },
    // Another worktree's uncommitted work, as a pane of its own.
    // `files=` / `tally=` agree only because both fold a path's two sides
    // into one (`Bucket::Whole` / `Kinds::folded`); `cut=` / `tip=` are a
    // name too long for the band; `stageFile=` / `pieces=` the diff
    // offering no whole-file word or hunk seat. `topic` holds one path
    // staged then written again (per side it would read 2 twice), `here`
    // two untracked files (the only worktree the arrows can step in), and the
    // third a name past the pane's floor (the only one cut).
    Verb {
        name: "carried-read",
        when: &[
            (
                Arg::Is("here"),
                "carried_read worktree=here files=2 tally=2,0,0,0,0,0 lit=true litRows=1 \
                 cut=false tip=false stepped=true stageFile=false pieces=false",
            ),
            (
                Arg::Starts("an-extremely-long"),
                "carried_read worktree=an-extremely-long-worktree-folder-name-for-the-edge-case \
                 files=1 tally=0,1,0,0,0,0 lit=true litRows=1 cut=true tip=true stepped=false \
                 stageFile=false pieces=false",
            ),
        ],
        plain: "carried_read worktree=topic files=1 tally=0,1,0,0,0,0 lit=true litRows=1 \
                cut=false tip=false stepped=false stageFile=false pieces=false",
    },
    // The same pane at rest, where picking the row leaves a reader.
    // `litRows=0`: the only light is the reading, and a list that cannot
    // tell its rows apart lights all of them — invisible to the verb
    // above, whose one open file lights the top row either way.
    // `folderPath=`: the model folds by `<run>:<path>`, so a folder row
    // handed the fold key says `whole:src`, and the hover is the only
    // place the path is spelled out. Empty on the worktree whose files are all
    // at the root, which is why the claim is made on the other two.
    // `corner=` is the version in the pane's foot, which this pane lends
    // like the commit pane; `corner` cannot claim it, since that verb picks
    // between this window's tree and a commit of its history.
    Verb {
        name: "carried-stand",
        when: &[
            (
                Arg::Is("here"),
                "carried_stand worktree=here files=2 litRows=0 reading=false corner=true folderPath=",
            ),
            (
                Arg::Starts("an-extremely-long"),
                "carried_stand worktree=an-extremely-long-worktree-folder-name-for-the-edge-case \
                 files=1 litRows=0 reading=false corner=true folderPath=docs",
            ),
        ],
        plain: "carried_stand worktree=topic files=1 litRows=0 reading=false corner=true folderPath=src",
    },
    // The menu on another worktree's uncommitted row: its door, named by
    // the worktree's folder, and the apply — read off the rows as drawn, so
    // `apply=true held=false` is the row there and live, not greyed.
    Verb {
        name: "carried-menu",
        when: &[(
            Arg::Is("here"),
            "carried_menu open=true apply=true held=false name=here",
        )],
        plain: "carried_menu open=true apply=true held=false name=",
    },
    // That worktree's work applied here, landing as `stash apply` lands:
    // `here=` is this tree's row after it and `there=` the row it came from,
    // still drawn with what it had (it is only read). `here` stands on this
    // window's commit and holds two untracked files; `topic` stands on
    // another branch with a file this one never had, so the apply lands on
    // a conflict — a landing, not a refusal (`notice=false`).
    Verb {
        name: "carried-apply",
        when: &[
            (
                Arg::Is("here"),
                "carried_apply notice=false here=2,1,0,0,0,0 there=2,0,0,0,0,0 conflicts=0",
            ),
            (
                Arg::Is("topic"),
                "carried_apply notice=false here=0,1,0,0,0,1 there=0,1,0,0,0,0 conflicts=1",
            ),
        ],
        plain: "carried_apply notice=false",
    },
    // The same apply git turns down over a file changed here too: a report
    // in the worktree's yellow, its name behind the tree mark inside the
    // heading (`mark=true`), our own second line (`why=true`), and the
    // command log left shut (`log=false`).
    Verb {
        name: "carried-apply-refused",
        when: &[
            (
                Arg::Starts("an-extremely-long"),
                "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                 said=Changes from an-extremely-long-worktree-folder-name-for-the-edge-case \
                 were not applied mark=true",
            ),
            // Over another git's index lock (`--preset carried-lock`): a line
            // that sends the reader to the log. Which one is git's to decide —
            // the lock's where git names it (2.55), the plain refusal where it
            // says nothing (the minimum) — and core holds that
            // (`carried_apply::an_index_another_git_holds_is_reported_as_locked_where_git_says_so`).
            (
                Arg::WithPreset("carried-lock"),
                "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                 said=Changes from topic were not applied mark=true place=GIT COMMANDS says=",
            ),
        ],
        plain: "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                said=Changes from",
    },
    // A display behind a menu raised over it through a real door: what was
    // out goes in the turn the door returns (`down=true`), another target
    // asked for under the menu stays in past its rest, and the menu gone —
    // by Escape, a row taken, a press outside — the ask kept comes out.
    // `eol` / `eol-key` / `author` run every stage; `author-files`,
    // `recover` and `tip` the first two.
    Verb {
        name: "menu-block",
        when: &[
            (
                Arg::Is("eol:card"),
                "menu_block owner=eol stage=card shown=true menu=false down=false path=src/flipped.kt",
            ),
            (
                Arg::Is("eol:other"),
                "menu_block owner=eol stage=other shown=false menu=true down=true path=",
            ),
            (
                Arg::OneOf(&["eol:back", "eol:back-item", "eol:back-outside"]),
                "shown=true menu=false down=true path=src/mixed.kt",
            ),
            (
                Arg::Is("eol-key:card"),
                "menu_block owner=eol-key stage=card shown=true menu=false down=false path=src/flipped.kt",
            ),
            (
                Arg::Is("eol-key:other"),
                "menu_block owner=eol-key stage=other shown=false menu=true down=true path=",
            ),
            (
                Arg::OneOf(&["eol-key:back", "eol-key:back-item", "eol-key:back-outside"]),
                "shown=true menu=false down=true path=src/mixed.kt",
            ),
            (
                Arg::Starts("eol-key"),
                "menu_block owner=eol-key stage=menu shown=false menu=true down=true path=",
            ),
            (
                Arg::Is("author:card"),
                "menu_block owner=author stage=card shown=true menu=false down=false author=true mates=false",
            ),
            (
                Arg::Is("author:other"),
                "menu_block owner=author stage=other shown=false menu=true down=true author=false mates=false",
            ),
            (
                Arg::OneOf(&["author:back", "author:back-item", "author:back-outside"]),
                "shown=true menu=false down=true author=false mates=true",
            ),
            (
                Arg::Is("author-files:card"),
                "menu_block owner=author-files stage=card shown=true menu=false down=false",
            ),
            (
                Arg::Starts("author-files"),
                "menu_block owner=author-files stage=menu shown=false menu=true down=true author=false",
            ),
            (
                Arg::Starts("author"),
                "menu_block owner=author stage=menu shown=false menu=true down=true author=false mates=false",
            ),
            (
                Arg::Is("recover:card"),
                "menu_block owner=recover stage=card shown=true menu=false down=false",
            ),
            (
                Arg::Starts("recover"),
                "menu_block owner=recover stage=menu shown=false menu=true down=true",
            ),
            (
                Arg::Is("tip:card"),
                "menu_block owner=tip stage=card shown=true menu=false down=false",
            ),
            (
                Arg::Starts("tip"),
                "menu_block owner=tip stage=menu shown=false menu=true down=true",
            ),
        ],
        plain: "menu_block owner=eol stage=menu shown=false menu=true down=true path=",
    },
    // One of the author row's cards out, and the hand off its line onto
    // another in one turn: the card opening takes the one before down in
    // that turn (`now:`), and one card is out after it (`later:`) — the
    // one the hand went to.
    Verb {
        name: "author-swap",
        when: &[
            (
                Arg::Is("mates:parents"),
                "author_swap from=mates to=parents now: open=1 mates=false parents=true \
                 later: open=1 parents=true",
            ),
            (
                Arg::Is("parents:author"),
                "author_swap from=parents to=author now: open=1 parents=false author=true \
                 later: open=1 author=true",
            ),
        ],
        plain: "author_swap from=author to=mates now: open=1 author=false mates=true \
                later: open=1 mates=true",
    },
    // A changed file's tip with the pane's hand still on its row, and a
    // folder above it shut: the tip goes in that turn, nothing comes out
    // on the row that slid under the still hand past the rest, and a
    // two-pixel move brings the tip back after the rest.
    Verb {
        name: "tip-slide",
        when: &[
            (
                Arg::Is("files:shown"),
                "tip_slide on=files stage=shown tip=true still=false",
            ),
            (
                Arg::Is("files:slid"),
                "tip_slide on=files stage=slid tip=false still=true",
            ),
            (
                Arg::Is("files:still"),
                "tip_slide on=files stage=still tip=false still=true",
            ),
        ],
        plain: "tip_slide on=files stage=moved tip=true still=false",
    },
];

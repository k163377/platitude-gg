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
    // built from what core classified, `why=true` the hook's own words,
    // `log=false wrong=false` nothing raised as an error
    // (デザイン規約 §答えの要らない報せ).
    Verb {
        name: "commit-refused",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The commit was not made",
    },
    // The same refusal with a file open in the middle, where a bar living
    // in the graph pane would be hidden. `clears=true`: the middle stands
    // under the bar.
    Verb {
        name: "notice-over-diff",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The commit was not made",
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
    // Another working copy's uncommitted work, as a pane of its own.
    // `files=` / `tally=` agree only because both fold a path's two sides
    // into one (`Bucket::Whole` / `Kinds::folded`); `cut=` / `tip=` are a
    // name too long for the band; `stageFile=` / `pieces=` the diff
    // offering no whole-file word or hunk seat. `topic` holds one path
    // staged then written again (per side it would read 2 twice), `here`
    // two untracked files (the only copy the arrows can step in), and the
    // third a name past the pane's floor (the only one cut).
    Verb {
        name: "carried-read",
        when: &[
            (
                Arg::Is("here"),
                "carried_read copy=here files=2 tally=2,0,0,0,0,0 lit=true litRows=1 \
                 cut=false tip=false stepped=true stageFile=false pieces=false",
            ),
            (
                Arg::Starts("an-extremely-long"),
                "carried_read copy=an-extremely-long-working-copy-name-for-the-edge-case \
                 files=1 tally=0,1,0,0,0,0 lit=true litRows=1 cut=true tip=true stepped=false \
                 stageFile=false pieces=false",
            ),
        ],
        plain: "carried_read copy=topic files=1 tally=0,1,0,0,0,0 lit=true litRows=1 \
                cut=false tip=false stepped=false stageFile=false pieces=false",
    },
    // The same pane at rest, where picking the row leaves a reader.
    // `litRows=0`: the only light is the reading, and a list that cannot
    // tell its rows apart lights all of them — invisible to the verb
    // above, whose one open file lights the top row either way.
    // `folderPath=`: the model folds by `<run>:<path>`, so a folder row
    // handed the fold key says `whole:src`, and the hover is the only
    // place the path is spelled out. Empty on the copy whose files are all
    // at the root, which is why the claim is made on the other two.
    // `corner=` is the version in the pane's foot, which this pane lends
    // like the commit pane; `corner` cannot claim it, since that verb picks
    // between this window's tree and a commit of its history.
    Verb {
        name: "carried-stand",
        when: &[
            (
                Arg::Is("here"),
                "carried_stand copy=here files=2 litRows=0 reading=false corner=true folderPath=",
            ),
            (
                Arg::Starts("an-extremely-long"),
                "carried_stand copy=an-extremely-long-working-copy-name-for-the-edge-case \
                 files=1 litRows=0 reading=false corner=true folderPath=docs",
            ),
        ],
        plain: "carried_stand copy=topic files=1 litRows=0 reading=false corner=true folderPath=src",
    },
];

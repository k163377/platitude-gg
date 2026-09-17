//! What a pane is judged against: the room it was given, and the grip
//! that asks for more of it.
//!
//! `solo` is here because it is the same kind of claim about the run
//! itself — the window had the machine to itself, which an ordinary
//! window photographs exactly like.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    Verb {
        name: "solo",
        when: &[],
        plain: "solo blocked=true",
    },
    // The same screen, with its words taken from the air around them.
    // This is the surface with nothing behind it: the gate stands
    // before any repository is open, so the command log that carries
    // git's words everywhere else does not exist yet. `held=true` is
    // the fixture half — the path is only on screen while another
    // build holds the store, and an empty gate has air and no fields.
    Verb {
        name: "gate-sweep",
        when: &[],
        plain: "gate_sweep all=true caret=true hand=true held=true",
    },
    // The two failures a folder that will not open lands on. The
    // dialog is the one with no way out to the log either — a modal
    // window carries no seat for the panel — and the tab's screen does
    // carry one, but only for git's half: the folder is written
    // nowhere else. `kind=` says which of the three screens was swept,
    // since only one of them has a line from git in it at all.
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
    // Two ways the pane's column stops being one, and a photograph
    // answers neither. `fits` is the overflow: a child that will not
    // shrink is a floor the whole column sits on, and every box in the
    // pane then paints past the window's edge with its glyphs cut in
    // half — which frames exactly like a pane that fits. `fills` is the
    // other end of the same axis: the author row was given the whole
    // block or it was not, and the hash plate is right-aligned against
    // it, so a row that took only its own width parks the plate against
    // the end of the name. A picture shows a
    // plate either way; it does not say which edge it was meant to be
    // on.
    Verb {
        name: "details-fit",
        when: &[],
        plain: "details_fit fills=true fits=true",
    },
    // The pane's values are fields, and `Ctrl+A` is what reaches the
    // whole of one — including a tail the row had to cut. A picture
    // cannot answer this: a `Text` put back in place of the field draws
    // the identical row, and the wash a selection leaves is a few pixels
    // of colour a scaled-down look drops. So the claim is that what came
    // out of the field is what the model says the pane is showing.
    Verb {
        name: "details-select",
        when: &[],
        plain: "details_select match=true",
    },
    // One selection in the window. The pane shipped with every field
    // keeping what it held, so a reader who swept a second value found
    // the first still lit (observed — three at once), and
    // no picture of a single run can say that: one lit field frames the
    // same either way. Both halves are claimed, because a pane that had
    // simply cleared everything would answer the first on its own.
    Verb {
        name: "details-select-away",
        when: &[],
        plain: "details_select_away dropped=true held=true",
    },
    // A range selection is taken from the room beside the values —
    // they are a few characters wide and one line tall, and aiming
    // at them misses. Two claims, because
    // either alone photographs as a working pane: the sweep reaches the
    // value, and a press on the value's own box is still whatever
    // control was there. On the plate's two that second answer must be
    // `false`, which is the whole of "the hit areas did not shrink".
    //
    // For a value the row shows whole: a sweep reaches what is on
    // screen, which is the point of it, and the tail lives in the card
    // and under Ctrl+A (`details-select`).
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
    // The plate's own gesture, told apart by whether the press
    // travelled: a click copies the whole hash, a drag picks out the
    // shown one. No half of that is on screen — the clipboard is not,
    // and a plate that copied on every press frames exactly like one
    // that told the two apart, selection and all. `quiet=` is the half
    // `held=` does not give; `let=` is what the press before the copy
    // did to the drag before it, without which the plate copies out
    // from under a wash nobody is making any more; and `hand=` is the
    // half no gesture gives itself: the run enters the hand's own
    // functions, so a hand taken out, disabled or shrunk would answer
    // every gesture it was asked and never see a press. Whether Qt
    // hands the press to the plate at all is a
    // question only a real pointer answers, and it is asked where one
    // can be made (`qmltest`, tst_hashplate).
    Verb {
        name: "details-hand",
        when: &[],
        plain: "details_hand acted=true held=true quiet=true let=true hand=true",
    },
    // The other half of that press: the one word the control has says
    // what it did, and it has to say so on the tip the reader is
    // already looking at — a tip does not go down to change its mind.
    // Neither claim spells the words, so a translation leaves this
    // green and a tip that kept the old sentence turns it red. The
    // photograph does carry it (the tip is in overlay.png), but only
    // for a reader who knows which of the two sentences was due.
    Verb {
        name: "hash-tip",
        when: &[],
        plain: "hash_tip offered=true said=true",
    },
    // And the other order, which is the one that breaks: the press
    // lands before the tip arrives. The attached tooltip takes its
    // words when `visible` rises, so a tip still counting out its
    // rest comes up answering a press with the
    // offer unless the answer re-arms it. `counting=` is the setup
    // half — nothing was up when the press went in — and it has to be
    // said, because a run where the tip had already arrived would
    // answer the other half on its own.
    Verb {
        name: "hash-tip-counting",
        when: &[],
        plain: "hash_tip_counting counting=true said=true",
    },
    // The author's name, capped at its own width so the signature's one
    // word stays against it. A ceiling drawn a fraction of a pixel under
    // that width cuts the name, and a cut is what a picture answers
    // worst — the mark is a few pixels wide, both OSes draw one, and the
    // box around it does not move, so the run that lost the last glyph
    // frames exactly like the run that kept it (observed:
    // Ubuntu drew `Yuki Tana…` where Windows drew `Yuki Tanaka`, in the
    // same 78px of ink). At the window the verbs open, the row has 200px
    // of slack — nothing here is meant to give.
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
    // A pull that took more room than the pane had leaves what sits
    // under the box — the author card, the commit button — drawn over
    // the window's own footer, and that frames like a pane that fits:
    // the same blind spot details-fit answers for. Only that half is
    // judged here: whether the grip was offered at all depends on the
    // message, and the run where it stays away is half of the pair.
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
    // Whether the corner text gives the pane's foot back. **The
    // working tree's foot stays its own** — a commit button is pinned
    // to it — so `shown=false room=0` is the whole of that pane's
    // answer, whatever list is above it; the give-and-take that can
    // go either way is the details pane's, and which way it goes
    // there is the preset's business, so only the pane is claimed
    // for a row.
    //
    // Judged on `shown=`, the label's own visibility. `room=` is what
    // was asked, and a run that claimed only that would be green with
    // the answer overwritten — which is exactly how this shipped: an
    // instance wrote its own `visible` over the corner's, and every
    // WIP pane carried the version across its commit button until a
    // picture was looked at.
    Verb {
        name: "corner",
        when: &[
            (Arg::Is(""), "git_corner pane=wip shown=false room=0"),
            (Arg::Is("wip"), "git_corner pane=wip shown=false room=0"),
        ],
        plain: "git_corner pane=details",
    },
    // The two bars a panel carries, each judged as a pair, because each
    // says its state in brightness alone: a few pixels of ink at the
    // edge of a 1440px frame, one step apart. A screenshot cannot
    // settle that, and neither state means anything without the other
    // one beside it.
    //
    // The panel's slab is opaque, so it steps between named inks and the
    // colour itself is the report — a frame's ink while the list is
    // being sent, one step off the pane's ground once the reader has
    // left (デザイン規約 §QML 実装ルール のバーの明るさ). The ink is the
    // paint that landed; the asked-for `bright` stays green with it
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
    // The bar inside the description box is the style's see-through one,
    // which reaches the same two inks by carrying three tenths of its
    // single one. That box is also the one place the window hands a bar
    // to a `ScrollView`, whose flickable never calls itself moving — the
    // lit half is what says the wiring around that is still there
    // (observed: no bar at all until the bar was told to
    // watch the text).
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
    // A commit git wrote, and the boxes it was typed into being let go.
    // **The picture cannot judge this either**, for a reason of its own:
    // git answers before the reading that redraws the pane
    // (`session::write::run_write`), so at the moment the shot is taken
    // an emptied editor and a full one frame identically — and the press
    // is two writes, the staging in front and the commit behind it, so
    // the write barrier is answered by whichever finished first.
    // `empty=true` is the whole claim: the answer found the editor that
    // sent it, out of the answers that notify carried (`ops::Press`),
    // where a property would be rewritten by the fetch behind it.
    // `amend=` is the row under the boxes going down with them, which is
    // why the two verbs want the same line — `amend` turns it on first.
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
    // A commit a hook would not have. **The picture cannot judge this**
    // for the same reason the far side's refusals cannot be: a bar
    // saying nothing frames exactly like a bar saying the right thing,
    // and the whole claim is that the sentence was built out of what
    // core classified. `why=true` is the hook's own words arriving, and
    // `log=false wrong=false` the half that says nothing was raised as
    // an error beside them (デザイン規約 §答えの要らない報せ).
    Verb {
        name: "commit-refused",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The commit was not made",
    },
    // The same refusal **with a file open in the middle**, which is the
    // arrangement the bar was moved out of the graph pane for: a bar
    // that lived in that pane says nothing at all here, and the two
    // windows photograph the same. `clears=true` is the claim itself —
    // the middle is standing under the bar.
    Verb {
        name: "notice-over-diff",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The commit was not made",
    },
    // A part of a file that is not that file any more. **This end is the
    // one that said no**, before git ran, so there is no row in the log to
    // read and never was — which is why it used to raise an empty panel
    // over the very diff it was about (P3-確認事項, observed).
    // `why=true` is this application's own second line arriving through
    // `Words.writeReportedWhy`, since nobody outside wrote one.
    Verb {
        name: "stale-part",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=Nothing was staged",
    },
    // The dress each kind of report wears, asked of the page's own door
    // (`RepoPage.showReport`) so the run reads the same `Words` the answer
    // does. **The colour is a two-pixel hairline** — the whole reason
    // this is a report line.
    //
    // **The pair is the claim**: every report is `danger` except the one
    // that left something standing, so a run of the second alone would
    // pass against an implementation that painted everything warning.
    Verb {
        name: "report-tone",
        when: &[
            (
                Arg::Is("half-rename"),
                "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                 said=The rename did not finish",
            ),
            (
                Arg::Is("update"),
                "write_notice open=true clears=true why=false tone=danger log=false wrong=false \
                 said=origin would not update main",
            ),
            // The two a rewrite is turned down with: the tip moved after
            // the plan was composed (`ReportKind::RewriteTipMoved`), or an
            // operation was already standing (`RewriteWhileStanding`).
            // **The verb hands the kind in**, so nothing is spawned here
            // and `log=false` is a claim about the dress — the plan's
            // own check refuses before any spawn, but the carry's guard
            // refuses after git's one refusal and that one does leave a
            // row (`session_integration::standing_op`).
            // `why=true` is this application's own second line, which is
            // what a log row cannot give either way.
            (
                Arg::Is("tip-moved"),
                "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                 said=The history was not rewritten",
            ),
            (
                Arg::Is("op-standing"),
                "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                 said=The history was not rewritten",
            ),
        ],
        plain: "write_notice open=true clears=true tone=danger",
    },
    // Another working copy's uncommitted work, read in this window as a
    // pane of its own. `copy=` is that pane standing on that copy;
    // `files=` and `tally=` are the one list and the row above it
    // agreeing, which they only do because both fold
    // a path's two sides into one (`Bucket::Whole` / `Kinds::folded`);
    // `cut=` and `tip=` are what a name too long for the band does; and
    // the two after them are the diff offering neither its whole-file
    // word nor a hunk's seat.
    //
    // **The three runs are the claim.** `topic` holds one path staged and
    // then written again, which is the fold — counted per side it would
    // read 2 twice. `here` holds two untracked files, which is the only
    // copy the arrows have anywhere to step in (`stepped=`). The third is
    // named past the pane's floor, and is the only one whose name is cut.
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
    // The same pane at rest, which is where picking the row leaves a reader
    // and where the verb above has already read a file past.
    //
    // **`litRows=0` is the whole of the first claim.** This pane's only light is
    // the reading, so with nothing open no row wears one — and a list that
    // cannot tell its rows apart lights all of them, which the verb above
    // cannot see: its one open file makes the top of a wholly lit list read
    // exactly like the one correct row.
    //
    // `folderPath=` is the other name this pane could not read: the model
    // under it folds by `<run>:<path>` and keeps the path itself beside
    // that, so a folder row handed the fold key says `whole:src` wherever
    // it says a name — the hover above all, which is the only place a
    // path is spelled out at all. Empty on the copy whose files are all
    // at the root, which is why the claim is made on the other two.
    //
    // `corner=` is the version in the pane's own foot. Nothing is pinned
    // there — this pane lends the seat the way the commit pane does — and
    // `corner` itself cannot make the claim: that verb picks between this
    // window's own tree and a commit of its history, and neither of those is
    // another copy's work.
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

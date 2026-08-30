//! The graph's rows and the columns they sit in: the cards a row opens,
//! where it divides, the floors a drag stops at, and the end of what was
//! loaded.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    Verb {
        name: "details-failure",
        when: &[],
        plain: "details_failure loading=false error=true",
    },
    Verb {
        name: "perf",
        when: &[
            (
                Arg::Is("scroll-none"),
                "perf_complete selection=none details=false diff=false graph=true scrolled=true",
            ),
            (
                Arg::Is("none"),
                "perf_complete selection=none details=false diff=false graph=true scrolled=false",
            ),
            (
                Arg::Is("details"),
                "perf_complete selection=first details=true diff=false graph=true scrolled=false",
            ),
            (
                Arg::Is("diff"),
                "perf_complete selection=first details=true diff=true graph=false scrolled=false",
            ),
        ],
        plain: "perf_complete selection=first details=true diff=true graph=true scrolled=true",
    },
    // The eye at the end of the TAGS band. Both sides of it frame as
    // a graph with the same tags listed beside it, and what separates
    // them — a row, and every tag chip — depends on the repository, so
    // the picture cannot say on its own whether the switch answered.
    // `there=` is the named tag's commit, held against `shown=`: off
    // takes the commits nothing but a tag reaches out of the walk, and
    // pressing again brings them back.
    Verb {
        name: "tags-eye",
        when: &[(Arg::Ends(":back"), "tags_eye shown=true there=true")],
        plain: "tags_eye shown=false there=false",
    },
    // The two verbs whose whole picture is the card in overlay.png,
    // and the one line that says the card is in it: `popups=` is the
    // window overlay's own count of what it was holding when the
    // mirror was refreshed for the shot, so a blank overlay.png can
    // only mean nothing was open. Any verb whose subject is a menu, a
    // dialog or a tooltip can be judged the same way — these two are
    // where it bit (observed: two of four concurrent `commit-menu`
    // runs photographed a blank overlay and passed). `reset-menu`
    // counts two because the submenu is a popup of its own.
    Verb {
        name: "commit-menu",
        when: &[],
        plain: "overlay saved=true popups=1",
    },
    Verb {
        name: "reset-menu",
        when: &[],
        plain: "overlay saved=true popups=2",
    },
    // The two cards this menu ends with, opened one level in (デザイン規約
    // §メニュー の入れ子). Judged the same way and for the same reason:
    // what is being looked at is entirely inside the second popup, so a
    // run that opened nothing frames as a menu nobody pressed. `branch-
    // card` is also the proof that a graph row offers what its own chip
    // offers — an empty card would take its row with it and the run
    // would come back with one popup.
    Verb {
        name: "branch-card",
        when: &[],
        plain: "overlay saved=true popups=2",
    },
    Verb {
        name: "tag-card",
        when: &[],
        plain: "overlay saved=true popups=2",
    },
    // The card that must *not* come out. A picture cannot carry an
    // absence on its own — an empty overlay would frame the same as a
    // run whose hover request never arrived — so the menu is judged
    // with it: `menu=true` says there was something for the card to be
    // behind. Read as a pair with `row-card`, which proves that same
    // input does open the card.
    Verb {
        name: "menu-hover",
        when: &[],
        plain: "menu_hover menu=true card=false",
    },
    // The card of a row's own message, and the row it came out of.
    // `lit=` is the half a picture answers badly: the band is one
    // shade off the ground under a card that covers the rows below
    // it, and the card takes the pointer off the row the moment the
    // hand walks in to read it — so the row went dark while its own
    // card stood, and nothing on screen said which commit the message
    // was of.
    Verb {
        name: "row-card",
        when: &[],
        plain: "row_card open=true lit=true",
    },
    // The same card's words taken from the air around them — the
    // padding band, the step between two lines, the room beside a
    // short one (規約 §hover のツールチップ). This card is the one
    // worth sweeping: several lines and a badge row beside them, where
    // a card of one sentence would prove the padding band and nothing
    // else.
    //
    // `all=` and not a count, for the reason `tip-sweep` carries — how
    // much air a card has depends on the words in it. `open=` rides
    // with it because a card that never came up has no air either, and
    // "nothing to press" must not read as "everything worked".
    Verb {
        name: "card-sweep",
        when: &[],
        plain: "card_sweep all=true caret=true hand=true open=true",
    },
    // Where a graph row divides into the chip's half and the commit's,
    // asked at one point along the row. The picture shows which card
    // came out, but not which point was asked for or whether that is
    // the one the reader would have called it, so the run carries the
    // question with the answer and is judged on the two agreeing —
    // and on the other card being shut, since only one of the two is
    // ever meant to be out.
    Verb {
        name: "row-part",
        when: &[],
        plain: "agrees=true",
    },
    // The two clicks of the rename gesture, at the row and at the card
    // its chip unfolds into. `armed=` is the half no picture answers —
    // a box that never opened and a wait that was never taken frame
    // the same — and the two are read as one run because the box is
    // what the wait turns into. The card's own run carries `list=` as
    // well: it was standing on the column the box opens in, and one
    // left up would be covering the whole subject of the shot.
    Verb {
        name: "graph-reclick",
        when: &[],
        plain: "armed=true box=true",
    },
    // The same gesture carried through to git. The name lands and the
    // row wears it whichever way the box was opened, so the write is
    // no answer about the gesture: this half asks for the two the
    // plain form does.
    Verb {
        name: "graph-rename",
        when: &[],
        plain: "armed=true box=true",
    },
    Verb {
        name: "graph-reclick-list",
        when: &[],
        plain: "armed=true box=true list=false",
    },
    // The same two clicks put in through the strip over the lane
    // column. That strip is up wherever the lanes overflow their
    // column — which is every repository wide enough for a reader to
    // aim at the middle of a row — and it took presses of its own
    // without handing them to the row, so the gesture did nothing
    // between the two dividers while working either side of them
    //. `strip=` says the layer was actually up:
    // with the lanes inside their column there is nothing to prove.
    Verb {
        name: "graph-reclick-lanes",
        when: &[],
        plain: "strip=true armed=true box=true",
    },
    // The same two clicks with the pointer rested on the chip, so the
    // card's own rest is running under the wait. Nothing may open or
    // close in that beat: the reader has clicked and is watching one
    // spot for the box, and a card that arrived halfway through is a
    // change they did not ask for. `list=` and
    // `card=` are what would have opened — a picture of the box says
    // nothing about what came and went before it.
    Verb {
        name: "graph-reclick-still",
        when: &[],
        plain: "armed=true box=true list=false card=false",
    },
    // The mark the chip wears while the second click waits out its
    // window — the one thing on screen between the press and the box
    //. This run ends inside the wait rather than
    // at the box, so the picture frames the mark; `mark=` is the chip's
    // own answer, since a shot taken a beat late frames the box and
    // would read the same either way.
    Verb {
        name: "graph-reclick-mark",
        when: &[],
        plain: "armed=true mark=true box=false",
    },
    // Every way out of the name box, one route per run. `armed=` is
    // the half that catches the box coming back by itself: a wait left
    // running after the box has been walked away from reopens it a
    // window later, which is what a row clicked while its own box was
    // up used to do. The picture cannot tell a
    // box that closed for good from one that is about to return.
    Verb {
        name: "rename-box-out",
        when: &[
            // The one route the box survives: something has been typed
            // into it, and a press elsewhere is not worth losing that.
            (Arg::Is("typed-away"), "box=true armed=false"),
        ],
        plain: "box=false armed=false",
    },
    // The two clicks put in at one spot but two surfaces: the row, and
    // then the card its chip opens into on the rest between them. That
    // is what a reader's hand does without knowing it, and with a
    // memory per surface the second click comes up as a first one — the
    // gesture does nothing, and nothing on screen says why
    // (observed).
    Verb {
        name: "graph-reclick-across",
        when: &[],
        plain: "armed=true box=true list=false",
    },
    // The same gesture with the history scrolled away under the wait
    // it opened. The row that was clicked is pooled by that scroll, so
    // a wait carried by the row would go down with it — and the box
    // that does open has to be sent back into sight, since a name
    // changing itself off screen is a name nobody agreed to. Neither
    // half is anything a picture answers: a run whose box never opened
    // frames the same history as one whose box opened two hundred rows
    // above the fold.
    Verb {
        name: "graph-reclick-scrolled",
        when: &[],
        plain: "armed=true box=true list=false card=false mode=rename kind=branch typed=main branch=main shown=true",
    },
    // The graph column pulled past its floor: the clamp has to land
    // on the floor exactly, and the floor is the message tick
    // brought up against lane 0's co-author badge without touching
    // it (GraphPane.graphColWMin). The number is the token
    // arithmetic spelled out — it moves only when those tokens do,
    // and a clamp that stopped anywhere else photographs just as
    // neatly, since the gap in question is one pixel of the frame.
    Verb {
        name: "graph-min",
        when: &[],
        plain: "graph_min w=21 min=21",
    },
    // A drag carried past one of a divider's bounds. The badge is 12
    // pixels in the middle of a pane, and a run where the hook never
    // reached the divider photographs a window that looks entirely
    // well — so the refusal is said out loud. `line=` rides with it
    // because the two are a pair: the boundary still moves the other
    // way, and one that withdrew its line would be answering a
    // different question (that is `graph-divider`'s squeezed half).
    // One wanted line for every case: `line=` is already whichever
    // divider has the hand, so the argument does not change it.
    Verb {
        name: "divider-refuse",
        when: &[],
        plain: "divider_refuse refuses=true line=true",
    },
    // The end of a history and the end of what was loaded are the
    // same picture but for one line, and a footer that failed to draw
    // takes that line with it — so the cut says itself. `shown=` is
    // the footer's own visible, beside the model's answer: the two
    // are what the verb is for, and only neighbours are caught in one
    // substring.
    Verb {
        name: "graph-tail",
        when: &[],
        plain: "graph_tail truncated=true shown=true",
    },
    // The press on that footer, and the four things the camera cannot
    // answer for. `waiting=` is the ring, which is gone before the
    // shutter — the press is answered in under a second. Whether the
    // rows arrived *under* the ones being read is the pair after it:
    // a window that grew by starting the stream over lands the same row
    // count and photographs the same. `truncated=false` closes it —
    // this preset's history is 2100 commits, so one step of 500 past a
    // window of 2000 reaches the end of it and the footer goes with it.
    Verb {
        name: "graph-tail-more",
        when: &[],
        plain: "graph_tail_more taken=true waiting=true restarted=false held=true truncated=false",
    },
    // The dotted edges under the uncommitted row, in the tokens the
    // delegate paints from (uppercase = dashed). Under `--preset
    // conflict` a merge is standing, so the row leashes HEAD and the
    // side being brought in: two lanes, both dotted. A lane is a
    // couple of pixels wide and its dashes are one each, so a row
    // that leashed only HEAD photographs as very nearly the same
    // picture.
    Verb {
        name: "wip-lanes",
        when: &[],
        plain: "wip_lanes geometry=O0.0;O1.1",
    },
    // The lanes' sideways bar: out while the hand is in the pane, gone
    // when it leaves, and at full ink only once the lanes have actually
    // been sent (デザイン規約 §グラフを横へ送る / §バーの明るさ). Neither
    // half frames as an answer on its own — a bar six pixels tall on a
    // pane's bottom edge is as easy to miss in a picture as it is to
    // imagine, and a quarter of opacity is not something a screenshot
    // settles. `overflow=` stands beside them so a run over lanes that
    // already fit — where the bar is rightly absent and nothing was
    // proven — cannot read as the half where it went away.
    Verb {
        name: "graph-bar",
        when: &[],
        plain: "graph_bar shown=true ink=1",
    },
    Verb {
        name: "graph-bar-away",
        when: &[],
        plain: "graph_bar shown=false",
    },
    // Which column the middle click landed in is the whole question,
    // and a photograph answers neither half of it: lanes carried
    // sideways and lanes left where they were frame alike at this
    // size, and so do a gesture that panned and one that never
    // started. The two halves want opposite lines, which is what the
    // argument is for.
    Verb {
        name: "middle-scroll",
        when: &[(Arg::Is("message"), "middle_scroll lanes=false x=0")],
        plain: "middle_scroll lanes=true",
    },
];

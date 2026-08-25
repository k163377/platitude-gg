//! The graph's rows and the columns they sit in: the cards a row opens,
//! where it divides, the floors a drag stops at, and the end of what was
//! loaded.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
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
    // where it bit (2026-08-16: two of four concurrent `commit-menu`
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
    // input does open the card (2026-08-17 ユーザー報告).
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
    // was of (2026-08-22 ユーザー報告).
    Verb {
        name: "row-card",
        when: &[],
        plain: "row_card open=true lit=true",
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
    // left up would be covering the whole subject of the shot
    // (2026-08-26 ユーザー報告「予想外に switch される」).
    Verb {
        name: "graph-reclick",
        when: &[],
        plain: "armed=true box=true",
    },
    Verb {
        name: "graph-reclick-list",
        when: &[],
        plain: "armed=true box=true list=false",
    },
    // The same two clicks with the pointer rested on the chip, so the
    // card's own rest is running under the wait. Nothing may open or
    // close in that beat: the reader has clicked and is watching one
    // spot for the box, and a card that arrived halfway through is a
    // change they did not ask for (2026-08-26 ユーザー指示). `list=` and
    // `card=` are what would have opened — a picture of the box says
    // nothing about what came and went before it.
    Verb {
        name: "graph-reclick-still",
        when: &[],
        plain: "armed=true box=true list=false card=false",
    },
    // The mark the chip wears while the second click waits out its
    // window — the one thing on screen between the press and the box
    // (2026-08-26 ユーザー判断). This run ends inside the wait rather than
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
    // up used to do (2026-08-26 ユーザー報告). The picture cannot tell a
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
    // gesture does nothing, and nothing on screen says why (2026-08-26
    // ユーザー報告「入力モードに切り替わらないケースも有った」).
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

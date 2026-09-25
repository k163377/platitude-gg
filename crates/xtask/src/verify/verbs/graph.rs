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
            (Arg::Is("colour"), "perf_colour_frame elapsed_ms="),
            (
                Arg::Is("diff-scroll"),
                "perf_diff_scroll_frame visible=true",
            ),
            (Arg::Is("sequence"), "perf_operations count=4"),
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
            // The last of the calibration run's three lines, said only
            // once the walk before it settled.
            (Arg::Is("font-walk"), "perf_font_walk_settled clock_ms="),
        ],
        plain: "perf_complete selection=first details=true diff=true graph=true scrolled=true",
    },
    // The eye at the end of the TAGS band. `there=` is the named tag's
    // commit, held against `shown=`: off takes the commits only a tag
    // reaches out of the walk, and pressing again brings them back.
    Verb {
        name: "tags-eye",
        when: &[(Arg::Ends(":back"), "tags_eye shown=true there=true")],
        plain: "tags_eye shown=false there=false",
    },
    // Committing what is staged over a tree that keeps the rest — the one
    // shape where the uncommitted row survives its own commit. `moved=` is
    // HEAD; `wipRow=true` is the row drawn again above where it went (a
    // graph walked a second time would be missing it from that frame).
    //
    // `during` is the write out and the picture not yet moved. The flags
    // are judged: if the swap lands between the latch and the grab they
    // still describe the moment the run stopped at.
    Verb {
        name: "wip-commit-half",
        when: &[(
            Arg::Is("during"),
            "wip_half stage=during busy=true moved=false",
        )],
        plain: "wip_half stage=after busy=false moved=true wipRow=true",
    },
    // The card in overlay.png is the whole picture. `popups=` is the
    // window overlay's count when the mirror was refreshed for the shot,
    // so a blank overlay.png can only mean nothing was open — otherwise a
    // mirror refreshed over an empty overlay passes. Any verb whose
    // subject is a menu, dialog or tooltip can be judged this way.
    // `reset-menu` counts two: the submenu is a popup of its own.
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
    // The menu's two nested cards (デザイン規約 §メニュー の入れ子),
    // judged as above. `branch-card` also proves a graph row offers what
    // its chip offers — an empty card takes its row with it, leaving one
    // popup.
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
    // The delete that card offers, run to git's answer. The row stays up
    // for a refusal (`AppMenuItem.staysOpen`), so what is judged is the
    // card going once none came; `code=` tells a landing (still
    // `branch --delete`) from a refusal (turned `branch -D`).
    Verb {
        name: "delete-branch-chip",
        when: &[],
        plain: "chip_delete branch=base row=-1 code=branch --delete menu=false card=false",
    },
    // The card a chip unfolds into must cover the chip. It sits at the
    // chip's near edge with rows measured to their names, so a chip with
    // a `+N` and a fan is wider and its end shows past the card by a few
    // pixels. `covers=` reads the two boxes, the chip as it stands once
    // its sheets are down.
    Verb {
        name: "ref-list",
        when: &[],
        plain: "covers=true",
    },
    // The same card from the stand-in for a HEAD scrolled off, which asks
    // for it itself: `on=` is the card landing on the stand-in's chip, not
    // a row's — the one that takes its sheets down and keeps its ground lit.
    Verb {
        name: "graph-head-list",
        // `click` presses the card's first row = the stand-in's press: its
        // row comes on screen and is read, card and stand-in gone, and the
        // rows under the hand are told the gesture is not theirs (a
        // double-click's second press lands there). `held=`: the row that
        // came under the still hand opens nothing on its chip.
        when: &[(
            Arg::Is("click"),
            "list=false landed=true pin=false hushed=true held=true",
        )],
        plain: "shown=true list=true on=true covers=true",
    },
    // The pointer resting on one of that card's rows. `nowhere=` pins the
    // aim to a row with nowhere to go: the others differ by name colour
    // too, and the one-shade wash is all that row has.
    Verb {
        name: "ref-list-lit",
        when: &[],
        plain: "list=true lit=true nowhere=true",
    },
    // A held click in that list moves the choice alone: the commit read
    // stays, two rows draw themselves chosen, and the card stays up.
    Verb {
        name: "ref-list-choose",
        when: &[],
        plain: "chosen=2 lit=2 read=true list=true",
    },
    // The name a row of that list opens under itself, pressed: the card
    // goes, the graph lights that name's commit, and the rows under the
    // card are told the gesture is not theirs (a double-click's second
    // press lands on them).
    Verb {
        name: "ref-list-follow",
        when: &[],
        plain: "followed=true list=false landed=true hushed=true",
    },
    // And the pointer resting on it: the band is drawn under that line,
    // read off the chip, with the card still up.
    Verb {
        name: "ref-list-follow-lit",
        when: &[],
        plain: "list=true aimed=true",
    },
    // The pointer resting on the note a cut message puts out. The rule
    // under the words takes the words' colour, so `lit=` alone passes;
    // `word=` pins the step — this note alone rests at `textMuted`, where
    // following the words moves half the step it makes elsewhere. The hex
    // is `textSecondary`'s value; move the token and this line moves too.
    Verb {
        name: "card-note-lit",
        when: &[],
        plain: "card_note lit=true word=#94a3b8",
    },
    // The card stays away while a menu is up: `menu=true` says there was
    // something for it to be behind. Paired with `row-card`, which proves
    // the same input opens it.
    Verb {
        name: "menu-hover",
        when: &[],
        plain: "menu_hover menu=true card=false",
    },
    // The card of a row's message. `lit=`: the card takes the pointer off
    // the row as the hand walks in, and a row gone dark leaves nothing on
    // screen saying which commit the message is of.
    Verb {
        name: "row-card",
        when: &[],
        plain: "row_card open=true lit=true",
    },
    // The hand walks into that card and back to its row, and the card
    // stays (rules-refs/app-ui.md「カードが既に出ている行へ手が戻ったら」).
    // `held=` is the claim: the hold is back in the same turn as the
    // return, the only way it beats `hoverKeepMs`. `moved=` is a guard for
    // a seat worked out from something a run can move: today the seat
    // comes off `MouseArea.mouseX`, which a headless run cannot write
    // (verify-ui §hover の絵の撮り方), so the card cannot slide here.
    Verb {
        name: "row-card-return",
        when: &[],
        plain: "row_card_return held=true open=true moved=false oid=true",
    },
    // The note under a cut message, pressed. `picked=` is the claim: the
    // press lands on the commit the card was of, whatever row the page
    // had before.
    Verb {
        name: "card-message",
        when: &[],
        plain: "card_message open=false picked=true mark=true shown=true",
    },
    // The same walk carried on to Escape. `took=`: the mark going is this
    // page's doing only if this page answered the key.
    Verb {
        name: "card-message-esc",
        when: &[],
        plain: "card_message_esc mark=false took=true picked=true",
    },
    // The card's words taken from the air around them — padding band,
    // line step, room beside a short line (規約 §hover のツールチップ).
    // This card has several lines and a badge row; a one-sentence card
    // proves only the padding. `all=` as in verbs.md §面の掃き; `open=`
    // fails a card that never came up.
    Verb {
        name: "card-sweep",
        when: &[],
        plain: "card_sweep all=true caret=true hand=true open=true",
    },
    // Where a graph row divides into the chip's half and the commit's.
    // The run carries the point asked with the card that came out and is
    // judged on the two agreeing, and on the other card being shut.
    Verb {
        name: "row-part",
        when: &[],
        plain: "agrees=true",
    },
    // `list-menu` (below) names one of the stacked names: the row's menu
    // aimed at the name under the press. `list=`: the card pressed on
    // still stands under the menu. `branch=` / `tag=`: which card the
    // naming brought up — the stacked list's tag row must swap them over.
    //
    // The chip's own entrance raises the row's menu aimed at that name —
    // no second menu on the chip's side (デザイン規約 §グラフ行の右クリック);
    // `ref=false` is the ref menu staying away. A window checks the aim:
    // the press reaches `CommitMenuState.askRefRows` with the chip's name
    // and the tree's branch (the drawing is `tst_commitrowmenu.qml`, the
    // answer `offers::ref_menu`). The two lines are the two aims — a name
    // the tree is not on, and the one it is; neither stands for the
    // other. A run naming no branch aims at an empty name, which the graph
    // never draws.
    Verb {
        name: "chip-menu",
        when: &[],
        plain: "chip_menu ref=false commit=true switch=true",
    },
    Verb {
        name: "chip-menu-current",
        when: &[],
        plain: "chip_menu ref=false commit=true switch=false",
    },
    Verb {
        name: "list-menu",
        when: &[(
            Arg::Is("1:1"),
            "list_menu list=true menu=true branch=false tag=true",
        )],
        plain: "list_menu list=true menu=true",
    },
    // Several commits held at once, by Ctrl and by Shift. The completion
    // owns the tally (the run finishes only once the page holds as many
    // as it pressed and that many rows are lit); this line is the rest: a
    // held press moves the choice alone, and three lit rows say nothing
    // of which one the right pane describes. `spare=0`: the list stands
    // as tall as it holds, its own top margin counted — counted from the
    // rows alone it scrolls two pixels to reach nothing.
    Verb {
        name: "graph-choose",
        when: &[],
        plain: "read=true wip=false spare=0",
    },
    Verb {
        name: "graph-choose-range",
        when: &[],
        plain: "read=true wip=false spare=0",
    },
    // The list's row cuts its summary to a line; the card its rest opens
    // holds nothing back and offers no way out of itself. `lit=` is the
    // band the row keeps under the card, which takes the pointer off it.
    // At the window's wall the card does stop — past it, it would cover
    // the pane it came from — so `7:8:11` drops `held=` and keeps the rest.
    Verb {
        name: "graph-choose-said",
        when: &[(Arg::Is("7:8:11"), "open=true door=false lit=true")],
        plain: "open=true door=false lit=true held=false",
    },
    // The row's words, reached from every corner of its air. `hand=` as in
    // verbs.md §面の掃き.
    Verb {
        name: "graph-choose-sweep",
        when: &[],
        plain: "all=true caret=true hand=true",
    },
    // A row of the merged file list opens each chosen commit's own patch
    // of that file, one band after another: `bands=2` because two of the
    // three touched it, where a diff across the span would be one block
    // carrying the work of a commit not chosen.
    Verb {
        name: "graph-choose-diff",
        when: &[],
        plain: "bands=2 chosen=3",
    },
    // Toggling a row out of the choice and back quickly is two presses at
    // one spot, which the area hands over as a double-click, modifier and
    // all (`tst_moddblclick`) — and the plain double-click is `switch`.
    // `led=false` is the row's own answer (the write lands ticks later);
    // `movable=true` keeps it from being vacuous; `naming=false`: the
    // spaced second click opens a name box, and building a choice is not
    // naming.
    Verb {
        name: "graph-choose-dbl",
        when: &[],
        plain: "led=false movable=true naming=false",
    },
    // A commit taken back out of the held list. Of two presses on a row
    // only the second is the row's: `takes=false kept=3` is the plain one
    // left to the words underneath (a smaller choice would mean the hand
    // took the drag from the text). `hand=true`: the run enters the row's
    // function, so an area taken out, disabled or shrunk would still pass.
    Verb {
        name: "graph-choose-drop",
        when: &[],
        plain: "takes=false kept=3 dropped=true hand=true chosen=2 lit=2",
    },
    // The rename gesture's two clicks, at the row and (`graph-reclick-list`)
    // at the card its chip unfolds into. `armed=` and `box=` are read as
    // one because the box is what the wait turns into. `list=false`: the
    // card stood on the column the box opens in, and left up covers it.
    Verb {
        name: "graph-reclick",
        when: &[],
        plain: "armed=true box=true",
    },
    // Carried through to git. The name lands whichever way the box
    // opened, so the write says nothing about the gesture: the same two.
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
    // The same clicks through the strip over the lane column, up wherever
    // the lanes overflow it; a strip keeping its presses kills the gesture
    // between the two dividers. `strip=` says it was up — with the lanes
    // inside their column there is nothing to prove.
    Verb {
        name: "graph-reclick-lanes",
        when: &[],
        plain: "strip=true armed=true box=true",
    },
    // The same clicks with the pointer rested on the chip, so the card's
    // rest runs under the wait. The screen holds still in that beat:
    // `list=` and `card=` say nothing opened before the box.
    Verb {
        name: "graph-reclick-still",
        when: &[],
        plain: "armed=true box=true list=false card=false",
    },
    // The mark the chip wears while the second click waits out its
    // window. The run ends inside the wait; `mark=` is the chip's own
    // answer, since a shot a beat late frames the box instead.
    Verb {
        name: "graph-reclick-mark",
        when: &[],
        plain: "armed=true mark=true box=false",
    },
    // Every way out of the name box, one route per run. `armed=false`: a
    // wait left running after the box is walked away from reopens it a
    // window later.
    Verb {
        name: "rename-box-out",
        when: &[
            // The one route the box survives: something has been typed
            // into it, and a press elsewhere is not worth losing that.
            (Arg::Is("typed-away"), "box=true armed=false"),
        ],
        plain: "box=false armed=false",
    },
    // The two clicks at one spot but on two surfaces: the row, then the
    // card its chip opened into on the rest between. With a memory per
    // surface the second click reads as a first and the gesture does
    // nothing.
    Verb {
        name: "graph-reclick-across",
        when: &[],
        plain: "armed=true box=true list=false",
    },
    // The history scrolled away under the wait. The clicked row is pooled
    // by the scroll, so a wait carried by the row goes down with it; the
    // box that opens has to be brought back into sight (`shown=true`) — a
    // name changing off screen is one nobody agreed to.
    Verb {
        name: "graph-reclick-scrolled",
        when: &[],
        plain: "armed=true box=true list=false card=false mode=rename kind=branch typed=main branch=main shown=true",
    },
    // The graph column pulled past its floor must clamp on it exactly: the
    // message tick against lane 0's co-author badge without touching it
    // (GraphPane.graphColWMin). The number is the token arithmetic — it
    // moves only when those tokens do.
    Verb {
        name: "graph-min",
        when: &[],
        plain: "graph_min w=21 min=21",
    },
    // A drag carried past one of a divider's bounds. `line=` rides with
    // the refusal: the boundary still moves the other way, and one that
    // withdrew its line answers a different question (`graph-divider`'s
    // squeezed half). `line=` is whichever divider has the hand, so one
    // line serves every argument.
    Verb {
        name: "divider-refuse",
        when: &[],
        plain: "divider_refuse refuses=true line=true",
    },
    // The end of what was loaded differs from the end of history by one
    // footer line. `shown=` is the footer's own visible, next to the
    // model's answer — only neighbours are caught in one substring.
    Verb {
        name: "graph-tail",
        when: &[],
        plain: "graph_tail truncated=true shown=true",
    },
    // The press on that footer. `waiting=` is the ring, gone before the
    // shot. `restarted=` / `held=`: the rows arrived under the ones being
    // read — restarting the stream lands the same count. The preset's
    // 2100 commits end within one step of 500 past a window of 2000, so
    // `truncated=false`.
    Verb {
        name: "graph-tail-more",
        when: &[],
        plain: "graph_tail_more taken=true waiting=true restarted=false held=true truncated=false",
    },
    // The dotted edges under the uncommitted row, in the tokens the
    // delegate paints from (uppercase = dashed). Under `--preset
    // conflict` a merge stands, so the row leashes HEAD and the side
    // being brought in: two lanes, both dotted.
    Verb {
        name: "wip-lanes",
        when: &[],
        plain: "wip_lanes geometry=O0.0;O1.1",
    },
    // The lanes' sideways bar: out while the hand is in the pane, gone
    // when it leaves, at full ink only once the lanes have been sent
    // (デザイン規約 §グラフを横へ送る /「バーの明るさ」). `overflow=` beside
    // them tells lanes that already fit (bar rightly absent, nothing
    // proven) from the half where it went away.
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
    // Which column the middle click landed in: lanes carried sideways,
    // or (`message`) left where they were. The argument picks the half.
    Verb {
        name: "middle-scroll",
        when: &[(Arg::Is("message"), "middle_scroll lanes=false x=0")],
        plain: "middle_scroll lanes=true",
    },
    // How the gesture ends: a drift that stays against one that ends with
    // the hand, and a hand still inside the dead zone against one that
    // never asked. The argument picks the half.
    Verb {
        name: "middle-scroll-exit",
        when: &[(
            Arg::Is("click"),
            "middle_scroll_exit case=click travelled=false scrolling=true sent=false",
        )],
        plain: "middle_scroll_exit case=held travelled=true scrolling=false sent=true",
    },
];

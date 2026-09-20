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
            // once the walk before it was paid and left to settle.
            (Arg::Is("font-walk"), "perf_font_walk_settled clock_ms="),
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
    // A commit of what is staged over a tree that keeps the rest: the
    // one shape where the uncommitted row survives its own commit, so
    // the pair shows the row moving. `moved=` is
    // HEAD, and `wipRow=true` on the far side is the row drawn again
    // above where it went — which is the whole claim, since a row the
    // graph had to be walked for a second time would be missing from
    // that frame.
    //
    // `during` is the write out and the picture not yet moved. **The
    // three flags are what is judged**: where the swap
    // lands between the latch and the grab the flags still describe the
    // moment the run stopped at, and the picture is read by eye.
    Verb {
        name: "wip-commit-half",
        when: &[(
            Arg::Is("during"),
            "wip_half stage=during busy=true moved=false",
        )],
        plain: "wip_half stage=after busy=false moved=true wipRow=true",
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
    // The delete that card offers, run to git's answer from this
    // entrance. **Nothing here is a picture**: the row is left standing
    // for a refusal (`AppMenuItem.staysOpen`), so what is judged is the
    // card going once none is coming — and a card that closed frames
    // exactly like one that was never opened. `code=` is the other half,
    // since a refused delete would leave the menu up for a reason of its
    // own: on a landing the row is still wearing `branch --delete`,
    // where a refusal turns it into `branch -D`.
    Verb {
        name: "delete-branch-chip",
        when: &[],
        plain: "chip_delete branch=base row=-1 code=branch --delete menu=false card=false",
    },
    // The pointer resting on one of that card's rows. The wash is one
    // shade over the card's own ground, so a row that never took the
    // answer frames the same as one that did — and the row worth aiming
    // at is one with nowhere to go, which is what `nowhere=` pins down:
    // every other row is told apart by the colour of its name as well,
    // so the wash is the only thing that row has.
    Verb {
        name: "ref-list-lit",
        when: &[],
        plain: "list=true lit=true nowhere=true",
    },
    // A held click in that list moves the choice and nothing else: the
    // commit read stays, two rows draw themselves chosen, and the card
    // stays up — a press that took the plain road would have closed it
    // and read the row instead.
    Verb {
        name: "ref-list-choose",
        when: &[],
        plain: "chosen=2 lit=2 read=true list=true",
    },
    // The pointer resting on the note a cut message puts out. `lit=`
    // alone is the rule that was always kept — the line under the words
    // is the words' own colour — so what pins this note's step down is
    // `word=`: the note is the one target in that family whose words are
    // `textMuted`, and a rule following them there moved by half the step
    // the same vocabulary makes anywhere else. The hex is the token's
    // value; move `textSecondary` and this line moves with it.
    Verb {
        name: "card-note-lit",
        when: &[],
        plain: "card_note lit=true word=#94a3b8",
    },
    // The card that stays away. A picture cannot carry an
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
    // The hand that walked down into that card and came back to the row
    // it came off. **Not a picture's question at all**: a card held up
    // and a card taken down and opened again frame identically once the
    // beats are over, and the only difference a reader sees is the blink
    // and the sideways step in between (P3-確認事項, observed).
    //
    // `held=` is the claim — the hold is back in the same turn as the
    // return, which is the only way it beats `hoverKeepMs`.
    //
    // **`moved=` is a guard.** The seat comes off the
    // row's own `pointerX`, which is `MouseArea.mouseX` and therefore the
    // one thing a headless run cannot write (verify-ui §hover の絵の撮り方):
    // the card cannot slide here whatever the code does, so this field
    // holds a future seat worked out from something a run *can* move, and
    // the sideways step a real hand sees is argued from where the seat
    // comes from.
    Verb {
        name: "row-card-return",
        when: &[],
        plain: "row_card_return held=true open=true moved=false oid=true",
    },
    // The note under a message that card had to cut, pressed. Three of
    // the four are things a picture frames the same either way: a card
    // that never opened and a card that closed leave the same empty
    // overlay, the mark the pane wears says nothing about *why* it is
    // there, and the row the selection landed on is not written
    // anywhere on screen. `picked=` is the whole of the claim — the
    // press has to land on the commit the card was of, whatever row
    // the page had before.
    Verb {
        name: "card-message",
        when: &[],
        plain: "card_message open=false picked=true mark=true shown=true",
    },
    // The same walk, carried on to Escape. A picture cannot answer any of
    // it: a mark put away frames exactly like a mark that was never
    // raised, and whether the page took the key or let it pass is not
    // drawn at all. `took=` is the half that matters — the mark going is
    // only this page's doing if this page is what answered.
    Verb {
        name: "card-message-esc",
        when: &[],
        plain: "card_message_esc mark=false took=true picked=true",
    },
    // The same card's words taken from the air around them — the
    // padding band, the step between two lines, the room beside a
    // short one (規約 §hover のツールチップ). This card is the one
    // worth sweeping: several lines and a badge row beside them, where
    // a card of one sentence would prove the padding band and nothing
    // else.
    //
    // `all=`, for the reason `tip-sweep` carries — how much air a card
    // has depends on the words in it. `open=` rides with it because a
    // card that never came up has no air either, and a run with
    // nothing to press fails here.
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
    // Naming one of the stacked names: the row's own menu, aimed at
    // the name under the press. Three things the picture cannot
    // carry. **`list=`** — the card the press was made
    // on has to still be standing under the menu, and a list that went
    // down leaves a menu that frames exactly like one raised from the
    // row. **`branch=` / `tag=`** — which card the naming brought up is
    // the whole of what this gesture decides, and a card that is on the
    // menu but not opened is not drawn at all. The tag row of the
    // stacked list is the case worth pinning: it must swap the cards
    // over.
    // The chip's own entrance, which raises the row's menu aimed at
    // that name — there is no second menu on the chip's side of the
    // column any more (デザイン規約 §グラフ行の右クリック). **`ref=`
    // is the one that stays away**: a picture of the ref menu
    // and a picture of this one differ by rows nobody counts by eye.
    //
    // **What a window is for here is the aim**, not the row: that the
    // press reaches `CommitMenuState.askRefRows` with the name the chip
    // drew and the branch the tree is on. Whether the row is then drawn
    // from that answer is `tst_commitrowmenu.qml`, and which answer the
    // state comes to is `offers::ref_menu`. So the two lines are the two
    // aims — a name the tree is not on, and the one it is — and neither
    // stands for the other: they read the lookup from opposite sides.
    // A run naming no branch at all is not one of them; it aims at an
    // empty name, which is a row the graph never draws.
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
    // Several commits held at once, taken one at a time with Ctrl and
    // swept with Shift. **The tally is the completion's** — the run
    // will not finish until the page holds as many as it pressed *and*
    // that many rows draw themselves lit, so a choice that never
    // reached the rows runs into the watchdog instead of reporting.
    // What is left for this line is the half the completion cannot
    // reach: a held press moves the choice alone, and a picture of
    // three lit rows says nothing about which of them the pane on
    // the right is describing.
    // `spare=0` is the other half a picture cannot answer: the list
    // stands as tall as what it holds, **its own top margin counted**.
    // Counted from the rows alone it came out one margin short, which is
    // a view that scrolls two pixels to reach nothing at all — and two
    // pixels of scroll frame exactly like none.
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
    // The row of that list cuts its summary to a line; the card its rest
    // opens holds nothing back and offers no way out of itself. Neither
    // half is a thing a picture answers — a mark is a few pixels wide, so
    // a row that dropped the tail frames like one that did not, and a
    // card that held its body back frames as a shorter card. `lit=` is
    // the band the row keeps under the card, which takes the pointer off
    // it the moment it is up.
    // At the wall the card does stop — the room the window has is what
    // bounds it, and a message past that has to end somewhere or the card
    // covers the pane it was opened from. So `held=` is dropped there and
    // the rest of the claim stands: the card is up, it offers no way out
    // of itself, and the row it came off keeps its band.
    Verb {
        name: "graph-choose-said",
        when: &[(Arg::Is("7:8:11"), "open=true door=false lit=true")],
        plain: "open=true door=false lit=true held=false",
    },
    // And the words being a reader's to take away, reached from every
    // corner of the row's air: a hand that works from the middle and
    // nowhere else is the fault this kind of row ships with. `hand=`
    // is the half a sweep cannot say for itself — a pad whose area had
    // been taken out would answer every sweep it was asked and never
    // see a press.
    Verb {
        name: "graph-choose-sweep",
        when: &[],
        plain: "all=true caret=true hand=true",
    },
    // And what a row of the merged file list opens: each chosen commit's
    // own patch of that file, one band after another. `bands=2` is the
    // whole claim — two of the three chosen commits touched it, and a
    // diff across the span would be one block carrying the work of the
    // one that was not chosen.
    Verb {
        name: "graph-choose-diff",
        when: &[],
        plain: "bands=2 chosen=3",
    },
    // The gesture a choice stops short of. Toggling a row out of the
    // choice and back in, quickly, is two presses at one spot — which the
    // area hands over as a double-click, modifier and all (measured,
    // `tst_moddblclick`). On the other side of the plain one is `switch`,
    // which moves the working tree and takes uncommitted changes with it.
    // `led=false` is the claim, and it is the row's own answer: the write
    // lands ticks later, so the branch on screen at the moment of the
    // press says nothing either way. `movable=true` is what keeps the run
    // from being vacuous — a row that leads nowhere would answer `false`
    // for a reason of its own. `naming=false` is the other half of the
    // same press: the spaced second click opens a name box, and a hand
    // building a choice is not naming anything.
    Verb {
        name: "graph-choose-dbl",
        when: &[],
        plain: "led=false movable=true naming=false",
    },
    // A commit taken back out from the list of what is held. Two presses
    // on one row and only the second is the row's: `takes=false kept=3`
    // is the plain one, which the words underneath have to keep — a
    // choice that came back smaller after it would mean this hand had
    // taken the drag away from the text. `hand=true` is the half a press
    // cannot say for itself, since the run enters the row's own
    // function: an area taken out, disabled or shrunk would answer
    // every press it was asked and never see one.
    Verb {
        name: "graph-choose-drop",
        when: &[],
        plain: "takes=false kept=3 dropped=true hand=true chosen=2 lit=2",
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
    // between the two dividers while working either side of them.
    // `strip=` says the layer was actually up: with the lanes
    // inside their column there is nothing to prove.
    Verb {
        name: "graph-reclick-lanes",
        when: &[],
        plain: "strip=true armed=true box=true",
    },
    // The same two clicks with the pointer rested on the chip, so the
    // card's own rest is running under the wait. The screen holds
    // still in that beat: the reader has clicked and is watching one
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
    // window — the one thing on screen between the press and the box.
    // This run ends inside the wait, so the picture frames the
    // mark; `mark=` is the chip's own answer, since
    // a shot taken a beat late frames the box and would read the same
    // either way.
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

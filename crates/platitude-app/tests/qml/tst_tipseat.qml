import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The window's one tooltip (`TipSeat`) under a real pointer, as the targets ask for it (`TipAsk`)
// (デザイン規約 §hover のツールチップ): every target waits its own rest and the neighbour's box goes first; the hand stepping
// into the box keeps it; a refusal of what is typed stands until the typing changes; where the box stands for a row
// among rows and beside its target; a dismissed box is not put back under the same still hand; a target that takes
// its ask back is heard again when it asks afresh. Only a real pointer can answer this: hover cannot be injected into
// the app (verify-ui スキル).
Item {
    id: root
    width: 1000
    height: 400

    HandWatch { id: hand }

    TipSeat {
        id: seat
        host: root
        hand: hand
    }
    readonly property var box: seat.box
    /// A menu clear of the rows: standing, it takes the box down; going, the seat answers the ask it kept.
    AppMenu {
        id: menu
        x: root.width - 200
        y: 10
        AppMenuItem {
            text: "Act"
        }
    }

    // Set apart, so the box one row puts out never covers the other.
    Item {
        id: rowA
        y: 120
        width: 300
        height: 40
        HoverHandler { id: onA }
        readonly property TipAsk askA: TipAsk { id: askA; target: rowA; words: "row A"; asked: onA.hovered }
    }
    Item {
        id: rowB
        y: 240
        width: 300
        height: 40
        HoverHandler { id: onB }
        readonly property TipAsk askB: TipAsk { id: askB; target: rowB; words: "row B"; asked: onB.hovered }
    }
    // The left panel's shape (`NavRowFacts`): a box that stands beside its target and is asked for from the line
    // above it as well as from the target itself.
    Item {
        id: nameLine
        y: 20
        width: 300
        height: 24
        HoverHandler { id: onName }
    }
    Item {
        id: aside
        y: 44
        width: 300
        height: 42
        HoverHandler { id: onAside }
        readonly property TipAsk askAside: TipAsk {
            id: askAside; target: aside; words: "beside"; beside: true; asked: onName.hovered || onAside.hovered
        }
    }
    // A name box's refusal (`NavNameBox` / the graph's box): asked for by what is typed, not by a hand.
    Item {
        id: typedBox
        x: 320
        y: 20
        width: 60
        height: 24
        property bool refused: false
        readonly property TipAsk askTyped: TipAsk {
            id: askTyped; target: typedBox; words: "A branch called that already exists"; typed: true
            asked: typedBox.refused
        }
    }
    // The same, wider than its words: what decides where the box stands across it. Off the window's edge, which the
    // seat keeps a step clear of.
    Item {
        id: wideTyped
        x: 40
        y: 190
        width: 300
        height: 24
        property bool refused: false
        readonly property TipAsk askWide: TipAsk {
            id: askWide; target: wideTyped; words: "A name is needed"; typed: true; asked: wideTyped.refused
        }
    }
    // The rows among rows: the right panel's (room on the left only), the left panel's (asks
    // for the left, has no room there), a menu's (room both ways, asks for the right), one as wide as the window.
    // None of them overlapping another target: a hover-taking item stacked over a row takes the pointer off it.
    Item {
        id: rightPane
        x: 700
        y: 300
        width: 300
        height: Theme.rowHeight
        HoverHandler { id: onRightPane }
        readonly property TipAsk rightTip: TipAsk {
            target: rightPane; words: "docs/install.md"; rowSide: "left"; asked: onRightPane.hovered
        }
    }
    Item {
        id: leftPane
        y: 300
        width: 260
        height: Theme.rowHeight
        HoverHandler { id: onLeftPane }
        readonly property TipAsk leftTip: TipAsk {
            target: leftPane; words: "feature"; rowSide: "left"; asked: onLeftPane.hovered
        }
    }
    Item {
        id: menuRow
        x: 400
        y: 330
        width: 200
        height: Theme.rowHeight
        HoverHandler { id: onMenuRow }
        readonly property TipAsk menuTip: TipAsk {
            target: menuRow; words: "Differs from origin/main"; rowSide: "right"; asked: onMenuRow.hovered
        }
    }
    Item {
        id: wideRow
        y: 360
        width: root.width
        height: Theme.rowHeight
        HoverHandler { id: onWideRow }
        readonly property TipAsk wideTip: TipAsk {
            target: wideRow; words: "src/main.rs"; rowSide: "left"; asked: onWideRow.hovered
        }
    }
    // A target whose words move while the rest counts, and one that takes its ask back.
    Item {
        id: mover
        x: 700
        y: 20
        width: 200
        height: 40
        property bool asks: false
        property string why: "Stage something to commit"
        property bool taken: false
        readonly property TipAsk askMover: TipAsk {
            id: askMover; target: mover; words: mover.why; asked: mover.asks && !mover.taken
        }
    }
    /// Every time the box comes out.
    SignalSpy {
        id: shows
        target: root.box
        signalName: "aboutToShow"
    }

    TestCase {
        name: "TipSeat"
        when: windowShown

        function init() {
            if (menu.visible)
                menu.dismiss()
            // A corner nothing reaches, and the beat the box leaves on.
            mouseMove(root, root.width - 1, root.height - 1)
            typedBox.refused = false
            wideTyped.refused = false
            mover.asks = false
            mover.taken = false
            Hand.heldBar = null
            tryVerify(() => !root.box.visible && seat.current === null, undefined, "each walk starts with nothing out")
            rowA.x = 0
            shows.clear()
        }

        function middleOf(row) {
            return row.y + row.height / 2
        }
        /// How far the box's middle stands off `x`, in the window.
        function boxOff(x) {
            const ground = root.box.background
            return Math.abs(ground.mapToItem(root, ground.width / 2, 0).x - x)
        }
        /// Rests the hand on `row` at `across` (a share of its width) and answers the box's seat in the row's terms.
        /// The box comes out at the previous words' size and takes its own a frame later, so the seat is read once the
        /// size is the words'.
        function restOn(row, across) {
            const box = root.box
            mouseMove(root, row.x + row.width * across, row.y + row.height / 2)
            tryVerify(() => box.visible && box.parent === row, undefined, "the rest puts the row's box out")
            tryVerify(() => box.width === box.implicitWidth && box.width > 0, undefined,
                      "the box takes its words' size")
            return { left: box.x, right: box.x + box.width, top: box.y, bottom: box.y + box.height }
        }
        /// The box's band is the row's, centred: the rows either side get no more than the overhang.
        function verifyOnTheRowsBand(row, at) {
            const overhang = (root.box.height - row.height) / 2
            fuzzyCompare(at.top, -overhang, 1, "centred on the row's band, not above it or below it")
            fuzzyCompare(at.bottom, row.height + overhang, 1)
        }

        // Every target waits its own rest; the neighbour's box goes first (規約「隣の的へは箱を下ろしてから移る」).
        function test_a_the_next_row_is_asked_the_question_too() {
            const box = root.box
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => box.visible, undefined, "the row the hand rested on puts a box out")
            compare(box.parent, rowA, "on that row")
            verify(askA.shown)
            mouseMove(root, 150, middleOf(rowB))
            tryVerify(() => onB.hovered, undefined, "the hand is on the next row")
            tryVerify(() => !box.visible, undefined, "the box does not travel to a row that has only been reached")
            verify(!askA.shown)
            // By identity: `compare` walks an object's properties, and an ask refers to its seat, which refers back.
            verify(seat.current === askB && seat.machineState === "waiting", "the next row's rest is counting")
            tryVerify(() => box.visible, undefined, "which puts it out once the rest is over")
            compare(box.parent, rowB, "on the row the hand is on now")
            verify(askB.shown)
        }

        // 規約「出ているものの的へ戻る手は即通す」: the hand off and back inside the beat keeps the box, no rest.
        function test_b_the_row_the_box_is_out_on_is_not() {
            const box = root.box
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => box.visible)
            shows.clear()
            mouseMove(root, root.width - 1, root.height - 1)
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => box.visible && !seat.settling, undefined, "the box is up and the beat is off")
            compare(shows.count, 0, "held, not opened again")
            compare(box.parent, rowA)
        }

        // The walk the beat exists for: into the box itself, which takes the pointer off the row that raised it.
        function test_c_a_hand_that_steps_into_the_box_keeps_it() {
            const box = root.box
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => box.visible)
            mouseMove(root, 150, rowA.y + box.y + box.height / 2)
            tryVerify(() => seat.pointed && box.visible, undefined,
                      "the hand is inside the box, and the box is the one that was already out")
            compare(box.parent, rowA, "still the row's")
            // waits(paced): the subject is a box that must **not** go while the hand is in it; the wait is the beat.
            wait(Metrics.hoverKeepMs * 3)
            verify(box.visible && askA.shown, "and stays as long as the hand is in it")
        }

        // Asked for from the line above, a box up on the target's top is a diagonal walk away: a hand going straight
        // sideways leaves both and it falls (デザイン規約 §hover のツールチップ「手の傍に」).
        function test_d_a_box_beside_its_target_opens_level_with_the_hand() {
            const box = root.box
            const at = middleOf(nameLine)
            mouseMove(root, 150, at)
            tryVerify(() => box.visible, undefined, "the rest on the line above opens the box")
            compare(box.parent, aside, "seated against the target under that line")
            const top = aside.y + box.y
            verify(top <= at && at <= top + box.height,
                   "the hand's own line is inside the box: " + top + ".." + (top + box.height) + " for " + at)
            mouseMove(root, aside.x + box.x + box.width / 2, at)
            tryVerify(() => seat.pointed && box.visible, undefined,
                      "a hand that only went sideways is inside the box, and the box is still out")
        }

        // A target that moved while its box was down, under a hand back on the pixel it rested on: the box comes out
        // on the hand, against where the target stands now (デザイン規約 §hover のツールチップ「手の傍に」).
        function test_e_a_box_comes_out_against_where_its_target_stands_now() {
            const box = root.box
            const at = Qt.point(150, middleOf(rowA))
            mouseMove(root, at.x, at.y)
            tryVerify(() => box.visible && box.parent === rowA, undefined, "the rest on the row opens its box")
            verify(box.width < rowA.width, "a box narrower than its row, so it stands on the hand")
            verify(boxOff(at.x) <= 1, "on the hand: " + boxOff(at.x) + " off")
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !box.visible && seat.current === null, undefined, "down once the hand has gone")
            rowA.x = 40
            mouseMove(root, at.x, at.y)
            tryVerify(() => box.visible && box.parent === rowA, undefined, "out again on the row that moved")
            verify(boxOff(at.x) <= 1, "on the hand again, with the row's move taken in: " + boxOff(at.x) + " off")
        }

        // A refusal of what is typed waits its rest before the box, as every ask does (「入力の拒否への補足は一律」):
        // typed, the seat is deciding and nothing is out; the rest run out, the box stands with the refusal's words.
        // Read off the seat's machine — the rest is the product's own (`Metrics.tipDelayMs`), and a box out at once
        // would be read here before the rest could run.
        function test_e2_a_refusal_of_typing_waits_its_rest_before_the_box() {
            const box = root.box
            typedBox.refused = true
            compare(seat.machineState, "waiting", "typed: the refusal's ask is resting")
            verify(!box.visible, "nothing is out while the rest counts")
            verify(seat.machine.rest.running, "and it is the seat's own rest that counts")
            tryCompare(seat, "machineState", "shown")
            verify(box.visible && box.parent === typedBox, "the rest run out, the refusal's box stands")
            compare(box.text, askTyped.words)
        }

        // A refusal of what is typed stands until the typing changes it (「入力の拒否への補足は一律」): a hand resting on
        // rows that ask for their own boxes, and a hand walking into the box and out again, leave it up with its own
        // words; the typing that ends it takes it down for good.
        function test_f_a_refusal_of_typing_stands_until_the_typing_changes() {
            const box = root.box
            typedBox.refused = true
            tryVerify(() => box.visible && box.parent === typedBox, undefined, "the refusal puts its box out")
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => onA.hovered, undefined, "the row is asking for its own box")
            // waits(paced): the subject is a box that must **not** move to the row; the wait is the row's rest.
            wait(Metrics.tipDelayMs * 2)
            verify(box.visible && box.parent === typedBox, "the refusal keeps the one box, with its own words")
            compare(box.text, askTyped.words)
            mouseMove(root, typedBox.x + box.x + box.width / 2, typedBox.y + box.y + box.height / 2)
            tryVerify(() => seat.pointed, undefined, "the hand inside the box")
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !seat.pointed && !seat.settling, undefined, "the hand left the box and no beat is counting")
            verify(box.visible && box.parent === typedBox, "a hand that left the box does not take it down")
            shows.clear()
            typedBox.refused = false
            tryVerify(() => !box.visible && seat.current === null, undefined,
                      "typing that ends the refusal takes it down")
            compare(shows.count, 0, "for good: the fall is the typing's, not a hand walking into the box")
        }

        // A refusal of what is typed stands on its field, not the hand: no hand asked for it, and the hand may be
        // anywhere in the window — here, the far corner `init` left it on.
        function test_g_a_refusal_of_typing_stands_on_its_field() {
            const box = root.box
            wideTyped.refused = true
            tryVerify(() => box.visible && box.parent === wideTyped, undefined, "the refusal puts its box out")
            verify(box.width < wideTyped.width, "a box narrower than its field, so where it stands across is asked")
            const ground = box.background
            const corner = ground.mapToItem(root, 0, 0)
            verify(Math.abs(corner.x - wideTyped.x) <= 1,
                   "from the field's left edge, not the hand at the corner: " + corner.x + " for " + wideTyped.x)
            verify(Math.abs(corner.y + ground.height - wideTyped.y) <= 1,
                   "flush on the field's top: " + (corner.y + ground.height) + " for " + wideTyped.y)
        }

        // Where the box stands for a row among rows (「行の的は、行の横に立つ」).
        function test_h_a_right_panels_row_puts_it_to_its_left() {
            const at = restOn(rightPane, 0.5)
            compare(at.right, 0, "flush against the row's left edge — out of the list, no gap to cross")
            verifyOnTheRowsBand(rightPane, at)
        }
        function test_h_a_left_panels_row_has_no_left_and_takes_its_right() {
            const at = restOn(leftPane, 0.5)
            compare(at.left, leftPane.width, "flush against the row's right edge")
            verifyOnTheRowsBand(leftPane, at)
        }
        function test_h_a_menu_row_takes_the_side_it_names() {
            const at = restOn(menuRow, 0.5)
            compare(at.left, menuRow.width, "on the right although the left has room too")
        }
        function test_h_a_row_as_wide_as_the_window_stands_it_beside_the_hand() {
            const across = 0.6
            const at = restOn(wideRow, across)
            const handX = wideRow.width * across
            verify(at.right < handX, "left of the hand, and not under it: " + at.right + " for " + handX)
            verify(handX - at.right <= Theme.spaceXs, "a step off the hand, not across the row from it")
            verifyOnTheRowsBand(wideRow, at)
        }

        // The words the box comes out with are the ones the target holds then, not the ones it held at the ask
        // and they follow the target while the box is up.
        function test_i_words_changed_while_the_rest_counts_are_the_ones_that_come_out() {
            const box = root.box
            mover.asks = true
            mover.why = "Amending would leave the last commit empty"
            verify(!box.visible, "the change is made while the rest still counts")
            tryVerify(() => box.visible, undefined, "the box comes out once the rest is over")
            compare(box.text, "Amending would leave the last commit empty", "with the words the target holds now")
            mover.why = "Copied!"
            compare(box.text, "Copied!", "and the box follows the words while it stands")
        }

        // A dismissed box — closed from outside the seat, as Escape with the keyboard in it would — is not put back
        // under the same still hand, though the seat answers the ask it kept for the row (a menu standing and going
        // meanwhile); the hand gone and back asks afresh.
        function test_j_a_dismissed_box_is_not_put_back_under_the_same_still_hand() {
            const box = root.box
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => box.visible)
            shows.clear()
            box.close()
            tryVerify(() => !box.visible && seat.current === null, undefined, "closed, and the seat let it go")
            verify(onA.hovered, "the hand is still on the row")
            menu.offerHere()
            tryVerify(() => menu.opened && onA.hovered, undefined, "a menu stands, clear of the hand")
            menu.dismiss()
            tryVerify(() => !menu.visible, undefined, "and goes: the seat answers what it kept")
            // waits(paced): the subject is a box that must **not** come back; the wait is a rest and the beat.
            wait(Metrics.tipDelayMs + Metrics.hoverKeepMs * 2)
            compare(shows.count, 0, "nothing put it back")
            mouseMove(root, root.width - 1, root.height - 1)
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => box.visible && box.parent === rowA, undefined, "the hand gone and back asks afresh")
        }

        // A menu raised over the hand takes the box down, and is the reader's input: the box stays down as it goes,
        // until the hand moves — whichever order Qt tells the leaves in. Come onto the row straight from outside the
        // window, the row's handler took the hover before the window's did, and Qt clears them in that order: the row's
        // leave comes while the window still holds the hand.
        function test_l_a_box_a_menu_over_the_hand_took_down_waits_for_the_hand_to_move() {
            const box = root.box
            mouseMove(root, root.width + 10, middleOf(rowA))
            tryVerify(() => !hand.known, undefined, "the hand is out of the window")
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => box.visible && box.parent === rowA, undefined, "the row's box is out")
            shows.clear()
            menu.popup(root, 150, middleOf(rowA))
            tryVerify(() => menu.opened && !box.visible && !onA.hovered, undefined,
                      "the menu over the hand took it down")
            menu.dismiss()
            tryVerify(() => !menu.visible && onA.hovered, undefined, "the menu went, the hand still on the row")
            // waits(paced): the subject is a box that must **not** come back; the wait is a rest and the beat.
            wait(Metrics.tipDelayMs + Metrics.hoverKeepMs * 2)
            compare(shows.count, 0, "the box the menu took down stays down")
            mouseMove(root, 152, middleOf(rowA))
            tryVerify(() => box.visible && box.parent === rowA, undefined, "the hand's own move brings it back")
        }

        // A target taking its ask back for a reason of its own (a tab taken up): the box goes now, not a beat
        // later, and the target asking again is heard.
        function test_k_a_target_that_takes_its_ask_back_is_heard_again() {
            const box = root.box
            mover.asks = true
            tryVerify(() => box.visible && box.parent === mover)
            shows.clear()
            mover.taken = true
            askMover.drop()
            verify(!seat.settling, "no beat: the ask was taken back, not left")
            tryVerify(() => !box.visible && seat.current === null)
            mover.taken = false
            tryVerify(() => box.visible && box.parent === mover, undefined, "asked afresh, the box comes back")
            compare(shows.count, 1)
        }

        // A scroll bar held takes the box down and drops the rest; a refusal of what is typed stays
        // (「スクロールバーを掴んだら、hover で開いたものは閉じる」/「hover で開いたのではないものは閉じない」).
        function test_l_a_held_bar_takes_the_box_down_and_the_refusal_stays() {
            const box = root.box
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => box.visible)
            Hand.heldBar = root
            tryVerify(() => !box.visible, undefined, "the box goes the moment the bar is taken")
            mouseMove(root, 150, middleOf(rowB))
            // waits(paced): the subject is a box that must **not** come out while the bar is held.
            wait(Metrics.tipDelayMs * 2)
            verify(!box.visible, "no row says anything while the bar is held")
            typedBox.refused = true
            tryVerify(() => box.visible && box.parent === typedBox, undefined,
                      "a refusal asked for while the bar is held comes out all the same")
            Hand.heldBar = null
            verify(box.visible && box.parent === typedBox, "and stays when it is let go")
        }
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// What hover does while a scroll bar is held (デザイン規約 §hover のツールチップ「スクロールバーを掴んだら、hover で
// 開いたものは閉じる」).
//
// Only a real pointer answers this: Qt delivers no hover at all while a bar holds the grab, so the rows keep what they
// had as they slide from under the hand, and a row the list reuses under the pointer takes the hover afresh
// (rules-refs/app-ui.md「バーを掴んでいる間、窓に hover は届かない」). Nothing a headless run injects goes through that.
Item {
    id: root
    width: 400
    height: 400

    PointerWatch { id: hand }

    SharedToolTip {
        id: shared
        host: root
        hand: hand
    }

    ListModel { id: files }

    // The right pane's changed files (`DetailsPane.fileList`): the pane's bar, over rows that reach under it.
    AppListView {
        id: list
        width: 200
        height: 240
        model: files
        verticalBar: PaneScrollBar {}
        delegate: FileRowDelegate {
            listWidth: list.width
        }
    }

    // A card read by scrolling it (`RefListPopup`, `SectionPeekPopup`): its own bar, inside it. Escape alone closes
    // it, as `SectionPeekPopup` and `BandStateCard` — a press outside is not what takes it down here.
    AppCard {
        id: peekCard
        x: 220
        y: 20
        width: 160
        height: 200
        closePolicy: Popup.CloseOnEscape
        tracksPointer: true
        contentPointed: cardHover.hovered
        contentItem: Item {
            HoverHandler { id: cardHover }
            AppListView {
                id: cardList
                anchors.fill: parent
                model: files
                verticalBar: PaneScrollBar {}
                delegate: FileRowDelegate {
                    listWidth: cardList.width
                }
            }
        }
    }
    HoverCardHost {
        id: peekKeep
        card: peekCard
    }

    /// A target asked while the bar is held — what a row reused under the hand does (it takes the hover afresh), with
    /// no rest to wait out.
    property bool askedWhileHeld: false
    Item {
        id: asker
        x: 220
        y: 250
        width: 160
        height: 40
        ToolTip.text: "asked"
        ToolTip.visible: root.askedWhileHeld
    }

    /// A box that answers typing, not a hand (`NavNameBox`'s refusal): it says so the way the box does.
    property bool refusedNow: false
    Item {
        id: typedBox
        y: 250
        width: 200
        height: 40
        readonly property bool tipTyped: true
        ToolTip.text: "refused"
        ToolTip.visible: root.refusedNow
    }
    // A bar over rows that ask for nothing, so the hand reaching it does not take the shared box off the name box —
    // which any hover ask does, bar or no bar.
    AppListView {
        id: quietList
        x: 220
        y: 300
        width: 160
        height: 90
        model: 40
        verticalBar: PaneScrollBar {}
        delegate: Item {
            width: quietList.width
            height: Theme.rowHeight
        }
    }

    TestCase {
        name: "BarGrab"
        when: windowShown

        function initTestCase() {
            shared.dressToolTip()
            for (let i = 0; i < 60; ++i)
                files.append({ "path": "src/file" + i + ".txt", "name": "file" + i + ".txt", "change": "M" })
        }

        function init() {
            // A corner nothing reaches, and the beat the box leaves on.
            mouseMove(root, root.width - 1, root.height - 1)
            peekKeep.pointedAt = false
            root.askedWhileHeld = false
            root.refusedNow = false
            peekCard.close()
            list.contentY = 0
            cardList.contentY = 0
            tryVerify(() => !shared.sharedTip.visible && !peekCard.visible, undefined,
                      "each case starts with nothing out")
            // The rows are laid out a frame after they are added; the bar stands once they overflow.
            tryVerify(() => barOf(list).visible, undefined, "the list has somewhere to go")
        }
        /// The bar a case took and did not get to let go of — a failed check leaves the button down for the next case.
        property Item heldOn: null
        function cleanup() {
            // On the window: the bar may be gone with what it stood in.
            if (heldOn !== null)
                mouseRelease(root, root.width - 1, root.height - 1)
            heldOn = null
        }
        function take(bar, x, y) {
            mousePress(bar, x, y)
            heldOn = bar
        }
        function letGo(item, x, y) {
            mouseRelease(item, x, y)
            heldOn = null
        }

        function barOf(view) {
            return view.ScrollBar.vertical
        }
        /// The middle of the bar's handle, in the bar's own coordinates — where a hand takes hold of it. Read off the
        /// handle itself: the arrows' ends are no part of its travel.
        function thumbY(bar) {
            return bar.contentItem.y + bar.contentItem.height / 2
        }
        /// The box is out and settled on the target it came out on. A list that has just built its rows can hand the
        /// row under the hand to a new delegate, and the box then goes down and counts again (`SharedToolTip.handOver`)
        /// — a press landing in that turn would be credited with the fall.
        function tipStands() {
            const tip = shared.sharedTip
            return tip.visible && shared.standing === tip.parent
        }

        // The case this was found by: the box of the row under the bar stands, the bar is taken, and the rows move.
        function test_a_the_box_goes_when_the_bar_is_taken() {
            const tip = shared.sharedTip
            const bar = barOf(list)
            const x = bar.width / 2
            const y = thumbY(bar)
            // The bar leaves hover to the rows under it (`AutoScrollBar.hoverEnabled`), so resting on it asks the row.
            mouseMove(bar, x, y)
            tryVerify(tipStands, undefined, "the row under the resting hand puts its box out")

            take(bar, x, y)
            verify(bar.pressed, "the hand holds the bar")
            verify(!tip.visible, "the box goes the moment the bar is taken")

            // Half way down, then to the foot, where the rows the list reuses come in under the hand.
            mouseMove(bar, x, bar.height / 2, -1, Qt.LeftButton)
            mouseMove(bar, x, bar.height - 1, -1, Qt.LeftButton)
            verify(list.contentY > 0, "the drag moved the rows")
            // waits(paced): the subject is a box that must **not** come out, so the wait is the rest a row would
            // count; nothing here answers sooner.
            wait(Metrics.tipDelayMs * 2)
            verify(!tip.visible, "no row the drag brought under the hand says anything while the bar is held")

            letGo(bar, x, bar.height - 1)
            // Hover comes back with the hand: a row it walks to puts its own box out, not one from before the drag.
            mouseMove(list, list.width / 2, Theme.rowHeight * 2 + Theme.rowHeight / 2)
            tryVerify(() => tip.visible, undefined, "after the bar is let go, a row the hand rests on is asked again")
            const row = list.itemAt(list.width / 2, list.contentY + Theme.rowHeight * 2 + Theme.rowHeight / 2)
            compare(tip.parent, row, "on the row the hand is on")
            compare(tip.text, row.pathText, "saying that row's path")
        }

        // The hand takes the bar before the row's rest is over: the count goes with the box.
        function test_b_a_rest_already_counting_opens_nothing_while_held() {
            const tip = shared.sharedTip
            const bar = barOf(list)
            const x = bar.width / 2
            const y = thumbY(bar)
            mouseMove(bar, x, y)
            take(bar, x, y)
            // waits(paced): the subject is a box that must **not** come out, so the wait is the rest it was counting.
            wait(Metrics.tipDelayMs * 2)
            verify(!tip.visible, "the rest that was counting when the bar was taken opens nothing")
            letGo(bar, x, y)
        }

        // Let go before that rest would have run out: it is not waiting on the far side of the release either. The
        // row under the still hand never stopped being hovered, so nothing asks again until the hand moves.
        function test_b_the_rest_does_not_outlive_a_short_hold() {
            const tip = shared.sharedTip
            const bar = barOf(list)
            const x = bar.width / 2
            const y = thumbY(bar)
            mouseMove(bar, x, y)
            take(bar, x, y)
            letGo(bar, x, y)
            // waits(paced): the subject is a box that must **not** come out, so the wait is the rest it was counting.
            wait(Metrics.tipDelayMs * 2)
            verify(!tip.visible, "the rest cut by the bar opens nothing after it is let go")

            mouseMove(bar, x, y + Theme.rowHeight)
            tryVerify(tipStands, undefined, "the hand moving on to the next row asks as ever")
        }

        // The exception: the bar is the card's own, and the hand holding it is reading the card.
        function test_c_the_card_the_bar_is_in_stays() {
            const tip = shared.sharedTip
            peekCard.open()
            tryVerify(() => peekCard.opened, undefined, "the card is out")
            const bar = barOf(cardList)
            tryVerify(() => bar.visible, undefined, "the card's rows have somewhere to go")
            const x = bar.width / 2
            const y = thumbY(bar)
            mouseMove(bar, x, y)
            tryVerify(() => peekCard.pointerInside, undefined, "the hand is in the card")
            tryVerify(tipStands, undefined, "and the row under it puts its box out")

            take(bar, x, y)
            verify(!tip.visible, "a row's box goes all the same: the rows are what the bar moves")
            verify(peekCard.visible, "but not the card the bar is in")
            mouseMove(bar, x, bar.height - 1, -1, Qt.LeftButton)
            verify(cardList.contentY > 0, "the drag moved the card's rows")
            // A drag takes the hand anywhere; the card stays while its bar is held.
            mouseMove(list, list.width / 2, list.height / 2, -1, Qt.LeftButton)
            // waits(paced): the subject is a card that must **not** go, so the wait is the beat it would go on.
            wait(Metrics.hoverKeepMs * 3)
            verify(peekCard.visible, "the card the bar is in stays up while its bar is held")

            // Let go outside it: hover is heard again, the hand is not in the card, and the beat takes it down.
            letGo(list, list.width / 2, list.height / 2)
            mouseMove(list, list.width / 2 + 1, list.height / 2)
            tryVerify(() => !peekCard.visible, undefined, "let go outside the card, it goes as any hover card does")
        }

        // A bar outside the card: what the card hangs off is about to slide from under the hand.
        function test_d_a_bar_outside_the_card_takes_it_down() {
            peekCard.open()
            // What the card hangs off still has the hand, as far as the card was told.
            peekKeep.pointedAt = true
            tryVerify(() => peekCard.opened, undefined, "the card is out")
            const bar = barOf(list)
            const x = bar.width / 2
            const y = thumbY(bar)
            mouseMove(bar, x, y)
            take(bar, x, y)
            tryVerify(() => !peekCard.visible, undefined, "the card goes the moment a bar outside it is taken")
            letGo(bar, x, y)
        }

        // Anything asked for while a bar is held — a rest that ran out, a row reused under the hand — goes at once.
        function test_e_nothing_opens_while_a_bar_is_held() {
            const bar = barOf(list)
            const x = bar.width / 2
            const y = thumbY(bar)
            mouseMove(bar, x, y)
            take(bar, x, y)
            peekCard.open()
            verify(!peekCard.visible, "a card asked for while a bar outside it is held does not come out")
            root.askedWhileHeld = true
            verify(!shared.sharedTip.visible, "nor does a box")
            letGo(bar, x, y)

            peekCard.open()
            tryVerify(() => peekCard.opened, undefined, "and a card opens as ever once the bar is let go")
            root.askedWhileHeld = false
            root.askedWhileHeld = true
            tryVerify(() => shared.sharedTip.visible, undefined, "and so does a box")
            root.askedWhileHeld = false
        }

        // What hover did not open is not the bar's to close: a name box's refusal answers what was typed, and in the
        // folded rail's peek the box stays open while the peek's bar looks through the names.
        function test_f_a_box_that_answers_typing_stays() {
            const tip = shared.sharedTip
            root.refusedNow = true
            tryVerify(() => tip.visible && tip.parent === typedBox, undefined, "the refusal is up")
            const bar = barOf(quietList)
            tryVerify(() => bar.visible, undefined, "the quiet list has somewhere to go")
            const x = bar.width / 2
            const y = thumbY(bar)
            mouseMove(bar, x, y)
            take(bar, x, y)
            verify(tip.visible && tip.parent === typedBox, "the bar leaves the refusal up")
            root.refusedNow = false
            root.refusedNow = true
            verify(tip.visible && tip.parent === typedBox, "and one asked for while the bar is held comes out")
            letGo(bar, x, y)
        }
    }
}

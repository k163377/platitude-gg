import QtQuick
import QtTest
import platitude.ui

// The reader moving the list under a still hand (デザイン規約 §左メニューの所作, §hover のツールチップ「的が手の下で
// 動けば hover は外れ、出したものはその瞬間に消える」): the row that slides out from under the hand drops what it put out
// — its open lines, its tip — at once and hands its light to the row now under the hand, and closing gives back only
// what the opening scrolled, never where the reader sent the list.
//
// The real list (`NavList`) and rows, in a panel that counts the hand the way the sidebar does
// (`SidebarPane.handWatch`), on the real open row (`OpenRowSeat`).
Item {
    id: root
    // Room right of the list, where a row's tip stands (`TipAsk.rowSide`) — not over the hand.
    width: 560
    // Room below the list, where the hand goes to be on no row.
    height: Theme.rowHeight * 8

    /// The section's model, cut down: rows by name, and one upstream line under every branch (`NavFacts.answers`) —
    /// also what a worktree's row asks about the branch it holds, as the branches' section would answer.
    component Rows: ListModel {
        function rowOfName(name) {
            for (let i = 0; i < count; ++i) {
                if (get(i).full === name)
                    return i
            }
            return -1
        }
        function upstreamOf(name) {
            return "origin/" + name
        }
        function upstreamOidOf(name) {
            return ""
        }
        function upstreamGoneOf(name) {
            return ""
        }
        function aheadOf(name) {
            return 0
        }
        function behindOf(name) {
            return 0
        }
        function oidOfName(name) {
            return ""
        }
    }
    Rows {
        id: rows
    }
    /// WORKTREES, for the one case on a row whose lines put a tip out (its path): each worktree holds the branch of its
    /// own name, so it has lines to open.
    Rows {
        id: worktrees
    }

    /// `SidebarRowGestures`, cut down to what the rows read off it, around the real open row (`OpenRowSeat`, as the
    /// gestures hold it): the key a row hands up, the seat, and what holds and closes it.
    readonly property QtObject gestures: QtObject {
        id: gestures
        property string activeKey: ""
        property string editKey: ""
        property bool menuOpen: false
        property string editMode: ""
        property string editText: ""
        property bool editRefused: false
        property string editRefusedWhy: ""
        property string editRefusedMark: ""
        property var reclick: null
        readonly property OpenRowSeat openRow: OpenRowSeat {
            id: openRowSeat
            hand: hand
            editKey: gestures.editKey
            host: panel
        }
        readonly property alias openKey: openRowSeat.openKey
        readonly property alias openSeat: openRowSeat.seat
        readonly property alias menuOnOpenRow: openRowSeat.menuOnRow
        function openFacts(key, at, row, now, model, name) {
            openRowSeat.openFacts(key, at, row, now, model, name)
        }
        function closeFacts(key, now) {
            openRowSeat.closeFacts(key, now)
        }
        function noteClick(key) {
        }
    }

    // The window's tooltip, as `Main` keeps it.
    HandWatch { id: windowHand }
    TipSeat {
        host: root
        hand: windowHand
    }

    Item {
        id: panel
        anchors.fill: parent
        /// Where the open row's lines are, as the sidebar answers it (`NavSections.openFactsItem`).
        function openFactsItem() {
            return list.openFactsItem() ?? worktreeList.openFactsItem()
        }
        // The panel's hand, as the sidebar keeps it (`SidebarPane`): the rows read their light and their rest off it.
        HandWatch {
            id: hand
        }

        NavList {
            id: list
            width: 260
            height: Theme.rowHeight * 5
            sectionModel: rows
            gestures: gestures
            paneHand: hand
            offersFacts: true
            kindHint: "branch"
        }
        /// The current branch's stand-in on the list's top edge, as `NavSections` lays one over BRANCHES
        /// (`HeadPinRow`, the list's `shield`): opaque, over the top row.
        Rectangle {
            id: cover
            parent: list
            width: list.width
            height: Theme.rowHeight
            visible: false
            color: Theme.bgSurface
        }
        /// WORKTREES in the same place, shown for the one case on it — a list of its own, as in the sidebar: a view
        /// handed another model keeps rows it uses again on the old model's names.
        NavList {
            id: worktreeList
            width: 260
            height: Theme.rowHeight * 5
            expanded: false
            sectionModel: worktrees
            branchesModel: worktrees
            paneHand: hand
            gestures: gestures
            offersFacts: true
            kindHint: "worktree"
        }
    }

    TestCase {
        id: testCase
        name: "WheelUnderOpenRow"
        when: windowShown

        readonly property real handX: list.width / 2
        /// The list the helpers act on.
        property Item view: list
        /// The one row that does not open: a folder, which says its path in a tip instead.
        readonly property int folderRow: 3

        function nameAt(index) {
            return (index === testCase.folderRow ? "f" : "b") + (index < 10 ? "0" : "") + index
        }
        function initTestCase() {
            for (let i = 0; i < 30; ++i) {
                const name = testCase.nameAt(i)
                rows.append({
                    "name": name, "full": name, "oid_hex": "", "change": "", "bucket": "", "orig_path": "",
                    "orig_name": "", "is_head": false, "has_remote": false, "only_remote": false, "has_pr": false,
                    "depth": 0, "folder": i === testCase.folderRow, "eol_mark": false, "ahead": 0, "behind": 0
                })
                // A worktree's row is named by its folder and carries the branch it holds in `bucket` (`NavFacts`).
                worktrees.append({
                    "name": "w" + name, "full": "w" + name, "oid_hex": "", "change": "", "bucket": name,
                    "orig_path": "", "orig_name": "", "is_head": false, "has_remote": false, "only_remote": false,
                    "has_pr": false, "depth": 0, "folder": false, "eol_mark": false, "ahead": 0, "behind": 0
                })
            }
        }
        /// Every case starts with the hand on no row and nothing open, the list at its top.
        function init() {
            testCase.handOff()
            testCase.scrollTo(0)
        }
        /// And ends with the list as it began: BRANCHES, the height of five rows.
        function cleanup() {
            cover.visible = false
            list.shield = null
            testCase.handOff()
            worktreeList.expanded = false
            list.expanded = true
            testCase.view = list
            list.height = Theme.rowHeight * 5
        }
        function handOff() {
            mouseMove(panel, testCase.handX, Theme.rowHeight * 7)
            tryCompare(gestures, "openKey", "", undefined, "the hand off the list leaves nothing open")
        }
        /// The list put somewhere before the hand arrives, its rows built there: a row built after the hand moved has
        /// arrived under a still hand, and does not open (デザイン規約 §左メニューの所作).
        function scrollTo(y) {
            const view = testCase.view
            view.contentY = y
            view.forceLayout()
            // A frame asked for: the list may already stand at `y`, and `waitForRendering` only waits for the next one
            // (rules-refs/app-ui.md「`frameSwapped` を待つなら頼むのは `window.update()`」).
            view.Window.window.update()
            verify(waitForRendering(view), "the list is drawn where it was put")
        }

        /// The hand rests at `y` in the list until the row there opens, and the lines are built under it.
        function restAt(y) {
            const view = testCase.view
            mouseMove(panel, testCase.handX, y)
            const index = view.indexAt(testCase.handX, view.contentY + y)
            tryCompare(gestures, "openKey", view.keyOf(view.sectionModel.get(index).full, ""), undefined,
                       "the row under the hand opens")
            const row = view.itemAtIndex(index)
            tryVerify(() => row.factsItem !== null && row.factsItem.height > 0, undefined,
                      "its lines are built under it")
            // The view lays its rows out again in its own pass, after the row grew: the row below stands under the
            // lines only once it has, and a place reckoned from the lines before then is on another row.
            tryVerify(() => {
                const next = view.itemAtIndex(index + 1)
                return next === null || next.y === row.y + row.height
            }, undefined, "the rows below stand under its lines")
            return row
        }
        /// A notch of the wheel up — the rows move down past the hand — sent its whole way: three rows, or to the
        /// list's top (`NavList.sendRows`). **A row closing on the way does not cut it short**, as Flickable's own
        /// wheel was cut on Windows (the content shrinking mid-way resets its timeline).
        function wheelUp(y) {
            const view = testCase.view
            const to = view.clampY(view.contentY - Metrics.wheelRows * Theme.rowHeight)
            mouseWheel(view, testCase.handX, y, 0, 120)
            tryCompare(view, "contentY", to, undefined, "the notch went its whole way")
        }

        function test_a_wheel_drops_the_lines_and_keeps_where_it_sent_the_list() {
            testCase.scrollTo(Theme.rowHeight * 5)
            const rest = list.contentY
            // Near the top of the row's own line, so the rows moving down take it from under the hand.
            const handY = Theme.rowHeight + 3
            const row = testCase.restAt(handY)
            const opened = row.index
            compare(list.contentY, rest, "a row that opened in full view moved nothing")

            const heard = hand.moves
            testCase.wheelUp(handY)
            tryCompare(gestures, "openKey", "", undefined, "the row slid out from under the hand and its lines went")
            const sent = list.contentY
            compare(hand.moves, heard, "with the hand never having moved")
            compare(row.index, opened, "the row is still the one that opened")
            // The light goes to the row under the hand, as Qt's own lists hand their hover on with a scroll (the wash
            // is no judgement — デザイン規約 §左メニューの所作「手が動いたかの見切り」).
            verify(!row.washLit, "the row that slid away lets its light go")
            verify(list.rowWashLit(list.indexAt(testCase.handX, sent + handY)), "to the row under the hand")

            // The hand moves a pixel: nothing scrolls, and the light stays where the hand is.
            mouseMove(panel, testCase.handX + 1, handY)
            testCase.turnPassed()
            compare(list.contentY, sent, "the hand moving takes back nothing the wheel sent")
            verify(!row.washLit, "the row that slid away stays dark")
            verify(list.rowWashLit(list.indexAt(testCase.handX, sent + handY)), "the row under the hand stays lit")
        }

        /// The reader's scroll stepping rows under the hand at once (the middle button's drift, `AppListView.hand`):
        /// the row that came under it asked in the same move as the open row that went let go — the rows read the hand
        /// again in the order their delegates heard it first — and that ask stands: the hand's own move opens it after
        /// the rest. Up, then down, so the row that came is read before the row that went in one of the two steps,
        /// whichever order the delegates hear in.
        function test_the_row_a_step_brings_under_the_hand_opens_once_the_hand_moves() {
            testCase.scrollTo(Theme.rowHeight * 8)
            const handY = Theme.rowHeight + 3
            testCase.restAt(handY)
            for (const down of [false, true]) {
                list.hand.scrolling = true
                list.contentY = list.contentY + (down ? 3 : -3) * Theme.rowHeight
                list.hand.scrolling = false
                tryCompare(gestures, "openKey", "", undefined, "the row slid away and its lines went")
                // waits(paced): a row that must **not** open under the still hand; the rest it would take, twice over.
                wait(Metrics.tipDelayMs * 2)
                compare(gestures.openKey, "", "nothing opens under a hand that did not move")
                // Read once the lines that went have folded away: the rows below them close up under the hand.
                const handX = testCase.handX + (down ? 0 : 1)
                const index = list.indexAt(handX, list.contentY + handY)
                verify(index !== testCase.folderRow, "a row that opens is under the hand")
                mouseMove(panel, handX, handY)
                tryCompare(gestures, "openKey", list.keyOf(list.sectionModel.get(index).full, ""), undefined,
                           "the hand's move opens the row under it (" + (down ? "down" : "up") + ")")
            }
        }

        /// A row folding its lines moves the rows under the hand, which the reader did not move: the row the hand
        /// walked to keeps the light, as Qt's own lists keep it on their item (デザイン規約 §左メニューの所作「行が自分で
        /// 伸び縮みした時だけは、手が歩いて来た行が光を持つ」), until the hand's own move hands it to the row under it.
        function test_a_row_folding_its_lines_leaves_the_light_on_the_row_the_hand_walked_to() {
            const top = testCase.restAt(Theme.rowHeight / 2)
            const lines = top.factsItem.height
            verify(lines > Theme.rowHeight / 2, "the open row's lines are deep enough to move a row: " + lines)
            // Down past the lines onto the next row: the hand has left the open row, whose lines fold away.
            const walkedY = Theme.rowHeight + lines + Theme.rowHeight / 2
            mouseMove(panel, testCase.handX, walkedY)
            const walked = list.itemAtIndex(1)
            verify(walked.washLit, "the hand lit the row it walked to")
            tryCompare(gestures, "openKey", "", undefined, "the open row closed as the hand left it")
            tryVerify(() => walked.y < Theme.rowHeight + lines / 2, undefined, "and the rows closed up under the hand")
            const under = list.itemAtIndex(list.indexAt(testCase.handX, list.contentY + walkedY))
            verify(under !== null && under !== walked, "another row is under the hand now")
            verify(walked.washLit, "the row the hand walked to keeps the light")
            verify(!under.washLit, "the row the fold brought under the hand does not take it")
            mouseMove(panel, testCase.handX + 1, walkedY)
            tryVerify(() => under.washLit && !walked.washLit, undefined, "the hand's own move hands it on")
        }

        /// The stand-in on the list's edge stands over a row (`NavList.shield`): the hand on it is not on that row,
        /// which neither lights nor opens; with the stand-in gone, the hand's move is on the row.
        function test_a_row_under_the_stand_in_is_not_under_the_hand() {
            list.shield = cover
            cover.visible = true
            mouseMove(panel, testCase.handX, Theme.rowHeight / 2)
            // waits(paced): a row that must **not** open or light; the rest it would take, twice over.
            wait(Metrics.tipDelayMs * 2)
            compare(gestures.openKey, "", "the row under the stand-in does not open")
            verify(!list.rowWashLit(0), "nor light")
            cover.visible = false
            mouseMove(panel, testCase.handX + 1, Theme.rowHeight / 2)
            tryCompare(gestures, "openKey", list.keyOf(list.sectionModel.get(0).full, ""), undefined,
                       "with the stand-in gone, the hand's move is on the row")
        }

        /// A row that says its path in a tip (a folder) drops the tip the same way.
        function test_a_wheel_drops_the_tip_of_the_row_it_slides_away() {
            testCase.scrollTo(Theme.rowHeight * (testCase.folderRow - 1))
            const handY = Theme.rowHeight + 3
            mouseMove(panel, testCase.handX, handY)
            const row = list.itemAtIndex(testCase.folderRow)
            tryVerify(() => row.tipShown, undefined, "the folder says its path after the rest")

            testCase.wheelUp(handY)
            tryVerify(() => !row.tipShown, undefined,
                      "the tip goes the moment the row slides from under the hand")
            verify(!row.washLit, "and the row lets its light go")
            verify(list.rowWashLit(list.indexAt(testCase.handX, list.contentY + handY)), "to the row under the hand")
        }

        /// A row whose lines have put a tip out (a worktree's path) is held by that tip while the hand walks into
        /// it — not when the reader's scroll takes the row away: its lines, the tip and its light go at once.
        function test_a_row_holding_its_path_tip_drops_it_with_its_lines() {
            list.expanded = false
            worktreeList.expanded = true
            testCase.view = worktreeList
            testCase.scrollTo(Theme.rowHeight * 5)
            const handY = Theme.rowHeight + 3
            const row = testCase.restAt(handY)
            tryVerify(() => row.factsTipOut, undefined, "the path comes out on a second rest")

            // What was open once the turn the path went in had run out: a row whose lines the seat held while its tip
            // stood (`SidebarRowGestures.openTipOut`) would still be open then, a beat behind the tip.
            let openAsTipWent = "unseen"
            const note = () => {
                if (!row.factsTipOut && openAsTipWent === "unseen") {
                    openAsTipWent = "pending"
                    Qt.callLater(() => openAsTipWent = gestures.openKey)
                }
            }
            row.factsTipOutChanged.connect(note)
            // A flick, not the wheel: QtTest's own tip stands over the row's line, where the hand is (the app's stands
            // beside the list — `TipAsk.beside`), and would take the notch.
            const from = worktreeList.contentY
            worktreeList.flick(0, 600)
            tryCompare(worktreeList, "moving", false, undefined, "the flick has come to rest")
            row.factsTipOutChanged.disconnect(note)
            verify(worktreeList.contentY < from, "the flick moved the rows down past the hand")
            compare(openAsTipWent, "", "the path went with the lines — none left up a beat")
            compare(gestures.openKey, "", "the row slid out from under the hand and its lines went")
            testCase.turnPassed()
            verify(!row.washLit, "and the row lets its light go")
        }

        /// A delegate the view uses again is another row (`ListView.reuseItems`): the light the hand left on one does
        /// not travel with it — nor the rest it was keeping, which would open that other row under a still hand.
        function test_a_delegate_used_again_does_not_carry_the_light() {
            const handY = Theme.rowHeight * 1.5
            mouseMove(panel, testCase.handX, handY)
            const row = list.itemAtIndex(1)
            tryVerify(() => row.washLit, undefined, "the row the hand walked onto is lit")

            // Before its rest runs out, the rows go past the hand a row at a time until this delegate is used again —
            // a flick leaves it in the pool on one run and on a row the next.
            for (let y = Theme.rowHeight; y <= Theme.rowHeight * 25 && (row.index < 0 || row.index === 1);
                 y += Theme.rowHeight) {
                list.contentY = y
                list.forceLayout()
            }
            verify(row.index >= 0 && row.index !== 1, "the delegate was used again for another row")
            verify(!row.washLit, "and does not carry the light to it")
        }

        /// The view rounds a place between whole pixels as it lays its rows on a list that is not moving
        /// (`QQuickFlickablePrivate::fixup`) — what a reveal writes where a row is not whole pixels tall. That is no
        /// one moving the list: closing still gives the opening's scroll back.
        function test_a_reveal_the_view_rounds_is_still_given_back() {
            list.height = Theme.rowHeight * 5 + 0.4
            testCase.scrollTo(Theme.rowHeight * 10)
            const rest = list.contentY
            testCase.restAt(Theme.rowHeight * 4.5)
            tryVerify(() => list.contentY > rest && list.contentY === Math.round(list.contentY), undefined,
                      "the list scrolled to show the lines, to a place the view rounded")

            mouseMove(panel, testCase.handX, Theme.rowHeight * 7)
            tryCompare(gestures, "openKey", "", undefined, "the hand leaving closes the row")
            compare(list.contentY, rest, "and the list is back where the hand found it")
        }

        /// The row closing on its way past the hand writes nothing to the list: a write there stops the scroll where it
        /// stands (`QQuickFlickable::setContentY` resets the timeline). Held against the same flick over closed rows.
        function test_a_row_closing_mid_scroll_leaves_the_scroll_running() {
            list.flick(0, -600)
            tryCompare(list, "moving", false, undefined, "the flick over closed rows has come to rest")
            const whole = list.contentY
            verify(whole > Theme.rowHeight * 3, "and went past the row the hand will rest on")
            testCase.scrollTo(0)

            testCase.restAt(Theme.rowHeight * 1.5)
            list.flick(0, -600)
            tryCompare(gestures, "openKey", "", undefined, "the row closed as it passed the hand")
            tryCompare(list, "moving", false, undefined, "the flick has come to rest")
            fuzzyCompare(list.contentY, whole, Theme.rowHeight / 2, "and ran its whole way")
        }

        /// A row at the foot of a section with no height to give scrolls the list to show its lines, and gives that
        /// back as it closes.
        function test_the_foot_row_gives_back_what_its_opening_scrolled() {
            testCase.scrollTo(Theme.rowHeight * 10)
            const rest = list.contentY
            testCase.restAt(Theme.rowHeight * 4.5)
            verify(list.contentY > rest, "the list scrolled to show the lines")

            mouseMove(panel, testCase.handX, Theme.rowHeight * 7)
            tryCompare(gestures, "openKey", "", undefined, "the hand leaving closes the row")
            compare(list.contentY, rest, "and the list is back where the hand found it")
        }

        /// Moved by the reader while open — a flick short enough to leave the hand on the row — the list is theirs:
        /// closing gives nothing back.
        function test_a_list_the_reader_moved_while_open_is_left_where_they_sent_it() {
            testCase.scrollTo(Theme.rowHeight * 10)
            const row = testCase.restAt(Theme.rowHeight * 4.5)
            const shown = list.contentY

            list.flick(0, 150)
            tryCompare(list, "moving", false, undefined, "the flick has come to rest")
            const sent = list.contentY
            verify(sent < shown, "the flick moved the list")
            testCase.turnPassed()
            verify(row.factsOpen, "and left the hand on the open row")

            mouseMove(panel, testCase.handX, Theme.rowHeight * 7)
            tryCompare(gestures, "openKey", "", undefined, "the hand leaving closes the row")
            compare(list.contentY, sent, "and the list stays where the reader sent it")
        }

        /// Rows read their hover a turn after the hand moved (`NavItemDelegate.onHandMovesChanged` →
        /// `Qt.callLater`); a call queued after the move runs after theirs, so waiting on it waits on that read.
        property int turns: 0
        function turnDone() {
            testCase.turns++
        }
        function turnPassed() {
            const before = testCase.turns
            Qt.callLater(testCase.turnDone)
            tryCompare(testCase, "turns", before + 1, undefined, "the turn after the hand moved has passed")
        }
    }
}

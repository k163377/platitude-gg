import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A row with its lines open is one row, lines included: a hand walking down into them has not left it, so the row
// stays lit and the lines stay out (デザイン規約 §左メニューの所作, 規約 §hover のツールチップ「hover が出した字は選べて、コピーできる」).
//
// The real row, in a panel that keeps the hand the way the sidebar does (`SidebarPane.hand` → `HandWatch` →
// `NavItemDelegate.syncHand`), on the real open row (`OpenRowSeat`). The lines take hover of their own (`CardText`
// among them), and stacked as a sibling over the row's handler they take the pointer off it (rules-refs/app-ui.md
// 「奪うのは覆う `MouseArea` の子孫でない hover 持ちだけ」) — so the row has to read its lines' hover as its own.
Item {
    id: root
    width: 320
    height: 240

    /// Empty leaves the line going nowhere; a commit makes it a target (`NavFacts.place`), which lays a band over it.
    property string readingAt: ""
    /// The page's menu, cut down: raised at the hand by the row's ask (`RepoPage.openRefMenu`) unless nothing is on
    /// offer (`refuse`, as `AppMenu.offer` with an empty menu), keyed by the row it was raised on while it stands
    /// (`RepoPage.menuDoor`, read as `OpenRowSeat.menuOnRow` reads it).
    property bool refuse: false
    property string menuKey: ""
    Popup {
        id: menu
        width: 160
        height: Theme.rowHeight * 6
        onClosed: root.menuKey = ""
    }
    /// The panel's open row, the real one (`OpenRowSeat`, as `SidebarRowGestures.openRow` holds it): the row asks it
    /// and reads it back, and the page's menu door is the menu above.
    readonly property OpenRowSeat openRow: OpenRowSeat {
        id: openRow
        hand: hand
        menuDoor: menu.visible && root.menuKey !== "" ? { "area": "sidebar", "key": root.menuKey } : null
        host: root
    }
    /// Where the open row's lines are, as the panel answers it (`NavSections.openFactsItem`).
    function openFactsItem() {
        return testCase.row !== null ? testCase.row.factsItem : null
    }

    // Stand-ins that give the row one line to walk into; the table itself is read in `tst_navfacts.qml`.
    QtObject {
        id: sections
        function upstreamOf(name) { return "origin/feature/topic-a" }
        function upstreamOidOf(name) { return root.readingAt }
        function trackedBy(full) { return "" }
        function tagRemotes(name, against) { return [] }
    }
    QtObject {
        id: branches
        function upstreamOf(name) { return "" }
        function upstreamOidOf(name) { return "" }
        function upstreamGoneOf(name) { return "" }
        function aheadOf(name) { return 0 }
        function behindOf(name) { return 0 }
        function oidOfName(name) { return "" }
    }
    QtObject {
        id: worktrees
        function worktreeHolding(name) { return "" }
        function headOfWorktree(path) { return "" }
    }

    Item {
        id: panel
        anchors.fill: parent
        // The panel's hand, as the sidebar keeps it (`SidebarPane`): the row reads its light off it.
        HandWatch {
            id: hand
        }
    }

    Component {
        id: rowMaker
        NavItemDelegate {
            id: row
            index: 0
            name: "feature/topic-a"
            full: "feature/topic-a"
            oid_hex: ""
            change: ""
            bucket: ""
            orig_path: ""
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 0
            folder: false
            eol_mark: false
            ahead: 0
            behind: 0
            kindHint: "branch"
            rowKey: "branch:feature/topic-a"
            listWidth: panel.width
            opensFacts: true
            sectionModel: sections
            branchesModel: branches
            worktreesModel: worktrees
            // The panel's hand, as the sidebar's lists hand it down (`NavList`).
            paneHand: hand
            // As the panel's lists hand the row's asks on (`NavList` → `SidebarRowGestures.openFacts` / `closeFacts`).
            onFactsAsked: (open, at, now) => {
                if (open)
                    openRow.openFacts(row.rowKey, at, row, now)
                else
                    openRow.closeFacts(row.rowKey, now)
            }
            openKey: openRow.openKey
            menuOnOpenRow: openRow.menuOnRow
            onRefMenuRequested: {
                if (root.refuse)
                    return
                root.menuKey = row.rowKey
                const at = panel.mapFromItem(null, hand.at.x, hand.at.y)
                menu.x = at.x
                menu.y = at.y
                menu.open()
            }
        }
    }

    TestCase {
        id: testCase
        name: "OpenRowHoldsTheHand"
        when: windowShown

        /// Built per case and taken down however the case ended — a leftover row is what the next case's hand would
        /// land on, and a leftover menu takes the hand off every row under it.
        property Item row: null
        function init() {
            root.readingAt = ""
            root.refuse = false
            testCase.row = rowMaker.createObject(panel)
        }
        function cleanup() {
            menu.close()
            root.menuKey = ""
            openRow.seat.takeDown("cleanup", false)
            openRow.seat.latchedKey = ""
            testCase.row.destroy()
            testCase.row = null
        }

        /// A turn after the hand moved: the row reads the hand on the delivery itself (`NavItemDelegate.syncHand`),
        /// and a call queued after the move runs after everything the move reached.
        /// Not a frame: a still offscreen scene need not draw one, and `waitForRendering` then returns false.
        property int turns: 0
        function turnDone() {
            testCase.turns++
        }
        function turnPassed() {
            const before = testCase.turns
            Qt.callLater(testCase.turnDone)
            tryCompare(testCase, "turns", before + 1, undefined, "the turn after the hand moved has passed")
        }

        function test_the_hand_walking_into_the_lines_is_still_on_the_row() {
            const row = testCase.row
            // The row opens after the usual rest (`Metrics.tipDelayMs`).
            mouseMove(panel, panel.width / 2, Theme.rowHeight / 2)
            tryCompare(row, "pointed", true, undefined, "the row is under the hand")
            tryCompare(row, "factsOpen", true, undefined, "and opens its lines under itself")
            tryVerify(() => row.factsItem !== null && row.factsItem.height > 0, undefined,
                      "the lines are built and stand under the row's own line")

            // Down onto the words. Two witnesses outside the row's own answer: the panel heard the move (so the row
            // was asked to read again), and the lines saw the hand arrive.
            const heard = hand.moves
            mouseMove(panel, panel.width / 2, Theme.rowHeight + row.factsItem.height / 2)
            verify(hand.moves > heard, "the panel hears the hand move into the lines")
            verify(row.factsItem.pointed, "and the lines see the hand on them")
            testCase.turnPassed()
            compare(row.pointed, true, "the hand on the lines is the hand on the row")
            compare(row.factsOpen, true, "so the lines stay out under it")

            // What holds the row open is the hand, not the test.
            mouseMove(panel, panel.width / 2, row.height + Theme.rowHeight)
            tryCompare(row, "factsOpen", false, undefined, "the row closes once the hand is off it")
        }

        /// Onto a line that goes somewhere (`NavRowFacts.aimRow`): its band lights from the line alone, never from the
        /// row's own line above, and the hand on it is still on the row.
        function test_the_hand_walking_onto_a_line_that_goes_lights_that_line_alone() {
            root.readingAt = "abc"
            const row = testCase.row
            mouseMove(panel, panel.width / 2, Theme.rowHeight / 2)
            tryCompare(row, "factsOpen", true, undefined, "the row opens under the hand")
            tryVerify(() => row.factsItem !== null && row.factsItem.height > 0, undefined, "and its lines stand")
            compare(row.factsLines[0].to.oid, "abc", "the line goes somewhere")
            // Every height of the row's own line, its foot included — the band must not reach up into it.
            for (let y = 1; y < Theme.rowHeight; y += 2) {
                mouseMove(panel, panel.width / 2, y)
                testCase.turnPassed()
                verify(row.factsItem !== null, "the row is still open with the hand on its own line (y " + y + ")")
                verify(!row.factsItem.lineAimed(0), "the line is not lit from the row's own line (y " + y + ")")
            }
            mouseMove(panel, panel.width / 2, Theme.rowHeight + row.factsItem.height / 2)
            testCase.turnPassed()
            verify(row.factsItem.lineAimed(0), "on the line, the line is lit")
            compare(row.pointed, true, "and the hand on it is the hand on the row")
            compare(row.factsOpen, true, "so the lines stay out under it")
        }

        /// A menu raised on the open row is about it (デザイン規約 §左メニューの所作): the hand going into the menu and
        /// down it, past the row's foot, has not left the row while the menu stands. Once the menu goes, the hand
        /// decides again — here it is off the row, which closes.
        function test_a_menu_raised_on_the_open_row_holds_it_while_it_stands() {
            const row = testCase.row
            row.oid_hex = "0de"
            const x = panel.width / 2
            mouseMove(panel, x, Theme.rowHeight / 2)
            tryCompare(row, "factsOpen", true, undefined, "the row opens under the hand")
            tryVerify(() => row.factsItem !== null && row.factsItem.height > 0, undefined, "and its lines stand")

            mouseClick(panel, x, Theme.rowHeight / 2, Qt.RightButton)
            verify(menu.visible, "the right-click on the row's own line raised its menu")
            const heard = hand.moves
            mouseMove(panel, x + Theme.spaceSm, Theme.rowHeight / 2 + Theme.spaceSm)
            mouseMove(panel, x + Theme.spaceSm, row.height + Theme.rowHeight)
            // The hand off the row is told to its seat at once (`NavItemDelegate.stillPointed`), and the open row's
            // seat keeps no beat (`SidebarRowGestures.openSeat`, `graceMs: 0`): a close would land within the turn.
            testCase.turnPassed()
            compare(row.factsOpen, true, "the row the menu is about stays open under it")
            verify(hand.moves > heard, "with the panel hearing the hand move, so the row was asked again")

            menu.close()
            tryCompare(row, "factsOpen", false, undefined, "once the menu goes, the hand off the row closes it")
        }

        /// A menu asked for on the open row that does not open (nothing on offer) holds nothing: the next menu, raised
        /// elsewhere, holds nothing open either, and the hand leaving the row closes it as usual.
        function test_a_refused_menu_leaves_nothing_holding_the_row() {
            const row = testCase.row
            row.oid_hex = "0de"
            const x = panel.width / 2
            mouseMove(panel, x, Theme.rowHeight / 2)
            tryCompare(row, "factsOpen", true, undefined, "the row opens under the hand")

            root.refuse = true
            mouseClick(panel, x, Theme.rowHeight / 2, Qt.RightButton)
            verify(!menu.visible, "nothing was on offer, so no menu stands")
            compare(row.menuOnOpenRow, false, "and nothing says a menu is on the row")

            // A menu raised elsewhere, away from the row; the hand goes there.
            menu.x = Theme.spaceSm
            menu.y = row.height + Theme.rowHeight
            menu.open()
            mouseMove(panel, x, row.height + Theme.rowHeight * 2)
            tryCompare(row, "factsOpen", false, undefined, "the row closes as the hand leaves it")
        }
    }
}

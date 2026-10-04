import QtQuick
// For the attached ToolTip alone.
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The reader moving the list under an open row (デザイン規約 §左メニューの所作): closing gives back only what the
// opening scrolled, never where the reader sent the list.
//
// The real list (`NavList`) and rows, in a panel that counts the hand the way the sidebar does
// (`SidebarPane.handWatch`), with the sidebar's gestures cut down to the open row's key.
Item {
    id: root
    width: 320
    // Room below the list, where the hand goes to be on no row.
    height: Theme.rowHeight * 8
    // What `Main.qml` and the popup bases declare (rules-refs/app-ui.md「ToolTip.policy」).
    ToolTip.policy: ToolTip.Manual

    /// The section's model, cut down: rows by name, and one upstream line under every branch (`NavFacts.answers`).
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
    }
    Rows {
        id: rows
    }

    /// `SidebarRowGestures`, cut down: the key a row hands up becomes the open one, and the hand is counted.
    QtObject {
        id: gestures
        property string openKey: ""
        property string activeKey: ""
        property string editKey: ""
        property int handMoves: 0
        property bool menuOpen: false
        property bool menuFromFacts: false
        property string editMode: ""
        property string editText: ""
        property bool editRefused: false
        property string editRefusedWhy: ""
        property string editRefusedMark: ""
        property var reclick: null
        function openFacts(key, at) {
            gestures.openKey = key
        }
        function closeFacts(key) {
            if (gestures.openKey === key)
                gestures.openKey = ""
        }
        function noteClick(key) {
        }
        function handStirred() {
            gestures.handMoves++
        }
    }

    Item {
        id: panel
        anchors.fill: parent
        property point handAt: Qt.point(-1, -1)
        HoverHandler {
            id: handWatch
            onPointChanged: {
                if (handWatch.point.position.x === panel.handAt.x
                        && handWatch.point.position.y === panel.handAt.y)
                    return
                panel.handAt = handWatch.point.position
                gestures.handStirred()
            }
            onHoveredChanged: {
                panel.handAt = Qt.point(-1, -1)
                gestures.handStirred()
            }
        }

        NavList {
            id: list
            width: panel.width
            height: Theme.rowHeight * 5
            sectionModel: rows
            gestures: gestures
            offersFacts: true
            kindHint: "branch"
        }
    }

    TestCase {
        id: testCase
        name: "WheelUnderOpenRow"
        when: windowShown

        readonly property real handX: list.width / 2

        function nameAt(index) {
            return "b" + (index < 10 ? "0" : "") + index
        }
        function initTestCase() {
            for (let i = 0; i < 30; ++i) {
                const name = testCase.nameAt(i)
                rows.append({
                    "name": name, "full": name, "oid_hex": "", "change": "", "bucket": "", "orig_path": "",
                    "orig_name": "", "is_head": false, "has_remote": false, "only_remote": false, "has_pr": false,
                    "depth": 0, "folder": false, "eol_mark": false, "ahead": 0, "behind": 0
                })
            }
        }
        /// Every case starts with the hand on no row and nothing open, the list at its top.
        function init() {
            testCase.handOff()
            testCase.scrollTo(0)
        }
        /// And ends with the list as it began, the height of five rows.
        function cleanup() {
            testCase.handOff()
            list.height = Theme.rowHeight * 5
        }
        function handOff() {
            mouseMove(panel, testCase.handX, Theme.rowHeight * 7)
            tryCompare(gestures, "openKey", "", undefined, "the hand off the list leaves nothing open")
        }
        /// The list put somewhere before the hand arrives, its rows built there: a row built after the hand moved has
        /// arrived under a still hand, and does not open (デザイン規約 §左メニューの所作).
        function scrollTo(y) {
            list.contentY = y
            list.forceLayout()
            verify(waitForRendering(list), "the list is drawn where it was put")
        }

        /// The hand rests at `y` in the list until the row there opens, and the lines are built under it.
        function restAt(y) {
            mouseMove(panel, testCase.handX, y)
            const index = list.indexAt(testCase.handX, list.contentY + y)
            tryCompare(gestures, "openKey", "branch:" + testCase.nameAt(index), undefined,
                       "the row under the hand opens")
            const row = list.itemAtIndex(index)
            tryVerify(() => row.factsItem !== null && row.factsItem.height > 0, undefined,
                      "its lines are built under it")
            return row
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

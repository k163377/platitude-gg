import QtQuick
// For the attached ToolTip alone.
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Which row of the left panel wears the light, with a real pointer over the real part (`NavItemDelegate`): **the row
// the hand walked onto**, and it keeps it while the list moves under a hand that stays where it is
// (デザイン規約 §左メニューの所作).
//
// **The rows read their own hover only when the hand moves.** Hover follows the item, so a row arriving under a still
// pointer is handed it (`tst_hoverunderstillhand.qml`); the panel answers that by counting the *place* it last saw
// the hand in and telling the rows to read again only when that place changed. This stands in for the panel with a
// count of its own — what a row is handed is an int, and the weighing is the panel's.
Item {
    id: root
    width: 300
    height: 200

    /// The panel's count, as the rows are handed it (`SidebarRowGestures.handMoves`).
    property int handMoves: 0

    Column {
        id: rows
        width: 260
        // What an open row does to the ones under it: the first grows, and the second stands where the first one's
        // line would end if it were shut.
        property bool firstOpen: false

        NavItemDelegate {
            id: top
            index: 0
            name: "topic"
            full: "C:/somewhere/topic"
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
            listWidth: rows.width
            height: Theme.rowHeight + (rows.firstOpen ? Theme.rowHeight * 3 : 0)
            handMoves: root.handMoves
            handCounted: true
            kindHint: "worktree"
        }
        NavItemDelegate {
            id: below
            index: 1
            name: "spike"
            full: "spike"
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
            listWidth: rows.width
            height: Theme.rowHeight
            handMoves: root.handMoves
        }
    }

    /// Another target for the one shared box, which is what the next section a hand walks on to is
    /// (`SharedToolTip.handOver`).
    Item {
        id: elsewhere
        width: 40
        height: 20
        property bool asking: false
        ToolTip.visible: elsewhere.asking
        ToolTip.text: "somewhere else"
    }

    TestCase {
        name: "NavRowLight"
        when: windowShown

        /// Every row reads the hand a turn after the panel counts it (`NavItemDelegate.syncHover`, on `Qt.callLater`).
        /// A call queued after those runs after them — the queue runs its calls in the order they were queued, and a
        /// call queued again moves to the back of it (qtdeclarative `QQmlDelayedCallQueue`) — so this one landing is
        /// the rows having read.
        function rowsRead() {
            let read = false
            Qt.callLater(() => { read = true })
            tryVerify(() => read, undefined, "the rows have read where the hand is")
        }

        /// The hand walks onto the first row, and the row grows the way an open one does. The pointer has not moved,
        /// so the row it is standing in keeps the light — and the row that arrived under it takes none.
        function test_the_row_the_hand_walked_onto_keeps_the_light() {
            rows.firstOpen = false
            root.handMoves = 0

            mouseMove(top, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => top.pointed, undefined, "the row the hand walked onto is lit")
            verify(!below.pointed, "and the one below it is not")

            // The first row grows: the second one is now below where the pointer is, and Qt hands the hover about
            // as the layout says. Nothing tells the rows to read again, because the hand did not move.
            rows.firstOpen = true
            verify(waitForRendering(rows), "the row is taller on screen")
            verify(top.pointed, "the row the hand is in keeps the light")
            verify(!below.pointed, "and nothing else takes it")
        }

        /// **The rest on the row's own name is what asks for the supplement**: a reader hovering a working copy is
        /// asking what that copy is, and where it stands is the one answer the row cannot show. The hand never
        /// leaves the name for it — walking down to the lines would be a second rest paid for an answer the first
        /// one had earned (デザイン規約 §左メニューの所作).
        function test_the_rest_on_the_rows_own_line_asks_for_the_supplement() {
            rows.firstOpen = false
            root.handMoves = 0
            top.opensFacts = true
            top.rowKey = "worktree:topic"
            top.openKey = "worktree:topic"
            verify(top.factsItem !== null, "the row has its lines open")

            // The hand is somewhere else in the list to begin with, so what the row answers below is the rest and
            // not whatever the run before this one left the pointer standing on.
            mouseMove(below, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => top.factsItem.says === "", undefined, "nothing is asked for off a row nobody is on")

            mouseMove(top, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => top.factsItem.says === top.factsPath, undefined,
                      "the hand on the row's own line asks for where the copy stands")
            tryVerify(() => top.factsTipOut, undefined, "and the box comes out on the rest already taken")
        }

        /// The supplement a row opens keeps the row: **the tip is a popup over the panel and takes the pointer as
        /// the hand walks into it**, so a row that read only its own hover would close under the words the reader
        /// was reaching for (デザイン規約 §hover のツールチップ「出したものは持ち帰れる」).
        function test_a_row_holds_itself_open_under_its_own_supplement() {
            rows.firstOpen = false
            root.handMoves = 0

            // The row opens on its own name and puts the path out — the stand-in stands for the hand resting on it,
            // which is the state a reader is in when they set off towards the words.
            top.opensFacts = true
            top.rowKey = "worktree:topic"
            top.openKey = "worktree:topic"
            verify(top.factsItem !== null, "the row has its lines open")
            top.pointedTipRow = top.index
            tryVerify(() => top.factsTipOut, undefined, "and the supplement is standing on it")
            // The row takes the light a turn after the box comes out (`NavItemDelegate.onFactsTipOutChanged`), so the
            // box standing is not yet the row answering it.
            tryVerify(() => top.pointed, undefined, "and the row wears the light its supplement holds")

            // The hand walks off the row and into the tip: nothing of the panel is under the pointer any more, and
            // the panel says so. The claim is about what the row reads after that, a turn later again.
            mouseMove(below, 100, Theme.rowHeight / 2)
            root.handMoves++
            rowsRead()
            verify(top.pointed, "the row stays the row the reader is on while its own supplement stands")

            // And it lets go when the supplement does.
            top.pointedTipRow = -1
            tryVerify(() => !top.pointed, undefined, "the row gives up the light with the words it put out")
        }

        /// The other way that supplement ends: **another target takes the shared box over**, which is what a hand
        /// walking on to the next section does. The box is handed across without this row being told
        /// (`SharedToolTip.handOver`), so a row that read its own attached property would hold itself open behind a
        /// hand that had left long ago (observed).
        function test_a_row_lets_go_when_the_box_is_handed_to_another_target() {
            rows.firstOpen = false
            root.handMoves = 0
            elsewhere.asking = false

            top.opensFacts = true
            top.rowKey = "worktree:topic"
            top.openKey = "worktree:topic"
            verify(top.factsItem !== null, "the row has its lines open")
            top.pointedTipRow = top.index
            tryVerify(() => top.factsTipOut, undefined, "the supplement is standing on this row")

            // Somewhere else asks for the same box — the one instance moves, words and target together.
            elsewhere.asking = true
            tryVerify(() => !top.factsTipOut, undefined, "the box standing is no longer this row's")
            tryVerify(() => !top.pointed, undefined, "so the row gives the light up")
            // The hand is taken off the row again: the stand-in is the pointer, and one left standing here would be
            // the next case's opening state.
            top.pointedTipRow = -1
        }

        /// And when the hand does move, the light is wherever the pointer now is — one row, whatever the events did
        /// in between.
        function test_a_hand_that_moves_takes_the_light_with_it() {
            rows.firstOpen = false
            root.handMoves = 0

            mouseMove(top, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => top.pointed, undefined, "the hand is on the first row")

            mouseMove(below, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => below.pointed, undefined, "the row it walked to takes the light")
            // **Waited for, because the count here is raised by hand**: the panel's own is raised by the delivery of
            // the move itself, so a row reads after the hover has landed; a test that posts the move and counts it
            // in the same breath reads once before Qt has handed it on.
            tryVerify(() => !top.pointed, undefined,
                      "and the one it left gives it up — the light stands on one row")
        }
    }
}

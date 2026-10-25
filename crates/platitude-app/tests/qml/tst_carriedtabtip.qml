import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The path a tab puts out, when the tab it stands on is taken up and carried (デザイン規約 §hover のツールチップ
// 「掴んだ手の下は空のまま」). The real tab, carry and the window's tip under a real pointer: hover cannot be injected
// into the app (verify-ui スキル).
//
// Under a press Qt delivers no hover at all, so to the window the carrying hand stays where it pressed. The ask falling
// as the tab is taken up is no hand walking into the box: the tab drops its ask (`TipAsk.drop`), and the seat takes the
// box down with no beat and does not put it back.
Item {
    id: root
    width: 600
    height: 300

    /// What the box did from the press on: "taken" and "dropped" for the tab, "up" and "down" for the box.
    property var seen: []
    /// The step of the carry the hand was on (`takeUp`), when the tab was taken up and when the box last went down.
    property int carryStep: 0
    property int takenStep: -1
    property int downStep: -1

    HandWatch { id: hand }

    TipSeat {
        id: shared
        host: root
        hand: hand
    }

    /// The strip's model, as much of it as a tab and the carry ask of it. Paths shorter than the names: a box narrower
    /// than its tab stands centred on the hand, in whatever font the runner has (`TipSeat.seatX`).
    ListModel {
        id: tabRows
        property int currentIndex: 0
        function setCurrentIndex(index) {
            tabRows.currentIndex = index
        }
        function moveTab(from, to) {
            tabRows.move(from, to, 1)
        }
        function closeTab(id) {}
        ListElement { tab_id: 1; title: "platitude-gg"; worktree_path: "/src/pg"; worktree_name: "" }
        ListElement { tab_id: 2; title: "sealed-class"; worktree_path: "/src/sc"; worktree_name: "" }
    }

    readonly property TabMetrics tabMetrics: TabMetrics { id: tabMetrics }

    // Wired as `TabStrip` wires its own, less the band's sharing out.
    ListView {
        id: tabs
        x: 40
        width: 500
        height: 40
        orientation: ListView.Horizontal
        interactive: false
        model: tabRows
        delegate: TabItemDelegate {
            id: tabItem
            tabsModel: tabRows
            metrics: tabMetrics
            titleCap: 400
            titleMinW: tabMetrics.titleMinW()
            titleEaseW: tabMetrics.titleEaseFullW()
            markRoom: tabMetrics.markRoomFull
            fadeW: 30
            bandColor: Theme.bgBase
            stripHeight: tabs.height
            held: carry.heldId === tabItem.tab_id
            heldX: carry.heldX
            onTabPressed: button => tabRows.setCurrentIndex(tabItem.index)
            onTabTaken: grabX => {
                root.seen.push("taken")
                root.takenStep = root.carryStep
                carry.takeTab(tabItem.index, grabX)
            }
            onTabDragged: sceneX => carry.carryTab(tabItem.index, sceneX)
            onTabDropped: {
                root.seen.push("dropped")
                carry.dropTab()
            }
        }
    }
    TabCarry {
        id: carry
        view: tabs
        tabsModel: tabRows
    }

    Connections {
        target: shared.box
        function onVisibleChanged() {
            root.seen.push(shared.box.visible ? "up" : "down")
            if (!shared.box.visible)
                root.downStep = root.carryStep
        }
    }

    TestCase {
        name: "CarriedTabTip"
        when: windowShown

        /// Where the button is still down, or null: a case that fails mid-carry leaves the press for `cleanup`, or
        /// every case after it starts under a grab, with no hover delivered.
        property var heldAt: null

        function init() {
            mouseMove(root, root.width - 1, root.height - 1)
            // The beat too: a box the beat is still keeping up stands where the next case's hand goes.
            tryVerify(() => !shared.box.visible && !shared.settling, undefined,
                      "each case starts with nothing out")
            // A carry reorders the strip: each case starts from the same one.
            if (tabRows.get(0).tab_id !== 1)
                tabRows.move(1, 0, 1)
            tabRows.currentIndex = 0
            tabs.forceLayout()
            root.seen = []
        }

        function cleanup() {
            setDown()
        }

        function setDown() {
            if (heldAt === null)
                return
            mouseRelease(root, heldAt.x, heldAt.y)
            heldAt = null
        }

        /// The first tab's box, out after the rest on it. Answers where the hand is.
        function tipOnFirstTab() {
            const tab = tabs.itemAtIndex(0)
            const at = Qt.point(tabs.x + tab.width / 2, tab.height / 2)
            mouseMove(root, at.x, at.y)
            tryVerify(() => shared.box.visible && shared.box.parent === tab, undefined,
                      "the rest on the tab puts its path out")
            return at
        }

        /// How far the box's middle stands off `x`, in the window.
        function boxOff(x) {
            const ground = shared.box.background
            return Math.abs(ground.mapToItem(root, ground.width / 2, 0).x - x)
        }

        /// Pressed where the hand rests and carried twenty frames of `step` pixels to the right, and left held
        /// (`heldAt`). `held` rises at the drag threshold (`tabTaken`), not at the press, so a hand that holds still
        /// after pressing is no case of its own.
        function takeUp(at, step) {
            const tab = tabs.itemAtIndex(0)
            root.seen = []
            root.takenStep = -1
            root.downStep = -1
            mousePress(root, at.x, at.y)
            heldAt = at
            for (let i = 1; i <= 20; i++) {
                root.carryStep = i
                heldAt = Qt.point(at.x + i * step, at.y)
                mouseMove(root, heldAt.x, heldAt.y, 16, Qt.LeftButton)
            }
            verify(tab.held, "the tab is in hand")
            // waits(paced): the subject is a box that must **not** come back, so the wait is the window it is watched
            // over — the beat a leave would have started, twice over.
            wait(Metrics.hoverKeepMs * 2)
        }

        function since(mark) {
            const from = root.seen.indexOf(mark)
            return from < 0 ? "never " + mark : root.seen.slice(from + 1).join(" ")
        }

        // Carried slowly: the carried tab still covers the press point, where the hand stays to the window.
        function test_a_a_slow_carry_takes_the_box_down_and_keeps_it_down() {
            takeUp(tipOnFirstTab(), 2)
            compare(since("taken"), "down", "the box goes as the tab is taken up, and is not put back")
            verify(!shared.box.visible)
            verify(!shared.settling, "and nothing is holding it open")
        }

        // Carried quickly: the press point is off the tab within a few steps, and the box still goes at the taking up —
        // in the step it was taken up in, not a beat of steps later.
        function test_b_a_quick_carry_takes_it_down_at_once() {
            takeUp(tipOnFirstTab(), 20)
            compare(since("taken"), "down", "the box goes as the tab is taken up, and is not put back")
            verify(root.takenStep > 0, "the tab was taken up")
            compare(root.downStep, root.takenStep, "in the step it was taken up in")
        }

        // Carried past its neighbour and set down under the hand, the tab answers the hand again: the rest puts a box
        // of its own out on the hand that rested on the tab's new place, not at the seat the first box stood at.
        function test_c_set_down_the_tab_answers_the_hand_again() {
            const tab = tabs.itemAtIndex(0)
            const at = tipOnFirstTab()
            verify(shared.box.width < tab.width, "a box narrower than its tab, so it stands on the hand")
            verify(boxOff(at.x) <= 1, "the first box stands on the hand that rested: " + boxOff(at.x) + " off")
            // Slow enough that the carried tab still covers the press point for a while after the taking up.
            takeUp(at, 6)
            const rested = heldAt
            setDown()
            compare(tab.index, 1, "carried past its neighbour")
            verify(!tab.held, "set down")
            tryVerify(() => shared.box.visible && shared.box.parent === tab, undefined,
                      "the rest on the set-down tab puts its path out again")
            compare(since("dropped"), "up", "as a box of its own")
            verify(boxOff(rested.x) <= 1, "on the hand where it rested this time: " + boxOff(rested.x) + " off")
        }

        // The keeper is every tip's, the tab's own included: a hand stepping off an uncarried tab into its box keeps
        // the box (規約 §hover のツールチップ). The rows' walks are `tst_tipseat`.
        function test_d_a_hand_walking_into_a_tabs_box_still_keeps_it() {
            const tab = tabs.itemAtIndex(0)
            tipOnFirstTab()
            const box = shared.box.contentItem
            const into = box.mapToItem(root, box.width / 2, box.height / 2)
            verify(into.y > tab.height, "the box stands below the strip, off the tab")
            mouseMove(root, into.x, into.y)
            tryVerify(() => shared.pointed && shared.box.visible, undefined,
                      "the hand is inside the box, and the box is the one that was already out")
            compare(shared.box.parent, tab, "still the tab's")
        }
    }
}

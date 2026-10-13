import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A name is asked by two clicks on one target in a row (デザイン規約 §左メニューの所作「間を空けた 2 回目」): anything
// between them — a press on the window or on a card standing over it, the history landing on another commit — makes
// the next click a first one (`ReclickGesture.broken`). The window hears a press before the row it lands on answers,
// a card's face after its content does, and the window never hears a press inside a card — Qt's delivery order, so
// measured with a real pointer. A row answering at the release (the left menu's) claims its press at the press and
// hears its release; a double-click spends the two clicks.
Item {
    id: root
    width: 600
    height: 400

    /// Who heard each press, in order: the window's watcher, `Hand`, and the two rows.
    property var heard: []

    readonly property string key: "branch:main"
    readonly property string navKey: "branch|main"
    readonly property var names: ({ "id": "main" })

    Connections {
        target: Hand
        function onActed() {
            root.heard.push("hand")
        }
    }

    ReclickGesture {
        id: gesture
    }

    // A graph row, as far as a press goes: answered at the press, and the history lands on its commit in the same
    // press (`RepoPage.activateRow`). A held Ctrl builds a choice instead; a double-click spends the two clicks.
    Rectangle {
        id: row
        x: 10
        y: 10
        width: 200
        height: 30
        MouseArea {
            anchors.fill: parent
            onPressed: mouse => {
                root.heard.push("row")
                if (!(mouse.modifiers & Qt.ControlModifier))
                    gesture.click(root.key, root.names)
                Hand.acted()
            }
            onDoubleClicked: gesture.drop()
        }
    }
    // A left menu row: it answers at the click — the release — so it claims the press at the press and hears its
    // release (`NavItemDelegate.takePress` / `endPress`); `claims` off is the row that forgot to claim.
    Rectangle {
        id: navRow
        property bool claims: true
        x: 10
        y: 220
        width: 200
        height: 30
        MouseArea {
            anchors.fill: parent
            onPressed: {
                if (navRow.claims)
                    gesture.take()
            }
            onReleased: gesture.hear()
            onClicked: gesture.click(root.navKey, root.names)
        }
    }
    // Something else in the window that takes its presses — a pane beside the rows.
    Rectangle {
        id: pane
        x: 10
        y: 100
        width: 200
        height: 100
        MouseArea {
            anchors.fill: parent
        }
    }

    // The chip's card: its row is the graph row's own target, as `RefListPopup`'s first row is.
    AppCard {
        id: card
        x: 300
        y: 10
        closesOnPressOutside: false
        Item {
            implicitWidth: 200
            implicitHeight: 120
            Rectangle {
                id: cardRow
                width: 200
                height: 30
                TapHandler {
                    onPressedChanged: {
                        if (!pressed)
                            return
                        root.heard.push("card row")
                        gesture.click(root.key, root.names)
                    }
                }
            }
        }
    }

    // As `Main`: the window's watcher tells `Hand`.
    FocusRelease {
        window: root.Window.window
        onPressedAnywhere: {
            root.heard.push("window")
            Hand.acted()
        }
    }

    TestCase {
        name: "ReclickAway"
        when: windowShown

        function init() {
            card.open()
            tryVerify(() => card.opened)
            gesture.forget()
            gesture.broken = false
            navRow.claims = true
            root.heard = []
        }
        function cleanup() {
            gesture.drop()
            card.close()
        }

        /// The gesture has settled the last input (`ReclickGesture.settle`).
        function settled() {
            tryVerify(() => !gesture.heard && !gesture.taken, undefined, "the input settled")
        }
        /// A click, settled, then the double-click window run out — so the next press is spaced from this one.
        function pressAt(item, x, y, modifiers) {
            mouseClick(item, x, y, Qt.LeftButton, modifiers === undefined ? Qt.NoModifier : modifiers)
            settled()
            tryVerify(() => !gesture.guarded, undefined, "the double-click window ran out")
        }
        /// A hand's click on a row answering at the release: the press, settled, then the release — a hand never lets
        /// go inside the delivery of its own press.
        function clickApart(item, x, y) {
            mousePress(item, x, y)
            settled()
            mouseRelease(item, x, y)
        }

        function test_the_next_click_on_the_row_names_it() {
            pressAt(row, 20, 15)
            verify(!gesture.armed)
            compare(root.heard, ["window", "hand", "row", "hand"],
                    "the window hears a press before the row answers it, and the row's landing is its own")
            verify(!gesture.broken)
            mouseClick(row, 20, 15)
            verify(gesture.armedFor(root.key))
        }

        function test_a_press_on_another_pane_between_makes_the_next_a_first_click() {
            pressAt(row, 20, 15)
            pressAt(pane, 20, 20)
            verify(gesture.broken)
            compare(gesture.activeKey, root.key, "the mark of the row last clicked stays")
            // Read at the press: an armed wait runs out with the double-click window, which `pressAt` waits out.
            mouseClick(row, 20, 15)
            verify(!gesture.armed, "back from elsewhere, one click is a first one")
            settled()
            tryVerify(() => !gesture.guarded)
            mouseClick(row, 20, 15)
            verify(gesture.armedFor(root.key), "and the next names")
        }

        function test_a_press_on_nothing_between_is_elsewhere_too() {
            pressAt(row, 20, 15)
            pressAt(root, 250, 350)
            mouseClick(row, 20, 15)
            verify(!gesture.armed)
        }

        /// The arrows, the search, a menu: the history landing with no press of this gesture's in it.
        function test_the_history_moving_between_makes_the_next_a_first_click() {
            pressAt(row, 20, 15)
            Hand.acted()
            settled()
            verify(gesture.broken)
            mouseClick(row, 20, 15)
            verify(!gesture.armed)
        }

        function test_a_press_on_a_card_between_is_elsewhere() {
            pressAt(row, 20, 15)
            root.heard = []
            pressAt(card.contentItem, 100, 100)
            verify(root.heard.indexOf("window") < 0, "the window never hears a press inside a card")
            verify(root.heard.indexOf("hand") >= 0, "the card's face does")
            verify(gesture.broken)
            mouseClick(row, 20, 15)
            verify(!gesture.armed)
        }

        /// The card stands on the chip's seat: a second click landing on it is the second.
        function test_the_second_click_on_the_card_counts() {
            pressAt(row, 20, 15)
            root.heard = []
            mouseClick(cardRow, 20, 15)
            verify(gesture.armedFor(root.key))
            compare(root.heard[0], "card row", "the card's row answers its press first")
            verify(root.heard.indexOf("hand") > 0, "and the face hears it after")
            settled()
            verify(!gesture.broken)
        }

        function test_a_press_on_the_card_between_two_on_its_row() {
            pressAt(cardRow, 20, 15)
            pressAt(card.contentItem, 100, 100)
            mouseClick(cardRow, 20, 15)
            verify(!gesture.armed)
        }

        /// A held click builds a choice and leaves the name alone; the hand went to something else.
        function test_a_held_click_on_the_same_row_between() {
            pressAt(row, 20, 15)
            pressAt(row, 20, 15, Qt.ControlModifier)
            mouseClick(row, 20, 15)
            verify(!gesture.armed)
        }

        function test_a_row_answering_at_the_release_names_on_the_next_click() {
            clickApart(navRow, 20, 15)
            settled()
            tryVerify(() => !gesture.guarded)
            clickApart(navRow, 20, 15)
            verify(gesture.armedFor(root.navKey))
        }

        function test_a_row_answering_at_the_release_starts_over_after_elsewhere() {
            clickApart(navRow, 20, 15)
            settled()
            pressAt(pane, 20, 20)
            clickApart(navRow, 20, 15)
            verify(!gesture.armed, "back from elsewhere, one click is a first one")
            settled()
            tryVerify(() => !gesture.guarded)
            clickApart(navRow, 20, 15)
            verify(gesture.armedFor(root.navKey), "and the next names")
        }

        /// A press the row claimed that made no click — let go off the row, as a sweep of its name or a press taken
        /// away also ends — came between two clicks all the same.
        function test_a_row_answering_at_the_release_starts_over_after_a_press_that_made_no_click() {
            clickApart(navRow, 20, 15)
            settled()
            tryVerify(() => !gesture.guarded)
            mousePress(navRow, 20, 15)
            settled()
            // Carried off first, as a hand is: a release alone never tells the row the hand left it.
            mouseMove(root, 400, 380)
            mouseRelease(root, 400, 380)
            settled()
            verify(gesture.broken)
            clickApart(navRow, 20, 15)
            verify(!gesture.armed)
        }

        /// By its release the press has been settled as one elsewhere: every click is a first one.
        function test_a_row_answering_at_the_release_without_claiming_its_press_never_names() {
            navRow.claims = false
            clickApart(navRow, 20, 15)
            settled()
            tryVerify(() => !gesture.guarded)
            clickApart(navRow, 20, 15)
            verify(!gesture.armed)
        }

        /// A double-click spends its two clicks on going somewhere: the next click is a first one, the one after names.
        function test_a_double_click_spends_the_two_clicks() {
            mouseDoubleClickSequence(row, 20, 15)
            settled()
            tryVerify(() => !gesture.guarded)
            verify(gesture.broken)
            mouseClick(row, 20, 15)
            verify(!gesture.armed)
            settled()
            tryVerify(() => !gesture.guarded)
            mouseClick(row, 20, 15)
            verify(gesture.armedFor(root.key))
        }
    }
}

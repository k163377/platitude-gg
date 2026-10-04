import QtQuick
import QtTest
import platitude.ui

// The discard log as it stands over the left menu (`RecoverPane`, 破棄記録仕様.md): nothing the hand does over it
// reaches the menu underneath, its `✕` is the band's whole right end, a ring says the reflogs are being read, an empty
// answer says so, and a rest on an entry opens its card.
Item {
    id: root
    width: 300
    height: 400

    /// What lies under the list in the left menu — the filter, its fold block, the sections. Anything it hears through
    /// the list is a leak; the strip right of the list leaves some of it bare, for the hand to start from.
    MouseArea {
        id: under
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        property int presses: 0
        property int wheels: 0
        onPressed: under.presses++
        onWheel: wheel => under.wheels++
    }

    RecoverEntries {
        id: words
    }

    /// The page's side of the list, in the shape `RecoverPane` and `RecoverToggle` read.
    QtObject {
        id: page
        property var recoverEntries: []
        property int recoverPick: -1
        property bool recoverAnswered: true
        property bool recoverReading: false
        property bool recoverFailed: false
        property string recoverError: ""
        property bool recoverOpen: true
        property bool recoverLit: false
        property var recoverWords: words
        property real recoverNow: 1790000000
        function toggleRecover() {
            page.recoverOpen = !page.recoverOpen
        }
        function pickRecover(index) {
            page.recoverPick = page.recoverPick === index ? -1 : index
        }
    }

    RecoverPane {
        id: pane
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 260
        curPage: page
    }

    TestCase {
        name: "RecoverPane"
        when: windowShown

        /// Two minutes before the moment the list reads its times against.
        function entry(title) {
            return { kind: "reset", mark: "branch", tint: Theme.accent, title: title, at: page.recoverNow - 120,
                     copy: "", parts: [{ mark: "branch", tint: Theme.accent, text: "2 commits",
                                         restore: "as branch main-pgg-restored", look: "commit" }] }
        }

        function init() {
            page.recoverEntries = []
            page.recoverPick = -1
            page.recoverAnswered = true
            page.recoverReading = false
            page.recoverFailed = false
            page.recoverError = ""
            page.recoverOpen = true
            under.presses = 0
            under.wheels = 0
        }

        /// Every visible item under `item` that `pick` answers for, depth first.
        function found(item, pick) {
            const out = []
            for (let i = 0; i < item.children.length; i++) {
                const child = item.children[i]
                if (child.visible && pick(child))
                    out.push(child)
                out.push(...found(child, pick))
            }
            return out
        }
        /// Where each turning ring stands, down the pane.
        function rings() {
            return found(pane, item => item.kind === "spinner").map(ring => ring.mapToItem(pane, 0, 0).y)
        }

        function test_nothing_the_hand_does_over_the_list_reaches_the_menu_under_it() {
            page.recoverEntries = [entry("Reset main")]
            verify(waitForRendering(pane))
            // The band's empty middle — between the seat and the `✕`, wherever the font puts the seat's end — the
            // strip along its foot where the filter runs on, and the ground below the rows.
            const seat = found(pane, item => item.captioned !== undefined)[0]
            const gap = (seat.width + pane.width - Theme.headerHeight) / 2
            verify(seat.width < gap - 1 && gap + 1 < pane.width - Theme.headerHeight, "the band has a gap")
            // Where, and whether the list is still up after the press there: the band's whole width is the close.
            for (const [x, y, stays] of [[gap, 10, false], [gap, Theme.headerHeight - 2, false], [gap, 300, true]]) {
                page.recoverOpen = true
                // Hover is handed out on the next frame: the hand starts on the bare menu, so a list that takes it
                // is seen taking it — the menu lets go.
                mouseMove(root, root.width - 10, y)
                tryVerify(() => under.containsMouse, undefined, "the bare menu has the hand")
                mouseMove(pane, x, y)
                tryVerify(() => !under.containsMouse, undefined, "hover reached the menu at " + [x, y])
                mouseClick(pane, x, y)
                compare(page.recoverOpen, stays, "a press at " + [x, y] + (stays ? " closed the list" : " missed the close"))
                mouseWheel(pane, x, y, 0, -120)
            }
            compare(under.presses, 0)
            compare(under.wheels, 0)
        }

        function test_the_close_is_the_bands_whole_right_end() {
            // The block's far corner, where the fold block's own corner lies under it.
            mouseClick(pane, pane.width - 2, 2)
            verify(!page.recoverOpen)
            page.recoverOpen = true
            mouseClick(pane, pane.width - Theme.headerHeight + 2, Theme.headerHeight - 2)
            verify(!page.recoverOpen)
            compare(under.presses, 0)
        }

        function test_a_ring_says_the_reflogs_are_being_read() {
            page.recoverAnswered = false
            page.recoverReading = true
            verify(waitForRendering(pane))
            const first = rings()
            compare(first.length, 1, "one ring before the first answer")
            verify(first[0] > Theme.headerHeight, "in the list, under the band")

            page.recoverEntries = [entry("Reset main")]
            verify(waitForRendering(pane))
            const again = rings()
            compare(again.length, 1, "one ring over the entries already up")
            verify(again[0] < Theme.headerHeight, "beside the heading")

            page.recoverReading = false
            page.recoverAnswered = true
            compare(rings().length, 0)
        }

        function test_an_empty_answer_says_nothing_was_thrown_away() {
            verify(waitForRendering(pane))
            const said = found(pane, item => item.text === "Nothing was thrown away here")
            compare(said.length, 1)
            const at = said[0].mapToItem(pane, 0, 0)
            verify(at.y > 2 * Theme.headerHeight, "in the list, under the filter")
            // Not a row: no seat's indent, centred across the list.
            fuzzyCompare(at.x + said[0].width / 2, pane.width / 2, 1)
        }

        function test_a_rest_on_an_entry_opens_its_card_and_the_row_keeps_its_time() {
            page.recoverEntries = [entry("Rebased feature/a-branch-name-long-enough-to-be-cut-at-the-time")]
            page.recoverPick = 0
            verify(waitForRendering(pane))
            const titles = found(pane, item => item.nameCutAt !== undefined)
            compare(titles.length, 1, "one row, its name cut — never opened out in the list")
            compare(found(pane, item => item.text === "2 min ago").length, 1, "the row's time stays")
            verify(pane.restOnShown(0))
            tryVerify(() => pane.cardOpen, undefined, "the card opened")
            compare(pane.cardAt, 0)
        }
    }
}

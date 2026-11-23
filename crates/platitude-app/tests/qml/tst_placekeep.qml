import QtQuick
import QtQuick.Layouts
import QtTest
import platitude.ui

// A list keeping its place through a swap of the bars over it at the same height (`PlaceKeep`), set up as the
// graph's (`GraphList.keepPlace`): one bar anchored over the list in its pane, which moves it at once, one in the
// layout around the pane, which moves it a polish later. Scrolled to the end of its rows, a list the pane's bar grew
// is pulled back by Flickable on the spot — the frame after the swap is read as it is about to be drawn.
Item {
    id: root
    width: 300
    height: 400

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        // The page's bar (`RepoPage.centreColumn`).
        Rectangle {
            id: pageBar
            Layout.fillWidth: true
            implicitHeight: 60
            visible: false
        }
        Item {
            id: pane
            Layout.fillWidth: true
            Layout.fillHeight: true
            // The pane's bar (`GraphPane`'s `AskBar`).
            Rectangle {
                id: paneBar
                anchors.top: parent.top
                width: parent.width
                height: 60
            }
            ListView {
                id: list
                anchors.top: paneBar.bottom
                anchors.bottom: parent.bottom
                width: parent.width
                model: 100
                delegate: Item {
                    width: list.width
                    height: 20
                }
                function clampY(y) {
                    const minY = list.originY - list.topMargin
                    const maxY = Math.max(minY, list.originY + list.contentHeight - list.height + list.bottomMargin)
                    return Math.max(minY, Math.min(y, maxY))
                }
            }
        }
    }
    readonly property PlaceKeep placeKeep: PlaceKeep {
        view: list
    }

    TestCase {
        id: tc
        name: "PlaceKeep"
        when: windowShown

        /// Where the list stood at the first frame after the swap, read once that frame is laid out.
        property real drawnY: NaN
        property bool watching: false
        // Heard after the keep's own (connected once `watching` is set, after `keep()`), as the frame is drawn next.
        readonly property Connections frameWatch: Connections {
            target: tc.watching ? root.Window.window : null
            function onAfterAnimating() {
                tc.drawnY = list.contentY
                tc.watching = false
            }
        }

        /// The pane's bar gives the middle to the page's, with the place kept or not: the bar leaving goes at once, the
        /// one coming stands at its height.
        function swap(kept) {
            if (kept)
                root.placeKeep.keep()
            tc.watching = true
            pageBar.visible = true
            paneBar.height = 0
            tryVerify(() => !tc.watching, undefined, "a frame comes after the swap")
        }

        /// The pane's bar standing, laid out, and the list at the end of its rows.
        function stand() {
            pageBar.visible = false
            paneBar.height = 60
            tryVerify(() => pane.height === root.height && list.height === root.height - 60
                      && list.contentHeight === 2000)
            list.contentY = list.clampY(1e9)
            verify(list.contentY > 0)
            return list.contentY
        }

        function cleanup() {
            tryVerify(() => !root.placeKeep.keeping, undefined, "the keeping ends with the frame after the swap")
        }

        function test_the_swap_moves_the_rows_unless_the_place_is_kept() {
            // The trap the keep is for: the list grows in the pane before the layout takes the height back.
            const end = tc.stand()
            tc.swap(false)
            compare(tc.drawnY, end - 60, "pulled back by the bar's height")
            tryVerify(() => list.height === root.height - 60)
            compare(list.contentY, end - 60, "and left there once the layout gives the height back")
        }

        function test_the_swap_leaves_the_rows_where_they_stood_with_the_place_kept() {
            const end = tc.stand()
            tc.swap(true)
            compare(tc.drawnY, end, "the frame after the swap draws the rows where they stood")
            compare(list.height, root.height - 60)
            compare(list.contentY, end)
        }

        function test_a_view_on_the_move_is_left_to_it() {
            tc.stand()
            root.placeKeep.still = false
            root.placeKeep.keep()
            verify(!root.placeKeep.keeping)
            root.placeKeep.still = true
        }
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtTest
import platitude.ui

// The description grip's refusal, read where the page reads it (`MessageEditor.descRefuses` / `descPoint`): a drag past
// either end is answered at the hand, and the automation hook that stands in for that drag (`DescriptionBox.pullPast`,
// headless `divider-refuse desc-max` / `desc-min`) leaves the hand and the box where the drag leaves them. The pair is
// laid out as the panes lay it out — its height bound to `pairHeight` through `Layout.preferredHeight`, a file list
// under it taking what is left — so a pull reaches the box on the next layout, not in the breath that asked for it.
Item {
    id: root
    width: 420
    height: 640

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        MessageEditor {
            id: editor
            Layout.fillWidth: true
            // A layout fills its layout by default; the pane's block stands at what the pair asks for.
            Layout.fillHeight: false
            Layout.preferredHeight: editor.pairHeight
            listHeight: fileList.height
        }
        // The file list: what a pull borrows from, short of its last two rows.
        Item {
            id: fileList
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    TestCase {
        name: "GripRefusal"
        when: windowShown

        /// How far past an end the hook carries the hand (`DescriptionBox.pullPast`).
        readonly property real over: 4 * Theme.splitterWidth
        /// Longer than the room the list lends: the ceiling is the list's, not the text's.
        readonly property string body: Array.from({ length: 60 }, (_, i) => "line " + i).join("\n")

        function descBox() {
            return editor.children.find(c => c.gripPoint !== undefined)
        }
        /// The grip's mark is an `iconMd` square in the box's corner; a hand takes it by its middle.
        function gripMiddle(box) {
            return Qt.point(box.width - Theme.iconMd / 2, box.height - Theme.iconMd / 2)
        }
        /// A new message: the box at rest, offered the grip, with room under it and more text than that room.
        function toRest() {
            editor.setMessage("feat: a summary", body)
            const box = descBox()
            tryVerify(() => box.grips && box.extra === 0 && editor.descHeight === box.restHeight && box.room > 0
                      && box.wants > box.ceiling, undefined, "the box at rest with room to pull into")
        }
        /// A real drag from the grip's middle to where it asks for the ceiling plus `over` (or the rest less it),
        /// read once the box has been laid out at what it took, then let go.
        function dragPast(down) {
            const box = descBox()
            const stopsAt = down ? box.ceiling : box.restHeight
            const want = stopsAt + (down ? over : -over)
            const from = gripMiddle(box)
            mousePress(box, from.x, from.y)
            verify(box.gripDragging, "the press is the grip's")
            const to = Qt.point(from.x, from.y + want - box.height)
            mouseMove(box, to.x, to.y, -1, Qt.LeftButton)
            tryVerify(() => editor.descHeight === stopsAt, undefined, "the box laid out where it stopped")
            // Copied out: a point read off a property is a reference to it, and would read the hook's point later.
            const read = {
                "refuses": editor.descRefuses,
                "point": Qt.point(editor.descPoint.x, editor.descPoint.y),
                "hand": box.mapToItem(null, to),
                "height": editor.descHeight,
                "stopsAt": stopsAt
            }
            mouseRelease(box, to.x, to.y)
            verify(!editor.descRefuses, "letting go takes the refusal down")
            return read
        }

        function init() {
            toRest()
        }
        function cleanup() {
            descBox().gripHeld = false
        }

        /// Pulled past either end, the box stops there and the refusal stands at the pointer — not at the corner the
        /// grip leaves, nor at the clamped edge.
        function test_a_drag_past_an_end_is_refused_at_the_hand_data() {
            return [{ "tag": "ceiling", "down": true }, { "tag": "rest", "down": false }]
        }
        function test_a_drag_past_an_end_is_refused_at_the_hand(data) {
            const drag = dragPast(data.down)
            verify(drag.refuses, "the pull past the end is refused")
            compare(drag.height, drag.stopsAt)
            fuzzyCompare(drag.point.x, drag.hand.x, 0.5)
            fuzzyCompare(drag.point.y, drag.hand.y, 0.5)
        }

        /// The hook stands where that drag stands: the same box height and the same hand, read once laid out. Read in
        /// the hook's own breath the box has not taken the pull yet, so a hand worked out from its height then is the
        /// corner before the pull.
        function test_the_hook_stands_where_the_drag_does_data() {
            return [{ "tag": "ceiling", "down": true }, { "tag": "rest", "down": false }]
        }
        function test_the_hook_stands_where_the_drag_does(data) {
            const drag = dragPast(data.down)
            toRest()
            editor.pullDescriptionPast(data.down)
            tryVerify(() => editor.descHeight === drag.height, undefined, "the hook's pull laid out")
            verify(editor.descRefuses, "the hook's pull is refused")
            fuzzyCompare(editor.descPoint.x, drag.point.x, 0.5)
            fuzzyCompare(editor.descPoint.y, drag.point.y, 0.5)
        }
    }
}

import QtQuick
import QtTest
import platitude.ui

// How the question bar's `<destination>/<name>` shares one row (デザイン規約 §レイアウト初期値).
//
// **The two boxes were a fixed 160 each and the rest of the bar stood empty**, so a destination longer than that was
// cut while half the window went unused. They fill the bar now, and the sharing has three rules that only a laid-out
// row can answer: the destination takes the room its own name needs, the name takes what is left, and neither goes
// under the floor they share — which is the width they each used to be given outright.
Item {
    id: root
    width: 900
    height: 200

    readonly property string longRemote: "origin-with-a-very-long-name-for-a-remote"
    readonly property string longBranch: "feature/a-branch-name-that-is-far-too-long-to-sit-in-one-box"

    PublishForm {
        id: form
        width: root.width
        choices: [root.longRemote, "origin"]
        remote: "origin"
        branch: "feature/new-thing"
    }

    TestCase {
        name: "AskFields"
        when: windowShown

        /// The two boxes and the `/` between them, as the row lays them out.
        function boxes() {
            return { remote: form.remotePick, name: form.branchField }
        }

        /// Back to two short names, **and laid out again**: a value put back arrives a frame before the row that is
        /// sized from it, so a width read in the same frame is the width the test before this one left.
        function init() {
            form.width = root.width
            form.remote = "origin"
            form.branch = "feature/new-thing"
            // Both halves, because a test before this one may have narrowed the bar: the destination is back on its
            // floor a frame before the name has grown into the room that gave back.
            tryVerify(() => form.remotePick.width === Metrics.askFieldMinW
                            && form.branchField.x + form.branchField.width >= form.width - 1)
        }

        /// **Nothing is left standing empty.** Whatever the row does not give the destination goes to the name, so the
        /// pair reaches the end of the bar — the gap before the answer pill is the bar's own margin, not slack.
        function test_the_two_boxes_fill_the_row() {
            const b = boxes()
            const reach = b.name.x + b.name.width
            verify(reach >= form.width - 1, "the pair reaches " + reach + " of " + form.width)
        }

        /// A short destination keeps the floor and no more; the name has the rest.
        function test_a_short_destination_keeps_the_floor_and_the_name_takes_the_slack() {
            const b = boxes()
            compare(b.remote.width, Metrics.askFieldMinW)
            verify(b.name.width > Metrics.askFieldMinW, "the name has the slack: " + b.name.width)
        }

        /// **A long destination is given the room its name needs.** It is the half that can be read back nowhere else
        /// — what is being typed stands at the caret — so the room comes out of the name's share.
        function test_a_long_destination_takes_the_room_its_name_needs() {
            const b = boxes()
            form.remote = root.longRemote
            tryVerify(() => b.remote.width > Metrics.askFieldMinW)
            compare(b.remote.width, b.remote.wantedWidth, "exactly what the whole name needs")
            // **And that width is enough**, which is the half the arithmetic can get wrong on its own: the room the
            // word has is the box less the control's own reservation for the arrow, not the box.
            compare(b.remote.contentItem.text, root.longRemote, "so nothing is cut out of it")
            verify(b.name.width >= Metrics.askFieldMinW, "and the name keeps its floor: " + b.name.width)
        }

        /// **What is being typed asks first.** A name that has outgrown its box takes the room it needs off the
        /// destination beside it — that is the half a reader is writing and reading back at the caret, and the bar is
        /// one row rather than two boxes that happen to share it.
        function test_a_long_name_takes_its_room_from_the_destination() {
            const b = boxes()
            form.remote = root.longRemote
            tryVerify(() => b.remote.width > Metrics.askFieldMinW)
            const was = b.remote.width
            form.branch = root.longBranch
            // **A bar with room for the name and the destination's floor and nothing more**, so the two claims
            // actually meet: how many pixels a name takes is the font's business and the two machines do not agree
            // (verify-ui §Linux での動確), so the bar is sized from the answer rather than from a guessed string.
            form.width = b.name.wantedWidth + Metrics.askFieldMinW
            tryVerify(() => b.remote.width === Metrics.askFieldMinW)
            verify(was > Metrics.askFieldMinW,
                   "the destination had more before the name needed it: " + was)
        }

        /// **A destination nobody would type twice does not push the name off the end of the bar.** A layout hands a
        /// box the width it asked for and lets the row overflow rather than taking a neighbour under its minimum, so
        /// the destination is given a ceiling as well as a floor: it grows up to the point where the name still has
        /// the floor they share.
        function test_an_absurd_destination_stops_where_the_name_would_lose_its_floor() {
            const b = boxes()
            form.remote = root.longRemote
            tryVerify(() => b.remote.wantedWidth > Metrics.askFieldMinW)
            // **A bar exactly as wide as the destination wants**, rather than a name long enough to overflow one:
            // how many pixels a name takes is the font's business and the two machines do not agree about it, so the
            // row is narrowed to the answer instead of guessing a string that beats it (the container's fonts are
            // narrower — verify-ui §Linux での動確).
            form.width = b.remote.wantedWidth
            // **Within a hair of the bar's end rather than to the pixel**: the ceiling is worked out from the `/` and
            // the gaps beside it, and a label's width is a fraction the row rounds its own way. Without the ceiling
            // the name went hundreds of pixels past the end, which is what this tells apart — and waiting on it is
            // also how the row is waited out, the narrowing landing a frame after the width is set.
            tryVerify(() => b.name.x + b.name.width - form.width <= Theme.spaceXs)
            verify(b.remote.width < b.remote.wantedWidth, "it wanted more: " + b.remote.wantedWidth)
            verify(b.name.width >= Metrics.askFieldMinW, "and the name keeps its floor: " + b.name.width)
        }

        /// **Both too long for one bar**: the destination is on the floor the two share, the name has everything
        /// else, and what still does not fit scrolls inside the box being typed into.
        function test_both_too_long_puts_the_destination_on_the_floor() {
            const b = boxes()
            form.remote = root.longRemote
            form.branch = root.longBranch
            form.width = 2 * Metrics.askFieldMinW
            // Waited out on the name: the destination is already on the floor from the line above, so it is the box
            // that still has a width to settle on.
            tryVerify(() => b.name.width === Metrics.askFieldMinW)
            compare(b.remote.width, Metrics.askFieldMinW, "and the destination is on the same floor")
            verify(b.name.wantedWidth > b.name.width, "with more text than box, which is where it scrolls")
            form.width = root.width
        }
    }
}

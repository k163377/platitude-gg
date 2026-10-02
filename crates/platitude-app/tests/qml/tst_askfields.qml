import QtQuick
import QtTest
import platitude.ui

// How the question bar's `<destination>/<name>` shares one row — the rules are デザイン規約 §レイアウト初期値
// (`askFieldMinW`), and only a laid-out row can answer them.
Item {
    id: root
    width: 900
    height: 200

    readonly property string longRemote: "origin-with-a-very-long-name-for-a-remote"
    // Wider than both floors and the `/` in any face: the scenes that hand the bar the width this name asks for are
    // about a destination too long for it, not one narrower than the two floors (which overflows on purpose).
    readonly property string absurdRemote: "origin-with-a-name-so-long-no-question-bar-could-hold-it-beside-a-branch"
    readonly property string longBranch: "feature/a-branch-name-that-is-far-too-long-to-sit-in-one-box"

    PublishForm {
        id: form
        width: root.width
        choices: [root.longRemote, root.absurdRemote, "origin"]
        remote: "origin"
        branch: "feature/new-thing"
    }

    // The other question in this row, whose name box also carries a list: an `AppCombo` keeps back the arrow's
    // width whether it draws one or not (`AppCombo.wantedWidth`), so it asks for more than a plain field would.
    UpstreamForm {
        id: upstream
        width: root.width
        remotes: [root.longRemote, root.absurdRemote, "origin"]
        branches: ["main", "feature/topic-a"]
        remote: "origin"
        branch: "feature/new-thing"
    }

    TestCase {
        name: "AskFields"
        when: windowShown

        function boxes() {
            return { remote: form.remotePick, name: form.branchField }
        }

        /// Waits for the row to be laid out again: a value put back arrives a frame before the row sized from it.
        function init() {
            form.width = root.width
            form.remote = "origin"
            form.branch = "feature/new-thing"
            // Both halves: the destination is back on its floor a frame before the name has grown into the room.
            tryVerify(() => form.remotePick.width === Metrics.askFieldMinW
                            && form.branchField.x + form.branchField.width >= form.width - 1)
        }

        /// The gap before the answer pill is the bar's own margin, not slack the row left.
        function test_the_two_boxes_fill_the_row() {
            const b = boxes()
            const reach = b.name.x + b.name.width
            verify(reach >= form.width - 1, "the pair reaches " + reach + " of " + form.width)
        }

        function test_a_short_destination_keeps_the_floor_and_the_name_takes_the_slack() {
            const b = boxes()
            compare(b.remote.width, Metrics.askFieldMinW)
            verify(b.name.width > Metrics.askFieldMinW, "the name has the slack: " + b.name.width)
        }

        function test_a_long_destination_takes_the_room_its_name_needs() {
            const b = boxes()
            form.remote = root.longRemote
            tryVerify(() => b.remote.width > Metrics.askFieldMinW)
            compare(b.remote.width, b.remote.wantedWidth, "exactly what the whole name needs")
            // And that width is enough: the word's room is the box less the control's reservation for the arrow.
            compare(b.remote.contentItem.text, root.longRemote, "so nothing is cut out of it")
            verify(b.name.width >= Metrics.askFieldMinW, "and the name keeps its floor: " + b.name.width)
        }

        function test_a_long_name_takes_its_room_from_the_destination() {
            const b = boxes()
            form.remote = root.longRemote
            tryVerify(() => b.remote.width > Metrics.askFieldMinW)
            const was = b.remote.width
            form.branch = root.longBranch
            // Room for the name and the destination's floor only, so the two claims meet. Sized from the box's
            // answer, not a number: the pixels are the font's and differ per OS.
            form.width = b.name.wantedWidth + Metrics.askFieldMinW
            tryVerify(() => b.remote.width === Metrics.askFieldMinW)
            verify(was > Metrics.askFieldMinW,
                   "the destination had more before the name needed it: " + was)
        }

        /// A layout hands a box the width it asked for and lets the row overflow, so the destination has a ceiling
        /// too: it grows only while the name keeps its floor.
        function test_an_absurd_destination_stops_where_the_name_would_lose_its_floor() {
            const b = boxes()
            form.remote = root.absurdRemote
            tryVerify(() => b.remote.wantedWidth > 2 * Metrics.askFieldMinW + Theme.spaceXl)
            form.width = b.remote.wantedWidth
            // Within a hair: the ceiling counts the `/` label, a fractional width the row rounds its own way.
            // Without the ceiling the name overflows by hundreds of px. The narrowing lands a frame late, so this
            // is also the wait.
            tryVerify(() => b.name.x + b.name.width - form.width <= Theme.spaceXs)
            verify(b.remote.width < b.remote.wantedWidth, "it wanted more: " + b.remote.wantedWidth)
            verify(b.name.width >= Metrics.askFieldMinW, "and the name keeps its floor: " + b.name.width)
        }

        /// The same rules with an `AppCombo` name box, whose ceiling is worked out from what that name asked for.
        /// Not asked: a bar narrower than the two floors and the `/` — nothing shrinks past the floor, so the row
        /// overflows there on purpose (デザイン規約 §窓の床).
        function test_the_upstream_row_shares_by_the_same_rules() {
            const remote = upstream.remotePick
            const name = upstream.branchPick
            upstream.width = root.width
            tryVerify(() => remote.width === Metrics.askFieldMinW)
            verify(name.x + name.width >= upstream.width - 1,
                   "the pair reaches " + (name.x + name.width) + " of " + upstream.width)
            verify(name.width > Metrics.askFieldMinW, "the name has the slack: " + name.width)

            // A destination too long for the bar comes down to where the name keeps its floor.
            upstream.remote = root.absurdRemote
            tryVerify(() => remote.wantedWidth > 2 * Metrics.askFieldMinW + Theme.spaceXl)
            upstream.width = remote.wantedWidth
            tryVerify(() => name.x + name.width - upstream.width <= Theme.spaceXs)
            verify(remote.width < remote.wantedWidth, "it wanted more: " + remote.wantedWidth)
            verify(name.width >= Metrics.askFieldMinW, "and the name keeps its floor: " + name.width)

            // Both too long: both on the floor, the name scrolling inside its box. The destination comes down a frame
            // after the name, so its width is waited for too.
            upstream.branch = root.longBranch
            upstream.width = 2 * Metrics.askFieldMinW
            tryVerify(() => name.width === Metrics.askFieldMinW)
            tryCompare(remote, "width", Metrics.askFieldMinW)
            verify(name.wantedWidth > name.width, "with more text than box, which is where it scrolls")
        }

        function test_both_too_long_puts_the_destination_on_the_floor() {
            const b = boxes()
            form.remote = root.longRemote
            form.branch = root.longBranch
            form.width = 2 * Metrics.askFieldMinW
            // Waited on the name: the destination is already on the floor, so it cannot tell the row has settled.
            tryVerify(() => b.name.width === Metrics.askFieldMinW)
            compare(b.remote.width, Metrics.askFieldMinW, "and the destination is on the same floor")
            verify(b.name.wantedWidth > b.name.width, "with more text than box, which is where it scrolls")
            form.width = root.width
        }
    }
}

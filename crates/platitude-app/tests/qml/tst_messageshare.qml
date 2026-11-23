import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtTest
import platitude.ui

// The message pair's height in a block that has room and in one that has not (`MessageEditor.descShare` /
// `descFloor`), against a wrapping summary (`descWordRoom`), and the two boxes measured by their placeholder only while
// it stands (`SummaryArea` / `DescriptionBox`). Laid out as the panes lay it out: its height bound to `pairHeight`
// through `Layout.preferredHeight`, the block's room and what stands beside the pair handed in as the blocks hand them.
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
        }
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    TestCase {
        name: "MessageShare"
        when: windowShown

        /// More lines than five, so the box has words past any height it is given.
        readonly property string body: Array.from({ length: 12 }, (_, i) => "line " + i).join("\n")
        /// Wraps over several lines at this width.
        readonly property string longWords: Array.from({ length: 24 }, () => "Merge").join(" ")
        /// What the pair stands at with the description at `desc`: one summary line at rest (or the summary's own
        /// lines where it has more), the step, and the description.
        function pairAt(desc) {
            return Math.max(Theme.fontLgLine + Theme.spaceSm, editor.summaryNeed) + Theme.spaceXs + desc
        }
        /// A block with no room past the summary: whatever the pair stands at is its floor.
        function squeeze() {
            editor.blockRoom = 100
            editor.besidePair = 95
        }

        function init() {
            editor.blockRoom = Infinity
            editor.besidePair = 0
            editor.standingSubject = ""
            editor.standingBody = ""
            editor.setMessage("", "")
        }

        function test_a_block_with_room_rests_at_five_lines() {
            editor.setMessage("feat: a summary", body)
            tryVerify(() => editor.pairHeight === pairAt(Theme.messageMaxHeight), undefined, "five lines at rest")
        }

        /// The description gives what keeps the block in its room, and only that.
        function test_a_short_block_takes_what_it_lacks_from_the_description() {
            editor.setMessage("feat: a summary", body)
            editor.blockRoom = 300
            editor.besidePair = 200
            tryVerify(() => editor.pairHeight === editor.pairRoom, undefined, "the pair fills the room it has")
            tryVerify(() => editor.descHeight < Theme.messageMaxHeight && editor.descHeight > editor.descFloor,
                      undefined, "the description gave way, short of its floor")
        }

        function test_an_empty_description_gives_down_to_one_line() {
            squeeze()
            compare(editor.descFloor, Theme.fontMdLine + Theme.spaceSm)
            tryVerify(() => editor.pairHeight === pairAt(editor.descFloor), undefined, "the pair at the empty floor")
        }

        /// Written words hold a line and the top half of the next — more than an empty box keeps.
        function test_words_hold_a_line_and_a_half() {
            editor.setMessage("feat: a summary", body)
            squeeze()
            compare(editor.descFloor, 1.5 * Theme.fontMdLine + Theme.spaceSm)
            tryVerify(() => editor.pairHeight === pairAt(editor.descFloor), undefined, "the pair at the words' floor")
        }

        /// A line of words is all the floor keeps: there is no second line to show the top of.
        function test_one_line_of_words_holds_its_own_line() {
            editor.setMessage("feat: a summary", "one line")
            squeeze()
            verify(editor.descWants < 1.5 * Theme.fontMdLine + Theme.spaceSm, "one line wants less than the floor")
            compare(editor.descFloor, editor.descWants)
        }

        /// The rest height: one summary line, the step and five description lines.
        function restHeight() {
            return Theme.fontLgLine + Theme.spaceSm + Theme.spaceXs + Theme.messageMaxHeight
        }
        /// A summary that wraps to more lines than one but stops short of the summary's cap.
        function setWrapping(bodyText) {
            editor.setMessage(Array.from({ length: 10 }, () => "Merge").join(" "), bodyText)
            tryVerify(() => editor.summaryNeed > Theme.fontLgLine + Theme.spaceSm, undefined, "the summary wraps")
            verify(editor.summaryNeed < Theme.messageMaxHeight, "the summary stops short of its cap")
        }

        /// An empty description gives a wrapping summary its empty room: the band stays.
        function test_a_wrapping_summary_takes_an_empty_description_s_room() {
            setWrapping("")
            tryVerify(() => editor.pairHeight === restHeight(), undefined, "the pair at its rest height")
        }

        /// An empty box holds only its floor against a wrapping summary, however long the standing body it shows.
        function test_a_standing_body_holds_no_room_against_a_wrapping_summary() {
            editor.standingBody = body
            setWrapping("")
            tryVerify(() => editor.descWants > Theme.messageMaxHeight, undefined, "the standing body runs long")
            tryVerify(() => editor.pairHeight === restHeight(), undefined, "the pair at its rest height")
        }

        /// Words that fit beside the wrapping summary keep the band too: the summary took empty room only.
        function test_words_that_fit_beside_a_wrapping_summary_keep_the_rest_height() {
            setWrapping("one line")
            tryVerify(() => editor.pairHeight === restHeight(), undefined, "the pair at its rest height")
            tryVerify(() => editor.descHeight >= editor.descWants, undefined, "the words whole")
        }

        /// Words that fill the box are not cut for the band while the pane has room: the block grows.
        function test_a_wrapping_summary_grows_the_block_over_words_that_fill_it() {
            setWrapping(body)
            compare(editor.descWordRoom, Theme.messageMaxHeight)
            tryVerify(() => editor.pairHeight === editor.summaryNeed + Theme.spaceXs + Theme.messageMaxHeight,
                      undefined, "the summary over the description's share")
            verify(editor.pairHeight > restHeight(), "the block grew")
        }

        /// The growth stops at the pane's room: the words give the rest.
        function test_the_growth_stops_at_the_pane_s_room() {
            setWrapping(body)
            // Half of what the block would grow by: the summary's lines past its first.
            editor.blockRoom = restHeight() + (editor.summaryNeed - Theme.fontLgLine - Theme.spaceSm) / 2
            tryVerify(() => editor.pairHeight === editor.pairRoom, undefined, "the pair fills the room it has")
            tryVerify(() => editor.descHeight < Theme.messageMaxHeight, undefined, "the words gave the rest")
        }

        /// A pane out of room brings the words down to their floor, wrapping summary or not.
        function test_a_pane_out_of_room_brings_the_words_to_their_floor() {
            setWrapping(body)
            squeeze()
            tryVerify(() => editor.pairHeight === pairAt(editor.descFloor), undefined, "the pair at the words' floor")
        }

        /// A summary typed over a standing subject that wraps stands at its own lines, not the subject's.
        function test_the_summary_is_measured_by_its_placeholder_only_while_it_shows() {
            editor.setMessage("feat: a summary", "")
            tryVerify(() => editor.summaryNeed > 0, undefined, "the summary measured")
            const ownLine = editor.summaryNeed
            editor.standingSubject = longWords
            editor.setMessage("", "")
            tryVerify(() => editor.summaryNeed > ownLine, undefined, "the standing subject's lines while it shows")
            editor.setMessage("feat: a summary", "")
            tryVerify(() => editor.summaryNeed === ownLine, undefined, "typed words over it: their own line")
        }

        /// The same for the description: words typed over a standing body want their own height.
        function test_the_description_is_measured_by_its_placeholder_only_while_it_shows() {
            editor.setMessage("feat", "one line")
            tryVerify(() => editor.descWants > 0, undefined, "the description measured")
            const ownLine = editor.descWants
            editor.standingBody = longWords
            editor.setMessage("feat", "")
            tryVerify(() => editor.descWants > ownLine, undefined, "the standing body's lines while it shows")
            editor.setMessage("feat", "one line")
            tryVerify(() => editor.descWants === ownLine, undefined, "typed words over it: their own line")
        }
    }
}

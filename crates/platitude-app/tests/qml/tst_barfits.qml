import QtQuick
import QtTest
import platitude.ui

// What a bar shows has to fit inside it (デザイン規約 §立っている質問は 1 か所で聞く): the bar clips, so any pass
// where its height and its row's disagree shears the last thing in the column — here the first push's two boxes.
// Narrow, as the middle of the window at its floor is, so the line under the heading wraps.
Item {
    id: root
    width: 335
    height: 400

    /// Arrives after the bar is standing (`PublishFlow` writes `askDetail` when the check lands); two lines here.
    readonly property string checkedDetail: "origin/feature/new-thing does not exist yet; this makes it."

    AskBar {
        id: bar
        width: root.width
        code: "push"
        label: "push feature/new-thing where?"
        labelSentence: "push %1 where?"
        labelRef: "feature/new-thing"
        neutral: true
        form: publishForm
    }
    Component {
        id: publishForm
        PublishForm {
            choices: ["origin"]
            remote: "origin"
            branch: "feature/new-thing"
        }
    }

    TestCase {
        name: "BarFits"
        when: windowShown

        function init() {
            bar.open = false
            bar.detail = ""
            tryVerify(() => bar.shut)
        }

        function footOf(item) {
            return item.mapToItem(bar, 0, item.height).y
        }

        function test_the_form_stands_inside_the_bar_that_carries_it() {
            bar.detail = root.checkedDetail
            bar.open = true
            tryVerify(() => bar.settled)
            verify(bar.formItem !== null, "the question has a form to stand")
            verify(footOf(bar.formItem) + Theme.spaceMd <= bar.height,
                   "the form ends " + footOf(bar.formItem) + " into a bar " + bar.height + " tall")
        }

        /// Only the travel is animated: a height still animating towards the late words' room draws the form
        /// outside the clip.
        function test_a_standing_bar_takes_late_words_at_their_full_height_at_once() {
            bar.open = true
            tryVerify(() => bar.settled)
            const before = bar.height

            bar.detail = root.checkedDetail
            // The layout answers a pass or two later.
            tryVerify(() => bar.openHeight > before, undefined,
                      "the line under the heading asked for a second line")
            // And the bar is already that tall — an animated height would still be on its way.
            compare(bar.implicitHeight, bar.openHeight, "the bar took the room in the same turn")
            verify(footOf(bar.formItem) + Theme.spaceMd <= bar.height,
                   "the form ends " + footOf(bar.formItem) + " into a bar " + bar.height + " tall")
        }

        /// The way out is still animated: a condition read off the bar's own standing would drop it in the frame it
        /// is dismissed in (`publish-dismiss` reads the words on the way up).
        function test_the_bar_goes_back_up_rather_than_vanishing() {
            bar.detail = root.checkedDetail
            bar.open = true
            tryVerify(() => bar.settled)

            bar.open = false
            verify(!bar.shut, "it is on its way up, not gone: " + bar.implicitHeight)
            tryVerify(() => bar.shut)
        }

        function test_the_bar_still_comes_down_over_the_list() {
            bar.detail = root.checkedDetail
            bar.open = true
            verify(waitForItemPolished(bar), "the words are laid out")
            verify(bar.implicitHeight < bar.openHeight,
                   "it is on its way, not there: " + bar.implicitHeight + " of " + bar.openHeight)
            tryVerify(() => bar.settled)
        }
    }
}

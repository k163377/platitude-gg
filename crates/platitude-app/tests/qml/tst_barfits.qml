import QtQuick
import QtTest
import platitude.ui

// **What a bar shows has to fit inside it** (デザイン規約 §答えの要らない報せ / §立っている質問は 1 か所で聞く): the bar is
// `clip: true` and it is as tall as its own row asked to be, so every pass where those two disagree shears whatever
// stands last in the column — here the two boxes a first push is answered in.
//
// **A narrow bar**, because that is where the words wrap: the middle of the window at its floor is about this wide,
// and the line under a heading takes two there.
Item {
    id: root
    width: 335
    height: 400

    /// The sentence the remote's answer puts under the heading, which arrives after the bar is already standing
    /// (`PublishFlow` writes it into `askDetail` when the check lands) — and takes two lines at this width.
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

        /// The foot of what the bar is showing, in the bar's own coordinates.
        function footOf(item) {
            return item.mapToItem(bar, 0, item.height).y
        }

        /// Nothing the bar draws is outside it once it has come down.
        function test_the_form_stands_inside_the_bar_that_carries_it() {
            bar.detail = root.checkedDetail
            bar.open = true
            tryVerify(() => bar.settled)
            verify(bar.formItem !== null, "the question has a form to stand")
            verify(footOf(bar.formItem) + Theme.spaceMd <= bar.height,
                   "the form ends " + footOf(bar.formItem) + " into a bar " + bar.height + " tall")
        }

        /// **The 200ms belongs to the coming down and the going up.** The line under the heading arrives after the
        /// bar is standing — the remote's answer lands there — and the words take their room in the pass that lays
        /// them out. A height still animating towards that room is a bar drawing its own content outside itself:
        /// what was seen was the form's boxes sheared off at the bottom with the hairline through them.
        function test_a_standing_bar_takes_late_words_at_their_full_height_at_once() {
            bar.open = true
            tryVerify(() => bar.settled)
            const before = bar.height

            bar.detail = root.checkedDetail
            // The room the words ask for, which is what the layout answers a pass or two later.
            tryVerify(() => bar.openHeight > before, undefined,
                      "the line under the heading asked for a second line")
            // **And the bar is already that tall**: an animated height would still be on its way here, which is
            // the frame the clip cuts through.
            compare(bar.implicitHeight, bar.openHeight, "the bar took the room in the same turn")
            verify(footOf(bar.formItem) + Theme.spaceMd <= bar.height,
                   "the form ends " + footOf(bar.formItem) + " into a bar " + bar.height + " tall")
        }

        /// **And the way out is still the 200ms**, which is the half a rule about "only the travel is animated" is
        /// easiest to lose: a bar already standing is not travelling, so a condition read off its own standing takes
        /// the bar off the screen in the frame it is dismissed in — with the words still being read
        /// (`publish-dismiss` reads them on the way up).
        function test_the_bar_goes_back_up_rather_than_vanishing() {
            bar.detail = root.checkedDetail
            bar.open = true
            tryVerify(() => bar.settled)

            bar.open = false
            verify(!bar.shut, "it is on its way up, not gone: " + bar.implicitHeight)
            tryVerify(() => bar.shut)
        }

        /// And the way in is still animated: a bar comes down over the list rather than appearing on it.
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

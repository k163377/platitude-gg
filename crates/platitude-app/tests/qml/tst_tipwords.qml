import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The words the one shared tooltip comes out with (`SharedToolTip.takeWords`). Qt gives the box the target's text when
// the ask rises and passes a change on only while the box is up: an ask and its words moving in one change — the
// commit button turning down an amend git has just answered for — open the box on the words from before whenever the
// ask is heard first. Set here in that order, so the case does not hang on which binding runs first.
Item {
    id: root
    width: 400
    height: 300
    // What `Main.qml` and the popup bases declare (rules-refs/app-ui.md「ToolTip.policy」).
    ToolTip.policy: ToolTip.Manual

    PointerWatch { id: hand }

    SharedToolTip {
        id: shared
        host: root
        hand: hand
    }

    Item {
        id: asker
        y: 120
        width: 300
        height: 40
        property bool asked: false
        property string why: "Stage something to commit"
        ToolTip.text: asker.why
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.visible: asker.asked
    }

    TestCase {
        name: "TipWords"
        when: windowShown

        function initTestCase() {
            shared.dressToolTip()
        }

        function test_words_changed_while_the_delay_counts_are_the_ones_that_come_out() {
            const tip = shared.sharedTip
            asker.asked = true
            asker.why = "Amending would leave the last commit empty"
            verify(!tip.visible, "the change is made while the delay still counts")
            tryVerify(() => tip.visible, undefined, "the box comes out once the delay is over")
            compare(tip.text, "Amending would leave the last commit empty", "with the words the target holds now")
        }
    }
}

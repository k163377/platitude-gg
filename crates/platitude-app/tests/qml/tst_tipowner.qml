import QtQuick
import QtQuick.Controls.Fusion
import QtTest

// What an attached `ToolTip.visible` answers when the one shared instance is standing somewhere else. Every
// `*TipShown` in this app is that read taken off the item the tip is about (`SignatureMark.tipShown`,
// `MessageEditor.summaryTipShown`, `NavItemDelegate.editTipShown`), and each of them is reported as "the tip on
// **this** target" — a claim that holds only while `QQuickToolTipAttached` weighs the instance's parent against its
// own. There is one tooltip in the window, so a Qt that dropped that test would leave every one of those reports
// saying "somebody's tip is up" in a line that names one target (app-ui.md §hover のツールチップ).
Item {
    id: root
    width: 400
    height: 200

    Item {
        id: mine
        width: 100
        height: 30
        ToolTip.text: "mine"
    }
    Item {
        id: theirs
        y: 60
        width: 100
        height: 30
        ToolTip.text: "theirs"
    }

    TestCase {
        name: "TipOwner"
        when: windowShown

        function init() {
            mine.ToolTip.visible = false
            theirs.ToolTip.visible = false
            tryCompare(mine.ToolTip.toolTip, "visible", false)
        }

        function test_the_attached_read_is_the_tip_on_this_target() {
            mine.ToolTip.visible = true
            // The instance itself, which is the same object either side and says only that something is up.
            const tip = mine.ToolTip.toolTip
            tryCompare(tip, "visible", true)
            compare(tip, theirs.ToolTip.toolTip, "one instance for the whole tree")
            compare(mine.ToolTip.visible, true, "the target it was raised on says so")
            compare(theirs.ToolTip.visible, false, "the one beside it does not")
            compare(tip.text, "mine", "and the instance is carrying that target's words")
        }

        // An attached property is no name a string can reach, so the waits here are expressions.
        function test_the_read_moves_with_the_instance() {
            mine.ToolTip.visible = true
            tryVerify(() => mine.ToolTip.visible)
            theirs.ToolTip.visible = true
            tryVerify(() => theirs.ToolTip.visible)
            compare(mine.ToolTip.visible, false, "the tip left the target it was on")
            compare(mine.ToolTip.toolTip.text, "theirs", "and took the other's words with it")
        }
    }
}

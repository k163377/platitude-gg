import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The tips inside every popup base keep Qt's manual timing: no delay but their own, and no timeout. A popup does not
// inherit `ToolTip.policy` from the window (Qt 6.12's attached-property propagation stops at the popup), and under the
// automatic policy each tip would wait the platform's wake-up delay and go after ten seconds — so each base declares
// it, and this root, unlike `Main.qml`, does not.
Item {
    id: root
    width: 480
    height: 360

    property bool up: false

    AppCard {
        id: card
        width: 160
        height: 80
        contentItem: Item {
            Item {
                id: inCard
                width: 40
                height: 20
                ToolTip.text: "in a card"
                ToolTip.visible: root.up && card.opened
            }
        }
    }

    AppMenu {
        id: menu
        Item {
            id: inMenu
            width: 40
            height: 20
            ToolTip.text: "in a menu"
            ToolTip.visible: root.up && menu.opened
        }
    }

    AppDialog {
        id: dialog
        Item {
            id: inDialog
            width: 40
            height: 20
            ToolTip.text: "in a dialog"
            ToolTip.visible: root.up && dialog.opened
        }
    }

    AppCombo {
        id: combo
        width: 160
        model: ["origin", "upstream"]
    }

    // The list's rows hold no tip of their own, so one is seated in the list.
    Item {
        id: inCombo
        parent: combo.popup.contentItem
        width: 40
        height: 20
        ToolTip.text: "in a combo's list"
        ToolTip.visible: root.up && combo.popup.opened
    }

    TestCase {
        name: "TipPolicy"
        when: windowShown

        function timingOf(target) {
            return "delay " + target.ToolTip.delay + ", timeout " + target.ToolTip.timeout
        }

        function test_a_tip_in_each_popup_base_keeps_manual_timing_data() {
            return [
                { tag: "card", popup: card, target: inCard },
                { tag: "menu", popup: menu, target: inMenu },
                { tag: "dialog", popup: dialog, target: inDialog },
                { tag: "combo", popup: combo.popup, target: inCombo },
            ]
        }

        function test_a_tip_in_each_popup_base_keeps_manual_timing(data) {
            data.popup.open()
            tryVerify(() => data.popup.opened, undefined, "the popup is up")
            root.up = true
            compare(timingOf(data.target), "delay 0, timeout -1", "the tip times itself")
            verify(data.target.ToolTip.visible, "and comes out as soon as it is asked for")
            root.up = false
            data.popup.close()
            tryVerify(() => !data.popup.visible, undefined, "the popup is down")
        }
    }
}

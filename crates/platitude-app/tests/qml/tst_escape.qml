import QtQuick
import QtQuick.Controls
import QtTest

// How Qt hands Escape out, which is what decides where anything in this window
// may answer it (デザイン規約 §hover のツールチップ). Four facts, and the
// product depends on every one of them:
//
//  - a `Shortcut` is matched before the key is delivered, so it beats a key
//    handler on the focus chain;
//  - a popup holding the focus answers for itself and keeps a window shortcut
//    out of it;
//  - an Escape nobody claimed reaches the handler on an ancestor;
//  - **two enabled `StandardKey.Cancel` shortcuts in one window fire neither.**
//
// The last one is the trap: the ask bar and the notice bar each own one
// already, so anything else that wants Escape has to take it as a key handler
// (`RepoPage.escapePressed`) rather than as a third shortcut.
TestCase {
    id: scene
    name: "escape"
    when: windowShown
    width: 200
    height: 200
    visible: true

    property int firstFired: 0
    property int secondFired: 0
    property bool secondOn: false
    property int handlerFired: 0

    Shortcut {
        id: loneShortcut
        sequences: [StandardKey.Cancel]
        onActivated: scene.firstFired++
    }
    Shortcut {
        sequences: [StandardKey.Cancel]
        enabled: scene.secondOn
        onActivated: scene.secondFired++
    }
    Popup {
        id: card
        closePolicy: Popup.CloseOnEscape
        // A popup only hears keys it was given: without this the key reaches
        // neither the card nor the shortcut, and the run says nothing at all.
        focus: true
        width: 40
        height: 40
    }
    Item {
        anchors.fill: parent
        Keys.onEscapePressed: event => {
            scene.handlerFired++
            event.accepted = true
        }
        Item {
            id: leaf
            focus: true
        }
    }

    function init() {
        scene.secondOn = false
        scene.firstFired = 0
        scene.secondFired = 0
        scene.handlerFired = 0
        loneShortcut.enabled = true
    }

    function test_one_alone() {
        keyClick(Qt.Key_Escape)
        compare(scene.firstFired, 1, "one enabled Cancel fires")
        compare(scene.secondFired, 0, "the disabled one does not")
    }

    // **The trap.** Neither is called — so a second owner does not win, it puts
    // the first one out of action as well.
    function test_two_at_once() {
        scene.secondOn = true
        keyClick(Qt.Key_Escape)
        compare("first=" + scene.firstFired + " second=" + scene.secondFired,
                "first=0 second=0",
                "two enabled Cancels cancel each other out")
    }

    function test_three_popup_versus_shortcut() {
        card.open()
        tryCompare(card, "opened", true)
        keyClick(Qt.Key_Escape)
        compare("closed=" + !card.opened + " shortcut=" + scene.firstFired,
                "closed=true shortcut=0",
                "an open popup takes Escape and the window shortcut stays out of it")
        card.close()
    }

    function test_four_handler_under_a_shortcut() {
        leaf.forceActiveFocus()
        keyClick(Qt.Key_Escape)
        compare("shortcut=" + scene.firstFired + " handler=" + scene.handlerFired,
                "shortcut=1 handler=0",
                "an enabled shortcut takes the key before the handler chain sees it")
    }

    function test_five_handler_with_no_shortcut() {
        leaf.forceActiveFocus()
        loneShortcut.enabled = false
        keyClick(Qt.Key_Escape)
        compare("shortcut=" + scene.firstFired + " handler=" + scene.handlerFired,
                "shortcut=0 handler=1",
                "with nothing claiming it, Escape reaches the ancestor's handler")
    }
}

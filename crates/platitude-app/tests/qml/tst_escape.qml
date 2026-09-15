import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
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
//
// And two more about the ancestor in that third fact, which is only an
// ancestor for as long as the keyboard stays inside it:
//
//  - a pane swapped off the screen that lets the keyboard go drops it **out of
//    a plain ancestor item altogether** — the window's content item takes it,
//    and the handler is never reached again;
//  - a `FocusScope` in that same place catches the fall and goes on hearing
//    the key, which is why the page is one.
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
        scene.plainFired = 0
        scene.scopeFired = 0
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

    // The page's own shape, twice: the panes swap in a stack, the one going off
    // screen lets the keyboard go the way every pane here does
    // (`GraphList.onVisibleChanged`), and what is under them is a plain item in
    // one and a focus scope in the other.
    property int plainFired: 0
    property int scopeFired: 0
    Item {
        id: plainPage
        width: 40
        height: 40
        Keys.onEscapePressed: event => {
            scene.plainFired++
            event.accepted = true
        }
        StackLayout {
            id: plainStack
            currentIndex: 0
            Item { id: plainFront }
            ListView {
                id: plainPane
                model: 1
                delegate: Item { width: 4; height: 4 }
                onVisibleChanged: if (!plainPane.visible) plainPane.focus = false
            }
        }
    }
    FocusScope {
        id: scopePage
        width: 40
        height: 40
        Keys.onEscapePressed: event => {
            scene.scopeFired++
            event.accepted = true
        }
        StackLayout {
            id: scopeStack
            currentIndex: 0
            Item { id: scopeFront }
            ListView {
                id: scopePane
                model: 1
                delegate: Item { width: 4; height: 4 }
                onVisibleChanged: if (!scopePane.visible) scopePane.focus = false
            }
        }
    }

    // **The hole.** The pane had it, the pane is gone, and the item that was
    // its ancestor is not on the key's way any longer — so an Escape after that
    // swap reaches nothing at all.
    function test_six_a_plain_item_loses_the_key_with_the_pane() {
        loneShortcut.enabled = false
        plainStack.currentIndex = 1
        verify(waitForRendering(plainStack))
        plainPane.forceActiveFocus()
        verify(plainPane.activeFocus)
        plainStack.currentIndex = 0
        verify(waitForRendering(plainStack))
        keyClick(Qt.Key_Escape)
        compare("inside=" + plainPage.activeFocus + " fired=" + scene.plainFired,
                "inside=false fired=0",
                "a plain ancestor keeps neither the focus nor the key")
    }

    // And the same swap under a scope, which is what the page is made of.
    function test_seven_a_scope_catches_the_fall() {
        loneShortcut.enabled = false
        scopeStack.currentIndex = 1
        verify(waitForRendering(scopeStack))
        scopePane.forceActiveFocus()
        verify(scopePage.activeFocus)
        scopeStack.currentIndex = 0
        verify(waitForRendering(scopeStack))
        keyClick(Qt.Key_Escape)
        compare("inside=" + scopePage.activeFocus + " fired=" + scene.scopeFired,
                "inside=true fired=1",
                "the scope holds the keyboard the pane let go of")
    }
}

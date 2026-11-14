import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The ☰'s card is built the first time it is asked for (`AppMenuButton.card`): the readings and the automation's door
// answer for a card never built, the press that builds it opens it, and a harness can have it built up front
// (`keepBuilt`).
Item {
    id: root
    width: 600
    height: 400

    Component {
        id: buttonComponent
        AppMenuButton {
            height: 40
        }
    }
    Component {
        id: spyComponent
        SignalSpy {
            signalName: "cloneRepositoryRequested"
        }
    }

    TestCase {
        name: "AppMenuButton"
        when: windowShown

        /// The seat the card is built in: the button's one loader.
        function seatOf(button) {
            for (const child of button.children)
                if (child instanceof Loader)
                    return child
            return null
        }

        function test_the_clone_door_answers_a_card_never_built() {
            const button = createTemporaryObject(buttonComponent, root)
            const asked = createTemporaryObject(spyComponent, root, { "target": button })
            compare(seatOf(button).item, null, "nothing has asked for the card")
            compare(button.menuOpen, false, "a card never built reads as one never opened")
            button.clickCloneRow()
            compare(asked.count, 1, "the row's own wiring ran")
            verify(seatOf(button).item !== null, "the door built the card to press its row")
            compare(button.menuOpen, false, "and left it shut")
        }

        function test_the_press_builds_the_card_and_toggles_it() {
            const button = createTemporaryObject(buttonComponent, root)
            compare(seatOf(button).item, null)
            button.clicked()
            tryCompare(button, "menuOpen", true)
            const card = seatOf(button).item
            // Hung off the button, not its seat: "outside" is measured from the card's parent, and the press that
            // shuts it has to land inside.
            compare(card.parent, button)
            button.clicked()
            tryCompare(button, "menuOpen", false)
            compare(seatOf(button).item, card, "built once, kept for the next press")
        }

        function test_a_card_kept_built_stands_from_the_start_and_shut() {
            const button = createTemporaryObject(buttonComponent, root, { "keepBuilt": true })
            verify(seatOf(button).item !== null)
            compare(button.menuOpen, false)
        }
    }
}

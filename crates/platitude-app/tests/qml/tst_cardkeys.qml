import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A card the hand opened leaves the window its keys; a card holding the keyboard answers Escape first
// (rules-refs/app-ui.md「hover のカードは窓の `Shortcut` を塞がない」). Qt keeps every window `Shortcut` out while a
// popup other than a tooltip stands with `CloseOnEscape` (`QQuickShortcutContext`'s `isBlockedByPopup`), and that
// popup hears Escape only from the keyboard it holds (`QQuickPopup::keyPressEvent`).
Item {
    id: root
    width: 480
    height: 360

    property int found: 0
    property int cancelled: 0

    // The window's own, as `Main` binds Ctrl+F and the bars bind Escape.
    Shortcut {
        sequences: [StandardKey.Find]
        onActivated: root.found++
    }
    Shortcut {
        sequences: [StandardKey.Cancel]
        onActivated: root.cancelled++
    }

    // Where the keyboard is while the hand reads a card: on the page, not in the card.
    Item {
        id: page
        anchors.fill: parent
    }

    AppCard {
        id: card
        x: 20
        y: 20
        contentItem: CardText {
            id: words
            text: "a1b2c3d Fix the band's width"
        }
    }
    // The pointer leaving is what closes it (`BandStateCard`, `SectionPeekPopup`).
    AppCard {
        id: leavingCard
        x: 20
        y: 160
        closesOnPressOutside: false
        contentItem: CardText {
            id: leavingWords
            text: "Ahead of origin/main by 2 commits"
        }
    }

    TestCase {
        name: "CardKeys"
        when: windowShown

        function init() {
            root.found = 0
            root.cancelled = 0
            page.forceActiveFocus()
        }

        function cleanup() {
            card.close()
            leavingCard.close()
            tryCompare(card, "visible", false)
            tryCompare(leavingCard, "visible", false)
        }

        function cards() {
            return [
                { tag: "a press outside closes it", popup: card, words: words },
                { tag: "the pointer leaving closes it", popup: leavingCard, words: leavingWords }
            ]
        }

        function test_a_card_the_hand_opened_leaves_the_window_its_keys_data() {
            return cards()
        }
        function test_a_card_the_hand_opened_leaves_the_window_its_keys(data) {
            data.popup.open()
            tryCompare(data.popup, "opened", true)
            verify(!data.popup.activeFocus, "a card the hand opened does not take the keyboard")
            keySequence(StandardKey.Find)
            keyClick(Qt.Key_Escape)
            compare("found=" + root.found + " cancelled=" + root.cancelled + " open=" + data.popup.opened,
                    "found=1 cancelled=1 open=true",
                    "Ctrl+F and Escape reach the window's shortcuts; the card is not the keyboard's to close")
        }

        function test_a_card_holding_the_keyboard_answers_escape_first_data() {
            return cards()
        }
        function test_a_card_holding_the_keyboard_answers_escape_first(data) {
            data.popup.open()
            tryCompare(data.popup, "opened", true)
            // A press on the words takes the keyboard, as a selection starting does.
            mouseClick(data.words, 4, data.words.height / 2)
            verify(data.popup.activeFocus, "a press on the words puts the keyboard in the card")
            keyClick(Qt.Key_Escape)
            compare("cancelled=" + root.cancelled + " open=" + data.popup.opened,
                    "cancelled=0 open=false",
                    "the card standing on top takes Escape before the window's shortcut")
        }

        function test_the_keyboard_leaving_gives_the_window_its_keys_back() {
            card.open()
            tryCompare(card, "opened", true)
            mouseClick(words, 4, words.height / 2)
            verify(card.activeFocus, "a press on the words puts the keyboard in the card")
            page.forceActiveFocus()
            verify(!card.activeFocus, "the keyboard went back to the page")
            keySequence(StandardKey.Find)
            compare("found=" + root.found + " open=" + card.opened, "found=1 open=true",
                    "the card still up, Ctrl+F is the window's again")
        }
    }
}

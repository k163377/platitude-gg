import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The popups that write themselves into `OutsidePress` while they stand — the menus and lists a press outside puts
// down, not the hover cards — and the press the scene never hears closing them (`closeAll`): the platform takes the
// presses on the band's grab-runs and the resize edges (`WindowChrome.frameYielded`), another window the ones on it
// (`Main.onActiveChanged`). The bases throughout, so what is measured is what every menu, card and list inherits.
Item {
    id: root
    width: 600
    height: 400

    AppMenu {
        id: host
        AppMenuItem {
            text: "plain"
        }
        AppMenu {
            id: card
            title: "CARD"
            AppMenuItem {
                text: "in the card"
            }
        }
    }

    /// Dropped from a mark, as the ☰'s card is: "outside" is measured from its parent.
    AppMenu {
        id: dropped
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
        AppMenuItem {
            text: "dropped"
        }
    }

    /// A hover card (`HoverSeat` opens every `AppCard`), under the policy a press outside closes.
    AppCard {
        id: hoverCard
        width: 100
        height: 60
    }

    AppCombo {
        id: combo
        y: 300
        width: 160
        model: ["origin", "upstream"]
    }

    Component {
        id: menuMaker
        AppMenu {
            AppMenuItem {
                text: "made"
            }
        }
    }

    TestCase {
        name: "OutsidePress"
        when: windowShown

        function init() {
            host.close()
            dropped.close()
            hoverCard.close()
            combo.popup.close()
            tryCompare(OutsidePress, "any", false)
        }

        function test_a_menu_is_written_in_while_it_stands() {
            verify(host.offer(), "the host has rows to offer")
            tryCompare(host, "opened", true)
            compare(OutsidePress.standing.length, 1)
            verify(OutsidePress.standing[0] === host, "the host is the one written in")
            host.close()
            tryCompare(OutsidePress, "any", false)
        }

        function test_a_submenu_stands_with_its_host_and_goes_with_it() {
            verify(host.offer())
            tryCompare(host, "opened", true)
            verify(host.openSub(card), "the card has rows to offer")
            tryCompare(card, "opened", true)
            compare(OutsidePress.standing.length, 2)
            card.dismiss()
            tryCompare(OutsidePress, "any", false)
        }

        function test_a_menu_measured_from_its_parent_counts_too() {
            verify(dropped.offerHere())
            tryCompare(dropped, "opened", true)
            verify(OutsidePress.any, "the ☰'s policy is closed by a press outside too")
        }

        function test_a_combo_list_counts() {
            combo.popup.open()
            tryCompare(combo.popup, "opened", true)
            verify(OutsidePress.any)
        }

        function test_a_hover_card_is_not_written_in() {
            hoverCard.open()
            tryCompare(hoverCard, "opened", true)
            verify(!OutsidePress.any, "the hand leaving closes it; written in, it would hold the edges from resizing")
        }

        function test_close_all_closes_every_menu_and_list_and_no_hover_card() {
            verify(host.offer())
            tryCompare(host, "opened", true)
            verify(host.openSub(card))
            tryCompare(card, "opened", true)
            combo.popup.open()
            tryCompare(combo.popup, "opened", true)
            hoverCard.open()
            tryCompare(hoverCard, "opened", true)
            OutsidePress.closeAll()
            tryCompare(host, "visible", false)
            tryCompare(card, "visible", false)
            tryCompare(combo.popup, "visible", false)
            compare(OutsidePress.standing.length, 0)
            verify(hoverCard.visible, "a hover card goes with the hand, not here")
        }

        function test_a_menu_destroyed_standing_is_written_out() {
            const made = createTemporaryObject(menuMaker, root)
            verify(made.offer())
            tryCompare(made, "opened", true)
            verify(OutsidePress.any)
            made.destroy()
            tryCompare(OutsidePress, "any", false)
        }
    }
}

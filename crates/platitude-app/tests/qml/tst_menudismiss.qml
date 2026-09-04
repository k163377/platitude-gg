import QtQuick
import QtTest
import platitude.ui

// What a row inside a nested card leans on when it takes the whole menu down: Qt's own `Menu.dismiss()` walks up
// from the menu it was called on and closes every level (`QQuickMenu::dismiss`). The product's cards call it and
// nothing else — no `closeRequested` back to the menu they hang off, no child-then-parent `close()` pair — so the
// fact is pinned here, where a Qt that changed it would go red before the app did (app-ui.md §メニューを閉じるのは自分).
//
// `AppMenu` rather than a bare `Menu`: the card's title row is built by the host's `delegate`, and `openSub` is
// the automation's own way into a card, so what is measured is the shape the app actually opens.
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
            AppMenu {
                id: deeper
                title: "DEEPER"
                AppMenuItem {
                    text: "two levels down"
                }
            }
        }
    }

    TestCase {
        name: "MenuDismiss"
        when: windowShown

        function init() {
            host.close()
            tryCompare(host, "visible", false)
        }

        function openHostAndCard() {
            verify(host.offer(), "the host has rows to offer")
            tryCompare(host, "opened", true)
            verify(host.openSub(card), "the card has rows to offer")
            tryCompare(card, "opened", true)
        }

        function test_dismiss_on_a_card_takes_the_host_down_with_it() {
            openHostAndCard()
            card.dismiss()
            tryCompare(card, "visible", false)
            tryCompare(host, "visible", false)
        }

        function test_dismiss_two_levels_down_takes_every_level() {
            openHostAndCard()
            verify(card.openSub(deeper), "the deeper card has rows to offer")
            tryCompare(deeper, "opened", true)
            deeper.dismiss()
            tryCompare(deeper, "visible", false)
            tryCompare(card, "visible", false)
            tryCompare(host, "visible", false)
        }

        function test_dismiss_at_the_top_is_a_plain_close() {
            verify(host.offer())
            tryCompare(host, "opened", true)
            host.dismiss()
            tryCompare(host, "visible", false)
        }

        // The other direction, which the rules also lean on: a host that closes takes its open card down.
        function test_closing_the_host_takes_the_open_card_with_it() {
            openHostAndCard()
            host.close()
            tryCompare(host, "visible", false)
            tryCompare(card, "visible", false)
        }
    }
}

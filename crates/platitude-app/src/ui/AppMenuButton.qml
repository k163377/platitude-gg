pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui
import platitude.ui.input
import platitude.ui.words

// The ☰ app menu, sized as the head of the folded sidebar's column: `railWidth` wide, the rail cells' wash, the band's
// full height (`NavRail.cellHeight`). Its mark is a step under the cells': it stands alone in a band (デザイン規約 §寸法).
ToolButton {
    id: menuButton

    /// The active tab's RepoPage; null while no tab is open.
    property var curPage: null
    /// The card stands built from the start — written only by the harness (`WindowHarness`, through `TopBar` and
    /// `TabStrip`), as `RepoPage.keepBuilt`. Otherwise it is built the first time it is asked for (`card`).
    property bool keepBuilt: false

    /// The card is standing — false while it has never been built.
    readonly property bool menuOpen: menuSeat.item !== null && menuSeat.item.opened

    signal openRepositoryRequested()
    signal cloneRepositoryRequested()
    signal settingsRequested()
    /// Exit takes the band's ✕ road (`TabStrip` → `TopBar.closeRequested`), so the close gate that waits a write out
    /// (`Main.qml`) has one door.
    signal exitRequested()

    /// The card, built now if it never was: every door builds it, then calls in synchronously (`RepoPage`'s menu
    /// seats).
    function card() {
        menuSeat.active = true
        return menuSeat.item
    }
    /// Automation (`_PGG_AUTO_ACT=clone-*`): emits the row's own `triggered`, so a run goes through the hand's wiring.
    function clickCloneRow() {
        menuButton.card().cloneItem.triggered()
    }

    width: Theme.railWidth
    padding: 0
    hoverEnabled: true
    Accessible.name: qsTr("Application menu")
    // The wash stays while the menu is open. `Hand.away`: headless, a pointer nobody put rests on this corner cell
    // (rules-refs/app-ui.md「ヘッドレスの窓には手が乗っている」).
    background: Rectangle {
        color: (menuButton.hovered && !Hand.away) || menuButton.menuOpen ? Theme.bgHover : "transparent"
    }
    contentItem: Item {
        NavIcon {
            anchors.centerIn: parent
            width: Theme.iconLg
            height: Theme.iconLg
            kind: "menu"
            tint: Theme.textPrimary
        }
    }
    // A toggle, paired with the close policy below (rules-refs/app-ui.md「`ToolButton` + `Menu` の押し直し」).
    // `open()`, not `offer()`: the rows are fixed, so the menu is never empty, and `offer()`'s bare `popup()` opens
    // at the pointer, losing `y` below (the right-click menus keep `offer()`).
    onClicked: menuButton.menuOpen ? menuSeat.item.close() : menuButton.card().open()
    // A seat, not the card: built on the first press (`card`). The card hangs off the button itself, not the seat —
    // "outside" is measured from its parent, and a press on the button has to land inside it for the toggle.
    Loader {
        id: menuSeat
        active: menuButton.keepBuilt
        sourceComponent: AppMenu {
            /// Automation: the `Clone repository…` row (`clickCloneRow`).
            readonly property AppMenuItem cloneItem: cloneRow
            parent: menuButton
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
            // A full-height cell ends where the band does, so the card would otherwise open on top of the divider.
            y: menuButton.height + Theme.splitterWidth
            AppMenuItem {
                text: Words.openRepository
                onTriggered: menuButton.openRepositoryRequested()
            }
            AppMenuItem {
                id: cloneRow
                text: qsTr("Clone repository…")
                onTriggered: menuButton.cloneRepositoryRequested()
            }
            AppMenuSeparator {}
            // A local re-read (no network), for where the on-tick refresh cannot reach.
            AppMenuItem {
                text: qsTr("Reload")
                doable: menuButton.curPage !== null
                onTriggered: menuButton.curPage.pageTab.refreshAll()
            }
            AppMenuSeparator {}
            // One row for the one screen: a row per category reads as a duplicate, and the menu starts telling the
            // screen's contents. It opens `Application` (`SettingsDialog.openAt`).
            AppMenuItem {
                text: qsTr("Settings…")
                onTriggered: menuButton.settingsRequested()
            }
            AppMenuItem {
                text: qsTr("About Platitude GG")
                doable: false
            }
            AppMenuSeparator {}
            AppMenuItem {
                text: qsTr("Exit")
                onTriggered: menuButton.exitRequested()
            }
        }
    }
}

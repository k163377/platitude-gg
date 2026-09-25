pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The ☰ app menu, sized as the head of the folded sidebar's column: `railWidth` wide, the rail cells' wash, the band's
// full height (`NavRail.cellHeight`). Its mark is a step under the cells': it stands alone in a band (デザイン規約 §寸法).
ToolButton {
    id: menuButton

    /// The active tab's RepoPage; null while no tab is open.
    property var curPage: null

    /// The card is standing: the band's grab run stops being the caption meanwhile, so a press there reaches the scene
    /// and closes it (`WindowChrome.captionYielded`).
    readonly property bool menuOpen: appMenu.opened

    signal openRepositoryRequested()
    signal cloneRepositoryRequested()
    signal settingsRequested()
    /// Exit takes the band's ✕ road (`TabStrip` → `TopBar.closeRequested`), so the close gate that waits a write out
    /// (`Main.qml`) has one door.
    signal exitRequested()

    /// Automation (`PGG_AUTO_ACT=clone-*`): emits the row's own `triggered`, so a run goes through the hand's wiring.
    function clickCloneRow() {
        cloneRow.triggered()
    }

    width: Theme.railWidth
    padding: 0
    hoverEnabled: true
    Accessible.name: qsTr("Application menu")
    // The wash stays while the menu is open. `Hand.away`: headless, a pointer nobody put rests on this corner cell
    // (rules-refs/app-ui.md「ヘッドレスの窓には手が乗っている」).
    background: Rectangle {
        color: (menuButton.hovered && !Hand.away) || appMenu.opened ? Theme.bgHover : "transparent"
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
    onClicked: appMenu.opened ? appMenu.close() : appMenu.open()
    AppMenu {
        id: appMenu
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
            enabled: menuButton.curPage !== null
            onTriggered: menuButton.curPage.pageTab.refreshAll()
        }
        AppMenuSeparator {}
        // One row for the one screen: a row per category reads as a duplicate.
        AppMenuItem {
            text: qsTr("Settings…")
            onTriggered: menuButton.settingsRequested()
        }
        AppMenuItem {
            text: qsTr("About Platitude GG")
            enabled: false
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: qsTr("Exit")
            onTriggered: menuButton.exitRequested()
        }
    }
}

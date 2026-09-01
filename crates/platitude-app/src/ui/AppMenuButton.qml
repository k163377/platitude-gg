pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// App menu; most entries are placeholders until their phases land. Sized as the head of the folded sidebar's
// column — `railWidth` wide, the rail cells' wash, and this band's own full height (`NavRail.cellHeight`). The
// mark inside is a step under theirs: this one stands alone in a band, theirs are the cells (デザイン規約 §寸法).
ToolButton {
    id: menuButton

    /// The RepoPage of the active tab (null while no tab is open). The one live entry below is the only thing here
    /// that asks anything of it.
    property var curPage: null

    /// Whether the ☰'s card is standing. The grab run gives up being the window's caption while it is, so that a
    /// press on the band's empty run reaches the scene and takes the card down (`WindowChrome.captionYielded`).
    readonly property bool menuOpen: appMenu.opened

    signal openRepositoryRequested()
    signal cloneRepositoryRequested()
    signal settingsRequested()
    /// The ☰'s Exit row. It means what the band's ✕ means, and it takes the same road (`TabStrip` and then `TopBar`
    /// fold it into `closeRequested`), so the close gate that waits a write out (`Main.qml`) has one door to stand at.
    signal exitRequested()

    /// Automation: the ☰'s `Clone repository…` row (`PG_AUTO_ACT=clone-*`). The row's own `triggered` — the signal a
    /// press on it emits — so the handler that runs is the row's, and everything it reaches from there is the wiring a
    /// hand goes through.
    function clickCloneRow() {
        cloneRow.triggered()
    }

    width: Theme.railWidth
    padding: 0
    hoverEnabled: true
    Accessible.name: qsTr("Application menu")
    // Written out instead of borrowing HoverToolButton: an open menu keeps the wash, which no hover of its own can
    // say — what is on screen has to say which mark put it there (`NavRail`).
    background: Rectangle {
        color: menuButton.hovered || appMenu.opened ? Theme.bgHover : "transparent"
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
    // The mark is the way in and the way out: a press on it while the card stands takes the card down, the way a
    // press anywhere else in the window does. Written as a toggle rather than as `open()` — `open()` on a card
    // already up does nothing, which reads as a mark that can never be pressed a second time.
    onClicked: appMenu.opened ? appMenu.close() : appMenu.open()
    AppMenu {
        id: appMenu
        // Outside the ☰ itself, not outside the card. The default policy calls the mark's own press "outside" and
        // closes on it — and the click that follows the same press opens the card again, so the second press never
        // shuts anything (`AppCombo.popup` was written from the same reading). Every press elsewhere in the window
        // still takes the card down, the band's own empty run included (`WindowChrome.captionYielded`).
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
        // One row, because there is one screen. A second entry naming a category of it would be a menu telling
        // the reader about the inside of the thing it opens (observed — the two rows read as a
        // duplicate).
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
            // Up to the window rather than `Qt.quit()`: quitting is a close of the window, and one named road
            // (`TopBar.closeRequested` → `root.close()`) is what keeps the close gate that waits a running
            // write out from having a second spelling (`Main.qml` onClosing).
            onTriggered: menuButton.exitRequested()
        }
    }
}

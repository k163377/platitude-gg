import QtQuick
import platitude.ui

// The right-click on a commit's changed file: the file's page on GitHub (デザイン規約 §GitHub のページへのリンク).
// Nothing here starts git, so it neither waits nor stands again.
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: commitFileMenu

    /// Whether the menu is up, for the panes that hold still under it. A plain property: `visible` read from another
    /// file comes back stale (`RefusalBadge`).
    readonly property bool showing: menu.visible

    readonly property alias menu: menu
    readonly property alias githubCard: githubMenu

    /// A link on its way to the clipboard (`GitHubMenu`); the page owns the clipboard.
    signal copyRequested(string text)

    anchors.fill: parent

    /// Stands the GITHUB card on `links` (`GitHubLinks.ofFile`) and opens. Says whether it opened.
    function offerOn(links) {
        menu.releaseShape()
        githubMenu.standOn(links)
        return menu.offer()
    }

    // Closes itself (rules-refs/app-ui.md「メニューを閉じるのは自分」).
    AppMenu {
        id: menu
        GitHubMenu {
            id: githubMenu
            onCopyRequested: text => commitFileMenu.copyRequested(text)
        }
    }
}

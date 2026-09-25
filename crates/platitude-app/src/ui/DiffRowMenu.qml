pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The right-click on the diff's own text (デザイン規約 §diff の中身をコピーする). Both rows act on the selection, settled
// before this is asked for (`DiffTextSelect.askMenu`): `Copy` takes the washed new side, `Copy removed lines` the old
// side's whole lines.
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: diffRowMenu

    required property DiffModel diffModel

    /// Text on its way to the clipboard; the page owns the clipboard.
    signal copyRequested(string text)

    /// Decided as it opens and held while it stands (規約 §メニュー).
    property bool canCopy: false
    property int removed: 0

    /// The card is on screen — what the page reads to know hover is behind a menu (デザイン規約 §メニュー). A plain
    /// property: `visible` read from another file comes back stale (`RefusalBadge`).
    readonly property bool showing: codeMenu.visible

    /// Automation: the rows themselves (the same exposure `GraphPane.view` is).
    readonly property alias menu: codeMenu

    anchors.fill: parent

    /// Automation: the card was asked for, and whether it came up (`offer()`'s answer) — a card with nothing to
    /// offer refuses to open, which a picture cannot tell from a card nobody asked for.
    signal offered(bool shown)

    /// Opens on whatever the selection holds; a selection that takes neither side opens nothing (規約 §メニュー).
    function offer() {
        diffRowMenu.canCopy = diffRowMenu.diffModel.selHasNew
        diffRowMenu.removed = diffRowMenu.diffModel.selRemoved
        diffRowMenu.offered(codeMenu.offer())
    }

    /// Named so a headless run enters where the row enters (verify-ui).
    function copyNow() {
        diffRowMenu.copyRequested(diffRowMenu.diffModel.selectionText())
    }
    function copyRemovedNow() {
        diffRowMenu.copyRequested(diffRowMenu.diffModel.removedText())
    }

    AppMenu {
        id: codeMenu
        AppMenuItem {
            text: qsTr("Copy")
            offered: diffRowMenu.canCopy
            onTriggered: diffRowMenu.copyNow()
        }
        AppMenuItem {
            text: diffRowMenu.removed > 1 ? qsTr("Copy removed lines") : qsTr("Copy removed line")
            offered: diffRowMenu.removed > 0
            onTriggered: diffRowMenu.copyRemovedNow()
        }
    }
}

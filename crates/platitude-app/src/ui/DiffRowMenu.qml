pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The right-click on the diff's own text.
//
// **Both rows act on the selection**, and what the selection is was settled before this was asked for: a click inside
// it leaves it alone, one outside it takes the row underneath (`DiffTextSelect.askMenu`). So there is nothing here
// about which row was clicked — the wash on screen already says what will be taken.
//
// Two rows, because a diff has two sides and only one of them can be dragged over
// (デザイン規約 §diff の中身をコピーする):
//
//  - `Copy` takes the new side — the unchanged and added lines, exactly the ones wearing the wash. The
//    word stands alone: the selection is showing what it is (規約 §メニュー「行が自分で示しているものは行に任せる」).
//  - `Copy removed lines` takes the old side, whole lines. Removed lines carry no wash, so there is nothing on screen
//    that could name a part of one — the same answer GitKraken gives from its own `Copy deleted line`.
//
// What it offers is decided as it opens and left alone while it stands (規約 §メニュー), like every other menu here.
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`); it draws
// nothing itself.
Item {
    id: diffRowMenu

    required property DiffModel diffModel

    /// Text on its way to the clipboard. The page owns the clipboard, the way it does for every other copy.
    signal copyRequested(string text)

    /// What this menu is standing on, decided as it opens and held while it stands.
    property bool canCopy: false
    property int removed: 0

    /// The card is on screen. A plain property — `visible` read from another file comes back
    /// stale (`RefusalBadge`) — and what the page reads to know that hover is behind a menu now (デザイン規約 §メニュー).
    readonly property bool showing: codeMenu.visible

    /// Automation: the rows themselves (app-ui.md — the same exposure `GraphPane.view` is).
    readonly property alias menu: codeMenu

    anchors.fill: parent

    /// The card was asked for, and whether it came up. `offer()`'s own answer: a card
    /// that refused to open because it had nothing to put in it is the one failure a picture of an empty overlay
    /// cannot tell from a card nobody asked for. An automation-only exposure, the same one `GraphPane.view` is
    /// (app-ui.md) — the rest of what it says is on this object already.
    signal offered(bool shown)

    /// Opens on whatever the selection holds, wherever the click landed: a selection that takes
    /// neither side has no row to offer, and an empty card is not an answer (規約 §メニュー).
    function offer() {
        diffRowMenu.canCopy = diffRowMenu.diffModel.selHasNew
        diffRowMenu.removed = diffRowMenu.diffModel.selRemoved
        diffRowMenu.offered(codeMenu.offer())
    }

    /// What each row does, named so a headless run enters where the row enters (verify-ui).
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

pragma Singleton

import QtQuick

// The keyboard's way into the right-click menus (デザイン規約 §メニュー のキーボード): the menu key, which every platform
// hands over as a key, and the request Qt raises for Windows' Shift+F10. Both are answered where the keyboard is — the
// surface holding the focus opens its menu at its own place (`at`) — never where the pointer is: xcb raises its request
// at the pointer (`QXcbKeyboard`), and Qt Quick hands a request with a place to whatever stands there
// (`QQuickDeliveryAgentPrivate::contextMenuTargets`).
//
// Qt's facts behind it (6.12): Windows' Shift+F10 is no key event — `QWindowsKeyMapper` passes it on unless a
// `Shortcut` takes Shift+F10, and `WM_CONTEXTMENU` becomes a request with no place, handed top down to the items
// standing at the focus item's origin rounded to whole pixels, its position still (0,0): the window's origin mapped
// into each (`placeless`). An item there that hears `requested` (a box) takes it first. xcb's menu key raises a request
// at the pointer and then the key event: the window turns the placed request away, and the key opens the keyboard's
// menu. A right-click's request is synthesized at the press on Linux and at the release on Windows
// (`maybeSynthesizeContextMenuEvent`); a `MouseArea`'s grab or a right-button `TapHandler` skips the press's but not
// the release's (only a menu opened inside the handler stops it), so a right-click a row opens nothing for (a worktree
// row, a list's blank) reaches the window's `ContextMenu`, placed, and is turned away. QtTest fires both through
// `Application.styleHints.contextMenuTrigger`; Windows' placeless request itself it cannot fire.
//
// **Not Shift+F10 as a `Shortcut`**: it would come as a key event, but the shortcut eats it, and a window's `Shortcut`
// is blocked while any CloseOnEscape popup stands — a menu, a card holding the keyboard (`QQuickShortcutContext`'s
// `isBlockedByPopup`).
//
// Headless reaches the key handlers and the window's handler of the request (`Main.keyMenuAsked`), no further
// (`key-menu` on the graph — the key, a placeless request, a placed one that opens nothing — and on the WIP list, the
// diff and the left menu; `tst_keymenu`, `tst_fieldmenu`). The OS's own delivery is checked on a real Windows window:
// on a graph row, a left menu row, a WIP file row, a dragged diff selection and the commit summary field, Shift+F10 and
// the menu key open the same one menu under the target (a row's left end, the selection's moving end, the caret), and
// one Esc closes it — Shift+F10 reaches the window as a placeless request, and the menu key raises no second menu. The
// menu key with the hand over a field opens under the caret too; a right-click on a field opens under the pointer,
// pushed inside at the window's right edge. A ↓ after a left menu row moves no graph selection. A real Linux (xcb)
// window is not checked yet (P5-確認事項).
QtObject {
    id: keyMenu

    /// Where the menu being asked for stands, in scene coordinates, while a keyboard request is on its way through the
    /// right-click's own door; null the rest of the time, when a menu opens under the pointer (`AppMenu.offer`). Read
    /// by `AppMenu.offer` for the one offer inside `ask`, so no page's `open*Menu` takes a place of its own.
    property var at: null

    /// Asks `door` — the right-click's own way to its menu — for a menu standing at `place` (scene coordinates) instead
    /// of under the pointer. Says what `door` answered.
    function ask(place, door) {
        keyMenu.at = place
        try {
            return door()
        } finally {
            keyMenu.at = null
        }
    }

    /// Whether Qt's request, at `scenePoint`, came with no place — the keyboard's: Windows raises its Shift+F10 with
    /// none, and the event goes on naming the window's origin (`QQuickContextMenu::event` maps it into each item it
    /// reaches). A right-click names the point it landed on and xcb's menu key the pointer's.
    function placeless(scenePoint) {
        return Math.abs(scenePoint.x) <= 1 && Math.abs(scenePoint.y) <= 1
    }

    /// A placeless request, answered where the keyboard is (`focus`, the window's active focus item): the box holding
    /// the caret through its own seat (`FieldMenuSeat.offer`), else the nearest surface up from it that answers the menu
    /// key (`menuFromKeys`). Taken from the window, not from the surface: Qt hands the request to the items standing at
    /// the focus's origin rounded to a whole pixel, which leaves out a surface whose origin has a fraction rounded down.
    /// Says whether one answered.
    function answer(focus) {
        for (let item = focus; item; item = item.parent) {
            for (let i = 0; i < item.children.length; i++) {
                const child = item.children[i]
                if (child.editor === item && typeof child.offer === "function")
                    return child.offer()
            }
            if (typeof item.menuFromKeys === "function")
                return item.menuFromKeys()
        }
        return false
    }
}

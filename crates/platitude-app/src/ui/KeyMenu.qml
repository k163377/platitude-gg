pragma Singleton

import QtQuick

// The keyboard's way into the right-click menus (デザイン規約 §メニュー のキーボード): the menu key, which every platform
// hands over as a key, and the request Qt raises for Windows' Shift+F10. Both are answered where the keyboard is — the
// surface holding the focus opens its menu at its own place (`at`) — never where the pointer is: xcb raises its request
// at the pointer (`QXcbKeyboard`), and Qt Quick hands a request with a place to whatever stands there
// (`QQuickDeliveryAgentPrivate::contextMenuTargets`).
QtObject {
    id: keyMenu

    /// Where the menu being asked for stands, in scene coordinates, while a keyboard request is on its way through the
    /// right-click's own door; null the rest of the time, when a menu opens under the pointer (`AppMenu.offer`).
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

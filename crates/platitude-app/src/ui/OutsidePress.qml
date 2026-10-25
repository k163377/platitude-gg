pragma Singleton

import QtQuick
import QtQuick.Controls.Fusion

// The menus and lists standing over this window that a press outside them puts down (`AppMenu`, `AppCombo`'s list).
// Not every press outside reaches the scene to do it — the platform takes the ones on the band's grab-runs and the
// resize edges (`WindowChrome.frameYielded`), and another window the ones on it (`Main.onActiveChanged`) — so those
// ask here. They write themselves in while they stand.
//
// Not the hover cards (`AppCard`, all kept by `HoverSeat`): the hand leaving closes them, and written in they
// would hand the edges to the scene under a row that reaches them — the row stays hovered, the card stays up, and the
// edge never resizes.
QtObject {
    id: outsidePress

    /// The popups standing now. Replaced, never changed in place, so a binding on it hears every change.
    property var standing: []
    readonly property bool any: outsidePress.standing.length > 0
    /// One of them has the hand no more (`AppMenu.pointerInside`) — walked out, or the popup gone from under it. A
    /// surface that lost the hand under one and has not heard it since weighs where it went (`HandWatch.weighCover`).
    signal handLeft()

    /// Whether a popup in this state counts: on screen, under a policy a press outside closes. Takes the two values
    /// rather than the popup, so the caller's binding follows both.
    function counts(visible, closePolicy) {
        return visible && (closePolicy & (Popup.CloseOnPressOutside | Popup.CloseOnPressOutsideParent)) !== 0
    }
    /// Writes `popup` in or out; writing it again changes nothing.
    function stand(popup, up) {
        const rest = outsidePress.standing.filter(p => p !== popup)
        if (up)
            rest.push(popup)
        if (rest.length !== outsidePress.standing.length)
            outsidePress.standing = rest
    }
    /// Closes every one, as the press outside that the scene never heard would have.
    function closeAll() {
        for (const popup of outsidePress.standing.slice())
            popup.close()
    }
    /// Whether one stands in `item`'s window — what a hover there is behind (デザイン規約 §メニュー「メニューが立って
    /// いる間、その裏の hover は引っ込む」), for what hangs off the window rather than a page (the tooltip, the band's
    /// card; a page's own cards read its menus' door, `RepoPage.menuDoor`). Reads `standing`, so a binding follows.
    function standsOver(item) {
        const window = outsidePress.windowOf(item)
        return outsidePress.standing.some(popup => outsidePress.windowOf(popup.parent) === window)
    }
    /// Whether the hand is inside one of them in `item`'s window: a surface that loses the hand then has lost it under
    /// a popup, not left (`HandWatch.hidden`). One that does not say (`pointerInside` undefined: a combo's list) counts
    /// while it stands, as Qt hands nothing under it its hover. Reads `standing` and what each says, so a binding
    /// follows both.
    function handIn(item) {
        const window = outsidePress.windowOf(item)
        return outsidePress.standing.some(popup => outsidePress.windowOf(popup.parent) === window
                                                   && popup.pointerInside !== false)
    }
    /// Whether `item` is inside one of them — a menu's own row, which is not behind the menu.
    function holds(item) {
        for (const popup of outsidePress.standing) {
            for (let at = item; at !== null && at !== undefined; at = at.parent) {
                if (at === popup.contentItem)
                    return true
            }
        }
        return false
    }
    function windowOf(item) {
        return item !== null && item !== undefined && item.Window !== undefined ? item.Window.window : null
    }
}

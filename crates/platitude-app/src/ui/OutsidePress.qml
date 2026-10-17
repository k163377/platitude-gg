pragma Singleton

import QtQuick
import QtQuick.Controls.Fusion

// The menus and lists standing over this window that a press outside them puts down (`AppMenu`, `AppCombo`'s list).
// Not every press outside reaches the scene to do it — the platform takes the ones on the band's grab-runs and the
// resize edges (`WindowChrome.frameYielded`), and another window the ones on it (`Main.onActiveChanged`) — so those
// ask here. They write themselves in while they stand.
//
// Not the hover cards (`AppCard`, all opened by `HoverCardHost`): the hand leaving closes them, and written in they
// would hand the edges to the scene under a row that reaches them — the row stays hovered, the card stays up, and the
// edge never resizes.
QtObject {
    id: outsidePress

    /// The popups standing now. Replaced, never changed in place, so a binding on it hears every change.
    property var standing: []
    readonly property bool any: outsidePress.standing.length > 0

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
}

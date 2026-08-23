import QtQuick
import platitude
import platitude.ui

// The identity dialog and the state that opens it: on startup when git has no name and email to put on a commit, and
// on demand from the app menu or the toolbar badge. An Item that fills the window — the dialog is a popup, and a
// popup measures the window through the item it was declared in (rules-refs/structure.md 描かないホスト).
//
// The window keeps one forward, `dismissIdentity` — the harness calls it there (`WindowAutoActDriver`).
Item {
    id: identityGate

    /// The dialog itself, for the window's harness (`WindowAutoActDriver.identityDialog`).
    readonly property alias dialog: identityDialog

    property bool identityDismissed: false
    property bool identityEditing: false
    // A half-landed save leaves an identity that *is* set, so `missing` alone takes the screen away at the one moment
    // it has something to say (measured: the state flipped to `ready` on the name that did land, and this window
    // closed the dialog out from under the answer). What holds it open is `identityUnsaved` — the flag the marks read.
    readonly property bool identityWanted: AppBackend.gitState === "ok"
                                           && (identityEditing
                                               || ((AppBackend.identityState === "missing"
                                                    || AppBackend.identityUnsaved)
                                                   && !identityDismissed))
    function dismissIdentity() {
        identityEditing = false
        identityDismissed = true
    }
    // Screenshot hook: PG_AUTO_IDENTITY="edit" opens the dialog on an identity that is already set, which is otherwise
    // a menu action.
    Connections {
        target: AppBackend
        function onIdentityChanged() {
            if (AppBackend.autoIdentity === "edit" && AppBackend.identityState === "ready"
                    && !identityGate.identityDismissed)
                identityGate.identityEditing = true
        }
    }

    // Opened and closed from the state above rather than by binding `visible`: Escape closes a popup imperatively,
    // which would overwrite such a binding and leave the menu entry unable to open it again. Every close answers the
    // state, so the two stay in step.
    IdentityDialog {
        id: identityDialog
        editing: identityGate.identityEditing
        onDismissed: identityGate.dismissIdentity()
        Connections {
            target: identityGate
            function onIdentityWantedChanged() {
                if (identityGate.identityWanted)
                    identityDialog.open()
                else
                    identityDialog.close()
            }
        }
    }
}

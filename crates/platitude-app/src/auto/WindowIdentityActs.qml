pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The two screenshot hooks that hang off `PG_AUTO_IDENTITY`, and nothing else.
///
/// A file of its own because these are the only harness hooks that are **not** verbs: they run with no
/// `PG_AUTO_ACT` at all — a picture of the identity question is a picture of a window nobody has asked to do
/// anything — so they are built off their own knob rather than beside the verbs (`WindowHarness`).
// An `Item` only because that is what the harness's children are; it draws nothing and is never given a size.
Item {
    id: hooks

    required property var identityGate
    required property var identityDialog

    // `"<name>|<email>"` fills the two boxes, `PG_AUTO_IDENTITY_SAVE=1` submits them, `skip` answers "Not now" to
    // show the state behind the dialog, and `edit` leaves an identity that is already set as it is.
    //
    // Deferred by a turn: the dialog puts the caret in the first box as it opens, and filling it in the same turn
    // would be writing into a box that is still being focused.
    Connections {
        target: hooks.identityDialog
        function onOpened() {
            Qt.callLater(hooks.applyIdentity)
        }
    }
    function applyIdentity() {
        if (AppBackend.autoIdentity === "skip") {
            hooks.identityDialog.close()
            return
        }
        if (AppBackend.autoIdentity === "edit")
            return
        const parts = AppBackend.autoIdentity.split("|")
        hooks.identityDialog.fill(parts[0], parts.length > 1 ? parts[1] : "")
        if (AppBackend.autoIdentitySave)
            hooks.identityDialog.submit()
    }

    // `edit` opens the settings screen on an identity that is already set, which is otherwise a menu action. It lands
    // where the menu entry lands, and it fires once — a menu entry is pressed once, and every later answer git gives
    // about the identity is not a second press.
    property bool settingsAsked: false
    Connections {
        target: AppBackend
        enabled: AppBackend.autoIdentity === "edit" && !hooks.settingsAsked
        function onIdentityChanged() {
            if (AppBackend.identityState !== "ready")
                return
            hooks.settingsAsked = true
            hooks.identityGate.askSettingsAtGit()
        }
    }
}

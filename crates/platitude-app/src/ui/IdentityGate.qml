import QtQuick
import platitude
import platitude.ui

// The identity gate and the state that opens it: on startup, when git has no name and email to put on a commit. An
// Item that fills the window — the dialog is a popup, and a popup measures the window through the item it was
// declared in (rules-refs/structure.md 描かないホスト).
//
// Nothing else opens it. An identity that is already set is edited in the settings screen's git chapter, which the
// app menu and the toolbar badge both lead to (`Main`); this one stands only where the reader was stopped.
//
// The window keeps one forward, `dismissIdentity` — the harness calls it there (`WindowAutoActDriver`).
Item {
    id: identityGate

    /// The dialog itself, for the window's harness (`WindowAutoActDriver.identityDialog`).
    readonly property alias dialog: identityDialog

    /// The settings screen is standing. The identity is edited there too, and its save can half-land the same way —
    /// so without this the gate would raise itself over the screen the reader is making that very write in, asking
    /// for what is already on their screen. It stands once the screen is out of the way, which is where the reader
    /// left it half-landed.
    property bool settingsOpen: false

    property bool identityDismissed: false
    // A half-landed save leaves an identity that *is* set, so `missing` alone takes the screen away at the one moment
    // it has something to say (measured: the state flipped to `ready` on the name that did land, and this window
    // closed the dialog out from under the answer). What holds it open is `identityUnsaved` — the flag the marks read.
    readonly property bool identityWanted: AppBackend.gitState === "ok"
                                           && (AppBackend.identityState === "missing"
                                               || AppBackend.identityUnsaved)
                                           && !identityDismissed && !settingsOpen

    /// The settings screen, asked for at its git chapter. Where the gate itself never opens it, the screenshot hook
    /// at the foot of this file does, and the road there belongs to the window
    /// (`Main` → `WindowDialogSeat.openSettingsAt`).
    signal settingsAtGitRequested()

    function dismissIdentity() {
        identityDismissed = true
    }

    // Opened and closed from the state above rather than by binding `visible`: Escape closes a popup imperatively,
    // which would overwrite such a binding and leave the state unable to open it again. Every close answers the
    // state, so the two stay in step.
    IdentityDialog {
        id: identityDialog
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

    // Screenshot hook: PG_AUTO_IDENTITY="edit" opens the settings screen on an identity that is already set, which is
    // otherwise a menu action. It lands where the menu entry lands, and it fires once — a menu entry is pressed once,
    // and every later answer git gives about the identity is not a second press.
    property bool identityEditShown: false
    Connections {
        target: AppBackend
        enabled: AppBackend.autoIdentity === "edit" && !identityGate.identityEditShown
        function onIdentityChanged() {
            if (AppBackend.identityState !== "ready")
                return
            identityGate.identityEditShown = true
            identityGate.settingsAtGitRequested()
        }
    }
}

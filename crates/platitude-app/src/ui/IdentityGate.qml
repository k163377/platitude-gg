import QtQuick
import platitude
import platitude.ui

// The identity gate and the state that opens it — only on startup, when git has no identity to commit with; one
// already set is edited in the settings screen. An Item filling the window, since the dialog measures the window
// through it (rules-refs/structure.md「描かないホスト」). The window forwards `dismissIdentity` for the harness
// (`WindowDialogActs`).
Item {
    id: identityGate

    /// For the window's harness (`WindowAutoActDriver.identityDialog`).
    readonly property alias dialog: identityDialog

    /// The settings screen is up. A half-landed save there would otherwise raise the gate over the screen making that
    /// write; the gate stands once the screen closes.
    property bool settingsOpen: false

    property bool identityDismissed: false
    // `identityUnsaved` too: a half-landed save leaves an identity that is set, and `missing` alone would close the
    // gate at the one moment it has something to say.
    readonly property bool identityWanted: AppBackend.gitState === "ok"
                                           && (AppBackend.identityState === "missing"
                                               || AppBackend.identityUnsaved)
                                           && !identityDismissed && !settingsOpen

    /// The settings screen at its git chapter; the window opens it (`WindowDialogSeat.openSettingsAt`).
    signal settingsAtGitRequested()

    function dismissIdentity() {
        identityDismissed = true
    }

    // Opened and closed by hand, not a `visible` binding: Escape's close would break the binding and the state could
    // not reopen it. Every close answers the state (`dismissed`).
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

    /// Automation-only: the gate itself never asks for the settings screen.
    function askSettingsAtGit() {
        identityGate.settingsAtGitRequested()
    }
}

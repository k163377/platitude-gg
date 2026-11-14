import QtQuick
import platitude
import platitude.ui

// The identity gate and the state that opens it — only on startup, when git has no identity to commit with; one
// already set is edited in the settings screen. An Item filling the window, since the dialog measures the window
// through it (rules-refs/structure.md「描かないホスト」). The window forwards `dismissIdentity` for the harness
// (`WindowDialogActs`).
Item {
    id: identityGate

    /// For the window's harness (`WindowAutoActDriver.identityDialog`): null until the gate first stands, or up front
    /// with [`keepBuilt`].
    readonly property var dialog: dialogSeat.item
    /// The dialog built up front, for a caller that reads it before it opens (the harness, `WindowHarness`). False in
    /// every window a person opens: most never ask, so the card is built on the first want
    /// (rules-refs/app-ui.md「窓が黙って実体化するダイアログは Loader の後ろに置く」).
    property bool keepBuilt: false

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
    // not reopen it. Every close answers the state (`dismissed`). The want builds the seat, then opens in the same
    // turn: the build is synchronous. The seat fills the gate, which the dialog sizes off.
    Loader {
        id: dialogSeat
        anchors.fill: parent
        active: identityGate.keepBuilt
        sourceComponent: IdentityDialog {
            onDismissed: identityGate.dismissIdentity()
        }
    }
    Connections {
        target: identityGate
        function onIdentityWantedChanged() {
            if (identityGate.identityWanted) {
                dialogSeat.active = true
                dialogSeat.item.open()
            } else if (dialogSeat.item !== null) {
                dialogSeat.item.close()
            }
        }
    }

    /// Automation-only: the gate itself never asks for the settings screen.
    function askSettingsAtGit() {
        identityGate.settingsAtGitRequested()
    }
}

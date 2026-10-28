pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The two screenshot hooks that hang off `PGG_AUTO_IDENTITY`. A file of its own because they are the only harness
/// hooks with no `PGG_AUTO_ACT`, so they are built off their own knob (`WindowHarness`).
// An `Item` only because that is what the harness's children are; it is sizeless.
Item {
    id: hooks

    required property var identityGate
    required property var identityDialog
    /// The tab in front (`WindowHarness.page`).
    required property var page

    // `"<name>|<email>"` fills the two boxes, `PGG_AUTO_IDENTITY_SAVE=1` submits them, `skip` answers "Not now" to
    // show the state behind the dialog, and `edit` leaves an identity that is already set as it is.
    //
    // Deferred a turn: the dialog focuses the first box as it opens, and a fill in that turn lands mid-focus.
    Connections {
        target: hooks.identityDialog
        function onOpened() {
            Qt.callLater(hooks.applyIdentity)
        }
    }
    function applyIdentity() {
        if (Harness.autoIdentity === "skip") {
            hooks.identityDialog.close()
            return
        }
        if (Harness.autoIdentity === "edit")
            return
        const parts = Harness.autoIdentity.split("|")
        hooks.identityDialog.fill(parts[0], parts.length > 1 ? parts[1] : "")
        if (Harness.autoIdentitySave)
            submitTimer.start()
    }
    // The save rewrites the global configuration, and on Windows a git started while git replaces it cannot read it —
    // the opening tab's reads among them, which the product asks once more (gitの扱い.md §設定の書き換えと重なった読み).
    // A hand types for seconds first; the run submits once the tab it opened has settled (`PageSettled`), as a hand
    // would, and says so. Never completes: the hooks run beside a verb, or with none, and the save is an input the
    // verb waits on, not a run's ending.
    WindowStage {
        id: submitTimer
        name: "identity_save"
        order: ["submitted"]
        onTriggered: {
            const opened = Harness.autoOpen === ""
                || hooks.page !== null && hooks.page.pageTab.state !== "" && PageSettled.settled(hooks.page)
            if (!Awaited.all("identity_save", { "page": opened }))
                return
            submitTimer.input("submitted", () => hooks.identityDialog.submit())
            submitTimer.stop()
        }
    }

    // `edit` opens the settings screen where the menu entry lands, once: a later identity answer is not a second press.
    property bool settingsAsked: false
    Connections {
        target: AppBackend
        enabled: Harness.autoIdentity === "edit" && !hooks.settingsAsked
        function onIdentityChanged() {
            if (AppBackend.identityState !== "ready")
                return
            hooks.settingsAsked = true
            hooks.identityGate.askSettingsAtGit()
        }
    }
}

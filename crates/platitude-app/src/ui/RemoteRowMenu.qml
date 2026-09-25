import QtQuick
import platitude.ui

// What a remote itself offers, from its REMOTES row (デザイン規約 §左メニューの所作) — configuration, so a menu of
// its own. No chips: the mark row runs `git config`, a word only the command log spells
// (デザイン規約 §git 用語のコード表記).
Item {
    id: remoteRowMenu

    required property var repoTab

    /// Why both rows are out while the write doors are held (`RepoPage.doorsHeldWhy`): configuration, but still a
    /// write (`AppMenu.heldReason`).
    property string heldReason: ""

    /// The remote the menu stands on.
    property string remote: ""

    readonly property alias menu: menu
    readonly property alias opened: menu.opened
    /// Whether the menu is up, for the pane that holds still under it.
    readonly property bool showing: menu.opened

    /// The form for this remote's URL — the same dialog `Add remote…` opens, with the half that is already known
    /// filled in (デザイン規約 §リモートを書き留める).
    signal urlRequested(string name)
    signal dismissed()

    /// The one door in. Says whether it opened.
    function offerOn(name) {
        remoteRowMenu.remote = name
        return menu.offer()
    }

    AppMenu {
        id: menu
        heldReason: remoteRowMenu.heldReason
        onClosed: remoteRowMenu.dismissed()

        AppMenuItem {
            //: Opens the form that holds this remote's URL. Says the same words as the form's own heading.
            text: qsTr("Where %1 is…").arg(remoteRowMenu.remote)
            onTriggered: remoteRowMenu.urlRequested(remoteRowMenu.remote)
        }
        // No divider: both rows are one group. Gone on the remote both keys already name; a remote only one key
        // names keeps it, to finish the mark (デザイン規約 §リモートを書き留める).
        AppMenuItem {
            id: markItem
            text: qsTr("Mark as origin (default remote)")
            offered: remoteRowMenu.remote !== "" && remoteRowMenu.remote !== remoteRowMenu.repoTab.markedOrigin
            blockedReason: remoteRowMenu.repoTab.busyCount === 0 ? "" : Words.otherCommandRunning
            onTriggered: remoteRowMenu.repoTab.markOrigin(remoteRowMenu.remote)
        }
    }
}

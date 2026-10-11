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
    /// What the rows say while they wait for a git command this window started (`RepoPage.runningWhy`): a menu opened
    /// under one greys them until it ends (`AppMenu.runningWhy`).
    property string runningWhy: ""

    /// The remote the menu stands on.
    property string remote: ""
    /// Whether the mark row is on offer: decided as the menu opens and held while it stands (デザイン規約 §メニュー).
    property bool canMark: false

    readonly property alias menu: menu
    readonly property alias opened: menu.opened
    /// Whether the menu is up, for the pane that holds still under it.
    readonly property bool showing: menu.opened

    /// The form for this remote's URL — the same dialog `Add remote…` opens, with the half that is already known
    /// filled in (デザイン規約 §リモートを書き留める).
    signal urlRequested(string name)

    /// The one door in. Says whether it opened.
    function offerOn(name) {
        menu.releaseShape()
        remoteRowMenu.remote = name
        remoteRowMenu.standOn()
        return menu.offer()
    }
    function standOn() {
        remoteRowMenu.canMark = remoteRowMenu.remote !== ""
                                && remoteRowMenu.remote !== remoteRowMenu.repoTab.markedOrigin
    }
    /// The command the menu waited for has ended with the menu up: the rows stand on what it left, the menu keeping
    /// its shape (`AppMenu.holdShape`). A remote the command took away leaves the menu nothing to act on.
    function standAgain() {
        if (!remoteRowMenu.showing)
            return
        if (remoteRowMenu.repoTab.remoteNames.indexOf(remoteRowMenu.remote) < 0) {
            menu.dismiss()
            return
        }
        menu.holdShape()
        remoteRowMenu.standOn()
    }

    AppMenu {
        id: menu
        heldReason: remoteRowMenu.heldReason
        runningWhy: remoteRowMenu.runningWhy
        onWaitEnded: remoteRowMenu.standAgain()

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
            offered: remoteRowMenu.canMark
            onTriggered: remoteRowMenu.repoTab.markOrigin(remoteRowMenu.remote)
        }
    }
}

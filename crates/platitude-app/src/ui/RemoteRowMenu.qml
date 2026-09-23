import QtQuick
import platitude.ui

// What a remote itself offers, raised from the row its branches hang under (デザイン規約 §左メニューの所作). A remote is
// repository configuration, so this menu is its own from the ground up: the two have nothing in
// common but the gesture that opens them.
//
// Both rows stand bare. One opens a form, and the other runs `git config` on `remote.pushDefault` and
// `checkout.defaultRemote` — `config` is a word only the command log spells (デザイン規約 §git 用語のコード表記).
Item {
    id: remoteRowMenu

    required property var repoTab

    /// Why both rows are out while the window's write doors are held (`RepoPage.doorsHeldWhy`). A remote is
    /// configuration rather than history, but writing it down mid-rewrite is still a write, and the reader who reached
    /// for the row is the one who needs the line (`AppMenu.heldReason`).
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
        // The two rows run on unbroken. **A divider is a claim that a group ends here** (`AppMenuSeparator`), and both
        // rows are the same group — what to do with this remote. That one opens a form and the other runs at once is
        // what the `…` on the first row already says.
        //
        // Gone on the remote both keys already name: git keeps one value for each, so there is nothing for this row to
        // do there (デザイン規約 §メニュー — 選べない行は消す). A remote only one of them names keeps the row — a rename in a
        // terminal carries the push's key along and leaves the other behind, and this row is how the mark is finished.
        // Clearing the mark is the box in the form, and moving it is this row on another remote.
        AppMenuItem {
            id: markItem
            text: qsTr("Mark as default remote (origin)")
            offered: remoteRowMenu.remote !== "" && remoteRowMenu.remote !== remoteRowMenu.repoTab.markedOrigin
            blockedReason: remoteRowMenu.repoTab.busyCount === 0 ? "" : Words.otherCommandRunning
            onTriggered: remoteRowMenu.repoTab.markOrigin(remoteRowMenu.remote)
        }
    }
}

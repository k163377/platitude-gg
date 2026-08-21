import QtQuick
import platitude.ui

// What a remote itself offers, raised from the row its branches hang under (デザイン規約 §左メニューの所作). A remote is
// repository configuration rather than a ref, so this is not the ref menu with rows removed: the two have nothing in
// common but the gesture that opens them.
//
// Neither row wears a code chip. One opens a form, and the other runs `git config remote.pushDefault <name>` — `config`
// is not a word this UI spells anywhere, and the command log is where its spelling shows up (デザイン規約 §git 用語のコード表記).
Item {
    id: remoteRowMenu

    required property var repoTab

    /// The remote the menu stands on.
    property string remote: ""

    readonly property alias menu: menu
    readonly property alias opened: menu.opened
    /// Whether the menu is up, for the pane that must not hover under it.
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
    function close() {
        menu.close()
    }

    AppMenu {
        id: menu
        onClosed: remoteRowMenu.dismissed()

        AppMenuItem {
            //: Opens the form that holds this remote's URL. Says the same words as the form's own heading.
            text: qsTr("Where %1 is…").arg(remoteRowMenu.remote)
            onTriggered: remoteRowMenu.urlRequested(remoteRowMenu.remote)
        }
        // No rule between the two. **A divider is a claim that a group ends here** (`AppMenuSeparator`), and both rows
        // are the same group — what to do with this remote. That one opens a form and the other runs at once is not a
        // second group; it is what the `…` on the first row already says (2026-08-21 ユーザー判断).
        //
        // Gone rather than greyed on the remote that already holds the mark: git keeps one value, so there is nothing
        // for this row to do there (デザイン規約 §メニュー — 選べない行は消す). Clearing the mark is the box in the form, and moving
        // it is this row on another remote.
        AppMenuItem {
            id: markItem
            text: qsTr("Mark as default remote (origin)")
            offered: remoteRowMenu.remote !== "" && remoteRowMenu.remote !== remoteRowMenu.repoTab.pushDefault
            blockedReason: remoteRowMenu.repoTab.busyCount === 0
                           ? "" : qsTr("Wait for the command that is running")
            onTriggered: remoteRowMenu.repoTab.setPushDefault(remoteRowMenu.remote)
        }
    }
}

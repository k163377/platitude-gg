import QtQuick
import platitude
import platitude.ui

// What a remote itself offers, from its REMOTES row — configuration, so a menu of its own. No chips: the mark row
// runs `git config`, a word only the command log spells (デザイン規約 §git 用語のコード表記). Which rows there are and
// whether each can be pressed is the type's answer (`GitFacts.remoteMenu` — core's `menu::On::Remote`).
//
// The origin mark (デザイン規約 §リモートを書き留める) has two doors with one sentence and one slot: this menu's row
// and `RemoteDialog`'s box, both `RepoTab.markOrigin` (`remote.pushDefault` and `checkout.defaultRemote` together; an
// empty name clears both). The row's badge reads `pushDefault`, its hover the same `markedOrigin` as the row here and
// the box (`NavItemDelegate.pushesHere` / `holdsOrigin`). Headless: `push-default` / `remote-menu` / `remote-url`.
Item {
    id: remoteRowMenu

    required property var repoTab
    /// What the GITHUB card at the foot stands on (`GitHubLinks.ofRemote`).
    required property var githubLinks

    /// Why both rows are out while the write doors are held (`RepoPage.doorsHeldWhy`): configuration, but still a
    /// write (`AppMenu.heldReason`).
    property string heldReason: ""
    /// What the rows say while they wait for a git command this window started (`RepoPage.runningWhy`): a menu opened
    /// under one greys them until it ends (`AppMenu.runningWhy`).
    property string runningWhy: ""

    /// The remote the menu stands on.
    property string remote: ""
    /// The rows and why each is greyed, read as the menu opens and as it stands again (デザイン規約 §メニュー), drawn
    /// into the menu — and when it stands again: on what the rows read (`standsOn`), outside too (`follow`;
    /// デザイン規約 §メニュー「今の内容」). A remote taken away leaves the menu nothing to act on.
    readonly property MenuLines lines: MenuLines {
        menu: remoteRowMenu.menu
        moved: remoteRowMenu.standsOn
        waiting: remoteRowMenu.treeAtAsk !== null
        onStandAgain: {
            if (remoteRowMenu.repoTab.remoteNames.indexOf(remoteRowMenu.remote) < 0)
                remoteRowMenu.menu.dismiss()
            else
                remoteRowMenu.standOn()
        }
    }

    readonly property alias menu: menu
    readonly property alias githubCard: githubMenu
    readonly property alias opened: menu.opened
    /// Whether the menu is up, for the pane that holds still under it.
    readonly property bool showing: menu.opened
    /// What the rows read off the tab, which the menu standing follows (`MenuLines.moved`).
    readonly property string standsOn: remoteRowMenu.repoTab.markedOrigin + "\n"
                                       + remoteRowMenu.repoTab.remoteNames.join("\n")
    /// The worktree as the command the menus wait for was asked (`RepoPage.treeAtAsk`); null while nothing is out.
    /// While it stands, the rows keep what they stood on, as every opener's (`MenuLines.waiting`); back to null, they
    /// stand on what the command left.
    property var treeAtAsk: null

    /// The form for this remote's URL — the same dialog `Add remote…` opens, with the half that is already known
    /// filled in (デザイン規約 §リモートを書き留める).
    signal urlRequested(string name)
    /// A link on its way to the clipboard (`GitHubMenu`); the page owns the clipboard.
    signal copyRequested(string text)

    /// The one door in. Says whether it opened.
    function offerOn(name) {
        remoteRowMenu.lines.release()
        remoteRowMenu.remote = name
        remoteRowMenu.standOn()
        // Only as the menu opens, as every GITHUB card (`CommitRowMenu.standGithub`).
        githubMenu.standOn(remoteRowMenu.githubLinks.ofRemote(name))
        return menu.offer()
    }
    function standOn() {
        remoteRowMenu.lines.answer = GitFacts.remoteMenu(remoteRowMenu.repoTab.state === "open", remoteRowMenu.remote,
                                                         remoteRowMenu.repoTab.markedOrigin)
    }

    AppMenu {
        id: menu
        heldReason: remoteRowMenu.heldReason
        runningWhy: remoteRowMenu.runningWhy

        AppMenuItem {
            //: Opens the form that holds this remote's URL. Says the same words as the form's own heading.
            refSentence: qsTr("Where %1 is…")
            refName: remoteRowMenu.remote
            refTint: Theme.textPrimary
            refMark: "remote"
            word: "remote-url"
            onTriggered: remoteRowMenu.urlRequested(remoteRowMenu.remote)
        }
        AppMenuItem {
            id: markItem
            text: qsTr("Mark as origin (default remote)")
            word: "mark-origin"
            onTriggered: remoteRowMenu.repoTab.markOrigin(remoteRowMenu.remote)
        }
        AppMenuSeparator {}
        // The remote's repository on GitHub, last: it leaves the window (デザイン規約 §GitHub のページへのリンク).
        GitHubMenu {
            id: githubMenu
            onCopyRequested: text => remoteRowMenu.copyRequested(text)
        }
    }
}

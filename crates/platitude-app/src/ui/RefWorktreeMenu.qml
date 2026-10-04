import QtQuick
import platitude
import platitude.ui

// Everything a working copy answers for, behind the copy's mark: making one (デザイン規約 §作業コピーを作る) and taking
// it away (デザイン規約 §左メニューの所作 の削除の表). One card, carried like `RefBranchMenu` by both menus — the sidebar's
// ref menu and the graph row's — and set above the TAG card (デザイン規約 §メニュー の入れ子). Like the TAG card it stands
// on every row with a commit: `Create worktree here…` is a row about the commit, so the card is there with that one row
// where the row names nothing else.
//
// It reads no model and runs no write: the carrying menu hands it what it read — the copy the row names
// (`NavSectionModel.copyFacts`) and where a new one would go (`NavSectionModel.newCopyFor`) — and runs the requests.
AppMenu {
    id: copyMenu

    /// The copy the card stands on, frozen by `standOn`: its path (what git removes) and its folder (what the row says).
    readonly property alias path: state.path
    readonly property alias name: state.name
    /// Automation's handles into the rows, passed on through the carrying menus. Not `removeItem`, which is `Menu`'s
    /// own way to take a row off.
    readonly property alias copyHereItem: refCopyHereItem
    readonly property alias copyAddItem: refCopyAddItem
    readonly property alias removeCopyItem: refCopyRemoveItem

    /// A new branch on the row's commit, out in a working copy of its own: the page opens the name box where the menu
    /// was raised (`RepoPage.startCopyAt`).
    signal copyHereRequested(string oidHex)
    /// The row's own branch out in a new copy at `path`: `mode` is `branch` / `track`, `start` the remote branch a
    /// tracking one is made off, `name` the folder.
    signal copyAddRequested(string mode, string branch, string start, string path, string name)
    signal removeRequested(string path, string name)

    QtObject {
        id: state
        property string path: ""
        property string name: ""
        property bool canRemove: false
        /// Why the row is out — `locked` / `here` / `busy`, or empty (`offers::worktree_card`).
        property string out: ""
        /// What `git worktree lock --reason` was given, empty for none.
        property string lockReason: ""
        /// The making side (`making` in `standOn`).
        property string oid: ""
        property bool canHere: false
        property string checkout: ""
        property string branch: ""
        property string start: ""
        property string addPath: ""
        property string place: ""
        property string taken: ""
    }

    /// Stands the card on a row. `facts` is the copy the row names (`NavSectionModel.copyFacts`) — undefined for none,
    /// which takes `worktree remove` off the card; `here` is whether this tab stands in it, `busy` the count the offers
    /// are asked with. `making` is what the carrying menu worked out for the two rows that make a copy
    /// (`offers::copy_rows`, `CommitMenuState.askCopyRows` / `RefRowMenu.askCopyRows`): `oid` (the row's commit),
    /// `here` (`Create worktree here…` has a row), `checkout` (`branch` / `track` / empty for no `worktree add`),
    /// `branch` / `start` (what that row takes out, and the remote branch a tracking one is made off), and where it
    /// goes — `path`, `place` (its folder) and `taken` (what is in the way there).
    function standOn(facts, here, busy, making) {
        const offers = facts ? GitFacts.worktreeCardOffers(facts.change, here, busy) : []
        state.path = facts ? facts.full : ""
        state.name = facts ? facts.name : ""
        state.lockReason = offers.includes("out-locked") ? facts.reason : ""
        state.canRemove = offers.includes("remove")
        state.out = offers.includes("out-locked") ? "locked"
                  : offers.includes("out-here") ? "here"
                  : offers.includes("out-busy") ? "busy" : ""
        state.oid = making.oid
        state.canHere = making.here
        state.checkout = making.checkout
        state.branch = making.branch
        state.start = making.start
        state.addPath = making.path
        state.place = making.place
        state.taken = making.taken
    }

    // `dismiss()` folds every level (rules-refs/app-ui.md「メニューを閉じるのは自分」).

    titleKind: "tree"
    titleTint: Theme.success
    title: qsTr("WORKTREE")
    // Off a row with nothing to make and nothing to take away — a stash, or no place for a new copy beside a copy git
    // never removes.
    applies: state.canHere || state.checkout !== "" || state.canRemove

    // Words, not a command: it opens a box, as `Create branch here…` and `Create tag here…` do.
    AppMenuItem {
        id: refCopyHereItem
        text: Words.createWorktreeHere
        offered: state.canHere
        onTriggered: copyMenu.copyHereRequested(state.oid)
    }
    // Names the folder it makes, as `worktree remove` names the one it takes (デザイン規約 §ref の種別「名前の印」);
    // greyed with the reason where something is in the way there, warned of before the press.
    AppMenuItem {
        id: refCopyAddItem
        code: "worktree add"
        nameMark: "tree"
        markName: state.place
        growsForText: false
        offered: state.checkout !== ""
        blockedReason: Words.copyPlaceTaken(state.taken, state.place)
        // The folder at the end of that line wears the tree mark, as it does on the row (デザイン規約 §ref の種別).
        reasonMarkWord: refCopyAddItem.blockedReason !== "" ? state.place : ""
        onTriggered: copyMenu.copyAddRequested(state.checkout, state.branch, state.start, state.addPath, state.place)
    }
    AppMenuSeparator {}
    // A click, as `branch --delete`: a local write, and git keeps a copy holding uncommitted work — that refusal comes
    // down as a report over the graph (デザイン規約 §左メニューの所作 の削除の表).
    AppMenuItem {
        id: refCopyRemoveItem
        code: "worktree remove"
        // The name, readable where the row is (デザイン規約 §メニュー「例外は削除行だけ」), behind the tree mark every
        // working copy's name wears (デザイン規約 §ref の種別「名前の印」).
        nameMark: "tree"
        markName: state.name
        growsForText: false
        offered: state.canRemove
        // Greyed, not gone: the reader asked why it cannot go, and this is where it says (デザイン規約 §メニュー の例外).
        blockedReason: state.out === "locked"
                       ? (state.lockReason === "" ? qsTr("This working copy is locked")
                                                  : qsTr("Locked — %1").arg(state.lockReason))
                       : state.out === "here" ? qsTr("This tab is showing this working copy")
                       : state.out === "busy" ? Words.otherCommandRunning : ""
        onTriggered: copyMenu.removeRequested(state.path, state.name)
    }
}

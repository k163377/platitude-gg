import QtQuick
import platitude
import platitude.ui

// Everything a working copy answers for, behind the copy's mark: taking it away (デザイン規約 §左メニューの所作 の削除の表).
// One card, carried like `RefBranchMenu` by both menus — the sidebar's ref menu (a WORKTREES row, or a branch another
// copy holds) and the graph row's (a copy's chip) — and set above the TAG card (デザイン規約 §メニュー の入れ子).
//
// It reads no model and runs no write: the carrying menu hands it the copy's row (`NavSectionModel.copyFacts`) and runs
// the request.
AppMenu {
    id: copyMenu

    /// The copy the card stands on, frozen by `standOn`: its path (what git removes) and its folder (what the row says).
    readonly property alias path: state.path
    readonly property alias name: state.name
    /// Automation's handle into the row, passed on through the carrying menus. Not `removeItem`, which is `Menu`'s own
    /// way to take a row off.
    readonly property alias removeCopyItem: refCopyRemoveItem

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
    }

    /// Stands the card on a copy. `facts` is its row (`NavSectionModel.copyFacts`) — undefined for none, which takes
    /// the card off the menu; `here` is whether this tab stands in it, `busy` the count the offers are asked with.
    function standOn(facts, here, busy) {
        const offers = facts ? GitFacts.worktreeCardOffers(facts.change, here, busy) : []
        state.path = facts ? facts.full : ""
        state.name = facts ? facts.name : ""
        state.lockReason = offers.includes("out-locked") ? facts.reason : ""
        state.canRemove = offers.includes("remove")
        state.out = offers.includes("out-locked") ? "locked"
                  : offers.includes("out-here") ? "here"
                  : offers.includes("out-busy") ? "busy" : ""
    }

    // `dismiss()` folds every level (rules-refs/app-ui.md「メニューを閉じるのは自分」).

    titleKind: "tree"
    titleTint: Theme.success
    title: qsTr("WORKTREE")
    // The repository's own copy has nothing here: git never removes it.
    applies: state.canRemove

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

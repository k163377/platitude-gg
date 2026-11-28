import QtQuick
import platitude
import platitude.ui
import platitude.ui.words

// Everything a worktree answers for, behind the worktree's mark: making one (デザイン規約 §worktree を作る — the two
// making rows live in this nested card, a user decision: ask before changing) and taking it away (デザイン規約
// §左メニューの所作 の削除の表). One card, carried like `RefBranchMenu` by both menus — the sidebar's ref menu and the
// graph row's (デザイン規約 §メニュー の入れ子).
//
// Making and removing share this card though the tab moves to a worktree it makes, and the card is nested, not on the
// first level: `worktree remove` is used less often than a branch delete (user decision — ask before changing). The two
// making rows repeat the first level's `Create branch here…` → `switch` pair in a new worktree: make one and go, then
// go to one that is there. Which rows there are, and their seats, is the type's (`menu::Card::Worktree`, drawn by
// `MenuLines`).
//
// Its doors — the WORKTREES rows, a commit row with the chip of a worktree with no branch, the rows and chips of a
// branch another worktree holds — all reach this one card through `RefRowMenu` and `CommitRowMenu`. A new door carries
// this card too: rows written per door would come to disagree.
//
// It reads no model and runs no write: the carrying menu hands it what it read off the row's lead
// (`NavSectionModel.leadOf` — the worktree the row names, `card`, and where a new one would go, `place`) — and runs the
// requests.
AppMenu {
    id: worktreeMenu

    /// The worktree the card stands on, set by `standOn`: its path (what git removes), its folder (what the row
    /// says) and the branch it has out (empty for none) — the folder and the branch are what it marks on the graph and
    /// the branches list, which go with the press.
    readonly property alias path: state.path
    readonly property alias name: state.name
    readonly property alias holds: state.holds
    /// Automation's handles into the rows, passed on through the carrying menus. Not `removeItem`, which is `Menu`'s
    /// own way to take a row off.
    readonly property alias worktreeHereItem: refWorktreeHereItem
    readonly property alias worktreeAddItem: refWorktreeAddItem
    readonly property alias removeWorktreeItem: refWorktreeRemoveItem

    /// A new branch on the row's commit, out in a worktree of its own: the page opens the name box where the menu
    /// was raised (`RepoPage.startWorktreeAt`).
    signal worktreeHereRequested(string oidHex)
    /// The row's own branch out in a new worktree at `path`: `mode` is `branch` / `track`, `start` the remote branch a
    /// tracking one is made off, `name` the folder.
    signal worktreeAddRequested(string mode, string branch, string start, string path, string name)
    signal removeRequested(string path, string name, string holds)

    readonly property QtObject state: QtObject {
        id: state
        property string path: ""
        property string name: ""
        property string holds: ""
        /// What `git worktree lock --reason` was given, empty for none.
        property string lockReason: ""
        /// The making side (`standOn`): the commit a new worktree starts on, how the row's own branch goes out
        /// (`branch` / `track`), the branch and where a tracking one starts, and where it goes — the path, the folder
        /// and what is in the way there.
        property string oid: ""
        property string checkout: ""
        property string branch: ""
        property string start: ""
        property string addPath: ""
        property string place: ""
        property string taken: ""
        /// What a greyed `worktree add` names: the local branch a remote row lands on, and the worktree holding the
        /// branch (`holderLeaf`, the repository's own wearing the house).
        property string local: ""
        property string holderLeaf: ""
        property bool holderHome: false
    }

    /// Stands the card on a row, off what the carrying menu read for it (`RefMenuFacts.stand`): the rows core
    /// answered (`menu` — which there are and why each greys, and `worktreeAdd`, how the row's own branch goes out);
    /// the worktree the row's lead names (`lead.card` — `NavSectionModel.leadOf`, undefined for none); the row's
    /// commit `oid`; where a new worktree goes (`lead.place` — its path, its folder and what is in the way there,
    /// undefined for nowhere); and the row and where it leads (`lead` — the branch it takes out, and the worktree
    /// holding it).
    function standOn(facts) {
        const lead = facts.lead
        const worktree = lead.card
        const menu = facts.menu
        const place = lead.place
        state.path = worktree === undefined ? "" : worktree.full
        state.name = worktree === undefined ? "" : worktree.name
        state.holds = worktree === undefined ? "" : worktree.branch
        state.lockReason = worktree === undefined ? "" : worktree.reason
        state.oid = facts.oid
        state.checkout = menu.worktreeAdd
        state.branch = menu.worktreeAdd === "" ? "" : lead.local
        state.start = menu.worktreeAdd === "track" ? lead.full : ""
        state.addPath = place === undefined ? "" : place.path
        state.place = place === undefined ? "" : place.name
        state.taken = place === undefined ? "" : place.taken
        state.local = lead.local
        state.holderLeaf = lead.called
        state.holderHome = lead.home
    }

    // `dismiss()` folds every level (rules-refs/app-ui.md「メニューを閉じるのは自分」).

    titleKind: "tree"
    titleTint: Theme.success
    title: qsTr("WORKTREE")
    word: "worktree"

    // Words, not a command: it opens a box, as `Create branch here…` and `Create tag here…` do.
    AppMenuItem {
        id: refWorktreeHereItem
        text: Words.createWorktreeHere
        word: "worktree-here"
        onTriggered: worktreeMenu.worktreeHereRequested(state.oid)
    }
    // Names the folder it makes, as `worktree remove` names the one it takes (デザイン規約 §ref の種別「名前の印」);
    // greyed with the reason where the branch is out already — git keeps it to one worktree —, where its own row takes
    // it out, where no place is known, or where something is in the way there, warned of before the press (user
    // decision — ask before changing).
    AppMenuItem {
        id: refWorktreeAddItem
        code: "worktree add"
        nameMark: "tree"
        codeName: state.place
        growsForText: false
        word: "worktree-add"
        whyNames: ({ "holder": state.holderLeaf, "local": state.local,
                     "place": { "taken": state.taken, "name": state.place } })
        // The name at the end of that line wears its mark, as it does on its row (デザイン規約 §ref の種別).
        reasonMarkWord: refWorktreeAddItem.why === "held" ? state.holderLeaf
                      : refWorktreeAddItem.why === "local" ? state.local
                      : refWorktreeAddItem.why === "taken" ? state.place : ""
        reasonMarkKind: refWorktreeAddItem.why === "held" && state.holderHome ? "home"
                      : refWorktreeAddItem.why === "local" ? "branch" : "tree"
        onTriggered: worktreeMenu.worktreeAddRequested(state.checkout, state.branch, state.start, state.addPath,
                                                       state.place)
    }
    // A plain delete, as `branch --delete`: two presses in the plain ink (デザイン規約 §回数押し) — a local write, and git
    // keeps a worktree holding uncommitted work; that refusal comes down as a report over the graph (デザイン規約
    // §左メニューの所作 の削除の表).
    AppMenuItem {
        id: refWorktreeRemoveItem
        code: "worktree remove"
        // The name, readable where the row is (デザイン規約 §メニュー「例外は削除行だけ」), behind the tree mark every
        // worktree's name wears (デザイン規約 §ref の種別「名前の印」).
        nameMark: "tree"
        codeName: state.name
        growsForText: false
        word: "remove-worktree"
        // Greyed while locked and while this tab stands in it (user decision — ask before changing): it is the card's
        // one row about that worktree, the only place to say why it cannot go (デザイン規約 §メニュー「右クリックの
        // メニューの行は、対象の分類で決まる」; a write running greys the whole menu instead). The repository's own worktree
        // is "nothing to act on": git never removes it, so core has no row and the card stands on the add row.
        whyNames: ({ "lockReason": state.lockReason })
        holdPresses: Metrics.holdPressesPlain
        holdTone: Theme.textPrimary
        premise: state.path + "\n" + state.holds
        onHeld: {
            refWorktreeRemoveItem.closeMenu()
            worktreeMenu.removeRequested(state.path, state.name, state.holds)
        }
    }
}

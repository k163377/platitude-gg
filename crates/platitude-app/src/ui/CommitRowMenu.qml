pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The right-click on a graph row: one menu for a commit, another for a stash. A stash is a commit git keeps off to one
// side of every branch, so none of the commit menu's rows land on it and it gets its own (デザイン規約 §グラフ行の右クリック).
//
// **The row is one target.** Wherever along it the press lands — the chip's column or the message's — this is what
// comes up: the rows of the first level are about the commit the row stands on, and the cards at the foot are about
// the name the chip is drawing. A row of the stacked list a chip unfolds into raises the same menu, with the card
// aimed at the name that was pressed instead (デザイン規約 §グラフ行の右クリック).
//
// What either of them offers is decided as it opens and handed in here — the conditions are live (a timer fetch alone
// moves `busyCount`), and a row that appears or vanishes under the pointer is a row clicked by accident (デザイン規約 §メニュー).
//
// **Nothing here is looked up and nothing here is run.** Every answer arrives as a plain value from the one place that
// opens this menu (`CommitMenuState`), and every press leaves as a request for the page to answer
// (`RepoPage`) — so what is left in this file is which rows there are, what each says, and what a press asks for, all
// of which `tst_commitrowmenu.qml` drives on the real thing.
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`); it draws
// nothing itself.
Item {
    id: rowMenu

    /// Why every row here is out, in one line, while the window's write doors are held — a write that replays is
    /// running behind the screen (`RepoPage.doorsHeldWhy`). Every card of this menu takes it: a reset, a fold, a drop
    /// and a cherry-pick all move the very history the replay is partway through, and the graph's rows are as much a
    /// door onto that as the left pane's are. The rows stay and grey (`AppMenu.heldReason`).
    property string heldReason: ""

    /// **The name this menu is aimed at** — the one the chip is drawing, which is the row's first record and the
    /// destination of a double-click on it; or, when the menu was raised from a row of the stacked list, the name that
    /// was pressed there. `kind` is `branch` / `remote` / `tag`, and empty on a row that draws no name at all — a
    /// commit nothing points at, or the detached marker, which names no ref (デザイン規約 §グラフ行の右クリック).
    ///
    /// Whichever card can name it is the card that comes up; the other holds only what is about to be made.
    property string targetKind: ""
    property string targetName: ""
    /// What the rows that bring two lines of history together are handed. **The name where the row draws one**: a
    /// merge of `main` says so in the commit it writes, where a merge of the same commit by its id would not — and
    /// what the reader pressed is the name on the screen.
    readonly property string integrateRef: rowMenu.targetName !== "" ? rowMenu.targetName : rowMenu.oid
    /// The branch every one of these rows makes its sentence about (規約 §履歴を合流させる).
    required property string branch

    /// The commit the menu was opened on, and the stash it is ("" on an ordinary commit).
    required property string oid
    required property string stashRef
    /// Whether a remote already has that commit. Rewriting it goes ahead unasked — nothing here leaves the machine —
    /// but デザイン規約 「push 済みの範囲は言うだけ」 wants it said, so the squash row carries a tag the way the amend editor does. The
    /// answer lands a frame after the menu opens.
    required property bool published
    /// What these menus offer. `canSequence` is the pair that only add a commit, and so ask less of the repository than
    /// the rest.
    required property bool canSequence
    required property bool canIntegrate
    required property bool canEditHistory
    required property bool canMoveBranch
    required property bool canBranchHere
    required property bool stashCanWrite
    /// Whether anything besides this branch still reaches the tip — what decides whether the drop row is held
    /// (`CommitMenuState.menuTipHeldElsewhere`, the same answer `RebasePlanRunBar` is handed).
    required property bool tipHeldElsewhere
    /// Whether the name this row draws is somewhere to move to, and whether that move raises a question first
    /// (offers::ref_menu — the same two answers the sidebar's row reads).
    required property bool canSwitch
    required property bool switchAsks
    /// …or leads to the working copy holding that name instead, which is that copy's folder (`RefRowMenu.heldLeaf`).
    /// **Same words in the same seat as the sidebar's row** — one operation reads the same however it is met
    /// (デザイン規約 §メニュー).
    required property string heldLeaf
    /// And whether that name has a far side to pull from — the third answer off those same rules.
    required property bool canPull
    /// Whether that press would be turned down before git did anything, which is what greys the row
    /// (`RefRowMenu.pullBlocked` — the same answer at the other entrance).
    required property bool pullBlocked
    /// How many files `--hard` takes besides the commits — the working tree's own, which that flag writes over
    /// (`status::Counts::hard_reset_takes`). Zero on a clean tree, and then the row says nothing extra.
    required property int hardResetTakes
    /// git's answers about the branch card's delete, live while the card stands: the refusal and the landing it goes
    /// or changes shape on, and the early check it dresses the row with. Passed straight through — the card reads
    /// them by its own name (`RefBranchMenu`), and the sidebar's entrance hands it the same four.
    property string refusedDelete: ""
    property string landedDelete: ""
    property string checkedBranch: ""
    property string checkedMerged: ""

    /// Where the working tree goes, when the name this row draws is somewhere to go. The page owns the road, which is
    /// the one a double-click on the row already takes (`RepoPage.switchToRef`).
    signal switchRequested(string kind, string name)
    /// The rows the page owns the answer to: they rewrite history or move the selection, which is the page's to do
    /// (`RepoPage`).
    signal branchHereRequested(string oidHex)
    signal tagHereRequested(string oidHex)
    signal squashRequested(string oidHex)
    signal dropRequested(string oidHex)
    /// The full interactive rebase, opened as a plan over `from^..HEAD` — nothing runs until its own button is
    /// pressed, so the row is a plain click however much the plan may come to take away.
    signal planRequested(string oidHex)
    signal resetRequested(string mode)
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    signal dropStashRequested(string selector)
    /// The branch card's two, passed straight up: the delete git may still refuse is the page's question, and the
    /// upstream is answered in the page's one question bar (`UpstreamFlow`). The row taken off the list ahead of the
    /// answer is neither — that leaves with the write itself (`ops_delete`).
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal upstreamRequested(string branch, string counterpart)
    /// The writes the rows of this menu run, in the words each takes. Nothing here runs git: what a press comes to is
    /// the page's, which is where the tab this menu was opened over is (`RepoPage`).
    signal cherryPickRequested(string oidHex)
    signal revertRequested(string oidHex)
    signal mergeRequested(string ref)
    signal rebaseRequested(string ref)
    signal pullRequested()
    /// And the two cards' writes, passed up the same way — the branch card's four and the tag card's four.
    signal checkDeleteRequested(string branch)
    signal forceDeleteRequested(string branch)
    signal deleteRemoteRequested(string remoteRef)
    signal deleteEverywhereRequested(string branch, string remoteRef, bool forced)
    signal pushTagRequested(string remote, string tag, bool lease)
    signal deleteTagRequested(string tag)
    signal deleteRemoteTagRequested(string remote, string tag, bool onlyThere)
    signal deleteTagEverywhereRequested(string tag, string remote)
    /// The menu went away — and the stacked list it may have been standing on is the pointer's to answer for again.
    signal dismissed()

    /// Either card is on screen. A plain property — `visible` read from another file comes back
    /// stale (`RefusalBadge`) — and what the page reads to know that hover is behind a menu now (デザイン規約 §メニュー).
    readonly property bool showing: commitMenu.visible || stashMenu.visible
    /// Whether it has finished opening — what the stacked list reads to know it is standing *on* this menu's row
    /// (`RowHoverHost.menuStanding`).
    readonly property bool opened: commitMenu.opened

    /// The automation's handles into these rows, an automation-only exposure the same as `GraphPane.view` is
    /// (app-ui.md).
    readonly property alias menu: commitMenu
    readonly property alias branchHereItem: branchHereCommitItem
    readonly property alias switchItem: switchCommitItem
    readonly property alias pullItem: pullCommitItem
    readonly property alias tagHereItem: tagCommitMenu.tagHereItem
    readonly property alias dropItem: dropCommitItem
    readonly property alias stashDropItem: stashDeleteItem
    readonly property alias resetSubmenu: resetMenu
    readonly property alias hardResetRow: hardResetItem
    /// The two cards the rows above hang behind (`AppMenu.openSub`).
    readonly property alias branchCard: branchCommitMenu
    readonly property alias tagCard: tagCommitMenu

    anchors.fill: parent

    // Each menu closes itself: a row that runs something takes its own menu down with `dismiss()`, and the branch
    // card standing for a delete's answer goes by itself when that answer lands (`RefBranchMenu`) — so this
    // component never has to name the two menus to close whichever one is up.
    function offerStash() {
        stashMenu.offer()
    }
    /// `facts` is what the two cards stand on, read where the models are (`CommitMenuState.cardFacts`): `branch` and
    /// `tag`, each already aimed at the target or emptied because the target is the other's kind.
    function offerCommit(facts) {
        // Both cards are asked before the menu opens, the way every other row's answer is settled first — their own
        // `applies` is what decides whether a card's row is there to count (`AppMenu.offeredRows`). Each is handed the
        // target only when the target is its own kind, so the other comes up holding what can still be made here.
        const branchy = rowMenu.targetKind === "branch" || rowMenu.targetKind === "remote"
        const branchKind = branchy ? rowMenu.targetKind : ""
        const branchName = branchy ? rowMenu.targetName : ""
        branchCommitMenu.standOn(branchKind, branchName, branchName, rowMenu.oid, facts.branch)
        const tagged = rowMenu.targetKind === "tag"
        tagCommitMenu.standOn(tagged ? "tag" : "", tagged ? rowMenu.targetName : "", rowMenu.oid, facts.tag)
        commitMenu.offer()
    }

    AppMenu {
        id: stashMenu
        heldReason: rowMenu.heldReason
        AppMenuItem {
            code: "apply"
            offered: rowMenu.stashCanWrite
            onTriggered: rowMenu.applyStashRequested(rowMenu.stashRef)
        }
        AppMenuItem {
            code: "pop"
            offered: rowMenu.stashCanWrite
            onTriggered: rowMenu.popStashRequested(rowMenu.stashRef)
        }
        AppMenuSeparator {}
        // Held (デザイン規約 §長押し).
        AppMenuItem {
            id: stashDeleteItem
            code: "drop"
            offered: rowMenu.stashCanWrite
            holdMs: Metrics.holdMs
            onHeld: {
                stashMenu.dismiss()
                rowMenu.dropStashRequested(rowMenu.stashRef)
            }
        }
    }

    AppMenu {
        id: commitMenu
        heldReason: rowMenu.heldReason
        onClosed: rowMenu.dismissed()
        // **The card holds what moves the reader** (デザイン規約 §メニュー の入れ子): a branch of one's own started here,
        // the commits replayed onto where they are, this one's place in the history rewritten, the current branch
        // brought over or taken back. What is *done to* a ref goes behind a mark at the foot.
        //
        // Down the card, what each row acts on widens: where the reader would stand, then moving there, then the rows
        // that only add a commit, then the rows that rewrite this one, then the rows that move the branch itself.
        //
        // Words, since no one command is what this row runs. It opens a box, and what git is finally
        // spawned with depends on what is typed into it — the ellipsis is that (`Add remote…` / `Open repository…`).
        // The one command that would fit, `switch --create`, is the spelling the row below would then share — which
        // is the same reason the two stand in one group: this row is that row's other form, the move to a name that
        // does not exist yet (デザイン規約 §ブランチ・コミットへの移動).
        //
        // Landing here is by name: the commit itself would leave HEAD on no branch (デザイン規約 §ブランチ・コミット
        // への移動). On a row that draws no name, this row is the whole of what the menu offers towards standing here.
        AppMenuItem {
            id: branchHereCommitItem
            text: Words.createBranchHere
            offered: rowMenu.canBranchHere
            onTriggered: rowMenu.branchHereRequested(rowMenu.oid)
        }
        // Where the reader goes, when the name on this row is somewhere to go. **The same group as the row above**:
        // that row runs `switch --create`, so a variant of this one is exactly what it is (デザイン規約
        // §ブランチ・コミットへの移動), and the two together are the whole of "where do I stand".
        //
        // **The name is the chip's**, so the row says nothing twice; landing is by name, since the commit itself
        // would leave HEAD on no branch (§ブランチ・コミットへの移動).
        AppMenuItem {
            id: switchCommitItem
            // A branch another working copy holds is a road to that copy rather than a move, and the row says where
            // it leads — the same words in the same seat as the sidebar's row (デザイン規約 §メニュー).
            code: rowMenu.heldLeaf === "" ? "switch" : ""
            //: The row that leads to the working copy holding this branch; the folder's name follows it.
            text: rowMenu.heldLeaf === "" ? "" : qsTr("Open")
            nameMark: rowMenu.heldLeaf === "" ? "" : "tree"
            nameMarkTint: Theme.success
            markName: rowMenu.heldLeaf
            offered: rowMenu.canSwitch
            // The press raises a question where something stands in the move's way, and the mark says so before it
            // is made — the same rule the sidebar's row follows (`RefRowMenu`).
            blockedReason: ""
            asks: rowMenu.switchAsks
            onTriggered: rowMenu.switchRequested(rowMenu.targetKind, rowMenu.targetName)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "cherry-pick"
            offered: rowMenu.canSequence
            onTriggered: rowMenu.cherryPickRequested(rowMenu.oid)
        }
        // Both cherry-pick and revert only add a commit, so both are a plain click.
        AppMenuItem {
            code: "revert"
            offered: rowMenu.canSequence
            onTriggered: rowMenu.revertRequested(rowMenu.oid)
        }
        AppMenuSeparator {}
        // The message is edited in the details pane's boxes, where the click that opens this menu has already put
        // it.
        AppMenuItem {
            code: "squash"
            //: Follows the `squash` chip: "squash into parent".
            text: qsTr("into parent")
            // Said (要望: rewriting a pushed commit shows a warning): the squash goes ahead, and this tag is
            // the warning.
            note: rowMenu.published ? qsTr("already pushed") : ""
            offered: rowMenu.canEditHistory
            onTriggered: rowMenu.squashRequested(rowMenu.oid)
        }
        // Held while this branch is the only thing holding its tip; a plain click once something else does — then the
        // replaced commits stay drawn and a cherry-pick brings any of them back (デザイン規約 §長押し). Taken from the
        // answer the menu opened with: a mark appearing later would re-indent every row
        // (`AppMenu.holdIndent`) with the hand already on its way.
        AppMenuItem {
            id: dropCommitItem
            code: "drop"
            note: rowMenu.published ? qsTr("already pushed") : ""
            offered: rowMenu.canEditHistory
            holdMs: rowMenu.tipHeldElsewhere ? 0 : Metrics.holdMs
            onTriggered: rowMenu.dropRequested(rowMenu.oid)
            onHeld: {
                commitMenu.dismiss()
                rowMenu.dropRequested(rowMenu.oid)
            }
        }
        // The whole range from this commit up, opened as a plan: verbs, reorders and rewords are
        // composed over the graph and nothing touches the repository until the plan's own button — so this row asks
        // for nothing, whatever the plan may come to take away (デザイン規約 §履歴を合流させる / P3-確認事項 §A). The
        // clicked commit is the oldest one *included*, and the plan shows the base it lands on as its own last row —
        // where the products disagree ("from here" in and out), showing the base is what settles it.
        AppMenuItem {
            code: "rebase --interactive"
            //: Follows the `rebase --interactive` chip: "from here up".
            text: qsTr("from here up")
            note: rowMenu.published ? Words.rewritesPushed : ""
            offered: rowMenu.canEditHistory
            onTriggered: rowMenu.planRequested(rowMenu.oid)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "merge"
            //: Follows the `merge` chip: "merge into main".
            refSentence: qsTr("into %1")
            refName: rowMenu.branch
            offered: rowMenu.canIntegrate
            onTriggered: rowMenu.mergeRequested(rowMenu.integrateRef)
        }
        AppMenuItem {
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            refSentence: qsTr("%1 onto it")
            refName: rowMenu.branch
            note: rowMenu.published ? Words.rewritesPushed : ""
            offered: rowMenu.canIntegrate
            onTriggered: rowMenu.rebaseRequested(rowMenu.integrateRef)
        }
        AppMenu {
            id: resetMenu
            heldReason: rowMenu.heldReason
            titleCode: "reset"
            //: Follows the `reset` chip: "reset main here".
            titleSentence: qsTr("%1 here")
            // `canMoveBranch` is false detached and on an unborn HEAD (offers::commit_menu), so the row never comes up
            // without a branch; the second half is only what the sentence would still read as if it did.
            titleRef: rowMenu.branch !== "" ? rowMenu.branch : qsTr("the branch")
            applies: rowMenu.canMoveBranch
            AppMenuItem {
                code: "--soft"
                text: qsTr("Keep everything, staged")
                onTriggered: rowMenu.resetRequested("soft")
            }
            AppMenuItem {
                code: "--mixed"
                text: qsTr("Keep everything, unstaged")
                onTriggered: rowMenu.resetRequested("mixed")
            }
            // Held (デザイン規約 §長押し).
            //
            // **The row's sentence is about the commits; the tag is about the working tree.** `--hard` writes over
            // every tracked path the index or the tree has changed, and those changes are not "after" the commit this
            // menu stands on — nothing else on screen would say they are about to go, and the reflog does not hold
            // them (規約 §ブランチを過去のコミットへ戻す). Said, the way the pushed range is
            // (デザイン規約 「push 済みの範囲はタグで言うだけ」): the hold is already the consent.
            //
            // Untracked files are not in the count — `--hard` leaves them where they are — so a tree dirty with
            // nothing but scratch files carries no tag at all.
            AppMenuItem {
                id: hardResetItem
                code: "--hard"
                text: qsTr("Discard everything after it")
                // **Two words, the way every other note in this menu is** (`already pushed` / `not merged`), and the
                // word kept is the one doing the distinguishing: what goes besides the commits is the part of the
                // tree that is *uncommitted*, and `too` is what says it goes as well.
                //
                // Words only. The number is already on screen behind this menu — the uncommitted row's tally is
                // drawn from the same status — and `%n … file(s)` would put the translator's brackets on the row:
                // no translation loads here, so `(s)` reaches the reader verbatim (measured).
                note: rowMenu.hardResetTakes > 0 ? qsTr("uncommitted too") : ""
                holdMs: Metrics.holdMs
                onHeld: {
                    // Qt's `dismiss()`: this card and the menu it hangs off go together.
                    resetMenu.dismiss()
                    rowMenu.resetRequested("hard")
                }
            }
        }
        AppMenuSeparator {}
        // **The chip's entrance to the row the sidebar draws, in the same seat** — a group of its own, right above
        // the cards, wherever the menu is opened (`RefRowMenu` — デザイン規約 §取り込んで合流させる). The rows above
        // are about the commit this row stands on; a pull is about the branch the working tree is on and the
        // upstream it is measured against, which is why it keeps a seat of its own in both menus.
        AppMenuItem {
            id: pullCommitItem
            code: "pull"
            offered: rowMenu.canPull
            // Greyed on the same answer the sidebar's row reads (`RefRowMenu`).
            blockedReason: rowMenu.pullBlocked ? Words.pullDiverged : ""
            onTriggered: rowMenu.pullRequested()
        }
        AppMenuSeparator {}
        // **The very card the sidebar's row opens** (RefBranchMenu): a branch met on a graph row and the same branch
        // met in the left pane are two ways at one thing, and what they offer has to be the same. The card works its
        // own answers out from the name it is handed, so the two entrances cannot drift; a row that draws no branch
        // is handed "" and the card goes with its row.
        RefBranchMenu {
            id: branchCommitMenu
            heldReason: rowMenu.heldReason
            refusedDelete: rowMenu.refusedDelete
            landedDelete: rowMenu.landedDelete
            checkedBranch: rowMenu.checkedBranch
            checkedMerged: rowMenu.checkedMerged
            onDeleteRequested: (kind, id, name, oidHex) => rowMenu.deleteRequested(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart) => rowMenu.upstreamRequested(branch, counterpart)
            onCheckDeleteRequested: branch => rowMenu.checkDeleteRequested(branch)
            onForceDeleteRequested: branch => rowMenu.forceDeleteRequested(branch)
            onDeleteRemoteRequested: remoteRef => rowMenu.deleteRemoteRequested(remoteRef)
            onDeleteEverywhereRequested: (branch, remoteRef, forced) =>
                rowMenu.deleteEverywhereRequested(branch, remoteRef, forced)
        }
        AppMenuSeparator {}
        // **The very card the tag's own chip opens** (RefTagMenu): the mark left on this commit, and — where the row
        // draws a tag — everything that name answers for. The box its first row opens is the same box the row at the
        // head opens, on the same commit and on the same answer; what the two do not share is what happens next — a
        // branch is somewhere to carry on from, a tag is a mark left behind.
        RefTagMenu {
            id: tagCommitMenu
            heldReason: rowMenu.heldReason
            canBranchHere: rowMenu.canBranchHere
            onTagHereRequested: oidHex => rowMenu.tagHereRequested(oidHex)
            onPushTagRequested: (remote, tag, lease) => rowMenu.pushTagRequested(remote, tag, lease)
            onDeleteTagRequested: tag => rowMenu.deleteTagRequested(tag)
            onDeleteRemoteTagRequested: (remote, tag, onlyThere) =>
                rowMenu.deleteRemoteTagRequested(remote, tag, onlyThere)
            onDeleteTagEverywhereRequested: (tag, remote) =>
                rowMenu.deleteTagEverywhereRequested(tag, remote)
        }
    }
}

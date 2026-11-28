pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui
import platitude.ui.words

// The right-click on a ref wherever it is drawn — sidebar row, graph chip, a row of a chip's stacked list — so a
// branch offers the same things everywhere. Decided as it opens and stood again on the tree as it is while it stands,
// its rows kept — when a command it waited for has ended (`standAgain`) and when what it reads moves (`MenuLines.moved`;
// デザイン規約 §メニュー).
//
// It carries only what the row's gestures do not reach: a rename is a click or two away on the row itself
// (デザイン規約 §メニュー「置くのは、そこにしか入口が無いものだけ」).
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: refRowMenu

    required property RepoTab repoTab
    required property WorktreeModel worktree
    /// The worktree as it stood when the command the menus wait for was asked (`RepoPage.treeAtAsk`); null while
    /// nothing is out. Back to null with the menu up, the rows stand on what the command left (`MenuLines.waiting`).
    property var treeAtAsk: null
    /// What the rows are read off, set by the door that stands them: the worktree's `reading` — or, for a menu raised
    /// under a command, `treeAtAsk`. Handed to `GitFacts` as it is, never built here. Null before the first door, as
    /// `CommitMenuState.tree`.
    property var tree: null
    /// Moves each time what the rows stand on has been read again and moved (`RepoPage.menuReadRevision`): a menu
    /// standing follows it (`MenuLines.moved`).
    property int readRevision: 0
    /// What the rows and cards stand on, read through `menuFacts`; the `rebase` row's warning off the drawn rows
    /// (`rebasePublished`), and a branch renamed with the menu up followed off the branches (`standAgain`).
    required property GraphModel graphModel
    required property NavSectionModel branchesModel
    required property NavSectionModel worktreesModel
    required property NavSectionModel tagsModel
    /// Read through `menuFacts` too; where a remote branch points now and which selector a stash answers to now, for
    /// the menu standing again (`standAgain`) — the stash off the listing, which holds every entry the graph's walk
    /// may not reach.
    required property NavSectionModel remotesModel
    required property NavSectionModel stashesModel
    /// What the GITHUB card at the foot stands on.
    required property GitHubLinks githubLinks

    /// Why every row is out while the window's write doors are held (`RepoPage.doorsHeldWhy`), handed down so this
    /// and the left pane read one answer. The rows grey (`AppMenu.heldReason`).
    property string heldReason: ""
    /// What the rows say while they wait for a git command this window started (`RepoPage.runningWhy`). No running
    /// command is asked about: a menu opened under one keeps its rows and greys them until it ends
    /// (`AppMenu.runningWhy`), so it has one shape whenever it is opened.
    property string runningWhy: ""

    /// The row the menu stands on. `refId` is what git knows it by, which on a stash is a selector, apart from
    /// the row's message, and on a worktree with no branch out its path. `refId` never appears in a sentence
    /// (デザイン規約 §メニュー).
    property string kind: ""
    property string refId: ""
    property string refOid: ""
    /// What else it was opened on — the name the row shows, the worktree the WORKTREE card stands on, a remote named on
    /// its own (`offerOn`) — for the menu standing again on a newer reading (`standAgain`).
    property string refName: ""
    property var openedFor: undefined
    property string openedAim: ""
    /// What merge / rebase are handed: the name, or the commit where the row is a worktree — a path is no ref.
    readonly property string integrateRef: refRowMenu.kind === "worktree" ? refRowMenu.refOid : refRowMenu.refId

    /// The rows the menu and its cards have, and why each is greyed, as core answered the stand, drawn into the menu
    /// (`MenuLines`) — and when the menu stands again (`standAgain`).
    readonly property MenuLines lines: MenuLines {
        id: lines
        menu: refMenu
        moved: refRowMenu.readRevision
        waiting: refRowMenu.treeAtAsk !== null
        onStandAgain: again => refRowMenu.standAgain(again)
    }
    /// Whether `switch` will raise a question (an operation standing, or unmerged files), worn as the row's `!` so it
    /// is known before the press (user decision — ask before changing; the mark's seat: `AppMenuItem.asks`).
    property bool switchAsks: false
    /// The name the worktree holding this row's branch goes by, empty when none does; the switch row then opens that
    /// worktree and names it (`RepoPage.switchToRef`). Its WORKTREES row's name — the folder, or the branch the
    /// repository's own has out (`NavSectionModel.leadOf`'s `called`, デザイン規約 §ref の種別).
    property string heldLeaf: ""
    /// That worktree is the repository's own: its name wears the house, not the tree (デザイン規約 §ref の種別).
    property bool heldHome: false
    /// What the `pull` row runs (`menu::pull_runs`): `here` — `git pull`, on either end of the current branch's
    /// upstream comparison —, `aside` — this row's own branch fast-forwarded, the worktree being on another —, or ""
    /// for no row.
    property string pullRuns: ""
    /// The remote branch this row's branch tracks, which the `own` reason names — the reading its BRANCH card speaks
    /// for (`NavSectionModel.cardOf`'s `remoteCounterpart`): a remote one only, so a branch tracking a local
    /// one (`branch.<n>.remote = .`) has no `aside` row.
    property string pullUpstream: ""

    // ---- bringing two lines of history together --------------------
    /// Whether a rebase onto this row would rewrite a commit a remote has (`<ref>..HEAD`), read off the drawn rows
    /// (`GraphModel.rebaseRewritesPublished`): a `git rev-list` would land after the card and widen it under the hand
    /// (規約 §行が読む答えはどこから来るか).
    property bool rebasePublished: false

    /// On screen, and finished opening — asked by the folded list and the chip's stacked list. Plain properties:
    /// `visible` read from another file comes back stale (`RefusalBadge`).
    readonly property bool showing: refMenu.visible
    readonly property bool opened: refMenu.opened

    /// Automation-only handles into these rows, like `GraphPane.view`.
    readonly property alias menu: refMenu
    readonly property alias branchHereItem: refBranchHereItem
    readonly property alias tagHereItem: tagMenu.tagHereItem
    readonly property alias pushTagItem: tagMenu.pushTagItem
    readonly property alias deleteTagItem: tagMenu.deleteTagItem
    readonly property alias deleteRemoteTagItem: tagMenu.deleteRemoteTagItem
    readonly property alias deleteTagBothItem: tagMenu.deleteTagBothItem
    readonly property alias deleteItem: refDeleteRow
    readonly property alias deleteRemoteItem: refRemoteDeleteRow
    readonly property alias deleteBothItem: refBothDeleteRow
    readonly property alias upstreamItem: branchMenu.upstreamItem
    readonly property alias pushBranchItem: refPushRow
    readonly property alias removeWorktreeItem: worktreeMenu.removeWorktreeItem
    readonly property alias stashDropItem: refStashDropItem
    readonly property alias switchItem: refSwitchItem
    readonly property alias pullItem: refPullItem
    readonly property alias mergeItem: refMergeItem
    readonly property alias rebaseItem: refRebaseItem
    /// The cards the rows used rarely hang behind; a run photographing one opens it first (`AppMenu.openSub`).
    readonly property alias branchCard: branchMenu
    readonly property alias worktreeCard: worktreeMenu
    readonly property alias tagCard: tagMenu
    readonly property alias githubCard: githubMenu
    /// What the rows and every card stand on, read in one place for this entrance and the graph row's
    /// (`CommitMenuState.menuFacts`).
    readonly property RefMenuFacts menuFacts: RefMenuFacts {
        id: menuFacts
        repoTab: refRowMenu.repoTab
        graphModel: refRowMenu.graphModel
        worktreesModel: refRowMenu.worktreesModel
        branchesModel: refRowMenu.branchesModel
        remotesModel: refRowMenu.remotesModel
        tagsModel: refRowMenu.tagsModel
    }

    /// What the page answers for: moving the worktree, the delete git may still refuse, and the stash drop that two
    /// menus share.
    signal switchRequested(string kind, string name)
    /// A new branch on this row's commit; the page opens the name box where the menu was opened.
    signal branchHereRequested(string oidHex)
    /// A tag on this row's commit — the same box, opened where the menu was.
    signal tagHereRequested(string oidHex)
    /// The WORKTREE card's two making rows (`RefWorktreeMenu`): a new branch on this row's commit in a worktree of
    /// its own — the same box again, opened where the menu was — and the row's own branch out in one.
    signal worktreeHereRequested(string oidHex)
    signal worktreeAddRequested(string mode, string branch, string start, string path, string name)
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    signal dropStashRequested(string selector)
    /// The WORKTREE card's `worktree remove`, which takes the worktree's row off the screen at the press (`RepoTab`).
    signal removeWorktreeRequested(string path, string name, string holds)
    /// Which remote branch a local one is measured against — the branch card's row, answered in the page's one bar
    /// (`UpstreamFlow`). `unsettable`: it has one an emptied name takes off (`RefBranchMenu.upstreamRequested`).
    signal upstreamRequested(string branch, string counterpart, bool unsettable)
    /// Where a branch's first push goes — the push row beside `pull`, asked in the page's one bar (`PublishFlow`).
    signal publishRequested(string branch, string upstream)
    /// A link on its way to the clipboard (`GitHubMenu`); the page owns the clipboard.
    signal copyRequested(string text)
    /// The branch the menu stands on was renamed with the menu up and the menu went with it (`standAgain`): the page's
    /// door and the row the name boxes open on answer to the new name (`RepoPage.followRenamedDoor`).
    signal renameFollowed(string from, string to)

    anchors.fill: parent

    /// Opens on that ref, deciding there and then what it offers. Says whether it opened at all: a ref with nothing to
    /// offer — the current branch met as a chip — falls back to the row's own menu.
    ///
    /// `worktreePath` is the worktree the WORKTREE card stands on — the WORKTREES row's own; left out, the worktree
    /// holding this row's branch, if another does. `aim` is a remote the reader named on its own
    /// (`RepoPage.openRefMenu`), which the TAG card acts on.
    function offerOn(kind, name, full, oidHex, worktreePath, aim) {
        // A WORKTREES row opens the same menu as a branch row (user decision — ask before changing): a worktree
        // with a branch out is that branch's menu, carrying the worktree's card — the worktree is where the branch
        // is, and the row says both. One with none stands on its commit (`integrateRef`, `standOn`).
        if (kind === "worktree") {
            const worktree = refRowMenu.worktreesModel.worktreeFacts(full)
            if (worktree !== undefined && worktree.branch !== "")
                return refRowMenu.offerOn("branch", worktree.branch, worktree.branch, oidHex, full)
        }
        lines.release()
        refRowMenu.tree = refRowMenu.treeAtAsk !== null ? refRowMenu.treeAtAsk : refRowMenu.worktree.reading
        const facts = refRowMenu.standOn(kind, name, full, oidHex, worktreePath, aim)
        // The same links the graph row's menu offers on the commit the ref points at and its name; only as the menu
        // opens, as there (`CommitRowMenu.standGithub`).
        githubMenu.standOn(refRowMenu.githubLinks.ofRef(oidHex, kind, full, facts.card))
        return refMenu.offer()
    }

    /// The menu stands again with the menu up (`MenuLines.standAgain`, whose `again` says why): what the rows stand on
    /// is read again, the menu keeping its shape (`AppMenu.holdShape`) — on the commit the ref points at now, a stash's
    /// row on the selector it now answers to. A ref taken away leaves the menu nothing to act on.
    function standAgain(again) {
        // A branch renamed outside with the menu up answers to its new name (`NavSectionModel.renamedTo`): the menu
        // follows it there, as it follows the branch to another commit, and the BRANCH card with its leases
        // (`RefBranchMenu.renamed`). Gone without a rename, the name below finds nothing and the menu closes.
        if (refRowMenu.kind === "branch" && refRowMenu.branchesModel.oidOfName(refRowMenu.refId) === "") {
            const to = refRowMenu.branchesModel.renamedTo(refRowMenu.refId)
            if (to !== "") {
                const from = refRowMenu.refId
                branchMenu.renamed(from, to)
                refRowMenu.refId = to
                refRowMenu.refName = to
                refRowMenu.renameFollowed(from, to)
            }
        }
        // A stash answers to its place in the list, which the command may have moved; the rest to their names. Looked
        // up in the stash list, not the graph: the graph's walk may not reach an old stash.
        const full = refRowMenu.kind === "stash" ? refRowMenu.stashesModel.fullOfOid(refRowMenu.refOid)
                                                 : refRowMenu.refId
        if (full === "" || !refRowMenu.stillThere(refRowMenu.kind, full)) {
            refMenu.dismiss()
            return
        }
        const oidHex = refRowMenu.oidNow(refRowMenu.kind, full)
        refRowMenu.tree = refRowMenu.worktree.reading
        refRowMenu.standOn(refRowMenu.kind, refRowMenu.refName, full, oidHex, refRowMenu.openedFor,
                           refRowMenu.openedAim, again)
    }
    /// Whether the ref `full` of `kind` the menu stands on is still there, by its name (a stash by its commit,
    /// `standAgain`): a branch with no commits yet is no ref git lists, and stands while the worktree the menu was
    /// opened on still has it out.
    function stillThere(kind, full) {
        if (kind === "branch") {
            if (refRowMenu.branchesModel.oidOfName(full) !== "")
                return true
            const opened = refRowMenu.openedFor === undefined || refRowMenu.openedFor === ""
                           ? undefined : refRowMenu.worktreesModel.worktreeFacts(refRowMenu.openedFor)
            return opened !== undefined && opened.branch === full
        }
        return refRowMenu.oidNow(kind, full) !== ""
    }
    /// Whether the worktree a WORKTREES row raised the menu on (`worktreePath`) has left the branch `full` the menu
    /// stands on, as the worktree listing tells it (`GitFacts.worktreeLeft`); false for a menu raised anywhere else.
    function worktreeLeft(kind, full, worktreePath) {
        if (kind !== "branch" || worktreePath === undefined || worktreePath === "")
            return false
        const now = refRowMenu.worktreesModel.worktreeFacts(worktreePath)
        return GitFacts.worktreeLeft(full, now !== undefined, now === undefined ? "" : now.branch)
    }
    /// The commit the ref `full` of `kind` points at now; empty where it is gone or has none. A stash's commit does
    /// not move. A remote branch that moved stands where it is — moved by a fetch outside, or by the one the menu
    /// waited for (デザイン規約 §hover「外乱で閉じない」) —, its `push --delete` still leased to where it stood as the
    /// menu went up and greyed, since on the new tip it would take commits the menu never showed
    /// (`RefBranchMenu.standOn`).
    function oidNow(kind, full) {
        if (kind === "branch")
            return refRowMenu.branchesModel.oidOfName(full)
        if (kind === "remote")
            return refRowMenu.remotesModel.oidOfName(full)
        if (kind === "tag")
            return refRowMenu.tagsModel.oidOfName(full)
        if (kind === "worktree") {
            const worktree = refRowMenu.worktreesModel.worktreeFacts(full)
            return worktree === undefined ? "" : worktree.head
        }
        return refRowMenu.refOid
    }

    /// Reads everything the rows stand on off `tree` and stands them on it: `offerOn` without raising the menu.
    /// `again`: why the menu stands again (`standAgain`), which the BRANCH and TAG cards read
    /// (`RefBranchMenu.standOn`); empty as it is raised. Answers what it stood on (`RefMenuFacts.stand`).
    function standOn(kind, name, full, oidHex, worktreePath, aim, again) {
        refRowMenu.kind = kind
        refRowMenu.refName = name
        refRowMenu.refId = full
        refRowMenu.refOid = oidHex
        refRowMenu.openedFor = worktreePath
        refRowMenu.openedAim = aim === undefined ? "" : aim
        // Core's type (`menu::lines`), asked once per stand: the rows and the cards read the answer, not the models.
        const facts = menuFacts.stand("listed", kind, full, oidHex, refRowMenu.tree, refRowMenu.openedAim, oidHex,
                                      worktreePath)
        branchMenu.standOn(kind, name, full, oidHex, facts, again)
        tagMenu.standOn(kind, full, oidHex, facts, again)
        worktreeMenu.standOn(facts)
        const menu = facts.menu
        // A menu raised on a worktree's row is the menu of the branch that worktree has out (`offerOn`). Once the
        // worktree has left it — removed, switched or taken off it outside, the menu up, the branch still there (one
        // gone with the worktree takes what was clicked, and the menu closes — §メニュー) —, only the rows on the
        // worktree itself still act on what was clicked: the answer keeps those alone (`menu::Row::on_worktree`),
        // and the rest grey where they stand (`AppMenuItem.withdrawn`) — until the menu is raised again, or the
        // worktree has that branch out again — rather than turn into another branch's, or a free branch's, under
        // the hand, where a press meant for the old ones would land (user decision — ask before changing). The
        // GITHUB card's links, made for that branch as the menu opened, grey with them.
        const left = refRowMenu.worktreeLeft(kind, full, worktreePath)
        lines.answer = left ? Object.assign({}, menu, { "lines": menu.lines.filter(line => line.onWorktree) }) : menu
        githubMenu.stillApplies = !left
        // Another worktree having this row's branch out (through the same-named local branch for a remote row) makes
        // `switch` open that worktree instead (offers::SwitchAction::OpenHolder); a worktree with no branch leads to
        // itself, unless this tab already stands in it. Named as its WORKTREES row names it
        // (`NavSectionModel.leadOf`'s `called`).
        refRowMenu.switchAsks = menu.switchAsks
        refRowMenu.heldLeaf = facts.lead.called
        refRowMenu.heldHome = facts.lead.home
        // The tree's own comparison for `git pull`, the row's branch's for its fast-forward (menu::pull_runs).
        refRowMenu.pullUpstream = kind === "branch" ? facts.card.remoteCounterpart : ""
        refRowMenu.pullRuns = menu.pullRuns
        // Read where the menu has the row; the row wears it while it can be pressed (`refRebaseItem.warning`).
        refRowMenu.rebasePublished =
            lines.has("rebase") && refRowMenu.graphModel.rebaseRewritesPublished(oidHex, refRowMenu.tree.headOid)
        return facts
    }

    // Closes itself (rules-refs/app-ui.md「メニューを閉じるのは自分」).
    AppMenu {
        id: refMenu
        heldReason: refRowMenu.heldReason
        runningWhy: refRowMenu.runningWhy
        // Nothing to clear at the close: a refused delete's `-D` is left standing, the card clears it as the menu
        // opens (`RefBranchMenu.standOn`), and nothing reads it while the menu is down.

        // `Create branch here…` runs `switch --create` (デザイン規約 §メニュー の入れ子). It gives the current branch's
        // menu one live row, which would otherwise open on nothing but a greyed delete table.
        AppMenuItem {
            id: refBranchHereItem
            text: Words.createBranchHere
            word: "branch-here"
            onTriggered: refRowMenu.branchHereRequested(refRowMenu.refOid)
        }
        AppMenuItem {
            id: refSwitchItem
            // A branch another worktree holds opens that worktree instead (`SwitchAction::OpenHolder`): a worded door
            // names where it goes before the press, so the chip gives way to `Open` and the worktree's name behind its
            // tree mark — no `!`, since nothing is asked.
            code: refRowMenu.heldLeaf === "" ? "switch" : ""
            //: The row that leads to the worktree holding this branch; its name follows it — the folder's, or the branch
            //: the repository's own worktree has out.
            text: refRowMenu.heldLeaf === "" ? "" : qsTr("Open")
            nameMark: refRowMenu.heldLeaf === "" ? "" : refRowMenu.heldHome ? "home" : "tree"
            codeName: refRowMenu.heldLeaf
            word: "reach"
            // Greyed for no reason of its own but the tab still opening: what stands in the move's way presses
            // through to a question, marked by `asks` (user decision — ask before changing) — a greyed row says why on
            // hover only, and the hand reaching for it is the reader who wants to know. Held doors still hold it
            // (`heldReason`) — a move mid-rewrite is the exit nobody meant (§フル interactive rebase) — and so does a
            // command the menu waits for (`runningWhy`).
            asks: refRowMenu.switchAsks
            // Through the chips' dispatcher: a remote branch whose local one already exists cannot simply be created.
            onTriggered: refRowMenu.switchRequested(refRowMenu.kind, refRowMenu.refId)
        }
        AppMenuItem {
            id: refMergeItem
            code: "merge"
            refSentence: Words.intoBranch
            // The branch the worktree holds, off the tree as it is (`WorktreeModel.movedBranch`) — the rebase and pull
            // rows below too.
            refName: refRowMenu.worktree.movedBranch
            refMark: "branch"
            word: "merge"
            onTriggered: refRowMenu.repoTab.merge(refRowMenu.integrateRef, false, false, "")
        }
        AppMenuItem {
            id: refRebaseItem
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            refSentence: qsTr("%1 onto it")
            refName: refRowMenu.worktree.movedBranch
            refMark: "branch"
            // A tag stays put under a rebase onto it: git peels it to its commit, and the rebase moves the current
            // branch alone (デザイン規約 §履歴を合流させる「既定のフラグ」).
            word: "rebase"
            whyNames: ({ "head": refRowMenu.worktree.movedBranch })
            // The branch with no commits yet in that reason wears its mark, as on the row.
            reasonMarkWord: refRebaseItem.why === "head-unborn" ? refRowMenu.worktree.movedBranch : ""
            reasonMarkKind: "branch"
            // Published = reachable from a remote-tracking ref, as of the last fetch. A greyed row, whatever greys it,
            // warns of a press that cannot be made.
            warning: refRowMenu.rebasePublished ? Words.rewritesPushed : ""
            onTriggered: refRowMenu.repoTab.rebase(refRowMenu.integrateRef, "")
        }
        // The sentence names the branch that moves (user decision — ask before changing), though a row's own name is
        // usually left to the row (規約 §メニュー「行が自分で示しているもの(右クリックした行のコミット)は行に任せる」):
        // one row moves the current branch on its upstream's row and the row's own branch elsewhere, so the words
        // alone would not say which moves before the press. Shaped as `merge` into <current> is. In the name's own
        // colour (規約 §ref の種別「文の中の名前」): the current branch `textLink`, another local branch white; a greyed
        // row greys its name too.
        AppMenuItem {
            id: refPullItem
            code: "pull"
            refSentence: Words.intoBranch
            refName: refRowMenu.pullRuns === "aside" ? refRowMenu.refId : refRowMenu.worktree.movedBranch
            refTint: refRowMenu.pullRuns === "aside" ? Theme.textPrimary : Theme.textLink
            refMark: "branch"
            // Greyed where git would turn the press down, saying where to go instead.
            word: "pull"
            whyNames: ({ "holder": refRowMenu.heldLeaf, "upstream": refRowMenu.pullUpstream })
            reasonMarkWord: refPullItem.why === "held" ? refRowMenu.heldLeaf : ""
            reasonMarkKind: refRowMenu.heldHome ? "home" : "tree"
            reasonMarks: refPullItem.why === "own"
                         ? Words.listMarks(refPullItem.blockedReason, refRowMenu.pullUpstream, "remote") : []
            // `git pull` resolves the other end itself; the fast-forward is told the branch. Headless: `pull-menu
            // [<branch>|<remote ref>]`, `pull-blocked [chip][:<branch>]`, `pull-go`, `pull-ahead`, `pull-branch-go
            // <branch>` (`--preset others-behind`; under a stopped merge `--preset others-merging`).
            onTriggered: refRowMenu.repoTab.pull(refRowMenu.pullRuns === "aside" ? refRowMenu.refId : "")
        }
        // The other way along the same upstream, on the BRANCH card's answers (デザイン規約 §リモートへ送る).
        BranchPushRow {
            id: refPushRow
            card: branchMenu
        }
        // The branch's delete table, on the BRANCH card's answers — the same rows the graph row's menu carries.
        BranchDeleteRow {
            id: refDeleteRow
            card: branchMenu
        }
        BranchRemoteDeleteRow {
            id: refRemoteDeleteRow
            card: branchMenu
        }
        BranchBothDeleteRow {
            id: refBothDeleteRow
            card: branchMenu
        }
        // A stash's `apply` / `pop`, as the graph's stash row says them (`CommitRowMenu`).
        AppMenuItem {
            code: "apply"
            word: "apply-stash"
            onTriggered: refRowMenu.applyStashRequested(refRowMenu.refId)
        }
        AppMenuItem {
            code: "pop"
            word: "pop-stash"
            onTriggered: refRowMenu.popStashRequested(refRowMenu.refId)
        }
        // Held: what is dropped leaves the list, and only the discard record brings it back. The graph's stash row
        // offers it in the same words (`CommitRowMenu`).
        AppMenuItem {
            id: refStashDropItem
            code: "drop"
            word: "drop-stash"
            holdPresses: Metrics.holdPressesWarned
            onHeld: {
                refStashDropItem.closeMenu()
                refRowMenu.dropStashRequested(refRowMenu.refId)
            }
        }
        // The same card the graph row's menu carries.
        RefBranchMenu {
            id: branchMenu
            heldReason: refRowMenu.heldReason
            runningWhy: refRowMenu.runningWhy
            // git's two answers, live: the card turns the delete rows on them while the menu stands.
            refusedDelete: refRowMenu.repoTab.branchDeleteRefused
            landedDelete: refRowMenu.repoTab.branchDeleteLanded
            checkedBranch: refRowMenu.repoTab.branchDeleteAsked
            checkedMerged: refRowMenu.repoTab.branchDeleteMerged
            onDeleteRequested: (kind, id, name, oidHex) => refRowMenu.deleteRequested(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart, unsettable) =>
                refRowMenu.upstreamRequested(branch, counterpart, unsettable)
            onUnsetUpstreamRequested: branch => refRowMenu.repoTab.unsetUpstream(branch)
            onPushRequested: (branch, force, expect) => refRowMenu.repoTab.pushNamed(branch, force, expect)
            onPublishRequested: (branch, upstream) => refRowMenu.publishRequested(branch, upstream)
            onCheckDeleteRequested: branch => refRowMenu.repoTab.checkBranchDelete(branch)
            onForceDeleteRequested: branch => refRowMenu.repoTab.deleteBranch(branch, true)
            // The remote row goes by its own name: the tab reads which branch over there it is
            // (`RepoTab.deleteRemoteBranch`). `expect` is the commit the card showed it on, the delete's lease.
            onDeleteRemoteRequested: (remoteRef, expect) =>
                refRowMenu.repoTab.deleteRemoteBranch(remoteRef, expect)
            onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
                refRowMenu.repoTab.deleteBranchEverywhere(branch, remoteRef, forced, expect)
        }
        // A worktree made off this row, and the one this row names or leads to (デザイン規約 §メニュー の入れ子). The
        // same card the graph row's menu carries.
        RefWorktreeMenu {
            id: worktreeMenu
            heldReason: refRowMenu.heldReason
            runningWhy: refRowMenu.runningWhy
            onWorktreeHereRequested: oidHex => refRowMenu.worktreeHereRequested(oidHex)
            onWorktreeAddRequested: (mode, branch, start, path, name) =>
                refRowMenu.worktreeAddRequested(mode, branch, start, path, name)
            onRemoveRequested: (path, name, holds) => refRowMenu.removeWorktreeRequested(path, name, holds)
        }
        // The same card the graph row's menu carries.
        RefTagMenu {
            id: tagMenu
            heldReason: refRowMenu.heldReason
            runningWhy: refRowMenu.runningWhy
            onTagHereRequested: oidHex => refRowMenu.tagHereRequested(oidHex)
            onPushTagRequested: (remote, tag, expect) => refRowMenu.repoTab.pushTag(remote, tag, expect)
            onDeleteTagRequested: tag => refRowMenu.repoTab.deleteTag(tag)
            onDeleteRemoteTagRequested: (remote, tag, rowGoes, expect) =>
                refRowMenu.repoTab.deleteRemoteTag(remote, tag, rowGoes, expect)
            onDeleteTagEverywhereRequested: (remote, tag, expect) =>
                refRowMenu.repoTab.deleteTagEverywhere(remote, tag, expect)
        }
        AppMenuSeparator {}
        // The same card the graph row's menu carries, last for the same reason: it leaves the window
        // (デザイン規約 §GitHub のページへのリンク).
        GitHubMenu {
            id: githubMenu
            onCopyRequested: text => refRowMenu.copyRequested(text)
        }
    }
}

import QtQuick
import platitude
import platitude.ui

// ---- what the commit menu is standing on ----------------------------
// The row the menu was opened on and what it may offer there, decided in `openRowMenu` and held still while the menu
// stands — read again only when a command the menu waited for has ended (`standAgain`, デザイン規約 §メニュー). One
// place to look for why a row is not on offer.
//
// A `QtObject`: it holds no child (rules-refs/structure.md「切り出した非表示のホストは `Item` にする」). The menu
// itself is declared on the page, which gives it an item to measure the window through (「描かないホスト」).
QtObject {
    id: menuState

    required property RepoTab repoTab
    required property WorktreeModel worktree
    /// The worktree as it stood when the command the menus wait for was asked (`RepoPage.treeAtAsk`); null while
    /// nothing is out.
    property var treeAtAsk: null
    /// What the rows are read off, set by the door that stands them: the worktree — or, for a menu raised under a
    /// command, `treeAtAsk`.
    property var tree: menuState.worktree
    required property GraphModel graphModel
    /// Which worktree has a branch checked out — what the `switch` row needs beyond the commit's own rules.
    required property NavSectionModel worktreesModel
    /// What the cards at the foot need worked out for them: the reading a branch carries and where a tag stands
    /// (`cardFacts`).
    required property NavSectionModel branchesModel
    required property NavSectionModel tagsModel
    /// Where a remote branch the menu is aimed at points now (`aimedAt`).
    required property NavSectionModel remotesModel
    /// The menu these answers are handed to, and the one this opens.
    required property CommitRowMenu menu

    property string menuOid: ""
    // The stash's selector ("" on an ordinary commit); a stash gets its own menu (デザイン規約 §グラフ行の右クリック).
    property string menuStashRef: ""
    // Whether a remote already has the menu's commit — the rewrite rows say so with a tag and do not ask
    // (デザイン規約 §回数押し「ローカルの履歴書き換えはクリック」). Read off the row (`GraphModel.publishedAt`), so it is
    // in hand as the menu opens (デザイン規約 §行が読む答えはどこから来るか).
    property bool menuPublished: false
    /// What these menus offer, held still while they stand. The rules are core's (`offers::commit_menu`).
    property bool menuCanSequence: false
    property bool menuCanIntegrate: false
    property bool menuCanEditHistory: false
    property bool menuCanMoveBranch: false
    property bool menuCanBranchHere: false
    property bool menuStashCanWrite: false
    /// Whether the row's name is somewhere to move to, and whether the press asks first (`offers::ref_menu`).
    property bool menuCanSwitch: false
    property bool menuSwitchAsks: false
    /// …or leads to another worktree instead: the name the one holding that name goes by, empty when none does
    /// (`RefRowMenu.heldLeaf`).
    property string menuHeldLeaf: ""
    /// That worktree is the repository's own (`RefRowMenu.heldHome`).
    property bool menuHeldHome: false
    /// What the `pull` row runs on that name and why it is greyed — the same `offers::pull_row` ask as
    /// `RefRowMenu.pullRuns` / `pullOut`.
    property string menuPullRuns: ""
    property string menuPullOut: ""
    property string menuPullUpstream: ""
    /// How many files the `--hard` row would take besides the commits — held still with the rest, since its tag
    /// widens the card (デザイン規約 §メニュー).
    property int menuHardResetTakes: 0
    /// Whether those files hold one the discard record cannot copy — held still for the same reason: it dresses the
    /// row.
    property bool menuHardResetNotCopied: false
    /// Whether anything besides this branch still reaches the tip, which decides whether the drop row is held. Held
    /// still too: a hold mark arriving later re-indents every row (`AppMenu.holdIndent`) — but for the menu standing
    /// again after a command it waited for, where the press count the commit now needs wins over the words' x.
    property bool menuTipHeldElsewhere: false

    /// What the WORKTREE card's two making rows stand on (`RefWorktreeMenu.standOn`'s `making`), asked of the commit
    /// and the name the menu is aimed at (`offers::worktree_rows`): any commit takes a new branch; a free branch, or a
    /// remote one with no local branch, goes out as it is — to the folder `newWorktreeFor` names, looked at here as the
    /// menu opens, the row greyed where something is already there.
    function askWorktreeRows(kind, name, oidHex) {
        const local = kind === "remote" ? menuState.repoTab.localNameFor(name) : name
        const offers = GitFacts.worktreeOffers(
            kind, name, oidHex,
            menuState.repoTab.state === "open",
            menuState.tree.branch,
            kind === "branch" || kind === "remote" ? menuState.worktreesModel.worktreeHolding(local) : "",
            kind === "remote" && menuState.branchesModel.oidOfName(local) !== "")
        const checkout = offers.includes("checkout-branch") ? "branch"
                       : offers.includes("checkout-track") ? "track" : ""
        const place = checkout === "" ? undefined : menuState.worktreesModel.newWorktreeFor(local)
        return {
            "oid": oidHex,
            // Only where worktrees have a place to go: a box whose Enter could make nothing is not offered.
            "here": offers.includes("here") && menuState.worktreesModel.worktreesPlaced(),
            // No place to name (the listing not in yet): no row, rather than one that cannot say where it goes.
            "checkout": place === undefined ? "" : checkout,
            "branch": place === undefined ? "" : local,
            "start": checkout === "track" ? name : "",
            "path": place === undefined ? "" : place.path,
            "place": place === undefined ? "" : place.name,
            "taken": place === undefined ? "" : place.taken
        }
    }

    /// What the `switch` and `pull` rows read, asked of the name the menu is aimed at. Empty on a row that draws none,
    /// which is what takes both rows off the menu.
    function askRefRows(kind, name, oidHex) {
        menuState.menuCanSwitch = false
        menuState.menuSwitchAsks = false
        menuState.menuHeldLeaf = ""
        menuState.menuHeldHome = false
        menuState.menuPullRuns = ""
        menuState.menuPullOut = ""
        menuState.menuPullUpstream = ""
        if (kind !== "branch" && kind !== "remote" && kind !== "worktree")
            return
        const held = menuState.worktreeLeadingFrom(kind, name)
        const offers = GitFacts.refMenuOffers(
            kind, name, oidHex,
            menuState.repoTab.state === "open",
            menuState.tree.branch, menuState.tree.detached,
            menuState.tree.opText, menuState.tree.conflictCount,
            // No drift: the rows that reach a remote are the card's (`RefBranchMenu`).
            held, "", false, false, menuState.repoTab.defaultRemote, "")
        menuState.menuCanSwitch = offers.includes("switch")
        menuState.menuSwitchAsks = offers.includes("asks")
        // As its WORKTREES row names it (`NavSectionModel.worktreeCalled`).
        menuState.menuHeldLeaf = held === "" ? "" : menuState.worktreesModel.worktreeCalled(held)
        menuState.menuHeldHome = held !== "" && menuState.worktreesModel.worktreeHome(held)
        menuState.menuPullUpstream = menuState.branchesModel.upstreamOf(name)
        const pull = GitFacts.pullRow(
            kind, name, menuState.repoTab.state === "open",
            menuState.tree.branch, menuState.tree.detached, menuState.tree.opText,
            menuState.tree.upstream, menuState.tree.pullBlocked,
            held, menuState.menuPullUpstream, menuState.branchesModel.aheadOf(name))
        menuState.menuPullRuns = pull.length > 0 ? pull[0] : ""
        menuState.menuPullOut = pull.length > 1 ? pull[1].replace("out-", "") : ""
    }

    /// The other worktree the menu's name leads to: the one holding the branch (a remote row through the local
    /// branch of the same name), or the worktree a folder's chip names — unless this tab stands in it. Empty for none.
    function worktreeLeadingFrom(kind, name) {
        if (kind === "branch")
            return menuState.worktreesModel.worktreeHolding(name)
        if (kind === "remote")
            return menuState.worktreesModel.worktreeHolding(menuState.repoTab.localNameFor(name))
        if (kind === "worktree" && !GitFacts.samePath(name, menuState.repoTab.repoPath))
            return name
        return ""
    }

    /// What the cards at the foot stand on, aimed at the menu's name — each emptied where that name is another's
    /// kind, so that card holds only what can still be made here. The BRANCH card stands where its branch points,
    /// `aimAt` (`aimedAt`). The WORKTREE card stands on the worktree a folder's chip names, or on the one holding the
    /// chip's local branch — not a remote chip's, which only leads there (`RefRowMenu.offerOn`) — and on what its
    /// making rows would make (`askWorktreeRows`). Read here, where the models are.
    function cardFacts(oidHex, aimAt) {
        const kind = menuState.menu.targetKind
        const branchy = kind === "branch" || kind === "remote"
        const name = menuState.menu.targetName
        const worktree = kind === "worktree" ? name : kind === "branch" ? menuState.worktreeLeadingFrom(kind, name) : ""
        return {
            "branch": menuState.branchFacts(branchy ? kind : "", branchy ? name : "", aimAt),
            "tag": menuState.tagFacts(kind === "tag" ? "tag" : "", kind === "tag" ? name : "", oidHex),
            "worktree": worktree === "" ? undefined : menuState.worktreesModel.worktreeFacts(worktree),
            "worktreeHere": GitFacts.samePath(worktree, menuState.repoTab.repoPath),
            "making": menuState.askWorktreeRows(kind, name, oidHex)
        }
    }

    /// What the BRANCH card stands on — the same answer `RefRowMenu.branchFacts` gives at the other entrance.
    function branchFacts(kind, full, oidHex) {
        if (kind !== "branch" && kind !== "remote")
            return { "heldByWorktree": "", "holderLeaf": "", "holderHome": false,
                     "remoteCounterpart": "", "remoteCounterpartOid": "",
                     "remoteDrifted": false, "offers": [], "open": false, "merged": "" }
        const held = kind === "branch"
            ? menuState.worktreesModel.worktreeHolding(full)
            : menuState.worktreesModel.worktreeHolding(menuState.repoTab.localNameFor(full))
        const counterpart = kind === "branch" ? menuState.branchesModel.upstreamOf(full) : ""
        const drifted = kind === "branch" && menuState.branchesModel.upstreamDrifted(full)
        // The one it is configured with, a gone one too, which names no reading.
        const upstream = kind === "branch" ? menuState.branchesModel.upstreamNamed(full) : ""
        const open = menuState.repoTab.state === "open"
        return {
            "heldByWorktree": held,
            "holderLeaf": held === "" ? "" : menuState.worktreesModel.worktreeCalled(held),
            "holderHome": held !== "" && menuState.worktreesModel.worktreeHome(held),
            "remoteCounterpart": counterpart,
            "upstream": upstream,
            "upstreamWhere": upstream === "" ? "" : menuState.branchesModel.upstreamWhere(full, menuState.tree.branch),
            "remoteCounterpartOid": counterpart === "" ? "" : menuState.branchesModel.upstreamOidOf(full),
            "remoteDrifted": drifted,
            "open": open,
            "push": kind === "branch"
                    ? menuState.branchesModel.branchPush(full, menuState.repoTab.defaultRemote) : undefined,
            "merged": !open || kind !== "branch" ? "" : menuState.graphModel.branchDeleteMerged(
                oidHex, menuState.branchesModel.upstreamOidOf(full), menuState.tree.headOid),
            "offers": GitFacts.refMenuOffers(
                kind, full, oidHex, open,
                menuState.tree.branch, menuState.tree.detached,
                menuState.tree.opText, menuState.tree.conflictCount,
                held, counterpart, drifted, upstream !== "", menuState.repoTab.defaultRemote, "")
        }
    }

    /// What the TAG card stands on — the same answer `RefRowMenu.tagFacts` gives at the other entrance. A chip drawing
    /// one remote's reading names that remote on its own, so the card acts on it (`NavSectionModel.tagAimAt`).
    function tagFacts(kind, full, oidHex) {
        if (kind !== "tag")
            return { "pushRemote": menuState.repoTab.defaultRemote, "offers": [] }
        const menu = menuState.tagsModel.tagMenu(full, menuState.repoTab.defaultRemote,
                                                 menuState.tagsModel.tagAimAt(full, oidHex))
        const sides = menuState.tagsModel.tagSides(full)
        return {
            "pushRemote": menu.pushRemote,
            "tagDriftOid": menu.lease,
            "tagReach": menu.reach,
            "tagHeldBack": menu.heldBack,
            "tagCarriers": menu.carriers,
            "tagOnlyThere": menu.rowGoes,
            "tagHere": sides === "here" || sides === "both",
            "offers": GitFacts.refMenuOffers(
                kind, full, oidHex,
                menuState.repoTab.state === "open",
                menuState.tree.branch, menuState.tree.detached,
                menuState.tree.opText, menuState.tree.conflictCount,
                "", "", menu.heldBack !== "", false, menu.pushRemote, sides)
        }
    }

    /// Delete the remote branch alone, or both at once — the ref split against the tab's configured remote names
    /// (`RefRowMenu.deleteRemoteNow`), leased to `expect`.
    function deleteRemoteNow(remoteRef, expect) {
        const remote = GitFacts.remoteOfRef(remoteRef, menuState.repoTab.remoteNames)
        if (remote === "")
            return
        menuState.repoTab.deleteRemoteBranch(
            remote, GitFacts.branchOfRef(remoteRef, menuState.repoTab.remoteNames), expect)
    }

    function deleteEverywhereNow(branch, remoteRef, forced, expect) {
        const remote = GitFacts.remoteOfRef(remoteRef, menuState.repoTab.remoteNames)
        if (remote === "")
            return
        menuState.repoTab.deleteBranchEverywhere(
            branch, remote, GitFacts.branchOfRef(remoteRef, menuState.repoTab.remoteNames), forced, expect)
    }

    /// The menu on another worktree's uncommitted row (デザイン規約 §別の worktree を読む), at `path` as the row names
    /// it — read off the tree as `openRowMenu` reads it, and greyed while a command is out as every menu is.
    function openCarriedMenu(path, name) {
        const menu = menuState.menu
        menu.releaseShape()
        menuState.tree = menuState.treeAtAsk !== null ? menuState.treeAtAsk : menuState.worktree
        menu.carriedPath = path
        menu.carriedName = name
        menu.carriedHome = menuState.graphModel.carriedHome(menuState.graphModel.carriedRowOf(path))
        menuState.standCarried()
        menu.offerCarried()
    }
    /// Whether that worktree's work can be applied here: not where a conflict stands on either side — there it leaves
    /// no telling which side to bring, and git applies nothing over one here. Both are already on screen, the row's
    /// tally and the band's badge (デザイン規約 §メニュー「選べない行は消す」). False where the row is gone.
    function standCarried() {
        const menu = menuState.menu
        const tally = menuState.graphModel.carriedTally(menuState.graphModel.carriedRowOf(menu.carriedPath))
        menu.carriedCanApply = menuState.repoTab.state === "open" && tally !== undefined
            && tally.conflicted === 0 && menuState.tree.conflictCount === 0
    }

    function openRowMenu(oidHex) {
        menuState.menu.releaseShape()
        menuState.tree = menuState.treeAtAsk !== null ? menuState.treeAtAsk : menuState.worktree
        const facts = menuState.standRow(oidHex, oidHex)
        if (facts === null)
            menuState.menu.offerStash()
        else
            menuState.menu.offerCommit(facts)
    }

    /// The command the menu waited for has ended with the menu up: what it left is read again and the rows stand on
    /// it, the menu keeping its shape (`AppMenu.holdShape`) — the stash's selector the row now answers to, the notes
    /// and the presses off where HEAD now is, the BRANCH card off where its branch now points, the rows an operation it
    /// stopped in takes away greyed. A row the command took off the graph (a stash popped, a commit dropped), or the
    /// name the menu was aimed at taken away, leaves the menu nothing to act on.
    function standAgain() {
        const menu = menuState.menu
        if (menu === null || !menu.showing)
            return
        // Another worktree's row: gone where that worktree went clean, its apply read again otherwise.
        if (menu.carriedShowing) {
            if (menuState.graphModel.carriedRowOf(menu.carriedPath) < 0) {
                menu.withdraw()
                return
            }
            menu.holdShape()
            menuState.tree = menuState.worktree
            menuState.standCarried()
            return
        }
        const wasStash = menuState.menuStashRef !== ""
        const aimAt = menuState.aimedAt()
        if (menuState.graphModel.rowOf(menuState.menuOid) < 0 || aimAt === ""
                || (menuState.graphModel.stashRefOf(menuState.menuOid) !== "") !== wasStash) {
            menu.withdraw()
            return
        }
        menu.holdShape()
        menuState.tree = menuState.worktree
        const facts = menuState.standRow(menuState.menuOid, aimAt)
        if (facts !== null)
            menu.standCards(facts, aimAt)
    }
    /// Where the name the menu was aimed at points now, empty where it is gone: a local branch may have moved off the
    /// row (a commit takes the current branch with it) and its card goes with it. A remote branch that moved counts as
    /// gone: its `push --delete` is leased to where it stood as the menu went up, and stood again on the new tip it
    /// would take commits the menu never showed. The rest stand on the row's commit — a tag's chip may be one remote's
    /// reading, and the WORKTREE card's rows act on the commit.
    function aimedAt() {
        const kind = menuState.menu.targetKind
        const name = menuState.menu.targetName
        if (kind === "branch")
            return menuState.branchesModel.oidOfName(name)
        if (kind === "remote")
            return menuState.remotesModel.oidOfName(name) === menuState.menuOid ? menuState.menuOid : ""
        const there = kind === "tag" ? menuState.tagsModel.oidOfName(name) !== ""
                    : kind !== "worktree" || menuState.worktreesModel.worktreeFacts(name) !== undefined
        return there ? menuState.menuOid : ""
    }

    /// Reads and freezes everything the rows stand on, for the row `oidHex` with its name at `aimAt`: what the cards
    /// stand on, or null for a stash's row, which has no cards.
    function standRow(oidHex, aimAt) {
        menuState.menuOid = oidHex
        menuState.menuStashRef = menuState.graphModel.stashRefOf(oidHex)
        // No running command is asked about: the menu waits for one with its rows greyed (`AppMenu.runningWhy`), so it
        // has one shape whenever it is opened.
        const offers = GitFacts.commitMenuOffers(
            menuState.repoTab.state === "open",
            menuState.tree.branch, menuState.tree.detached, menuState.tree.opText,
            oidHex, menuState.tree.headOid, menuState.menuStashRef)
        if (menuState.menuStashRef !== "") {
            menuState.menuStashCanWrite = offers.includes("stash-write")
            return null
        }
        menuState.menuPublished = menuState.graphModel.publishedAt(oidHex)
        menuState.menuHardResetTakes = menuState.tree.hardResetTakes
        menuState.menuHardResetNotCopied = menuState.tree.hardResetNotCopied
        menuState.menuTipHeldElsewhere = menuState.tree.headReachedElsewhere
        menuState.menuCanSequence = offers.includes("sequence")
        menuState.menuCanIntegrate = offers.includes("integrate")
        menuState.menuCanEditHistory = offers.includes("edit-history")
        menuState.menuCanMoveBranch = offers.includes("move-branch")
        menuState.menuCanBranchHere = offers.includes("branch-here")
        menuState.askRefRows(menuState.menu.targetKind, menuState.menu.targetName, aimAt)
        return menuState.cardFacts(oidHex, aimAt)
    }
}

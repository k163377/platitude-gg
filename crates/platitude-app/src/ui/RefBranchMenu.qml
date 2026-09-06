import QtQuick
import platitude
import platitude.ui

// Everything a branch's name answers for once it exists, behind the branch's own mark: the delete here, the delete over
// there, and the pair at once (デザイン規約 §メニュー の入れ子). Starting a branch is not one of them — that is where the
// reader carries on from, and it stands on the level above with the other moves.
//
// **One card, two entrances.** The chip on a graph row and the row itself are two ways at the same branch, and what
// they offer has to be the same thing — so the card is a component, and it works out its own
// answers rather than being handed them: `offerOn` freezes them as the menu opens, the way every other menu freezes
// what it shows (デザイン規約 §メニュー).
AppMenu {
    id: branchCard

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    /// Which remote reading a branch speaks for (`upstreamOf`), and which other working copy has one checked out
    /// (`worktreeHolding`) — the lists live in those sections alone.
    required property NavSectionModel branchesModel
    required property NavSectionModel worktreesModel
    /// The drawn rows, which answer whether the branch is merged as the menu opens (`reaches`).
    required property GraphModel graphModel

    /// The branch this card is about, frozen by `offerOn`. `kind` is `branch` or `remote`; empty on a row that names
    /// no branch, which is what takes the card off the menu.
    readonly property alias kind: state.kind
    readonly property alias refId: state.refId
    readonly property alias refName: state.refName
    /// Whether the everyday delete's answer was in hand as the card opened, off the graph's rows, and what it was
    /// — the automation reads them (`delete-branch-early`); the row reads `refusedRow`.
    readonly property alias deleteAnswered: state.deleteAnswered
    readonly property alias deleteMerged: state.deleteMerged
    /// Whether the delete row has an answer coming at all — in hand off the rows, or asked of git. **False is a card
    /// standing over a row nobody asked about**: the delete is out (the branch the tree is on, one another working
    /// copy holds, anything running), and this is frozen as the card opens like the rest of it, so no answer is
    /// coming later either. The automation reads it as the answer to "did the input land"
    /// (app-ui.md §UI 自動化の因果性); nothing on screen needs it.
    readonly property alias deleteAsked: state.deleteAsked

    /// The branch git has just refused to delete, while the card that asked is still standing — so the delete row
    /// turns into the held `-D` where the hand already is (デザイン規約 §左メニューの所作). Written by the card itself
    /// off git's answer (`refusedDelete`), and cleared as the card next opens.
    property string forceDeleteBranch: ""
    /// git's answer to the plain delete this card stayed up for, read off the tab by the card's own name — the same
    /// place the early check's answer already comes from (`RepoTab.branchDeleteRefused` / `branchDeleteLanded`), and
    /// standing there until the next delete is asked, so a fetch answering in the same drain cannot take it down
    /// before this card has seen it. **The card answers for itself**: refused, its row turns into the held `-D`;
    /// taken, there is nothing left to catch and the card goes, and the menu it hangs off with it (`dismiss()` is
    /// Qt's, and walks every level down). Whichever of the two entrances raised this card, it is the one standing,
    /// so nothing above it has to know which one asked (app-ui.md §メニューを閉じるのは自分).
    readonly property string refusedDelete: branchCard.repoTab.branchDeleteRefused
    readonly property string landedDelete: branchCard.repoTab.branchDeleteLanded

    /// What the page answers for: the delete git may still refuse opens a question there, and the row taken off the
    /// list ahead of git's answer is the page's list (デザイン規約 §消す操作は先に画面から消す).
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal deleting(string kind, string id)
    /// Which remote branch this one is measured against — the page opens the question, because the bar it stands in
    /// is the one every other question in the window stands in (`UpstreamFlow`). The remote branch this one already
    /// speaks for goes with it: that is where the question opens, and it is read here while the row still answers to
    /// its own name.
    signal upstreamRequested(string branch, string counterpart)

    /// The automation's handles into these rows, passed on by whichever menu carries the card (app-ui.md).
    readonly property alias deleteItem: refDeleteItem
    readonly property alias deleteRemoteItem: refRemoteDeleteItem
    readonly property alias deleteBothItem: refBothDeleteItem
    readonly property alias upstreamItem: refUpstreamItem

    /// Everything this card reads, worked out once as the menu opens and left alone while it stands. `kind` empty
    /// takes the card with it, which is how a row that names no branch — a tag, a stash, a commit nothing points at —
    /// gets the same answer from either entrance.
    QtObject {
        id: state
        property string kind: ""
        property string refId: ""
        property string refName: ""
        property string refOid: ""
        property string remoteCounterpart: ""
        /// That reading is standing on another commit, so the two rows that reach it say why instead of running.
        property bool remoteDrifted: false
        property string heldByWorktree: ""
        property bool canDelete: false
        property bool canDeleteRemote: false
        property bool canSetUpstream: false
        property bool onCurrentBranch: false
        /// git's safety valve, answered off the rows as the card opens: whether an answer is in hand, and whether the
        /// tip is reachable from the branch's reference point. Merged until answered, so a card still waiting on git
        /// wears the plain row.
        property bool deleteAnswered: false
        property bool deleteMerged: true
        /// Whether anybody was asked (see the alias).
        property bool deleteAsked: false
    }

    function offerOn(kind, name, full, oidHex) {
        branchCard.forceDeleteBranch = ""
        state.kind = kind
        state.refName = name
        state.refId = full
        state.refOid = oidHex
        state.deleteAnswered = false
        state.deleteMerged = true
        state.deleteAsked = false
        if (kind !== "branch" && kind !== "remote") {
            state.remoteCounterpart = ""
            state.remoteDrifted = false
            state.heldByWorktree = ""
            state.canDelete = false
            state.canDeleteRemote = false
            state.canSetUpstream = false
            state.onCurrentBranch = false
            return
        }
        // A remote row lands on the local branch of the same name, so it is that one another copy can be holding.
        state.heldByWorktree = kind === "branch"
            ? branchCard.worktreesModel.worktreeHolding(full)
            : branchCard.worktreesModel.worktreeHolding(branchCard.repoTab.localNameFor(full))
        state.remoteCounterpart = kind === "branch" ? branchCard.branchesModel.upstreamOf(full) : ""
        // And whether that reading is standing where this branch is. **Drifted, the two rows that reach it are out**
        // — what a delete over there would take away is not what this row stands on, and the reading has a row of its
        // own where it does stand (デザイン規約 §左メニューの所作 の削除の表).
        state.remoteDrifted = kind === "branch" && branchCard.branchesModel.upstreamDrifted(full)
        // The lookups above are the models'; what the rows may offer on them is core's rule, with the measured
        // refusals it encodes — the current branch keeps its delete table and says why (offers::ref_menu).
        const offers = GitFacts.refMenuOffers(
            kind, full, oidHex,
            // Nothing running, while the doors are held: the hold answers for every row of this card, and a row the
            // busy count took away would be a row the reader never sees come back (`RefRowMenu.askBusy`).
            branchCard.repoTab.state === "open",
            branchCard.heldReason !== "" ? 0 : branchCard.repoTab.busyCount,
            branchCard.workTree.branch, branchCard.workTree.detached,
            branchCard.workTree.opText, branchCard.workTree.conflictCount,
            state.heldByWorktree, state.remoteCounterpart, state.remoteDrifted,
            branchCard.repoTab.defaultRemote, "").split(" ")
        state.canDelete = offers.includes("delete")
        state.canDeleteRemote = offers.includes("delete-remote")
        state.canSetUpstream = offers.includes("set-upstream")
        state.onCurrentBranch = offers.includes("current")
        // Whether the everyday delete would be refused, answered as the menu opens so the delete row wears `-D` from
        // the start instead of only after a refused click (§左メニューの所作). Put to the graph's rows first — in hand
        // in the same frame for every branch the window draws, with git's own reference point worked out on that side
        // (`GraphModel.branchDeleteMerged`, 規約 §行が読む答えはどこから来るか) — and to git only for a tip or a
        // reference older than the window, whose answer lands as `branchDeleteAsked` / `branchDeleteMerged`. The chip
        // column is settled at open, so the swap moves no other row.
        if (branchCard.repoTab.state === "open" && kind === "branch" && state.canDelete) {
            const merged = branchCard.graphModel.branchDeleteMerged(
                oidHex, branchCard.branchesModel.upstreamOidOf(full), branchCard.workTree.headOid)
            if (merged !== "") {
                state.deleteMerged = merged === "yes"
                state.deleteAnswered = true
            } else {
                branchCard.repoTab.checkBranchDelete(full)
            }
            state.deleteAsked = true
        }
    }

    /// Whether an answer that names a branch is about the one this card stands on.
    function answersHere(name) {
        return name !== "" && state.kind === "branch" && name === state.refId
    }

    onRefusedDeleteChanged: {
        if (branchCard.answersHere(branchCard.refusedDelete))
            branchCard.forceDeleteBranch = branchCard.refusedDelete
    }
    onLandedDeleteChanged: {
        if (branchCard.answersHere(branchCard.landedDelete))
            branchCard.dismiss()
    }

    /// The remote reading gone without touching the local branch. The configured names say where the cut is (a
    /// remote's own name may contain `/`); an unconfigured remote still cuts at the first slash, so the press acts and
    /// git answers (`GitFacts.remoteOfRef`).
    function deleteRemoteNow(remoteRef) {
        const remote = GitFacts.remoteOfRef(remoteRef, branchCard.repoTab.remoteNames)
        if (remote === "")
            return
        branchCard.deleting("remote", remoteRef)
        branchCard.repoTab.deleteRemoteBranch(
            remote, GitFacts.branchOfRef(remoteRef, branchCard.repoTab.remoteNames))
    }

    /// Why the delete table's rows are out — worn as the rows' `blockedReason` (デザイン規約 §無効).
    readonly property string deleteBlockedOnCurrent:
        qsTr("Switch away first — this is the branch you are on")
    readonly property string deleteBlockedWhileBusy: Words.otherCommandRunning
    // Why git keeps a branch to one working copy is the causal half, and the tooltip rule drops it (デザイン規約 §hover の
    // ツールチップ) — what is left is the state that blocks the row and the one thing the menu cannot show: where. The
    // whole path is what git answers with and is the only unambiguous form, but nobody reads a tooltip that wide.
    //: %1 is the folder of the other working copy that has this branch checked out.
    readonly property string blockedByWorktree:
        qsTr("Checked out in another working copy — %1").arg(GitFacts.pathLeaf(state.heldByWorktree))

    titleKind: "branch"
    titleTint: Theme.accent
    title: qsTr("BRANCH")
    // Only a row that names a branch has any of this, so only such a row puts the card up.
    applies: state.kind === "branch" || state.kind === "remote"

    // Which remote branch this one is measured against — the counts beside it, the point `branch --delete` calls
    // merged, and where it pushes when nothing is marked all come off this one setting (デザイン規約
    // §ブランチが測られる相手を決める).
    //
    // **No chip, and a `…`**: what runs is `branch --set-upstream-to=<答え>`, and the flag's value is not settled
    // until the question has been answered — so the row is no more 1:1 with a command than `Create branch here…` is
    // (§git 用語のコード表記). The spelling would not be the short one either: a menu row wears the long form, and
    // `branch --set-upstream-to` in the shared chip column would push every delete row's name across for a row that
    // is not even a command yet. Above the deletes with a line between, since a table of things that take a name away
    // is not where a row that only writes configuration belongs (§メニュー の入れ子).
    //
    // Only a local branch: a remote-tracking ref is the far side of somebody's setting and has none of its own.
    AppMenuItem {
        id: refUpstreamItem
        text: qsTr("Set upstream…")
        offered: state.kind === "branch" && state.canSetUpstream
        onTriggered: branchCard.upstreamRequested(state.refId, state.remoteCounterpart)
    }
    AppMenuSeparator {}
    AppMenuItem {
        id: refDeleteItem
        // Alone in this menu the delete re-states its target: during the hold the name of what is about to go has to
        // be readable on the row itself (デザイン規約 §メニュー 言い直さない、の例外).
        readonly property bool remoteRow: state.kind === "remote"
        readonly property bool branchRow: state.kind === "branch"
        // git already refused `--delete` while this menu stood — or the check run at open came back unmerged, the same
        // answer a click ahead of time (§左メニューの所作): the rows' own where they could say, git's echo otherwise.
        // **Only `"no"` dresses the row.** The echo also carries `"unknown"` for reads that fell over, and that is a
        // delete nobody has shown to be refused — offered plain, with git answering the press.
        readonly property bool refusedRow:
            branchRow
            && (branchCard.forceDeleteBranch === state.refId
                || (state.deleteAnswered ? !state.deleteMerged
                    : (branchCard.repoTab.branchDeleteAsked === state.refId
                       && branchCard.repoTab.branchDeleteMerged === "no")))
        readonly property bool heldRow: remoteRow || refusedRow
        code: refusedRow ? "branch -D"
            : branchRow ? "branch --delete"
            : "push --delete"
        // The name is data, not sentence: never translated, and it does not bid for the menu's width
        // (`growsForText`).
        text: state.refId
        growsForText: false
        note: refusedRow ? qsTr("not merged") : ""
        // On a branch the three delete forms are a fixed table — rows that cannot be chosen stay and grey out, the
        // app-menu rule rather than the assembled-menu one (デザイン規約 §メニュー、by design): the current branch
        // keeps its rows, saying why nothing here answers. The other kinds keep the assembled rule.
        offered: branchRow || (heldRow && state.canDelete)
        blockedReason: !branchRow || state.canDelete ? ""
                     : state.onCurrentBranch ? branchCard.deleteBlockedOnCurrent
                     : state.heldByWorktree !== "" ? branchCard.blockedByWorktree
                     : branchCard.deleteBlockedWhileBusy
        holdMs: heldRow ? Metrics.holdMs : 0
        // A branch's plain delete keeps the menu up: git's answer has nowhere to land otherwise, and this row is where
        // it lands.
        staysOpen: branchRow
        // Reaching past this machine is the warning tone; throwing away what is in hand is danger (デザイン規約 §状態).
        holdTone: remoteRow ? Theme.warning : Theme.danger
        onPicked: branchCard.deleteRequested(state.kind, state.refId, state.refName, state.refOid)
        onHeld: {
            branchCard.dismiss()
            if (remoteRow) {
                branchCard.deleteRemoteNow(state.refId)
            } else {
                branchCard.deleting("branch", state.refId)
                branchCard.repoTab.deleteBranch(state.refId, true)
            }
        }
    }
    // The branch's remote reading, deleted without touching the local one — on the current branch the one delete on
    // offer at all (デザイン規約 §左メニューの所作).
    AppMenuItem {
        id: refRemoteDeleteItem
        code: "push --delete"
        text: state.remoteCounterpart
        growsForText: false
        // In the table only while the branch has a remote reading at all: a row for a target that does not exist keeps
        // no seat. Grey is for "there is one, but not to press now" — busy, or standing on another commit — not for
        // "no such thing" (デザイン規約 §左メニューの所作 の削除の表).
        offered: state.kind === "branch" && state.remoteCounterpart !== ""
        blockedReason: state.canDeleteRemote ? ""
                     : state.remoteDrifted ? Words.remoteOnAnotherCommit
                     : branchCard.deleteBlockedWhileBusy
        holdMs: Metrics.holdMs
        holdTone: Theme.warning
        onHeld: {
            branchCard.dismiss()
            branchCard.deleteRemoteNow(state.remoteCounterpart)
        }
    }
    // A composite of two commands is no one command, so words rather than a chip (§git 用語のコード表記 の 1:1 規則). The local
    // half runs first and a refusal stops the pair with nothing touched.
    AppMenuItem {
        id: refBothDeleteItem
        text: qsTr("Delete both")
        note: refDeleteItem.refusedRow ? qsTr("not merged") : ""
        offered: state.kind === "branch" && state.remoteCounterpart !== ""
        // The local half's refusal names the row first — it is the half that runs first — and the drifted reading is
        // read after it.
        blockedReason: state.canDelete && state.canDeleteRemote ? ""
                     : state.onCurrentBranch ? branchCard.deleteBlockedOnCurrent
                     : state.heldByWorktree !== "" ? branchCard.blockedByWorktree
                     : state.remoteDrifted ? Words.remoteOnAnotherCommit
                     : branchCard.deleteBlockedWhileBusy
        holdMs: Metrics.holdMs
        // The colour of the half that decides: reaching past this machine is warning, but once the local half runs as
        // `-D` this row throws away commits that live nowhere else, and that is danger (デザイン規約 §状態).
        holdTone: refDeleteItem.refusedRow ? Theme.danger : Theme.warning
        onHeld: {
            branchCard.dismiss()
            const c = state.remoteCounterpart
            const remote = GitFacts.remoteOfRef(c, branchCard.repoTab.remoteNames)
            if (remote === "")
                return
            // Both halves go at once: the pair is one write with one answer, so it is one thing to put back.
            branchCard.deleting("branch", state.refId)
            branchCard.deleting("remote", c)
            branchCard.repoTab.deleteBranchEverywhere(
                state.refId, remote,
                GitFacts.branchOfRef(c, branchCard.repoTab.remoteNames),
                refDeleteItem.refusedRow)
        }
    }
}

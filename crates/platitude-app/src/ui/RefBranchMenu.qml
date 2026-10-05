import QtQuick
import platitude.ui

// The BRANCH card: set upstream and the delete table — local, remote, both (デザイン規約 §メニュー の入れ子).
// Starting a branch stands on the level above with the other moves.
//
// **One card, two entrances** (`RefRowMenu` / `CommitMenuState`), so both offer the same thing. **It reads no model
// and runs no write**: the entrance reads the facts (`branchFacts`), `standOn` freezes them as the menu opens, and
// the card signals what was pressed (rules/app-ui.md の「配線」). What is decided here is the table — which row keeps
// a seat, which greys and with which sentence, which gesture and colour each takes. Standing on plain values is what
// lets `tests/qml/tst_branchcard.qml` drive the real card, the only place the sentences are checked (see `holderLeaf`).
AppMenu {
    id: branchCard

    /// git's answers to the plain delete this card stayed up for, handed down by the carrying menu. Live, not frozen:
    /// they land while the card stands (`forceDeleteBranch`).
    property string refusedDelete: ""
    property string landedDelete: ""
    /// The branch the early check was asked about and what git answered for it (`yes` / `no` / `unknown`, empty for
    /// nobody asked) — `RepoTab.branchDeleteAsked` / `branchDeleteMerged` at the entrance.
    property string checkedBranch: ""
    property string checkedMerged: ""

    /// The branch this card is about, frozen by `standOn`. `kind` is `branch` or `remote`; empty on a row that names
    /// no branch, which is what takes the card off the menu.
    readonly property alias kind: state.kind
    readonly property alias refId: state.refId
    readonly property alias refName: state.refName
    /// Whether the plain delete's answer was in hand off the graph's rows as the card opened, and what it was. For the
    /// automation (`delete-branch-early`); the row reads `refusedRow`.
    readonly property alias deleteAnswered: state.deleteAnswered
    readonly property alias deleteMerged: state.deleteMerged
    /// Whether the delete row has an answer coming at all (off the rows, or asked of git). False when the delete is
    /// out (current branch, held by another working copy, busy); frozen at open, so no answer comes later either.
    /// Only the automation reads it, as "did the input land".
    readonly property alias deleteAsked: state.deleteAsked
    /// The folder the other working copy stands in, as the blocked row names it. The automation reads this as the
    /// witness that the copy was looked up.
    readonly property alias holderLeaf: state.holderLeaf

    /// The branch git has just refused to delete while this card stands, so the delete row turns into the held `-D`
    /// where the hand already is (デザイン規約 §左メニューの所作). Written off `refusedDelete`, cleared at the next open.
    /// The card answers for itself — refused, the row turns; landed, it `dismiss()`es with every level above — so
    /// neither entrance has to know which one raised it (rules-refs/app-ui.md「メニューを閉じるのは自分」).
    property string forceDeleteBranch: ""

    /// The plain delete, for the page (a refusal opens a question there). The row leaves the list ahead of git's
    /// answer out of the same slot as the write, so the card writes no page state
    /// (`ops_delete`, デザイン規約 §消す操作は先に画面から消す).
    signal deleteRequested(string kind, string id, string name, string oidHex)
    /// Set upstream: the page opens the question in the window's one question bar (`UpstreamFlow`), starting from
    /// `counterpart`, the remote branch this one already reads.
    signal upstreamRequested(string branch, string counterpart)
    /// Requests to the carrying menu, which holds the tab (rules/app-ui.md の「配線」) and cuts a ref path into remote
    /// and branch against the configured remotes: the early `merged` check, the held `-D` after a refusal, the remote
    /// delete alone, and the pair.
    signal checkDeleteRequested(string branch)
    signal forceDeleteRequested(string branch)
    signal deleteRemoteRequested(string remoteRef, string expect)
    signal deleteEverywhereRequested(string branch, string remoteRef, bool forced, string expect)

    /// The automation's handles into these rows, passed on by the carrying menu.
    readonly property alias deleteItem: refDeleteItem
    readonly property alias deleteRemoteItem: refRemoteDeleteItem
    readonly property alias deleteBothItem: refBothDeleteItem
    readonly property alias upstreamItem: refUpstreamItem

    /// Everything this card reads, set once by `standOn` and left alone while it stands.
    QtObject {
        id: state
        property string kind: ""
        property string refId: ""
        property string refName: ""
        property string refOid: ""
        property string remoteCounterpart: ""
        /// The commit that reading showed as the menu opened: the lease of every delete that reaches it.
        property string remoteCounterpartOid: ""
        /// That reading stands on another commit, so the two rows that reach it say why.
        property bool remoteDrifted: false
        property string heldByWorktree: ""
        /// The leaf of the path git prints for that copy, cut at the entrance.
        property string holderLeaf: ""
        property bool canDelete: false
        property bool canDeleteRemote: false
        property bool canSetUpstream: false
        property bool onCurrentBranch: false
        /// The plain delete's merged check off the rows: whether answered, and whether the tip is reachable from the
        /// branch's reference point. Merged until answered, so a card still waiting on git wears the plain row.
        property bool deleteAnswered: false
        property bool deleteMerged: true
        property bool deleteAsked: false
    }

    /// Freezes the card on that branch. `facts` is what the carrying menu read for it: `heldByWorktree` the other
    /// working copy holding this row's local branch and `holderLeaf` its folder, `remoteCounterpart` the reading a
    /// local branch speaks for, `remoteCounterpartOid` its commit and `remoteDrifted` whether that stands on another
    /// commit, `offers` core's words,
    /// `open` whether the tab is, and `merged` what the drawn rows say about the plain delete (`yes` / `no`, empty
    /// for a tip or a reference older than the window, whose answer only git has).
    function standOn(kind, name, full, oidHex, facts) {
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
            state.remoteCounterpartOid = ""
            state.remoteDrifted = false
            state.heldByWorktree = ""
            state.holderLeaf = ""
            state.canDelete = false
            state.canDeleteRemote = false
            state.canSetUpstream = false
            state.onCurrentBranch = false
            return
        }
        state.heldByWorktree = facts.heldByWorktree
        state.holderLeaf = facts.holderLeaf
        state.remoteCounterpart = facts.remoteCounterpart
        state.remoteCounterpartOid = facts.remoteCounterpartOid
        state.remoteDrifted = facts.remoteDrifted
        // What the rows may offer is core's rule (`offers::ref_menu`).
        const offers = facts.offers
        state.canDelete = offers.includes("delete")
        state.canDeleteRemote = offers.includes("delete-remote")
        state.canSetUpstream = offers.includes("set-upstream")
        state.onCurrentBranch = offers.includes("current")
        // Whether the plain delete would be refused, answered at open so the row wears `-D` from the start: the drawn
        // rows answer first (規約 §行が読む答えはどこから来るか), git only where they cannot (→ `checkedMerged`).
        if (facts.open && kind === "branch" && state.canDelete) {
            if (facts.merged !== "") {
                state.deleteMerged = facts.merged === "yes"
                state.deleteAnswered = true
            } else {
                branchCard.checkDeleteRequested(full)
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

    /// Why the delete table's rows are out — worn as the rows' `blockedReason` (デザイン規約 §無効).
    readonly property string deleteBlockedOnCurrent:
        qsTr("Switch away first — this is the branch you are on")
    readonly property string deleteBlockedWhileBusy: Words.otherCommandRunning
    // The state and where, not why (デザイン規約 §hover のツールチップ). The folder, not the whole path git prints:
    // nobody reads a tooltip that wide.
    //: %1 is the folder of the other working copy that has this branch checked out.
    readonly property string blockedByWorktree:
        qsTr("Checked out in another working copy — %1").arg(state.holderLeaf)

    titleKind: "branch"
    titleTint: Theme.accent
    title: qsTr("BRANCH")
    applies: state.kind === "branch" || state.kind === "remote"

    // Set upstream (デザイン規約 §ブランチが測られる相手を決める). Words and a `…`, no code: the flag's value is not
    // settled until the question is answered (§git 用語のコード表記). Only a local branch: a remote-tracking ref has
    // no upstream of its own.
    AppMenuItem {
        id: refUpstreamItem
        text: qsTr("Set upstream…")
        offered: state.kind === "branch" && state.canSetUpstream
        onTriggered: branchCard.upstreamRequested(state.refId, state.remoteCounterpart)
    }
    AppMenuSeparator {}
    AppMenuItem {
        id: refDeleteItem
        // The delete names its target: during the hold, what is about to go has to be readable on the row itself
        // (デザイン規約 §メニュー「例外は削除行だけ」).
        readonly property bool remoteRow: state.kind === "remote"
        readonly property bool branchRow: state.kind === "branch"
        // git refused `--delete` while this menu stood, or the check at open came back unmerged (off the rows, else
        // git's echo). Only `"no"` dresses the row: `"unknown"` is a read that fell over, offered plain.
        readonly property bool refusedRow:
            branchRow
            && (branchCard.forceDeleteBranch === state.refId
                || (state.deleteAnswered ? !state.deleteMerged
                    : (branchCard.checkedBranch === state.refId
                       && branchCard.checkedMerged === "no")))
        readonly property bool heldRow: remoteRow || refusedRow
        /// `refusedRow` as latched at the press: on a branch row a hold length (`AppMenuItem.armedMs`) means refused.
        /// Read live, git's answer to the last press would re-word the row under the hand (デザイン規約 §長押し).
        readonly property bool shownRefused: refDeleteItem.branchRow && refDeleteItem.armedMs > 0
        code: shownRefused ? "branch -D"
            : branchRow ? "branch --delete"
            : "push --delete"
        // Data: untranslated, and it does not widen the menu (`growsForText`).
        text: state.refId
        growsForText: false
        note: shownRefused ? qsTr("not merged") : ""
        // On a branch the delete forms are a fixed table whose rows grey out rather than go
        // (デザイン規約 §メニュー「例外はブランチの削除の表」). The other kinds keep the assembled rule.
        offered: branchRow || (heldRow && state.canDelete)
        blockedReason: !branchRow || state.canDelete ? ""
                     : state.onCurrentBranch ? branchCard.deleteBlockedOnCurrent
                     : state.heldByWorktree !== "" ? branchCard.blockedByWorktree
                     : branchCard.deleteBlockedWhileBusy
        // Asked of the sentence this row shows, not of the state: a row blocked for another reason names no copy.
        reasonMarkWord: refDeleteItem.blockedReason === branchCard.blockedByWorktree ? state.holderLeaf : ""
        holdMs: heldRow ? Metrics.holdMs : 0
        // A branch's plain delete keeps the menu up: git's answer lands on this row.
        staysOpen: branchRow
        // `warning` both ways: a remote's name stands back up here, and a local branch's tip and upstream come back
        // from the discard record (デザイン規約 §長押し の色の表).
        holdTone: Theme.warning
        onPicked: branchCard.deleteRequested(state.kind, state.refId, state.refName, state.refOid)
        onHeld: {
            branchCard.dismiss()
            if (remoteRow) {
                // Leased to the commit this row showed (`remote::delete_remote_branch`).
                branchCard.deleteRemoteRequested(state.refId, state.refOid)
            } else {
                branchCard.forceDeleteRequested(state.refId)
            }
        }
    }
    // The remote reading alone — on the current branch the only delete on offer (デザイン規約 §左メニューの所作).
    AppMenuItem {
        id: refRemoteDeleteItem
        code: "push --delete"
        text: state.remoteCounterpart
        growsForText: false
        // Seated only while a remote reading exists; grey is "not now" — busy, or on another commit
        // (デザイン規約 §左メニューの所作 の削除の表).
        offered: state.kind === "branch" && state.remoteCounterpart !== ""
        blockedReason: state.canDeleteRemote ? ""
                     : state.remoteDrifted ? Words.differsFrom(state.remoteCounterpart)
                     : branchCard.deleteBlockedWhileBusy
        holdMs: Metrics.holdMs
        holdTone: Theme.warning
        onHeld: {
            branchCard.dismiss()
            branchCard.deleteRemoteRequested(state.remoteCounterpart, state.remoteCounterpartOid)
        }
    }
    // Two commands, so words and no code (§git 用語のコード表記). The local half runs first; its refusal stops the
    // pair with nothing touched, and a refused lease on the remote half stops it with the local one done
    // (`session::delete_branch_everywhere`).
    AppMenuItem {
        id: refBothDeleteItem
        /// Whether the local half goes as `-D`, latched for the press under way (デザイン規約 §長押し): read live, git's
        /// answer landing mid-hold would turn a hold begun on the warning into a force delete. The hold length cannot
        /// carry it (this row is held either way), so the `Binding` freezes it while `AppMenuItem.gesturing`.
        property bool forces: refDeleteItem.refusedRow
        readonly property Binding forcesHeld: Binding {
            target: refBothDeleteItem
            property: "forces"
            value: refDeleteItem.refusedRow
            when: !refBothDeleteItem.gesturing
            restoreMode: Binding.RestoreNone
        }
        text: qsTr("Delete both")
        note: refBothDeleteItem.forces ? qsTr("not merged") : ""
        offered: state.kind === "branch" && state.remoteCounterpart !== ""
        // The local half's reasons first: it runs first.
        blockedReason: state.canDelete && state.canDeleteRemote ? ""
                     : state.onCurrentBranch ? branchCard.deleteBlockedOnCurrent
                     : state.heldByWorktree !== "" ? branchCard.blockedByWorktree
                     : state.remoteDrifted ? Words.differsFrom(state.remoteCounterpart)
                     : branchCard.deleteBlockedWhileBusy
        // As on the local row.
        reasonMarkWord: refBothDeleteItem.blockedReason === branchCard.blockedByWorktree ? state.holderLeaf : ""
        holdMs: Metrics.holdMs
        // `warning` whether the local half runs as `-d` or `-D`: the commits only it reached come back from the discard
        // record with the name (デザイン規約 §長押し の色の表).
        holdTone: Theme.warning
        onHeld: {
            branchCard.dismiss()
            // One write with one answer, so one thing to put back.
            branchCard.deleteEverywhereRequested(state.refId, state.remoteCounterpart,
                                                 refBothDeleteItem.forces, state.remoteCounterpartOid)
        }
    }
}

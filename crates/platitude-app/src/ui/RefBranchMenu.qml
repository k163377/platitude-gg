import QtQuick
import platitude.ui

// The BRANCH card: the branch's upstream set and taken off (デザイン規約 §メニュー の入れ子). The rows a branch is used
// through often — sent to its remote (`BranchPushRow`, beside `pull`) and the delete table, local / remote / both
// (`BranchDeleteRow` / `BranchRemoteDeleteRow` / `BranchBothDeleteRow`, the last group) — stand on the level above,
// reading this card's frozen answers (`state`).
//
// **One card, two entrances** (`RefRowMenu` / `CommitMenuState`), so both offer the same thing. **It reads no model
// and runs no write**: the entrance reads the facts (`branchFacts`), `standOn` freezes them as the menu opens, and
// the card and the rows on it signal what was pressed (rules/app-ui.md の「配線」). Standing on plain values is what
// lets `tests/qml/tst_branchcard.qml` drive the real card, the only place the sentences are checked (see `holderLeaf`).
AppMenu {
    id: branchCard

    /// git's answers to the plain delete the menu stayed up for (`BranchDeleteRow`), handed down by the carrying menu.
    /// Live, not frozen: they land while the menu stands (`forceDeleteBranch`).
    property string refusedDelete: ""
    property string landedDelete: ""
    /// The branch the early check was asked about and what git answered for it (`yes` / `no` / `unknown`, empty for
    /// nobody asked) — `RepoTab.branchDeleteAsked` / `branchDeleteMerged` at the entrance.
    property string checkedBranch: ""
    property string checkedMerged: ""

    /// The branch this card is about, frozen by `standOn`. `kind` is `branch` or `remote` — the delete table on the
    /// level above stands on either —, empty on a row that names no branch. Only a local branch has the card's own
    /// rows (`applies`).
    readonly property alias kind: state.kind
    readonly property alias refId: state.refId
    readonly property alias refName: state.refName
    /// Whether the plain delete's answer was in hand off the graph's rows as the menu opened, and what it was. For the
    /// automation (`delete-branch-early`); the delete rows read `refused`.
    readonly property alias deleteAnswered: state.deleteAnswered
    readonly property alias deleteMerged: state.deleteMerged
    /// Whether the delete row has an answer coming at all (off the rows, or asked of git). False when the delete is
    /// out (current branch, held by another worktree); frozen at open, so no answer comes later either.
    /// Only the automation reads it, as "did the input land".
    readonly property alias deleteAsked: state.deleteAsked
    /// The folder the other worktree stands in, as the blocked row names it. The automation reads this as the
    /// witness that the worktree was looked up.
    readonly property alias holderLeaf: state.holderLeaf

    /// The branch git has just refused to delete while the menu stands, so the delete row turns into the held `-D`
    /// where the hand already is (デザイン規約 §左メニューの所作). Written off `refusedDelete`, cleared at the next open.
    /// The menu answers for itself — refused, the rows turn (`refused`); landed, the delete row closes every level
    /// (`BranchDeleteRow.landing`) — so neither entrance has to know which one raised it
    /// (rules-refs/app-ui.md「メニューを閉じるのは自分」).
    property string forceDeleteBranch: ""
    /// Whether the branch's plain delete is one git refuses: refused while the menu stood, or the check at open came
    /// back unmerged (off the rows, else git's echo). Only `"no"` counts: `"unknown"` is a read that fell over, offered
    /// plain. The delete table's local row and its pair read it (`BranchDeleteRow` / `BranchBothDeleteRow`).
    readonly property bool refused:
        state.kind === "branch"
        && (branchCard.forceDeleteBranch === state.refId
            || (state.deleteAnswered ? !state.deleteMerged
                : (branchCard.checkedBranch === state.refId && branchCard.checkedMerged === "no")))

    /// The plain delete, for the page; git's answer comes back by name (`refusedDelete` / `landedDelete`). The row
    /// leaves the list ahead of that answer out of the same slot as the write, so the card writes no page state
    /// (`ops_delete`, デザイン規約 §消す操作は先に画面から消す).
    signal deleteRequested(string kind, string id, string name, string oidHex)
    /// Set upstream: the page opens the question in the window's one question bar (`UpstreamFlow`), starting from
    /// `counterpart`, the remote branch this one already reads. `unsettable`: it has an upstream to take off, which an
    /// emptied name asks to do.
    signal upstreamRequested(string branch, string counterpart, bool unsettable)
    /// The upstream taken off (`RepoTab.unsetUpstream`), for the carrying menu, which holds the tab.
    signal unsetUpstreamRequested(string branch)
    /// The branch sent where its marks and upstream say (`RepoTab.pushNamed`): `force` `""` or `"lease"`, `lease` the
    /// commit the card showed the far side on.
    signal pushRequested(string branch, string force, string lease)
    /// Its first push: the page asks where it goes, in the window's one question bar (`PublishFlow.startPublishAsk`),
    /// opening on `upstream`, where the branch already points.
    signal publishRequested(string branch, string upstream)
    /// Requests to the carrying menu, which holds the tab (rules/app-ui.md の「配線」) and cuts a ref path into remote
    /// and branch against the configured remotes: the early `merged` check, the held `-D` after a refusal, the remote
    /// delete alone, and the pair.
    signal checkDeleteRequested(string branch)
    signal forceDeleteRequested(string branch)
    signal deleteRemoteRequested(string remoteRef, string expect)
    signal deleteEverywhereRequested(string branch, string remoteRef, bool forced, string expect)

    /// The automation's handles into these rows, passed on by the carrying menu.
    readonly property alias upstreamItem: refUpstreamItem
    readonly property alias unsetUpstreamItem: refUnsetUpstreamItem
    /// What the push row stands on (`NavSectionModel.branchPush`'s `state`), for the automation's report.
    readonly property alias pushState: state.pushState

    /// Everything this card and the rows it lends the level above read (`BranchPushRow` / `BranchDeleteRow` /
    /// `BranchRemoteDeleteRow` / `BranchBothDeleteRow`), set once by `standOn` and left alone while it stands.
    readonly property QtObject state: QtObject {
        id: state
        property string kind: ""
        property string refId: ""
        property string refName: ""
        property string refOid: ""
        property string remoteCounterpart: ""
        /// The upstream the branch is configured with, listed or gone — what taking it off names —, and where that
        /// name stands (`remote` / `local` / `current`), which is the colour it wears (デザイン規約 §ref の種別).
        property string upstream: ""
        property string upstreamWhere: ""
        /// The commit that reading showed as the menu opened: the lease of every delete that reaches it.
        property string remoteCounterpartOid: ""
        /// That reading stands on another commit, so the two rows that reach it say why.
        property bool remoteDrifted: false
        property string heldByWorktree: ""
        /// The name that worktree goes by, read at the entrance (`RefRowMenu.heldLeaf`).
        property string holderLeaf: ""
        /// That worktree is the repository's own: its name wears the house (デザイン規約 §ref の種別).
        property bool holderHome: false
        property bool canDelete: false
        property bool canDeleteRemote: false
        property bool canSetUpstream: false
        property bool canUnsetUpstream: false
        property bool onCurrentBranch: false
        /// The plain delete's merged check off the rows: whether answered, and whether the tip is reachable from the
        /// branch's reference point. Merged until answered, so a card still waiting on git wears the plain row.
        property bool deleteAnswered: false
        property bool deleteMerged: true
        property bool deleteAsked: false
        /// The push row's standing — the toolbar's words (`PublishFlow.pushState`) —, where it goes, the commit an
        /// overwrite is leased to, and the upstream the first push's question opens on. `pushState` is empty where
        /// the row has no seat.
        property string pushState: ""
        property string pushTarget: ""
        property string pushLease: ""
        property string pushUpstream: ""
        /// The push would stay in this repository (`stays-here` — measured against another branch here and marked to no
        /// remote, or marked to `.` itself): the row stands out, and says why.
        property bool pushStaysHere: false
    }

    /// Freezes the card on that branch. `facts` is what the carrying menu read for it: `heldByWorktree` the other
    /// worktree holding this row's local branch, `holderLeaf` the name it goes by and `holderHome` whether it is the
    /// repository's own, `remoteCounterpart` the reading a
    /// local branch speaks for, `remoteCounterpartOid` its commit and `remoteDrifted` whether that stands on another
    /// commit, `upstream` the one it is configured with (a gone one too) and `upstreamWhere` where that name stands,
    /// `offers` core's words,
    /// `open` whether the tab is, `merged` what the drawn rows say about the plain delete (`yes` / `no`, empty
    /// for a tip or a reference older than the window, whose answer only git has), and `push` what the push row stands
    /// on (`NavSectionModel.branchPush`: `state`, `target`, `lease`, `upstream`).
    function standOn(kind, name, full, oidHex, facts) {
        branchCard.forceDeleteBranch = ""
        state.kind = kind
        state.refName = name
        state.refId = full
        state.refOid = oidHex
        state.deleteAnswered = false
        state.deleteMerged = true
        state.deleteAsked = false
        state.pushState = ""
        state.pushTarget = ""
        state.pushLease = ""
        state.pushUpstream = ""
        state.pushStaysHere = false
        if (kind !== "branch" && kind !== "remote") {
            state.remoteCounterpart = ""
            state.upstream = ""
            state.upstreamWhere = ""
            state.remoteCounterpartOid = ""
            state.remoteDrifted = false
            state.heldByWorktree = ""
            state.holderLeaf = ""
            state.holderHome = false
            state.canDelete = false
            state.canDeleteRemote = false
            state.canSetUpstream = false
            state.canUnsetUpstream = false
            state.onCurrentBranch = false
            return
        }
        state.heldByWorktree = facts.heldByWorktree
        state.holderLeaf = facts.holderLeaf
        state.holderHome = facts.holderHome === true
        state.remoteCounterpart = facts.remoteCounterpart
        state.upstream = facts.upstream
        state.upstreamWhere = facts.upstreamWhere
        state.remoteCounterpartOid = facts.remoteCounterpartOid
        state.remoteDrifted = facts.remoteDrifted
        // What the rows may offer is core's rule (`offers::ref_menu`).
        const offers = facts.offers
        state.canDelete = offers.includes("delete")
        state.canDeleteRemote = offers.includes("delete-remote")
        state.canSetUpstream = offers.includes("set-upstream")
        state.canUnsetUpstream = offers.includes("unset-upstream")
        state.onCurrentBranch = offers.includes("current")
        // A local branch the worktree is not on — the one it is on has the toolbar's own button (デザイン規約 §メニュー
        // 「そこにしか入口が無いものだけ」) — and only where a push can go: nothing to send, or nothing to name, is no row.
        // Where the push would stay in this repository, it stands out instead (デザイン規約 §リモートへ送る).
        const push = facts.push
        if (kind === "branch" && facts.open && !state.onCurrentBranch && push !== undefined) {
            state.pushStaysHere = push.state === "stays-here"
            if (["publish", "elsewhere", "ready", "behind", "diverged"].includes(push.state)) {
                state.pushState = push.state
                state.pushTarget = push.target
                state.pushLease = push.lease
                state.pushUpstream = push.upstream
            }
        }
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

    // A landing closes the menu through the delete row itself (`BranchDeleteRow.landing`).
    onRefusedDeleteChanged: {
        if (branchCard.answersHere(branchCard.refusedDelete))
            branchCard.forceDeleteBranch = branchCard.refusedDelete
    }

    /// Why the delete table's rows are out — worn as the rows' `blockedReason` (デザイン規約 §無効). Here because both
    /// the local row and its pair say them.
    readonly property string deleteBlockedOnCurrent:
        qsTr("Switch away first — this is the branch you are on")
    readonly property string blockedByWorktree: Words.checkedOutElsewhere(state.holderLeaf)

    titleKind: "branch"
    titleTint: Theme.accent
    title: qsTr("BRANCH")
    // A remote-tracking branch has no upstream of its own: its rows, the delete, are the level above's.
    applies: state.kind === "branch"

    // Set upstream (デザイン規約 §ブランチが測られる相手を決める). Words and a `…`, no code: the flag's value is not
    // settled until the question is answered (§git 用語のコード表記). Only a local branch: a remote-tracking ref has
    // no upstream of its own.
    AppMenuItem {
        id: refUpstreamItem
        text: qsTr("Set upstream…")
        offered: state.kind === "branch" && state.canSetUpstream
        onTriggered: branchCard.upstreamRequested(state.refId, state.remoteCounterpart, state.canUnsetUpstream)
    }
    // The upstream taken off (デザイン規約 §ブランチが測られる相手を決める): words, the pair of `Set upstream…` — git's
    // spelling is the card's longest chip and would push every row's words over, and a flag alone is no command. It
    // names what goes, as the delete rows do (§メニュー「例外は削除行だけ」). The plain delete's two presses in
    // the plain ink: what goes is one setting, and `Set upstream…` puts it back (§回数押しの色).
    AppMenuItem {
        id: refUnsetUpstreamItem
        //: Followed by the upstream it takes off: "Unset upstream origin/feature".
        refSentence: qsTr("Unset upstream %1")
        // As code, in its own colour where it is the branch stood on (`Words.nameInk`).
        refName: state.upstream
        refTint: state.upstreamWhere === "current" ? Theme.textLink : Theme.textPrimary
        refMark: state.upstreamWhere === "current" || state.upstreamWhere === "local" ? "branch" : "remote"
        // The name is data: it does not widen the menu (`growsForText`).
        growsForText: false
        offered: state.kind === "branch" && state.canUnsetUpstream
        holdPresses: Metrics.holdPressesPlain
        holdTone: Theme.textPrimary
        onHeld: {
            refUnsetUpstreamItem.closeMenu()
            branchCard.unsetUpstreamRequested(state.refId)
        }
    }
}

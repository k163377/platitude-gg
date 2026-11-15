import QtQuick
import platitude.ui
import platitude.ui.words

// The BRANCH card: the branch's upstream set and taken off (デザイン規約 §メニュー の入れ子). The rows a branch is used
// through often — sent to its remote (`BranchPushRow`, beside `pull`) and the delete table, local / remote / both
// (`BranchDeleteRow` / `BranchRemoteDeleteRow` / `BranchBothDeleteRow`, the last group) — stand on the level above,
// reading this card's answers (`state`) (規約 §メニュー「第 1 階層か入れ子かは、使う頻度で決める」). Every entrance
// carries this card nested, so those answers are in hand as the menu opens, whether or not the card is ever opened.
//
// The delete table is a fixed table (user decision — ask before changing): a row that cannot be done right now stays,
// greyed. The current branch, and a branch another worktree holds, keep their local and both rows greyed
// (`deleteAsked`) — taken away, a right-click on the current branch in the stacked chips' list would do nothing at all
// (an empty menu does not open, and that list has nothing to fall back on). Only a row whose target does not exist goes
// with its seat: a branch with no remote reading (its upstream, the one the cloud badge resolves) has neither remote
// row (user decision).
//
// **One card, two entrances** (`RefRowMenu` / `CommitMenuState`), so both offer the same thing, letter for letter (user
// decision — ask before changing) — one part both menus carry, standing on its own answers, leaves no room to wire one
// entrance and forget the other; the first-level push and delete rows are carried the same way. Only on a local
// branch (`applies`): the sidebar row itself, or on a graph row the name its chip draws. A remote branch has no
// upstream rows, so the card goes title row and all; its delete is a row of the first-level table. **It reads no model
// and runs no write**: the entrance reads the facts (`branchFacts`), `standOn` stands the card on them as the menu
// opens and as it stands again, and the card and the rows on it signal what was pressed (rules/app-ui.md の「配線」).
// Standing on plain values is what lets `tests/qml/tst_branchcard.qml` drive the real card, the only place the
// sentences are checked (see `holderLeaf`).
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

    /// The branch this card is about, set by `standOn`. `kind` is `branch` or `remote` — the delete table on the
    /// level above stands on either —, empty on a row that names no branch. Only a local branch has the card's own
    /// rows (`applies`).
    readonly property alias kind: state.kind
    readonly property alias refId: state.refId
    readonly property alias refName: state.refName
    /// Whether the plain delete's answer was in hand off the graph's rows as the menu opened, and what it was. For the
    /// automation (`delete-branch-early`, which leaves `from=` out of its claim: which road answers is the refs'
    /// timing — git's road is `delete-branch-early-far`, `--preset deep-parked`); the delete rows read `refused`.
    readonly property alias deleteAnswered: state.deleteAnswered
    readonly property alias deleteMerged: state.deleteMerged
    /// Whether the delete row has an answer coming at all (off the rows, or asked of git). False when the delete is
    /// out (current branch, held by another worktree); set by `standOn`, so no answer comes for a stand that asked
    /// none. Only the automation reads it, as "did the input land".
    readonly property alias deleteAsked: state.deleteAsked
    /// The folder the other worktree stands in, as the blocked row names it. The automation reads this as the
    /// witness that the worktree was looked up.
    readonly property alias holderLeaf: state.holderLeaf

    /// The branch git has just refused to delete while the menu stands, so the delete row turns into the held `-D`
    /// where the hand already is (`BranchDeleteRow.onHeld`). Written off `refusedDelete`, cleared as the card stands
    /// on another branch, on another commit, afresh, or again once a command it waited for ends (`standOn`).
    /// The menu answers for itself — refused, the rows turn (`refused`); landed, the delete row closes every level
    /// (`BranchDeleteRow.landing`) — so neither entrance has to know which one raised it
    /// (rules-refs/app-ui.md「メニューを閉じるのは自分」). Each answer has its own line: the landing is
    /// `delete-branch base`'s `was=true menu=false`, the refusal `delete-branch-go`'s `forced=true` — a row a refusal
    /// meets already wears `-D` from the check at open, so picture and press count cannot tell a refusal that never
    /// arrived.
    property string forceDeleteBranch: ""
    /// Whether the branch's plain delete is one git refuses: refused while the menu stood, or the check at open came
    /// back unmerged (off the rows, else git's echo). Only `"no"` counts: `"unknown"` is a read that fell over, offered
    /// plain. The delete table's local row and its pair read it (`BranchDeleteRow` / `BranchBothDeleteRow`).
    readonly property bool refused:
        state.kind === "branch"
        && (branchCard.forceDeleteBranch === state.refId
            || (state.deleteAnswered ? !state.deleteMerged
                : (state.askedAs !== "" && branchCard.checkedBranch === state.askedAs
                   && branchCard.checkedMerged === "no")))

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
    /// `BranchRemoteDeleteRow` / `BranchBothDeleteRow`), set by `standOn` alone: as the menu opens, and as it stands
    /// again.
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
        /// The commit that reading showed as the menu opened: the lease of every delete that reaches it
        /// (`branchesModel.upstreamOidOf`, kept through the menu standing again, so a fetch behind the open menu leaves
        /// the press on the tip the reader saw — or greys it, `remoteMoved`). Headless: `delete-both-go side` /
        /// `delete-upstream-go next`, `--preset stack` — a lease that misses fails the write.
        property string remoteCounterpartOid: ""
        /// That reading stands on another commit, so the two rows that reach it say why.
        property bool remoteDrifted: false
        /// The reading as the tree has it now, which `Set upstream…` starts from: the delete rows keep the one the menu
        /// went up with (`remoteCounterpart`), the upstream rows read the tree as it is.
        property string readingNow: ""
        /// The menu stood again on a reading where the remote branch the delete rows are leased to — the local
        /// branch's reading, or a remote branch's own row — has moved from that commit or been pointed elsewhere:
        /// they no longer say what a press would take, and grey asking for the menu to be opened again (`standOn`'s
        /// `again`).
        property bool remoteMoved: false
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
        /// The name git's early check was asked under (`checkDeleteRequested`), which its answer comes back by
        /// (`checkedBranch`): a rename with the menu up leaves it, the commits being the same (`renamed`).
        property string askedAs: ""
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
        /// The menu stood again on a reading where the push's far side has moved from the commit the row is leased to
        /// or names another place: the row keeps what it showed and greys asking for the menu to be opened again
        /// (`standOn`'s `again`).
        property bool pushMoved: false
    }

    /// Stands the card on that branch. `facts` is what the carrying menu read for it: `heldByWorktree` the other
    /// worktree holding this row's local branch, `holderLeaf` the name it goes by and `holderHome` whether it is the
    /// repository's own, `remoteCounterpart` the reading a
    /// local branch speaks for, `remoteCounterpartOid` its commit and `remoteDrifted` whether that stands on another
    /// commit, `upstream` the one it is configured with (a gone one too) and `upstreamWhere` where that name stands,
    /// `offers` core's words,
    /// `open` whether the tab is, `merged` what the drawn rows say about the plain delete (`yes` / `no`, empty
    /// for a tip or a reference older than the window, whose answer only git has), and `push` what the push row stands
    /// on (`NavSectionModel.branchPush`: `state`, `target`, `lease`, `upstream`). Every `facts` a branch card stands on
    /// carries `upstream` / `upstreamWhere`, a QML test's hand-built ones too: they are written below as they come, and
    /// the unset row stands on them.
    ///
    /// `again`: why the menu stands again with the card up — `follow`, on what moved while it stood
    /// (`RefRowMenu.follow`, `CommitMenuState.follow`), or `wait`, on what the command it waited for left
    /// (`AppMenu.waitEnded`) —; empty as the menu is raised. Every value is worked out first and written once: a row
    /// read part-way, its push or delete answer emptied for a moment, would grey and blank the presses under way
    /// (`AppMenuItem.onBlockedChanged`). Following on the same branch at the same commit, a refusal git gave while the
    /// menu stood holds, and so does git's answer to the plain delete — asked again on every move, it would blank
    /// between the ask and the answer; after a wait both are asked again, since the command may have merged the branch.
    /// What reaches over there stays on what the menu went up with either way (デザイン規約 §メニュー「表示も押下も開いた
    /// 時」): the remote branch the delete rows are leased to (the reading, or a remote branch's own row) and the push's
    /// far side. That moved or was pointed elsewhere since — a fetch outside, or the one the menu waited for — the rows
    /// keep what they showed and grey asking for the menu to be opened again (`remoteMoved`, `pushMoved`): a press
    /// would take or overwrite commits they never showed. The branch here moving is not that: the card stands on it
    /// where it is.
    function standOn(kind, name, full, oidHex, facts, again) {
        const sameRef = (again === "follow" || again === "wait") && kind === state.kind && full === state.refId
        const answersHold = again === "follow" && sameRef && oidHex === state.refOid
        if (!answersHold || kind !== "branch")
            branchCard.forceDeleteBranch = ""
        if (kind !== "branch" && kind !== "remote") {
            state.kind = kind
            state.refName = name
            state.refId = full
            state.refOid = oidHex
            state.remoteCounterpart = ""
            state.upstream = ""
            state.upstreamWhere = ""
            state.remoteCounterpartOid = ""
            state.remoteDrifted = false
            state.readingNow = ""
            state.remoteMoved = false
            state.heldByWorktree = ""
            state.holderLeaf = ""
            state.holderHome = false
            state.canDelete = false
            state.canDeleteRemote = false
            state.canSetUpstream = false
            state.canUnsetUpstream = false
            state.onCurrentBranch = false
            state.deleteAnswered = false
            state.deleteMerged = true
            state.deleteAsked = false
            state.pushState = ""
            state.pushTarget = ""
            state.pushLease = ""
            state.pushUpstream = ""
            state.pushStaysHere = false
            state.pushMoved = false
            return
        }
        // What the rows may offer is core's rule (`offers::ref_menu`).
        const offers = facts.offers
        const canDelete = offers.includes("delete")
        const onCurrentBranch = offers.includes("current")
        // A remote branch's own delete is leased to the commit its row stood on (`BranchDeleteRow`); a local branch's
        // remote rows to its reading as it was, name and commit (`BranchRemoteDeleteRow` / `BranchBothDeleteRow`).
        const ownLease = sameRef && kind === "remote"
        const readingHeld = sameRef && kind === "branch" && state.remoteCounterpart !== ""
        const remoteMoved = ownLease ? state.remoteMoved || oidHex !== state.refOid
                          : readingHeld && (state.remoteMoved || facts.remoteCounterpart !== state.remoteCounterpart
                                            || facts.remoteCounterpartOid !== state.remoteCounterpartOid)
        // A local branch the worktree is not on — the one it is on has the toolbar's own button (デザイン規約 §メニュー
        // 「そこにしか入口が無いものだけ」) — and only where a push names somewhere: nothing to name is no row. Nothing to
        // send stands out (`clean`), and so does a push that would stay in this repository (デザイン規約 §メニュー
        // 「「今できない」行は無効で残す」, §リモートへ送る). Its far side is where it goes and the commit over there an
        // overwrite is leased to (`branch_push`'s `lease`, counted against in every form).
        const push = facts.push
        const pushed = kind === "branch" && facts.open && !onCurrentBranch && push !== undefined
        const pushSeated = pushed
                           && ["publish", "elsewhere", "ready", "clean", "behind", "diverged"].includes(push.state)
        const pushHeld = sameRef && kind === "branch" && (state.pushState !== "" || state.pushStaysHere)
        const farSide = pushSeated ? push.target + "\n" + push.lease : ""
        const farSideThen = state.pushState !== "" ? state.pushTarget + "\n" + state.pushLease : ""
        const pushMoved = pushHeld && (state.pushMoved || farSide !== farSideThen)
        // Whether the plain delete would be refused, answered at open so the row wears `-D` from the start: the drawn
        // rows answer first (規約 §行が読む答えはどこから来るか), git only where they cannot (→ `checkedMerged`).
        const asks = facts.open && kind === "branch" && canDelete
        const drawn = asks && facts.merged !== ""
        const keeps = asks && !drawn && answersHold && state.deleteAsked
        state.kind = kind
        state.refName = name
        state.refId = full
        state.refOid = ownLease ? state.refOid : oidHex
        state.heldByWorktree = facts.heldByWorktree
        state.holderLeaf = facts.holderLeaf
        state.holderHome = facts.holderHome === true
        state.upstream = facts.upstream
        state.upstreamWhere = facts.upstreamWhere
        state.remoteMoved = remoteMoved
        state.remoteCounterpart = readingHeld ? state.remoteCounterpart : facts.remoteCounterpart
        state.remoteCounterpartOid = readingHeld ? state.remoteCounterpartOid : facts.remoteCounterpartOid
        state.remoteDrifted = readingHeld ? state.remoteDrifted : facts.remoteDrifted
        state.readingNow = facts.remoteCounterpart
        state.canDelete = canDelete
        state.canDeleteRemote = offers.includes("delete-remote")
        state.canSetUpstream = offers.includes("set-upstream")
        state.canUnsetUpstream = offers.includes("unset-upstream")
        state.onCurrentBranch = onCurrentBranch
        state.pushMoved = pushMoved
        state.pushStaysHere = pushMoved ? state.pushStaysHere : pushed && push.state === "stays-here"
        state.pushState = pushMoved ? state.pushState : pushSeated ? push.state : ""
        state.pushTarget = pushMoved ? state.pushTarget : pushSeated ? push.target : ""
        state.pushLease = pushMoved ? state.pushLease : pushSeated ? push.lease : ""
        state.pushUpstream = pushMoved ? state.pushUpstream : pushSeated ? push.upstream : ""
        state.deleteMerged = drawn ? facts.merged === "yes" : keeps ? state.deleteMerged : true
        state.deleteAnswered = drawn || (keeps && state.deleteAnswered)
        state.deleteAsked = asks
        state.askedAs = keeps ? state.askedAs : asks && !drawn ? full : ""
        if (asks && !drawn && !keeps)
            branchCard.checkDeleteRequested(full)
    }

    /// The branch the card stands on renamed with the menu up (`NavSectionModel.renamedTo`): the card goes on standing
    /// on it under the new name, and the stand that follows treats it as the same branch — what reaches over there
    /// keeps its lease, and git's answers about the plain delete hold as they would on the same commit (`standOn`),
    /// the early check's under the name it was asked by (`askedAs`). Presses under way on the rows that name the branch
    /// count for nothing (`AppMenuItem.premise`).
    function renamed(from, to) {
        if (state.kind !== "branch" || state.refId !== from)
            return
        state.refId = to
        state.refName = to
        if (branchCard.forceDeleteBranch === from)
            branchCard.forceDeleteBranch = to
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
        onTriggered: branchCard.upstreamRequested(state.refId, state.readingNow, state.canUnsetUpstream)
    }
    // The upstream taken off (デザイン規約 §ブランチが測られる相手を決める): the one word `Unset`, the pair of
    // `Set upstream…`, the same word as the emptied question's heading and pill (user decision — ask before
    // changing). Words, no chip — git's spelling is the card's longest chip and would push every row's words over, and
    // a flag alone is no command (§git 用語のコード表記). It names what goes, as the delete rows do
    // (§メニュー「例外は削除行だけ」) — the upstream, not the branch, as code behind its mark. The plain delete's two
    // presses in the plain ink (user decision — ask before changing): what goes is one setting, and `Set upstream…`
    // puts it back (§回数押しの色). The press runs
    // `unsetUpstreamRequested` → the carrying menu → `RepoTab.unsetUpstream` → `RepoSession::unset_upstream`
    // (`AfterWrite::Config`). Headless: `unset-upstream [chip:]<branch>` reads the row, `unset-upstream-go` presses it
    // through until the list's `upstreamNamed` is empty.
    AppMenuItem {
        id: refUnsetUpstreamItem
        //: Followed by the upstream it takes off: "Unset upstream origin/feature".
        refSentence: qsTr("Unset upstream %1")
        // As code, in its own colour where it is the branch stood on (`Words.nameInk`).
        refName: state.upstream
        refTint: state.upstreamWhere === "current" ? Theme.textLink : Theme.textPrimary
        refMark: state.upstreamWhere === "current" || state.upstreamWhere === "local" ? "branch" : "remote"
        // No chip to fall back on: with the upstream taken off while the menu stood, the sentence goes with its name
        // (`AppMenuItem.sentence`) and the row keeps its words, greyed where it stands.
        //: The same row once the branch has no upstream left to name, taken off with the menu up.
        text: state.upstream !== "" ? refUnsetUpstreamItem.sentence.arg(state.upstream) : qsTr("Unset upstream")
        // The name is data: it does not widen the menu (`growsForText`).
        growsForText: false
        offered: state.kind === "branch" && state.canUnsetUpstream
        holdPresses: Metrics.holdPressesPlain
        holdTone: Theme.textPrimary
        // The upstream it names: git takes off whatever the branch has by the last press.
        premise: state.refId + "\n" + state.upstream
        onHeld: {
            refUnsetUpstreamItem.closeMenu()
            branchCard.unsetUpstreamRequested(state.refId)
        }
    }
}

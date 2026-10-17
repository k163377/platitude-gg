import QtQuick
import platitude.ui

// The BRANCH card: the branch sent to its remote, its upstream set and taken off, and the delete table — local,
// remote, both (デザイン規約 §メニュー の入れ子). Starting a branch stands on the level above with the other moves.
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
    /// out (current branch, held by another worktree); frozen at open, so no answer comes later either.
    /// Only the automation reads it, as "did the input land".
    readonly property alias deleteAsked: state.deleteAsked
    /// The folder the other worktree stands in, as the blocked row names it. The automation reads this as the
    /// witness that the worktree was looked up.
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
    readonly property alias deleteItem: refDeleteItem
    readonly property alias deleteRemoteItem: refRemoteDeleteItem
    readonly property alias deleteBothItem: refBothDeleteItem
    readonly property alias upstreamItem: refUpstreamItem
    readonly property alias unsetUpstreamItem: refUnsetUpstreamItem
    readonly property alias pushItem: refPushItem
    /// What the push row stands on (`NavSectionModel.branchPush`'s `state`), for the automation's report.
    readonly property alias pushState: state.pushState

    /// Everything this card reads, set once by `standOn` and left alone while it stands.
    QtObject {
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

    onRefusedDeleteChanged: {
        if (branchCard.answersHere(branchCard.refusedDelete))
            branchCard.forceDeleteBranch = branchCard.refusedDelete
    }
    onLandedDeleteChanged: {
        // Through the row: a git that answers inside the floor still leaves the full fill its time.
        if (branchCard.answersHere(branchCard.landedDelete))
            refDeleteItem.closeMenu()
    }

    /// Why the delete table's rows are out — worn as the rows' `blockedReason` (デザイン規約 §無効).
    readonly property string deleteBlockedOnCurrent:
        qsTr("Switch away first — this is the branch you are on")
    readonly property string blockedByWorktree: Words.checkedOutElsewhere(state.holderLeaf)
    /// …and the push row's, where the push would stay in this repository — the toolbar's words for the same standing.
    readonly property string pushBlockedStaysHere: qsTr("Cannot push — its upstream is not supported")

    titleKind: "branch"
    titleTint: Theme.accent
    title: qsTr("BRANCH")
    applies: state.kind === "branch" || state.kind === "remote"

    // The toolbar's push for a branch the worktree is not on, in the toolbar's forms (デザイン規約 §リモートへ送る): a
    // plain click where git would take it; the leased overwrite, held, where the far side has commits this branch
    // lacks; and a first push that asks where it goes. The flag long, as every menu row's (§git 用語のコード表記).
    // Out, in the first push's words, where the push would stay here: there is no remote to send it to.
    AppMenuItem {
        id: refPushItem
        readonly property bool publishing: state.pushState === "publish"
        /// The first push's words: where it goes is asked — or, out, there is nowhere to name.
        readonly property bool firstPushWords: refPushItem.publishing || state.pushStaysHere
        readonly property bool overwrites: state.pushState === "behind" || state.pushState === "diverged"
        // Off the running press's length, so the chip cannot change under a hand already on the row
        // (`AppMenuItem.armedPresses`, デザイン規約 §回数押し).
        code: refPushItem.armedPresses <= 0 ? "push" : "push --force"
        // Where it goes in its kind's colour; the first push asks it, which the `…` says
        // (デザイン規約 §はじめてリモートへ送る).
        //: Follows the `push` chip: "push to origin/feature".
        refSentence: refPushItem.firstPushWords ? "" : qsTr("to %1")
        refName: state.pushTarget
        refTint: Theme.textSecondary
        //: Follows the `push` chip: "push to…", where the branch goes being asked first.
        text: refPushItem.firstPushWords ? qsTr("to…") : refPushItem.refSentence.arg(refPushItem.refName)
        offered: state.pushState !== "" || state.pushStaysHere
        blockedReason: state.pushStaysHere ? branchCard.pushBlockedStaysHere : ""
        holdPresses: refPushItem.overwrites ? Metrics.holdPressesWarned : 0
        // Reaching past this machine is the warning tone (デザイン規約 §状態).
        holdTone: Theme.warning
        onTriggered: {
            if (refPushItem.publishing)
                branchCard.publishRequested(state.refId, state.pushUpstream)
            else
                branchCard.pushRequested(state.refId, "", "")
        }
        onHeld: {
            refPushItem.closeMenu()
            branchCard.pushRequested(state.refId, "lease", state.pushLease)
        }
    }
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
    // names what goes, as the delete rows below do (§メニュー「例外は削除行だけ」). The plain delete's two presses in
    // the plain ink: what goes is one setting, and `Set upstream…` puts it back (§回数押しの色).
    AppMenuItem {
        id: refUnsetUpstreamItem
        //: Followed by the upstream it takes off: "Unset upstream origin/feature".
        refSentence: qsTr("Unset upstream %1")
        // In its own colour: a plain delete's words keep the ordinary rules (`AppMenuItem.inkTakesNames`).
        refName: state.upstream
        refTint: state.upstreamWhere === "current" ? Theme.textLink
               : state.upstreamWhere === "local" ? Theme.textPrimary : Theme.textSecondary
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
        /// `refusedRow` as latched at the first press: on a branch row the warning's count (`AppMenuItem.armedPresses`)
        /// means refused, the plain delete's means not. Read live, git's answer to the last attempt would re-word the
        /// row between presses (デザイン規約 §回数押し).
        readonly property bool shownRefused:
            refDeleteItem.branchRow && refDeleteItem.armedPresses >= Metrics.holdPressesWarned
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
                     : branchCard.blockedByWorktree
        // Asked of the sentence this row shows, not of the state: a row blocked for another reason names no worktree.
        reasonMarkWord: refDeleteItem.blockedReason === branchCard.blockedByWorktree ? state.holderLeaf : ""
        reasonMarkKind: state.holderHome ? "home" : "tree"
        // The plain delete is a delete too: two presses, in the plain ink (デザイン規約 §回数押し). Refused, or reaching
        // past this machine, it is the warning's three.
        holdPresses: heldRow ? Metrics.holdPressesWarned : Metrics.holdPressesPlain
        // `warning` both ways when held: a remote's name stands back up here, and a local branch's tip and upstream
        // come back from the discard record (デザイン規約 §回数押し の色の表). Off the latched count, as the words are.
        holdTone: refDeleteItem.armedPresses >= Metrics.holdPressesWarned ? Theme.warning : Theme.textPrimary
        // At the last press, so `shownRefused` is the count these presses were given: an unmerged answer landing on the
        // floor turns no plain delete into `-D`.
        onHeld: {
            // A branch's plain delete keeps the menu up: git's answer lands on this row, which a refusal turns into
            // `branch -D` (`AppMenuItem.armedPresses` reopens with the warning's count).
            if (refDeleteItem.branchRow && !refDeleteItem.shownRefused) {
                branchCard.deleteRequested(state.kind, state.refId, state.refName, state.refOid)
                return
            }
            refDeleteItem.closeMenu()
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
        // Seated only while a remote reading exists; grey is "not now" — the reading stands on another commit
        // (デザイン規約 §左メニューの所作 の削除の表).
        offered: state.kind === "branch" && state.remoteCounterpart !== ""
        blockedReason: state.canDeleteRemote ? "" : Words.differsFrom(state.remoteCounterpart)
        holdPresses: Metrics.holdPressesWarned
        holdTone: Theme.warning
        onHeld: {
            refRemoteDeleteItem.closeMenu()
            branchCard.deleteRemoteRequested(state.remoteCounterpart, state.remoteCounterpartOid)
        }
    }
    // Two commands, so words and no code (§git 用語のコード表記). The local half runs first; its refusal stops the
    // pair with nothing touched, and a refused lease on the remote half stops it with the local one done
    // (`session::delete_branch_everywhere`).
    AppMenuItem {
        id: refBothDeleteItem
        /// Whether the local half goes as `-D`, latched for the presses under way (デザイン規約 §回数押し): read live, git's
        /// answer landing between presses would turn presses begun on the plain delete into a force delete. The count
        /// cannot carry it (this row takes three either way), so the `Binding` freezes it while
        /// `AppMenuItem.gesturing`.
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
                     : Words.differsFrom(state.remoteCounterpart)
        // As on the local row.
        reasonMarkWord: refBothDeleteItem.blockedReason === branchCard.blockedByWorktree ? state.holderLeaf : ""
        reasonMarkKind: state.holderHome ? "home" : "tree"
        holdPresses: Metrics.holdPressesWarned
        // `warning` whether the local half runs as `-d` or `-D`: the commits only it reached come back from the discard
        // record with the name (デザイン規約 §回数押し の色の表).
        holdTone: Theme.warning
        onHeld: {
            refBothDeleteItem.closeMenu()
            // One write with one answer, so one thing to put back. `forces` is still the latched one.
            branchCard.deleteEverywhereRequested(state.refId, state.remoteCounterpart,
                                                 refBothDeleteItem.forces, state.remoteCounterpartOid)
        }
    }
}

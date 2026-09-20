import QtQuick
import platitude.ui

// Everything a branch's name answers for once it exists, behind the branch's own mark: the delete here, the delete over
// there, and the pair at once (デザイン規約 §メニュー の入れ子). Starting a branch stands on the level above with
// the other moves — that is where the reader carries on from.
//
// **One card, two entrances.** The chip on a graph row and the row itself are two ways at the same branch, and what
// they offer has to be the same thing — so the card is a component, and both entrances hand it what it stands on:
// `standOn` freezes it as the menu opens, the way every other menu freezes what it shows (デザイン規約 §メニュー).
//
// **The card reads no model and runs no write.** What a state may offer is core's rule (`offers::ref_menu`), the
// readings the rows are told apart by live in their own sections, and the menu carrying this card is where all of
// them are — so that is where they are read (`RefRowMenu.branchFacts` / `CommitMenuState.branchFacts`), and the card
// says what was pressed instead of pressing it (規約 §コンポーネント配線規約). What is decided here is the table:
// which row keeps a seat, which greys and **with which sentence**, which gesture each takes and what colour it
// wears. That leaves the card standing on plain values, which is what lets `tests/qml/tst_branchcard.qml` drive the
// real one — the only place those sentences are read at all, since a headless run cannot match a non-ASCII line
// (verify-ui §Windows での実行・デバッグの罠).
AppMenu {
    id: branchCard

    /// git's answer to the plain delete this card stayed up for, and to the early check it asked — handed down by
    /// the menu carrying the card, which is where the tab is. **Live, not frozen**: the whole point of the two is
    /// that they land while the card stands (see `refusedDelete` below).
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
    /// The folder the other working copy stands in, as the blocked row names it. **The automation reads this and not
    /// the sentence**: that one carries an em dash, and a non-ASCII `must_say` never matches on Windows
    /// (verify-ui §Windows での実行・デバッグの罠), so a run claiming the sentence would be claiming nothing. The
    /// sentence itself is read off the card in `tests/qml/tst_branchcard.qml`.
    readonly property alias holderLeaf: state.holderLeaf

    /// The branch git has just refused to delete, while the card that asked is still standing — so the delete row
    /// turns into the held `-D` where the hand already is (デザイン規約 §左メニューの所作). Written by the card itself
    /// off git's answer (`refusedDelete`), and cleared as the card next opens.
    ///
    /// The two answers above stand at the entrance until the next delete is asked, so a fetch answering in the same
    /// drain cannot take one down before this card has seen it. **The card answers for itself**: refused, its row
    /// turns into the held `-D`; taken, there is nothing left to catch and the card goes, and the menu it hangs off
    /// with it (`dismiss()` is Qt's, and walks every level down). Whichever of the two entrances raised this card, it
    /// is the one standing, so nothing above it has to know which one asked (app-ui.md §メニューを閉じるのは自分).
    property string forceDeleteBranch: ""

    /// What the page answers for: the delete git may still refuse opens a question there.
    ///
    /// The row taken off the list ahead of git's answer goes with the write, out of the same slot, so this card
    /// never writes the page's state to say it pressed something
    /// (`ops_delete`, デザイン規約 §消す操作は先に画面から消す).
    signal deleteRequested(string kind, string id, string name, string oidHex)
    /// Which remote branch this one is measured against — the page opens the question, because the bar it stands in
    /// is the one every other question in the window stands in (`UpstreamFlow`). The remote branch this one already
    /// speaks for goes with it: that is where the question opens, and it is read here while the row still answers to
    /// its own name.
    signal upstreamRequested(string branch, string counterpart)
    /// What this card asks the menu carrying it to run: the early check git answers `merged` with, the held `-D`
    /// after a refusal, the reading over there on its own, and the pair at once. **Requests and not writes** — the
    /// tab belongs to that menu (規約 §コンポーネント配線規約), and the remote name a ref path has to be cut into is
    /// read there too, against the remotes this repository has configured.
    signal checkDeleteRequested(string branch)
    signal forceDeleteRequested(string branch)
    signal deleteRemoteRequested(string remoteRef)
    signal deleteEverywhereRequested(string branch, string remoteRef, bool forced)

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
        /// That reading is standing on another commit, so the two rows that reach it say why.
        property bool remoteDrifted: false
        property string heldByWorktree: ""
        /// The folder that copy stands in — the leaf of the path git prints, cut at the entrance (the whole path is
        /// the only unambiguous form and nobody reads a tooltip that wide).
        property string holderLeaf: ""
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

    /// Opens on that branch, off answers already in hand.
    ///
    /// `facts` is what the menu carrying this card read for it: `heldByWorktree` the other working copy holding this
    /// row's local branch and `holderLeaf` the folder it stands in, `remoteCounterpart` the reading a local branch
    /// speaks for and `remoteDrifted` whether that reading stands on another commit, `offers` the words core
    /// answered with, `open` whether the tab is, and `merged` what the drawn rows already say about the everyday
    /// delete (`yes` / `no`, empty for a tip or a reference older than the window, whose answer only git has).
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
        state.remoteDrifted = facts.remoteDrifted
        // What the rows may offer is core's rule, with the measured refusals it encodes — the current branch keeps
        // its delete table and says why (offers::ref_menu).
        const offers = facts.offers.split(" ")
        state.canDelete = offers.includes("delete")
        state.canDeleteRemote = offers.includes("delete-remote")
        state.canSetUpstream = offers.includes("set-upstream")
        state.onCurrentBranch = offers.includes("current")
        // Whether the everyday delete would be refused, answered as the menu opens so the delete row wears `-D` from
        // the start (§左メニューの所作). The drawn rows answer first — in hand in the same frame for every branch the
        // window draws (規約 §行が読む答えはどこから来るか) — and git is asked only where they cannot, its answer
        // landing on `checkedBranch` / `checkedMerged`. The chip column is settled at open, so the swap moves no
        // other row.
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
    // Why git keeps a branch to one working copy is the causal half, and the tooltip rule drops it (デザイン規約 §hover の
    // ツールチップ) — what is left is the state that blocks the row and the one thing the menu cannot show: where. The
    // whole path is what git answers with and is the only unambiguous form, but nobody reads a tooltip that wide.
    //: %1 is the folder of the other working copy that has this branch checked out.
    readonly property string blockedByWorktree:
        qsTr("Checked out in another working copy — %1").arg(state.holderLeaf)

    titleKind: "branch"
    titleTint: Theme.accent
    title: qsTr("BRANCH")
    // Only a row that names a branch has any of this, so only such a row puts the card up.
    applies: state.kind === "branch" || state.kind === "remote"

    // Which remote branch this one is measured against — the counts beside it, the point `branch --delete` calls
    // merged, and where it pushes when nothing is marked all come off this one setting (デザイン規約
    // §ブランチが測られる相手を決める).
    //
    // **Words and a `…`**: what runs is `branch --set-upstream-to=<答え>`, and the flag's value is not settled
    // until the question has been answered — so the row is no more 1:1 with a command than `Create branch here…` is
    // (§git 用語のコード表記). The spelling would not be the short one either: a menu row wears the long form, and
    // `branch --set-upstream-to` in the shared chip column would push every delete row's name across for a row that
    // is not even a command yet. Above the deletes with a line between, since this row writes configuration where
    // the table under it takes names away (§メニュー の入れ子).
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
                    : (branchCard.checkedBranch === state.refId
                       && branchCard.checkedMerged === "no")))
        readonly property bool heldRow: remoteRow || refusedRow
        /// The same answer as the press under way was given it: `refusedRow` follows git's answer to the *last* press,
        /// which is exactly what a hand on this row is waiting for, so read live the row re-words itself and changes
        /// gesture under that hand (デザイン規約 §長押し). `remoteRow` cannot move while the menu stands, so the length
        /// the press was given says the whole of it (`AppMenuItem.armedMs`).
        readonly property bool shownRefused: refDeleteItem.branchRow && refDeleteItem.armedMs > 0
        code: shownRefused ? "branch -D"
            : branchRow ? "branch --delete"
            : "push --delete"
        // The name is data: untranslated, and it leaves the menu's width to the other rows
        // (`growsForText`).
        text: state.refId
        growsForText: false
        note: shownRefused ? qsTr("not merged") : ""
        // On a branch the three delete forms are a fixed table — rows that cannot be chosen stay and grey out, the
        // app-menu rule (デザイン規約 §メニュー、by design): the current branch keeps its rows, saying why nothing
        // here answers. The other kinds keep the assembled rule.
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
                branchCard.deleteRemoteRequested(state.refId)
            } else {
                branchCard.forceDeleteRequested(state.refId)
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
        // no seat. Grey is for "there is one, but not to press now" — busy, or standing on another commit
        // (デザイン規約 §左メニューの所作 の削除の表).
        offered: state.kind === "branch" && state.remoteCounterpart !== ""
        blockedReason: state.canDeleteRemote ? ""
                     : state.remoteDrifted ? Words.remoteOnAnotherCommit
                     : branchCard.deleteBlockedWhileBusy
        holdMs: Metrics.holdMs
        holdTone: Theme.warning
        onHeld: {
            branchCard.dismiss()
            branchCard.deleteRemoteRequested(state.remoteCounterpart)
        }
    }
    // A composite of two commands is no one command, so the row says it in words (§git 用語のコード表記 の 1:1 規則).
    // The local half runs first and a refusal stops the pair, nothing touched.
    AppMenuItem {
        id: refBothDeleteItem
        /// **Whether the local half goes as `-D`, as the press under way was given it** (デザイン規約 §長押し).
        /// `refusedRow` follows git's answer to the *last* press, which can land while a hand is already on this row:
        /// read live, a hold begun on the warning comes down as a force delete, throwing away commits nobody was shown
        /// losing. The length cannot carry it — this row is held either way — so the latch is its own, declared live
        /// and kept by the `Binding` for as long as a gesture lasts (`AppMenuItem.gesturing`).
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
        holdTone: refBothDeleteItem.forces ? Theme.danger : Theme.warning
        onHeld: {
            branchCard.dismiss()
            // Both halves go at once: the pair is one write with one answer, so it is one thing to put back. Where
            // the reading's name is cut into a remote and a branch is the entrance's to say — it is the configured
            // names that decide, and they are the tab's (`RefRowMenu.deleteRemoteNow`).
            branchCard.deleteEverywhereRequested(state.refId, state.remoteCounterpart,
                                                 refBothDeleteItem.forces)
        }
    }
}

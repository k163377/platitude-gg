import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Push, in whichever shape this branch's standing with its remote allows (デザイン規約 §リモートへ送る). The counts behind
// it are from the last fetch, so they are believed only where they refuse. The band hands in the active page and its
// own automation latch; the shared width box stays the band's to assign.
ActionButton {
    id: pushButton

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// The waiting visual, held up by the band past the push that raised it (`TopBar.holdPushBusy`).
    property bool busyLatched: false
    readonly property string mode: pushButton.curPage !== null ? pushButton.curPage.pushState : "closed"
    /// The last go at sending this branch came back refused. No stopped step past it: nothing sends on its own
    /// to be stopped (デザイン規約 §リモートへ送る).
    readonly property bool failed: pushButton.curPage !== null && pushButton.curPage.pushFailed
    /// The standings where an overwrite is the only send there is, and so the shape the button takes. `behind` stands
    /// here beside `diverged` for the reason that shape is allowed at all: a plain push does not stand in either, so
    /// the one button never carries two meanings (デザイン規約 §相手の履歴を置き換える). Both have the tracking ref on
    /// screen for the lease to pin to — `elsewhere`, which has no such read, keeps the plain shape.
    readonly property bool forceShape: pushButton.mode === "diverged" || pushButton.mode === "behind"
    /// The same answer as the press under way was given it — **the shape is settled the moment the button goes down**
    /// (デザイン規約 §長押し). `mode` follows the fetch that runs behind every press, so it moves under a hand that is
    /// already holding: read live, the word said `push -f` and the release ran a plain `push`, or the other way
    /// about. Everything the shape decides — the word, the two colours, the length and what the release runs — reads
    /// this one (`ActionButton.armedMs`), and a press whose shape went is dropped rather than answered with the other
    /// command (`HoldDriver.stale`).
    readonly property bool shownForce: pushButton.armedMs > 0
    /// The frame's warning colour, for either of the two things that call for it: what an overwrite would do,
    /// and what the last go did. The word takes it for only one of them (below).
    readonly property bool warned: pushButton.shownForce || pushButton.failed

    kind: "push"
    busy: pushButton.busyLatched
          || (pushButton.curPage !== null && pushButton.curPage.pageTab.busyOp === "push")
    // Two fixed wordings, no counts: a number here would make the button a different width for every value it
    // took (デザイン規約 §リモートへ送る).
    text: pushButton.shownForce ? "push -f" : "push"
    code: true
    // The overwrite colours its word; the refusal does not — only one of them changes what the press costs
    // (デザイン規約 §長押し — 警告の色は語ではなく枠と印が持つ).
    tone: pushButton.shownForce ? Theme.warning : Theme.textPrimary
    // The ring and the frame keep the warning through the wait, a step down. The word does not follow them: it
    // takes the disabled step, which is the one way a word says "not now" (デザイン規約 §暗く落とした段 / §無効).
    toneDim: pushButton.warned ? Theme.warningDim : Theme.textMuted
    frameColor: pushButton.warned ? Theme.warning : "transparent"
    // The frame's colour is worn by the diverged shape as well, so on its own it would not tell "cannot land
    // plainly" from "did not land".
    alert: pushButton.failed
    alertTone: Theme.warning
    holdMs: pushButton.forceShape ? Metrics.holdMs : 0
    // **Which tab, which branch, and where that branch is going** (`HoldDriver.premise`). The band outlives the page
    // it points at: switch tabs with a hand on this button and the length is unchanged — two tabs behind their
    // remotes arm the same 500ms — while the press is now aimed at another repository's branch, which is what the
    // release would overwrite (measured, `tst_holdlatch`).
    //
    // **The destination alone does not say which branch it is.** What a push sends is the branch the working tree is
    // on (`PublishFlow.forcePush` → `workTree.branch`), and two local branches can track one remote one: switching
    // between them leaves the destination reading `origin/main` either way, and the hold began on the other branch's
    // history. Which of the two commands it is needs no saying here — that is the length. The joiner is a colon
    // because git allows none in a ref name, so no pair of these can spell the same premise.
    premise: pushButton.curPage === null
             ? ""
             : pushButton.curPage.tab_id + ":" + pushButton.curPage.pageWt.branch
               + ":" + pushButton.curPage.pushTargetLabel
    // Held down while a rebase plan is being composed, like the stash button: what a push moves is the remote's
    // story of the very commits the plan is about to rewrite.
    enabled: pushButton.curPage !== null && !pushButton.curPage.planShown
             && (pushButton.curPage.canPush || (pushButton.forceShape && pushButton.curPage.canForcePush))
    onHeld: pushButton.curPage.forcePush()
    tip: {
        if (pushButton.curPage === null)
            return ""
        // The freeze speaks for itself the way the tree-shaped refusals below do: a frozen button still takes
        // hover, and nothing else on screen says the plan is what is holding it down.
        if (pushButton.curPage.planShown)
            return qsTr("A rebase plan is being composed — it rewrites the very commits a push would send")
        const to = pushButton.curPage.pushTargetLabel
        const what = pushButton.mode === "unborn"
                     ? qsTr("No commits yet — git has no branch to send")
                   : pushButton.mode === "publish"
                     ? qsTr("This branch has not been sent anywhere yet — asks where it goes")
                   // No count: the marked remote is not the one this branch tracks, so `ahead` is the
                   // standing with somewhere else (デザイン規約 §リモートへ送る).
                   : pushButton.mode === "elsewhere"
                     ? qsTr("Push to %1").arg(to)
                   : pushButton.mode === "ready"
                     ? qsTr("Push %n commit(s) to %1", "", pushButton.curPage.pageWt.ahead).arg(to)
                   : pushButton.mode === "clean"
                     ? qsTr("Nothing to push — %1 is up to date").arg(to)
                   // One sentence for both overwrite standings: what is lost is what the far side has either way,
                   // and `behind` says the rest of itself by having no count of its own to send.
                   : pushButton.forceShape
                     ? qsTr("Hold to overwrite %1, dropping %n commit(s) it has (as of the last fetch)", "",
                            pushButton.curPage.pageWt.behind).arg(to)
                   : ""
        // What git said, under what the button would do next — this is the one place with room for why.
        const why = pushButton.curPage.pushFailReason
        if (pushButton.failed && why !== "")
            return what + "\n\n" + why
        return what
    }
    onActivated: pushButton.curPage.pushNow()
}

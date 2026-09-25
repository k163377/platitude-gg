import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Push, in whichever shape this branch's standing with its remote allows (デザイン規約 §リモートへ送る). The shared
// width box stays the band's to assign.
ActionButton {
    id: pushButton

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// The waiting visual, held up by the band past the push that raised it (`TopBar.holdPushBusy`).
    property bool busyLatched: false
    readonly property string mode: pushButton.curPage !== null ? pushButton.curPage.pushState : "closed"
    /// The last push of this branch was refused.
    readonly property bool failed: pushButton.curPage !== null && pushButton.curPage.pushFailed
    /// The standings where an overwrite is the only send there is (デザイン規約 §リモートへ送る). `elsewhere` stays
    /// plain: it has no tracking ref on screen for the lease to pin to.
    readonly property bool forceShape: pushButton.mode === "diverged" || pushButton.mode === "behind"
    /// The shape as the press under way was given it, settled when the button goes down (デザイン規約 §長押し): `mode`
    /// moves under a holding hand with the fetch behind every press. Everything the shape decides reads this
    /// (`ActionButton.armedMs`); a press whose shape went is dropped (`HoldDriver.stale`).
    readonly property bool shownForce: pushButton.armedMs > 0
    /// The frame's warning: an overwrite, or the last push refused. The word takes it only for the overwrite.
    readonly property bool warned: pushButton.shownForce || pushButton.failed

    kind: "push"
    busy: pushButton.busyLatched
          || (pushButton.curPage !== null && pushButton.curPage.pageTab.busyOp === "push")
    // No counts: the label would change width with them (デザイン規約 §リモートへ送る).
    text: pushButton.shownForce ? "push -f" : "push"
    code: true
    // Only the overwrite colours its word (デザイン規約 §長押し「警告の色は枠と印が持つ」).
    tone: pushButton.shownForce ? Theme.warning : Theme.textPrimary
    // Through the wait: ring and frame a step down, the word the disabled step (デザイン規約 §暗く落とした段 / §無効).
    toneDim: pushButton.warned ? Theme.warningDim : Theme.textMuted
    // Framed in every state, like the two beside it (`BandFetchButton`); what the warning owns is the colour.
    frameColor: pushButton.warned ? Theme.warning : Theme.borderStrong
    // The canvas inside the frame, like the two beside it (`BandFetchButton`).
    faceColor: Theme.bgSurface
    // The `!` tells "did not land" apart from the diverged shape, which wears the same frame.
    alert: pushButton.failed
    alertTone: Theme.warning
    holdMs: pushButton.forceShape ? Metrics.holdMs : 0
    // Tab, branch and destination (`HoldDriver.premise`): the band outlives the page, so a tab switch under a holding
    // hand would aim the release at another repository (`tst_holdlatch`); two local branches can track one remote
    // branch, so the destination alone does not name what is sent. `:` cannot occur in a ref name.
    premise: pushButton.curPage === null
             ? ""
             : pushButton.curPage.tab_id + ":" + pushButton.curPage.pageWt.branch
               + ":" + pushButton.curPage.pushTargetLabel
    // Down while a rebase plan is composed: it rewrites the very commits a push would send.
    enabled: pushButton.curPage !== null && !pushButton.curPage.planShown
             && (pushButton.curPage.canPush || (pushButton.forceShape && pushButton.curPage.canForcePush))
    onHeld: pushButton.curPage.forcePush()
    tip: {
        if (pushButton.curPage === null)
            return ""
        // Said though disabled: nothing else on screen says the plan is what holds it down.
        if (pushButton.curPage.planShown)
            return qsTr("A rebase plan is being composed — it rewrites the very commits a push would send")
        const to = pushButton.curPage.pushTargetLabel
        const wt = pushButton.curPage.pageWt
        // A sentence for each count — `commit(s)` would reach the reader as written (デザイン規約 §タイポグラフィ).
        const what = pushButton.mode === "unborn"
                     ? qsTr("No commits yet — git has no branch to send")
                   : pushButton.mode === "publish"
                     ? qsTr("This branch has not been sent anywhere yet — asks where it goes")
                   // No count: `ahead` is against another remote (デザイン規約 §リモートへ送る).
                   : pushButton.mode === "elsewhere"
                     ? qsTr("Push to %1").arg(to)
                   : pushButton.mode === "ready"
                     ? (wt.ahead === 1 ? qsTr("Push %n commit to %1", "", wt.ahead)
                                       : qsTr("Push %n commits to %1", "", wt.ahead)).arg(to)
                   : pushButton.mode === "clean"
                     ? qsTr("Nothing to push — %1 is up to date").arg(to)
                   // One sentence for both overwrite standings (デザイン規約 §リモートへ送る).
                   : pushButton.forceShape
                     ? (wt.behind === 1
                        ? qsTr("Hold to overwrite %1, dropping %n commit it has (as of the last fetch)", "",
                               wt.behind)
                        : qsTr("Hold to overwrite %1, dropping %n commits it has (as of the last fetch)", "",
                               wt.behind)).arg(to)
                   : ""
        // What git said, under what the button would do next (デザイン規約 §リモートへ送る).
        const why = pushButton.curPage.pushFailReason
        if (pushButton.failed && why !== "")
            return what + "\n\n" + why
        return what
    }
    onActivated: pushButton.curPage.pushNow()
}

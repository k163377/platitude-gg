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
    /// The last push of this branch was refused (remembered per branch: `PublishFlow.pushFails`). Over any standing,
    /// a `warning` frame and a `!` past the word: the frame too, since nothing happens until someone presses again.
    /// The word stays — a refused push is still one click, and `push -f`'s warning colour would borrow the hold's look
    /// with it (規約 §回数押し「警告の色は枠と印が持つ」). git's words end the tip. Two stages only: unlike fetch's
    /// `Resume`, nothing pushes by itself, so nothing stopped and there is no new move to offer.
    readonly property bool failed: pushButton.curPage !== null && pushButton.curPage.pushFailed
    /// The standings where an overwrite is the only send there is (デザイン規約 §リモートへ送る). There the button takes
    /// the hold and a click does nothing: a plain push cannot land in either, so one button never carries two
    /// meanings, and the hold is the confirmation (no dialog — the lease guards it, §相手の履歴を置き換える). `elsewhere`
    /// stays plain: nothing here tracks the branch over there, so no commit is on screen for the lease to pin to.
    /// `behind` takes the overwrite shape too: the only send that reaches from there rewinds the far side, a plain
    /// push being answered "nothing to send". What it costs is the far side's `behind` commits, said in the same
    /// sentence as `diverged` (`sentenceTo`). Out of reach, the way back from a history sent and then regretted would
    /// leave the screen.
    readonly property bool forceShape: pushButton.mode === "diverged" || pushButton.mode === "behind"
    /// The shape as the press under way was given it, settled when the button goes down (デザイン規約 §回数押し): `mode`
    /// moves between presses with the fetch behind every press. Everything the shape decides reads this
    /// (`ActionButton.armedPresses`); a press whose shape went is dropped (`HoldDriver.stale`).
    readonly property bool shownForce: pushButton.armedPresses > 0
    /// The frame's warning: an overwrite, or the last push refused. The word takes it only for the overwrite.
    readonly property bool warned: pushButton.shownForce || pushButton.failed

    kind: "push"
    busy: pushButton.busyLatched
          || (pushButton.curPage !== null && pushButton.curPage.pageTab.busyOp === "push")
    // The command in every state; a first push is not called `Publish` (規約 §リモートへ送る). No counts: the label would
    // change width with them and move the panel's row — they are on the sidebar's `↑N ↓M` and in the tip. Every state
    // is the shared box's width (`TopBar.widestAction`).
    text: pushButton.shownForce ? "push -f" : "push"
    code: true
    // Only the overwrite colours its word (デザイン規約 §回数押し「警告の色は枠と印が持つ」).
    tone: pushButton.shownForce ? Theme.warning : Theme.textPrimary
    // Through the wait: ring and frame a step down, the word the disabled step (デザイン規約 §暗く落とした段 / §無効).
    toneDim: pushButton.warned ? Theme.warningDim : Theme.textMuted
    // Framed in every state, like the two beside it (`BandFetchButton`); what the warning owns is the colour.
    frameColor: pushButton.warned ? Theme.warning : Theme.borderStrong
    // The canvas inside the frame, like the two beside it (`BandFetchButton`).
    faceColor: Theme.bgSurface
    // The `!` tells "did not land" apart from the diverged shape, which wears the same frame — so a diverged branch
    // whose push was refused wears both. It says only that: why, when the far side's own rules said no, comes down on
    // the notice bar and stays in the log (規約 §答えの要らない報せ).
    alert: pushButton.failed
    alertTone: Theme.warning
    holdPresses: pushButton.forceShape ? Metrics.holdPressesWarned : 0
    // Tab, branch and destination (`HoldDriver.premise`): the band outlives the page, so a tab switch between presses
    // would aim the last press at another repository (`tst_holdlatch`); two local branches can track one remote
    // branch, so the destination alone does not name what is sent. `:` cannot occur in a ref name.
    premise: pushButton.curPage === null
             ? ""
             : pushButton.curPage.tab_id + ":" + pushButton.curPage.pageWorktree.branch
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
        const what = pushButton.sentenceTo(pushButton.curPage.pushTargetLabel)
        // What git said, under what the button would do next (`failed`).
        const why = pushButton.curPage.pushFailReason
        if (pushButton.failed && why !== "")
            return what + "\n\n" + why
        return what
    }
    /// Where that sentence names the branch it goes to, behind the remote's cloud (デザイン規約 §ref の種別「名前の印」,
    /// `TipAsk.marksNow`).
    readonly property var tipMarks: pushButton.curPage === null || pushButton.curPage.planShown ? []
        : Words.standSeats(pushButton.sentenceTo(Words.stand), pushButton.curPage.pushTargetLabel)
              .map(at => [at, "remote", undefined, pushButton.curPage.pushTargetLabel.length])
    /// What the button would do next, with `to` where it goes.
    function sentenceTo(to) {
        // The counts the standing read: the destination's own where the push goes elsewhere.
        const ahead = pushButton.curPage.pushAhead
        const behind = pushButton.curPage.pushBehind
        // A sentence for each count — `commit(s)` would reach the reader as written (デザイン規約 §タイポグラフィ).
        const what = pushButton.mode === "unborn"
                     ? qsTr("No commits yet — git has no branch to send")
                   : pushButton.mode === "publish"
                     ? qsTr("This branch has not been sent anywhere yet — asks where it goes")
                   // The push would stay in this repository, which is no send (`remote::PushStanding::StaysHere`).
                   : pushButton.mode === "stays-here"
                     ? qsTr("Cannot push — its upstream is not supported")
                   // No count: nothing here tracks the branch over there, so the press stays plain and the tip
                   // names the destination only (`remote::PushStanding::Elsewhere`).
                   : pushButton.mode === "elsewhere"
                     ? qsTr("Push to %1").arg(to)
                   : pushButton.mode === "ready"
                     ? (ahead === 1 ? qsTr("Push %n commit to %1", "", ahead)
                                    : qsTr("Push %n commits to %1", "", ahead)).arg(to)
                   : pushButton.mode === "clean"
                     ? qsTr("Nothing to push — %1 is up to date").arg(to)
                   // One sentence for both overwrite standings (`forceShape`). A count says when it is from, in a short
                   // bracket (`as of the last fetch`; 規約 §hover のツールチップ).
                   : pushButton.forceShape
                     ? (behind === 1
                        ? qsTr("%1 to overwrite %2, dropping %n commit it has (as of the last fetch)", "", behind)
                        : qsTr("%1 to overwrite %2, dropping %n commits it has (as of the last fetch)", "", behind))
                       .arg(Words.pressCount(Metrics.holdPressesWarned)).arg(to)
                   : ""
        return what
    }
    onActivated: pushButton.curPage.pushNow()
}

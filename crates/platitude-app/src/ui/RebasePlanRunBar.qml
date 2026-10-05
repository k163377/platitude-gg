import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The plan's one door onto the repository, in the commit button's seat (the exit card takes it if the run stops
// part-way — §進行中の操作から出る). The hold follows the drop table (§履歴を合流させる「見るのは先端」).
Item {
    id: bar

    /// The plan whose run this fires.
    required property var plan
    /// The rewrite warning's count for the plan's own range (`RepoPage.planPushed`).
    required property int pushedCount
    /// Whether anything besides this branch still reaches the tip — with the drops, what decides the hold.
    required property bool tipHeldElsewhere
    /// The tab is running a git command — the run keeps out of an unknown write's way.
    required property bool busy

    height: runButton.implicitHeight + Theme.spaceMd
    /// A hold: rows leave the history and nothing else reaches the tip.
    readonly property bool holds: bar.plan.dropCount > 0 && !bar.tipHeldElsewhere

    ActionButton {
        id: runButton
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spaceSm
        centred: true
        phraseHead: "rebase -i"
        // A phrase for each count — `commit(s)` would reach the reader as written (デザイン規約 §タイポグラフィ).
        text: bar.plan.stepCount === 1 ? qsTr("%n commit", "", bar.plan.stepCount)
                                       : qsTr("%n commits", "", bar.plan.stepCount)
        // The warning is a clause at the end of the phrase, not a second line in the seat.
        phraseNote: bar.pushedCount > 0 ? qsTr("%n already pushed", "", bar.pushedCount) : ""
        // The frame carries the warning; only a hold colours the word (§長押し). `warning` held too: the commits a
        // rewrite leaves come back from the branch's reflog (デザイン規約 §長押し の色の表). The dressing reads
        // `armedMs`, not `holds`: `tipHeldElsewhere` follows a fetch that keeps running, and could drop the colour
        // under a holding hand.
        frameColor: runButton.armedMs > 0 || bar.pushedCount > 0 ? Theme.warning : Theme.accent
        tone: runButton.armedMs > 0 ? Theme.warning : Theme.textPrimary
        holdMs: bar.holds ? Metrics.holdMs : 0
        enabled: bar.plan.dirty && !bar.busy
        onActivated: bar.plan.runPlan()
        onHeld: bar.plan.runPlan()
    }
}

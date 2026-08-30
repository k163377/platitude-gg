import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The plan's one door onto the repository, in the seat where things are concluded — the commit button's, which the
// exit card will take over the moment the run stops part-way (§進行中の操作から出る). The warning is the note's own
// amber, said inside the phrase and never a gate; the hold is the drop table's (§履歴を合流させる — 見るのは先端).
Item {
    id: bar

    /// The plan whose run this fires (the page owns it; a pane may call a model's slots — app-ui.md).
    required property var plan
    /// The rewrite warning's count for the plan's own range (`RepoPage.planPushed`).
    required property int pushedCount
    /// Whether anything besides this branch still reaches the tip — with the drops, what decides the hold.
    required property bool tipHeldElsewhere
    /// The tab is running a git command — the run keeps out of an unknown write's way.
    required property bool busy

    height: runButton.implicitHeight + Theme.spaceMd
    readonly property bool holds: bar.plan.dropCount > 0 && !bar.tipHeldElsewhere

    ActionButton {
        id: runButton
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spaceSm
        centred: true
        phraseHead: "rebase -i"
        text: qsTr("%n commit(s)", "", bar.plan.stepCount)
        // **The warning is a clause of the phrase, not a line above the button.** The count is the fact, said in the
        // tag's own words at the end of what the button says; a line of its own would put a second thing to read in
        // the seat this button fills, and the two colours would then have to be told apart before either is read.
        phraseNote: bar.pushedCount > 0 ? qsTr("%n already pushed", "", bar.pushedCount) : ""
        // The frame carries the warning and only a hold colours the word
        // (§長押し — BandPushButton と同じ線).
        frameColor: bar.holds ? Theme.danger
                    : bar.pushedCount > 0 ? Theme.warning : Theme.accent
        tone: bar.holds ? Theme.danger : Theme.textPrimary
        holdMs: bar.holds ? Metrics.holdMs : 0
        enabled: bar.plan.dirty && !bar.busy
        onActivated: bar.plan.runPlan()
        onHeld: bar.plan.runPlan()
    }
}

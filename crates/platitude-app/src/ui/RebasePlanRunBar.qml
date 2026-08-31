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
    /// What the run asks for as things stand: rows are leaving the history and nothing else reaches the tip.
    readonly property bool holdsNow: bar.plan.dropCount > 0 && !bar.tipHeldElsewhere
    /// The same answer as the press under way was given it — **whether this is a hold is settled the moment the
    /// button goes down** (デザイン規約 §フル interactive rebase). `tipHeldElsewhere` follows the fetch the plan's
    /// freeze deliberately leaves running, so it can move under a hand that is already holding, and `ActionButton`
    /// reads the length again at the release: a hold begun on the red button would come back as a click and run the
    /// plan on a gesture nobody made. The frame and the word are frozen with it, so nothing about the button changes
    /// under the hand.
    ///
    /// Declared with the live value so it reads right from the start; the `Binding` below is what keeps it, dropped
    /// while a press is under way with nothing put back after (`RestoreNone`) — which is the freeze itself.
    property bool holds: bar.holdsNow
    /// A gesture is under way. The pointer's is `down`; the keyboard's is not — an armed hold takes Space itself
    /// (`HoldDriver.pressKey`), so `down` never rises for it and the climbing fill is what says the press is there.
    /// The fill also covers the slide back out, which is that press's own tail.
    readonly property bool pressing: runButton.down || runButton.holdProgress > 0

    Binding {
        target: bar
        property: "holds"
        value: bar.holdsNow
        when: !bar.pressing
        restoreMode: Binding.RestoreNone
    }

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

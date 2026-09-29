pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// What happens on the remote after a name is changed here — a question, since which the reader wanted is not the
// repository's to know (デザイン規約 §手元の改名の後のリモート). Draws nothing: the bar belongs to the graph.
Item {
    id: carryFlow

    required property RepoTab repoTab
    /// The one bar this question stands in, and whose pill it dresses while it stands (`RepoPage.startRowAsk` raises
    /// it).
    required property GraphPane graphPane

    /// Raise the standing question with this form in it; the page owns the bar.
    signal askRequested(string oidHex, string label, string accept, var run, var form)
    /// Automation: the question was raised, and the two names it is between.
    signal carryAsked(string kind, string from, string to)

    /// Whether this question is the one standing, which is what its bindings on the bar hang off.
    property bool asking: false
    /// What was renamed here — `branch` or `tag` — the remote the old name also stands on, and the two names.
    property string kind: ""
    property string remote: ""
    property string from: ""
    property string to: ""
    /// The commit the old name was shown on over there, which `Replace` leases its delete to
    /// (`remote::replace_remote_branch` / `replace_remote_tag`).
    property string shown: ""
    /// Which answer is picked. The question opens on the one that takes nothing away, so the pill is live from the
    /// first frame (デザイン規約 §手元の改名の後のリモート).
    property int choice: carryFlow.keepsBoth

    /// The name the remote carries now, and the one it would carry. A branch's live under `<remote>/`; a tag's have
    /// no namespace of their own, so they are said with the remote beside them (デザイン規約 §タグを作る・送る).
    readonly property string oldSide:
        carryFlow.kind === "branch" ? carryFlow.remote + "/" + carryFlow.from : carryFlow.from
    readonly property string newSide:
        carryFlow.kind === "branch" ? carryFlow.remote + "/" + carryFlow.to : carryFlow.to

    /// The three answers, in the words they are picked by. The first names the delete first but runs it last
    /// (`remote::replace_remote_branch`; デザイン規約 §手元の改名の後のリモート「行は削除を先に言い、走るのは push が先」).
    readonly property var choices: carryFlow.kind === ""
        ? []
        : [
            carryFlow.kind === "branch"
                //: %1 and %2 are remote branches, e.g. origin/main. Picked from a chooser.
                ? qsTr("Delete %1, create %2").arg(carryFlow.oldSide).arg(carryFlow.newSide)
                //: %1 and %2 are tag names, %3 a remote. Picked from a chooser.
                : qsTr("Delete %1 on %3, create %2").arg(carryFlow.oldSide).arg(carryFlow.newSide)
                                                   .arg(carryFlow.remote),
            carryFlow.kind === "branch"
                ? qsTr("Create %1, keep %2").arg(carryFlow.newSide).arg(carryFlow.oldSide)
                : qsTr("Create %1 on %3, keep %2").arg(carryFlow.newSide).arg(carryFlow.oldSide)
                                                 .arg(carryFlow.remote),
            qsTr("Do nothing"),
        ]

    readonly property int replacesIt: 0
    readonly property int keepsBoth: 1
    readonly property int leavesAlone: 2
    /// The answer that cannot be taken back: a name goes from the remote, and whatever hung off it stays behind
    /// (デザイン規約 §長押し).
    readonly property bool takesAway: carryFlow.choice === carryFlow.replacesIt
    /// …and the one that writes nothing at all.
    readonly property bool leavesIt: carryFlow.choice === carryFlow.leavesAlone

    anchors.fill: parent

    /// Opens the question. `oidHex` is the commit the bar marks, taken by the caller before the rename: the rows are
    /// rebuilt behind the answer that raises this (`RepoPage.armRenameTagRemote`). It is where the old name stands over
    /// there too — the caller asks only where the two agree — so it is also the lease.
    function startAsk(kind, remote, from, to, oidHex) {
        carryFlow.kind = kind
        carryFlow.remote = remote
        carryFlow.from = from
        carryFlow.to = to
        carryFlow.shown = oidHex
        carryFlow.choice = carryFlow.keepsBoth
        carryFlow.askRequested(
            oidHex,
            kind === "branch"
                //: The question a branch renamed here raises about the remote branch it was measured against.
                ? qsTr("Change the upstream too?")
                //: %1 is a remote. The question a tag renamed here raises about the remote's copy of it.
                : qsTr("Change the tag on %1 too?").arg(remote),
            carryFlow.pillWord(),
            carryFlow.answer,
            carryForm)
        // After the bar is up: raising it resets the properties the `asking` bindings below own, and a binding whose
        // value has not changed does not push back.
        carryFlow.asking = true
        carryFlow.carryAsked(kind, carryFlow.oldSide, carryFlow.newSide)
    }

    function pick(index) {
        if (index < 0 || index >= carryFlow.choices.length)
            return
        carryFlow.choice = index
    }

    /// The picked answer's own word for the pill; at `Replace` the gesture becomes a hold
    /// (デザイン規約 §手元の改名の後のリモート).
    function pillWord() {
        if (carryFlow.takesAway)
            //: The pill answering a standing question: the old name goes off the remote and the new one goes up.
            return qsTr("Replace")
        if (carryFlow.leavesIt)
            //: The pill answering a standing question: nothing over there is touched.
            return qsTr("Leave")
        //: The pill answering a standing question: the new name is made and the old one left where it is.
        return qsTr("Create")
    }

    function answer() {
        if (carryFlow.kind === "branch") {
            if (carryFlow.takesAway)
                carryFlow.repoTab.replaceRemoteBranch(carryFlow.remote, carryFlow.from, carryFlow.to, carryFlow.shown)
            else if (carryFlow.choice === carryFlow.keepsBoth)
                carryFlow.repoTab.pointUpstreamAndPush(carryFlow.to, carryFlow.remote, carryFlow.to)
        } else if (carryFlow.kind === "tag") {
            if (carryFlow.takesAway)
                carryFlow.repoTab.replaceRemoteTag(carryFlow.remote, carryFlow.from, carryFlow.to, carryFlow.shown)
            else if (carryFlow.choice === carryFlow.keepsBoth)
                carryFlow.repoTab.pushTag(carryFlow.remote, carryFlow.to, "")
        }
        // The third answer runs nothing: the bar coming down is the whole of it.
    }

    // Bindings, since the bar changes with the pick while it stands. `RestoreNone`: letting go happens at the press
    // that walks away, with the bar still on screen (`PublishFlow`). The heading stays plain ink
    // (`AskBar.plainWords`).
    Binding {
        target: carryFlow.graphPane
        property: "askPlainWords"
        value: true
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: carryFlow.graphPane
        property: "askAccept"
        value: carryFlow.pillWord()
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: carryFlow.graphPane
        property: "askHold"
        value: carryFlow.takesAway
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // The warning is for taking away, not for reaching the remote (デザイン規約 §手元の改名の後のリモート).
    Binding {
        target: carryFlow.graphPane
        property: "askNeutral"
        value: !carryFlow.takesAway
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // The hold's tooltip, which says the running order (デザイン規約 §長押し); empty for the two click answers.
    Binding {
        target: carryFlow.graphPane
        property: "askTip"
        value: !carryFlow.takesAway ? "" : carryFlow.kind === "branch"
            ? qsTr("Hold to replace. %1 goes up first, then %2 comes off — so a push the far side turns down leaves the old name where it is. Anything it carried — an open pull request, a running check — does not follow the new name.")
              .arg(carryFlow.newSide).arg(carryFlow.oldSide)
            : qsTr("Hold to replace. %1 goes up first, then %2 comes off — so a push the far side turns down leaves the old name where it is. Anything built from it stays with the name that goes.")
              .arg(carryFlow.newSide).arg(carryFlow.oldSide)
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    /// Automation: the chooser's list, which no injected click can reach — the same door a press uses
    /// (`AppCombo.pressField`), so what answers is the wiring.
    function openChoices() {
        const form = carryFlow.graphPane.askForm
        if (!form || !form.pick)
            return false
        form.pick.pressField()
        return true
    }
    /// Automation: pick one, through the field's own report of a row having been taken.
    function pickChoice(index) {
        const form = carryFlow.graphPane.askForm
        if (!form || !form.pick || index < 0 || index >= carryFlow.choices.length)
            return false
        form.pick.activated(index)
        return true
    }

    Component {
        id: carryForm
        RenameCarryForm {
            choices: carryFlow.choices
            picked: carryFlow.choices[carryFlow.choice] || ""
            onChoicePicked: index => carryFlow.pick(index)
        }
    }
}

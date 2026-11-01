pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// What happens on the remote after a name is changed here — a question, since which the reader wanted is not the
// repository's to know (デザイン規約 §手元の改名の後のリモート). Draws nothing: the bar belongs to the graph.
// Branches and tags take one road; only how the two names are spelled differs (`oldSide` / `newSide`). No grey
// line under the heading (user decision — ask before changing): the three answers say it themselves, words once in
// the bar (デザイン規約 §答えの要らない報せ「言葉はバーに 1 度だけ」); `askDetail` stays empty and `AskBar` folds
// it. Asked only after a branch with an upstream, or a tag the remote holds under the same name on the same commit,
// was renamed (`RepoPage.carryBranchRenameOver` / `armRenameTagRemote`). Raised once the rename has landed
// (`RepoPage`, on `writeBranchOp` / `writeTagOp`), from what was read before it. Headless: `rename-local-upstream*` /
// `rename-tag-remote*`.
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
    /// Which answer is picked. The question opens on "create and keep", the side that can be taken back (user
    /// decision — ask before changing), so the pill is live from the first frame.
    property int choice: carryFlow.keepsBoth

    /// The name the remote carries now, and the one it would carry. A branch's live under `<remote>/`; a tag's have
    /// no namespace of their own, so they are said with the remote beside them (デザイン規約 §タグを作る・送る).
    readonly property string oldSide:
        carryFlow.kind === "branch" ? carryFlow.remote + "/" + carryFlow.from : carryFlow.from
    readonly property string newSide:
        carryFlow.kind === "branch" ? carryFlow.remote + "/" + carryFlow.to : carryFlow.to

    /// The three answers, in the words they are picked by — never "rename" (user decision — ask before changing): the
    /// far side sees a create and a delete, and what hung off the old name (pull request, protection rule, release)
    /// does not follow, so the rows say those two words. The first names the delete first but runs it last (user
    /// decision — ask before changing; `remote::replace_remote_branch`): the reader consents to the half that cannot
    /// be taken back, so it leads the row, while the running order follows the damage of a stop half way — push
    /// first leaves one name too many, delete first a name lost. The hold's tip says the running order (`tipOf`).
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
    /// The answer that takes a name from the remote: whatever hung off it stays behind, and only the old tip comes
    /// back, from the discard record (デザイン規約 §回数押しの色).
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
                : carryFlow.tagHeading(remote),
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

    /// The picked answer's own word for the pill — no one word stands for all three (user decision — ask before
    /// changing); where the word moves the gesture moves with it: at `Replace` it becomes a hold.
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
    // (`AskBar.plainWords`; user decision — ask before changing), the one question that does: every other question's
    // colour is fixed as it opens, so its heading says the cost (デザイン規約 §状態), but here the answer in the box
    // moves the frame and the pill, and a heading moving with them would say it a third time.
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
    // The warning is for taking away, not for reaching the remote (user decision — ask before changing): `Replace`
    // is warning + hold, `Create` and `Leave` the information colour + click.
    Binding {
        target: carryFlow.graphPane
        property: "askNeutral"
        value: !carryFlow.takesAway
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // The hold's tooltip, which says the running order (デザイン規約 §回数押し); empty for the two click answers.
    Binding {
        target: carryFlow.graphPane
        property: "askTip"
        value: carryFlow.tipOf(carryFlow.newSide, carryFlow.oldSide)
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    /// A tag's heading, naming the remote — one spelling for the words and the seat of its mark.
    function tagHeading(remote) {
        //: %1 is a remote. The question a tag renamed here raises about the remote's copy of it.
        return qsTr("Change the tag on %1 too?").arg(remote)
    }
    // The names the bar says, each behind its mark (デザイン規約 §ref の種別「名前の印」): the remote in a tag's heading,
    // the two names in the tip — remote branches, or tags.
    Binding {
        target: carryFlow.graphPane
        property: "askLabelMarks"
        value: carryFlow.kind === "tag"
               ? Words.marksOf(carryFlow.tagHeading, [[carryFlow.remote, "remote"]])
               : []
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: carryFlow.graphPane
        property: "askTipMarks"
        value: Words.marksOf(carryFlow.tipOf, [[carryFlow.newSide, carryFlow.kind === "branch" ? "remote" : "tag"],
                                               [carryFlow.oldSide, carryFlow.kind === "branch" ? "remote" : "tag"]])
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    /// The hold's tip, with the name that comes and the name that goes.
    function tipOf(comes, goes) {
        return !carryFlow.takesAway ? "" : carryFlow.kind === "branch"
            ? qsTr("%1 to replace. %2 goes up first, then %3 comes off — so a push the far side turns down leaves the old name where it is. Anything it carried — an open pull request, a running check — does not follow the new name.")
              .arg(Words.pressCount(Metrics.holdPressesWarned)).arg(comes).arg(goes)
            : qsTr("%1 to replace. %2 goes up first, then %3 comes off — so a push the far side turns down leaves the old name where it is. Anything built from it stays with the name that goes.")
              .arg(Words.pressCount(Metrics.holdPressesWarned)).arg(comes).arg(goes)
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

pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// What happens on the remote after a name is changed here (デザイン規約 §手元の改名をリモートへ運ぶ).
//
// **A question, because git has no answer.** A branch renamed here goes on being measured against the name it was
// renamed away from, and a tag renamed here leaves the remote's copy under the old spelling — and which of those the
// reader wanted is not something the repository knows. The three answers are the three things that can be done over
// there, and the one thing git cannot do is a rename: what the first of them runs is a create and a delete, which is
// why no word here says otherwise.
//
// Sized to the page: nothing is drawn here, and the bar belongs to the graph.
Item {
    id: carryFlow

    required property RepoTab repoTab
    /// The one bar this question stands in, and whose pill it dresses while it stands (`RepoPage.startRowAsk` raises
    /// it).
    required property GraphPane graphPane

    /// Raise the standing question with this form in it. The page owns the bar — every other question in the window
    /// goes up the same way.
    signal askRequested(string oidHex, string label, string accept, var run, var form)
    /// Automation: the question was raised, and the two names it is between (app-ui.md).
    signal carryAsked(string kind, string from, string to)

    /// Whether this question is the one standing, which is what its bindings on the bar hang off.
    property bool asking: false
    /// What was renamed here — `branch` or `tag` — the remote the old name also stands on, and the two names.
    property string kind: ""
    property string remote: ""
    property string from: ""
    property string to: ""
    /// Which answer is picked. **The question opens on the one that takes nothing away**:
    /// a name made over there can be taken off again by the row that deletes it, while a name deleted is gone with
    /// whatever hung off it — so the answer standing in the box when the reader arrives is the one they can walk
    /// back from. It also means the pill is live from the first frame, which is what the reader is agreeing to.
    property int choice: carryFlow.keepsBoth

    /// The name the remote carries now, and the one it would carry. A branch's live under `<remote>/`; a tag's have
    /// no namespace of their own, so they are said with the remote beside them (デザイン規約 §タグを作る・送る).
    readonly property string oldSide:
        carryFlow.kind === "branch" ? carryFlow.remote + "/" + carryFlow.from : carryFlow.from
    readonly property string newSide:
        carryFlow.kind === "branch" ? carryFlow.remote + "/" + carryFlow.to : carryFlow.to

    /// The three answers, in the words they are picked by.
    ///
    /// **The first says the delete first, and runs it last** (CLAUDE.md
    /// 「内部コマンドと UI 表記は分ける」). The row is read before it is picked, and what the reader is agreeing to
    /// there is the half that cannot be walked back: a name going off the remote leads. What runs puts the push
    /// first for the opposite reason — a pair that stops half-way has then made a name rather than lost one
    /// (`remote::rename_remote_branch`). The hold's own words say the running order (`askTip` below).
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

    /// Where each answer stands in the chooser. Named, because two of them are read back as a state of the bar and
    /// one of them is where the question opens.
    readonly property int replacesIt: 0
    readonly property int keepsBoth: 1
    readonly property int leavesAlone: 2
    /// The answer that cannot be taken back: a name goes from the remote, and whatever hung off it stays behind
    /// (デザイン規約 §長押し).
    readonly property bool takesAway: carryFlow.choice === carryFlow.replacesIt
    /// …and the one that writes nothing at all, which is the only one this bar has no warning to give about.
    readonly property bool leavesIt: carryFlow.choice === carryFlow.leavesAlone

    anchors.fill: parent

    /// Opens the question. `oidHex` is the commit the bar marks — **taken by the caller before the write**, since a
    /// rename moves the name and never the object and the rows are rebuilt behind the answer that raises this
    /// (`RepoPage.armRenameTagRemote`).
    function startAsk(kind, remote, from, to, oidHex) {
        carryFlow.kind = kind
        carryFlow.remote = remote
        carryFlow.from = from
        carryFlow.to = to
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

    /// The word on the pill, which is the picked answer's own — **not one word standing for all three**
    /// (デザイン規約 §手元の改名をリモートへ運ぶ). A pill that named the act it was about to run would otherwise be
    /// the one part of this bar that did not, and "do nothing" is not a change. It moves with the chooser, and where
    /// it moves to `Replace` the gesture becomes a hold — the pair the publish question already makes when its own
    /// word turns (§はじめてリモートへ送る).
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
                carryFlow.repoTab.renameRemoteBranch(carryFlow.remote, carryFlow.from, carryFlow.to)
            else if (carryFlow.choice === carryFlow.keepsBoth)
                carryFlow.repoTab.pointUpstreamAndPush(carryFlow.to, carryFlow.remote, carryFlow.to)
        } else if (carryFlow.kind === "tag") {
            if (carryFlow.takesAway)
                carryFlow.repoTab.renameRemoteTag(carryFlow.remote, carryFlow.from, carryFlow.to)
            else if (carryFlow.choice === carryFlow.keepsBoth)
                carryFlow.repoTab.pushTag(carryFlow.remote, carryFlow.to, "")
        }
        // The third answer runs nothing: the bar coming down is the whole of it.
    }

    // The bar's own state follows what has been picked into it, which is why these are bindings: the question changes
    // what it is saying — and what its pill costs — while it stands. **Nothing is put back when they let go**, for
    // the reason the other questions of this kind spell out (`UpstreamFlow`): letting go happens at the press that
    // walks away, and the bar is still on screen for the 200ms after it.
    // **The heading stays the plain ink** while the rest of the bar turns with the answer picked (`AskBar.plainWords`).
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
    // Only the answer that takes a name off the remote is held down. The other two are a click: one adds a name over
    // there and one writes nothing (デザイン規約 §長押し).
    Binding {
        target: carryFlow.graphPane
        property: "askHold"
        value: carryFlow.takesAway
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // And the colour: **the warning is for the answer that takes something away**, not for reaching the remote at all
    // (the same reading the tag menu's rows already carry: a plain `push` adds a name over
    // there and takes nothing, so it is an ordinary row, and the warning belongs to the half that destroys,
    // デザイン規約 §タグを作る・送る). So the answer that makes a name beside the old one is plain information, and so
    // is the one that writes nothing.
    Binding {
        target: carryFlow.graphPane
        property: "askNeutral"
        value: !carryFlow.takesAway
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // What the hold is for, said where a hold says it (デザイン規約 §長押し). Empty for the two answers that are a
    // click, since a pill nobody holds has nothing to explain.
    //
    // **And this is where the running order is said**: the row leads with the delete because that is what is being
    // agreed to, while what runs pushes first so a pair that stops half-way has made a name rather than lost one.
    Binding {
        target: carryFlow.graphPane
        property: "askTip"
        value: !carryFlow.takesAway ? "" : carryFlow.kind === "branch"
            ? qsTr("Hold to change. git has no rename on a remote: %1 goes up first, then %2 comes off — so a push the far side turns down leaves the old name where it is. Anything it carried — an open pull request, a running check — does not follow the new name.")
              .arg(carryFlow.newSide).arg(carryFlow.oldSide)
            : qsTr("Hold to change. git has no rename on a remote: %1 goes up first, then %2 comes off — so a push the far side turns down leaves the old name where it is. Anything built from it stays with the name that goes.")
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

pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui
import platitude.ui.words

// What happens on the remote after a name is changed here — a question, since which the reader wanted is not the
// repository's to know (デザイン規約 §手元の改名の後のリモート). Draws nothing: the bar belongs to the graph.
// Branches and tags take one road; only how the two names are spelled differs (`oldSide` / `newSide`). No grey
// line under the heading (user decision — ask before changing): the answers say it themselves, words once in the
// bar (デザイン規約 §立っている質問は 1 か所で聞く「言葉はバーに 1 度だけ」); `askDetail` stays empty and `AskBar` folds it —
// except while the old remote branch is `gone`: there the answers are not the three, and the line says why
// (user decision — ask before changing). Asked only after a branch with an upstream, or a tag the remote holds
// under the same name on the same commit, was renamed (`RepoPage.carryBranchRenameOver` / `armRenameTagRemote`).
// Raised once the rename has landed
// (`RepoPage`, on `writeBranchOp` / `writeTagOp`), from what was read before it. Headless: `rename-local-upstream*` /
// `rename-tag-remote*`.
Item {
    id: carryFlow

    required property RepoTab repoTab
    /// The remote branches as they are listed: whether the old one still is (`listed`).
    required property NavSectionModel remotesModel
    /// The one bar this question stands in, and whose pill it dresses while it stands (`RepoPage.startRowAsk` raises
    /// it).
    required property GraphPane graphPane

    /// Raise the standing question with this form in it; the page owns the bar.
    signal askRequested(string oidHex, string label, string accept, var run, var form)
    /// Automation: the question was raised, and the two names it is between.
    signal carryAsked(string kind, string from, string to)

    /// Whether this question is the one standing, which is what its bindings on the bar hang off.
    property bool asking: false
    /// What was renamed here — `branch` or `tag`.
    property string kind: ""
    /// The name the remote carries now, and the one it would carry, as the bar says them. A branch's are remote
    /// branches as they are listed (`origin/main`) — the old one the row itself, handed in whole and sent whole. A
    /// tag's have no namespace of their own, so they are said with `remote` beside them (デザイン規約 §タグを作る・送る).
    property string oldSide: ""
    property string newSide: ""
    /// The remote a tag's old name also stands on; empty for a branch, whose row says it.
    property string remote: ""
    /// The branch renamed here, under its new name: what the answers point at the new side, send there, or name
    /// the new side by. Empty for a tag.
    property string branch: ""
    /// The commit the old name was shown on over there as the question rose; empty for a branch whose row was gone
    /// by then.
    property string shown: ""
    /// What `Replace` leases its delete to (`remote::replace_remote_branch` / `replace_remote_tag`): `shown` — a
    /// remote that moved while the question stood is refused —, and for a question that rose with the row gone, the
    /// commit the row shows once it is back.
    readonly property string pin: carryFlow.shown !== "" ? carryFlow.shown : carryFlow.rowOid
    /// The answer the reader picked last. The question opens on "create and keep", the side that can be taken back
    /// (user decision — ask before changing), so the pill is live from the first frame.
    property int wanted: carryFlow.keepsBoth
    /// The answer standing: the one picked, while it is offered (`answers`) — else the one that writes least and
    /// still answers the heading. The pick is kept, so an answer that comes back with its row is standing again.
    readonly property int choice: carryFlow.answers.indexOf(carryFlow.wanted) >= 0 ? carryFlow.wanted
                                : carryFlow.gone ? carryFlow.pointsOnly : carryFlow.keepsBoth

    readonly property int replacesIt: 0
    readonly property int keepsBoth: 1
    readonly property int leavesAlone: 2
    readonly property int pointsOnly: 3
    /// The answers offered, in the chooser's order: the three — but for a branch whose old remote branch is `gone`,
    /// where nothing is there to take away or to keep beside a new one, and what is left to change is the upstream
    /// alone.
    readonly property var answers: carryFlow.kind === "" ? []
        : carryFlow.gone ? [carryFlow.pointsOnly, carryFlow.leavesAlone]
        : [carryFlow.replacesIt, carryFlow.keepsBoth, carryFlow.leavesAlone]
    /// …and their rows (`wordsOf`).
    readonly property var choices: carryFlow.answers.map(answer => carryFlow.wordsOf(answer))
    /// The answer that takes a name from the remote: whatever hung off it stays behind, and only the old tip comes
    /// back, from the discard record (デザイン規約 §回数押しの色).
    readonly property bool takesAway: carryFlow.choice === carryFlow.replacesIt
    /// …and the one that writes nothing at all.
    readonly property bool leavesIt: carryFlow.choice === carryFlow.leavesAlone
    /// The commit the old remote branch's row shows now, empty while the list holds no such row. `total` is read so
    /// a refs read that moves the rows asks again (a binding does not follow a slot call); the lookup is the name
    /// index's.
    readonly property string rowOid: {
        carryFlow.remotesModel.total
        return carryFlow.kind === "branch" ? carryFlow.remotesModel.oidOfName(carryFlow.oldSide) : ""
    }
    /// The old remote branch is gone from over there, as far as this window has read: its row is not in the list —
    /// a fetch pruned it, before the rename, while it ran or while the question stands. Asked, and left standing,
    /// all the same: the upstream still names the old branch. What changes, as the list does, is what is offered
    /// (`answers`) — the upstream's setting alone, nothing sent — and the line under the heading says so
    /// (`goneLine`); the row back, the three answers are back. Never a tag's: its commit is read here, before the
    /// rename.
    readonly property bool gone: carryFlow.kind === "branch" && carryFlow.rowOid === ""

    anchors.fill: parent

    /// An answer in the words it is picked by — never "rename" (user decision — ask before changing): the far side
    /// sees a create and a delete, and what hung off the old name (pull request, protection rule, release) does not
    /// follow, so the rows say those two words. The first names the delete first but runs it last (user decision —
    /// ask before changing; `remote::replace_remote_branch`): the reader consents to the half that cannot be taken
    /// back, so it leads the row, while the running order follows the damage of a stop half way — push first leaves
    /// one name too many, delete first a name lost. The hold's tip says the running order (`tipOf`).
    function wordsOf(answer) {
        if (answer === carryFlow.leavesAlone)
            return qsTr("Do nothing")
        if (answer === carryFlow.pointsOnly)
            //: %1 is a remote branch, e.g. origin/main. Picked from a chooser: the setting alone, nothing is sent.
            return qsTr("Set upstream to %1").arg(carryFlow.newSide)
        const branch = carryFlow.kind === "branch"
        if (answer === carryFlow.replacesIt)
            return branch
                //: %1 and %2 are remote branches, e.g. origin/main. Picked from a chooser.
                ? qsTr("Delete %1, create %2").arg(carryFlow.oldSide).arg(carryFlow.newSide)
                //: %1 and %2 are tag names, %3 a remote. Picked from a chooser.
                : qsTr("Delete %1 on %3, create %2").arg(carryFlow.oldSide).arg(carryFlow.newSide)
                                                   .arg(carryFlow.remote)
        return branch
            ? qsTr("Create %1, keep %2").arg(carryFlow.newSide).arg(carryFlow.oldSide)
            : qsTr("Create %1 on %3, keep %2").arg(carryFlow.newSide).arg(carryFlow.oldSide).arg(carryFlow.remote)
    }

    /// Opens the question a renamed branch raises: `oldRef` the remote branch it was measured against and `newRef`
    /// the one its new name would be, both as listed; `branch` its new name here. `oidHex` is the commit `oldRef`'s
    /// row shows — the bar's mark, and the lease (`pin`) —, empty where the row has gone (`gone`).
    function startBranchAsk(oldRef, newRef, branch, oidHex) {
        carryFlow.remote = ""
        carryFlow.branch = branch
        carryFlow.raise("branch", oldRef, newRef, oidHex)
    }
    /// …and a renamed tag: the two names, and the remote the old one also stands on. `oidHex` is taken by the caller
    /// before the rename: the rows are rebuilt behind the answer that raises this (`RepoPage.armRenameTagRemote`). It
    /// is where the old name stands over there too — the caller asks only where the two agree — so it is also the
    /// lease.
    function startTagAsk(remote, from, to, oidHex) {
        carryFlow.remote = remote
        carryFlow.branch = ""
        carryFlow.raise("tag", from, to, oidHex)
    }
    function raise(kind, oldSide, newSide, oidHex) {
        carryFlow.kind = kind
        carryFlow.oldSide = oldSide
        carryFlow.newSide = newSide
        carryFlow.shown = oidHex
        carryFlow.wanted = carryFlow.keepsBoth
        carryFlow.askRequested(
            oidHex,
            kind === "branch"
                //: The question a branch renamed here raises about the remote branch it was measured against.
                ? qsTr("Change the upstream too?")
                : carryFlow.tagHeading(carryFlow.remote),
            carryFlow.pillWord(),
            carryFlow.answer,
            carryForm)
        // After the bar is up: raising it resets the properties the `asking` bindings below own, and a binding whose
        // value has not changed does not push back.
        carryFlow.asking = true
        carryFlow.carryAsked(kind, carryFlow.oldSide, carryFlow.newSide)
    }

    /// The chooser's row `index` taken: the answer standing there (`answers`).
    function pick(index) {
        if (index < 0 || index >= carryFlow.answers.length)
            return
        carryFlow.wanted = carryFlow.answers[index]
    }

    /// The standing answer's own word for the pill — no one word stands for them all (user decision — ask before
    /// changing); where the word moves the gesture moves with it: at `Replace` it becomes a hold.
    function pillWord() {
        if (carryFlow.takesAway)
            //: The pill answering a standing question: the old name goes off the remote and the new one goes up.
            return qsTr("Replace")
        if (carryFlow.leavesIt)
            //: The pill answering a standing question: nothing over there is touched.
            return qsTr("Leave")
        if (carryFlow.choice === carryFlow.pointsOnly)
            //: The pill answering a standing question: the upstream is recorded, and nothing is sent.
            return qsTr("Set")
        //: The pill answering a standing question: the new name is made and the old one left where it is.
        return qsTr("Create")
    }

    function answer() {
        if (carryFlow.kind === "branch") {
            if (carryFlow.takesAway)
                carryFlow.repoTab.replaceRemoteBranch(carryFlow.oldSide, carryFlow.branch, carryFlow.pin)
            else if (carryFlow.choice === carryFlow.keepsBoth)
                carryFlow.repoTab.pointUpstreamAndPush(carryFlow.branch, carryFlow.oldSide)
            else if (carryFlow.choice === carryFlow.pointsOnly)
                carryFlow.repoTab.pointUpstream(carryFlow.branch, carryFlow.oldSide)
        } else if (carryFlow.kind === "tag") {
            if (carryFlow.takesAway)
                carryFlow.repoTab.replaceRemoteTag(carryFlow.remote, carryFlow.oldSide, carryFlow.newSide,
                                                   carryFlow.shown)
            else if (carryFlow.choice === carryFlow.keepsBoth)
                carryFlow.repoTab.pushTag(carryFlow.remote, carryFlow.newSide, "")
        }
        // `Leave` runs nothing: the bar coming down is the whole of it.
    }

    // Bindings, since the bar changes with the pick while it stands. `RestoreNone`: letting go happens at the press
    // that walks away, with the bar still on screen (`PublishFlow`). The heading stays plain ink
    // (`AskBar.plainWords`; user decision — ask before changing), as only this and `UpstreamFlow`'s unset do: other
    // questions' colour is fixed as they open, so the heading says the cost (デザイン規約 §状態), but here the answer
    // in the box moves the frame and the pill, and a heading moving with them would say it a third time.
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
    // Why the answers are the two they are, while the old remote branch is `gone`, the name behind its mark
    // (デザイン規約 §ref の種別「名前の印」).
    Binding {
        target: carryFlow.graphPane
        property: "askDetail"
        value: carryFlow.gone ? carryFlow.goneLine(carryFlow.oldSide) : ""
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: carryFlow.graphPane
        property: "askDetailMarks"
        value: carryFlow.gone ? Words.marksOf(carryFlow.goneLine, [[carryFlow.oldSide, "remote"]]) : []
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    /// That line — one spelling for the words and the seat of the mark.
    function goneLine(name) {
        //: %1 is a remote branch, e.g. origin/main, that the last fetch no longer found.
        return qsTr("%1 is gone; only the upstream can be set.").arg(name)
    }
    Binding {
        target: carryFlow.graphPane
        property: "askHold"
        value: carryFlow.takesAway
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // The warning is for taking away, not for reaching the remote (user decision — ask before changing): `Replace`
    // is warning + hold, every other answer the information colour + click.
    Binding {
        target: carryFlow.graphPane
        property: "askNeutral"
        value: !carryFlow.takesAway
        when: carryFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // The hold's tooltip, which says the running order (デザイン規約 §回数押し); empty for the click answers.
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
    /// Automation: a row of the chooser picked, through the field's own report of a row having been taken; answers
    /// whether one was.
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
            picked: carryFlow.kind === "" ? "" : carryFlow.wordsOf(carryFlow.choice)
            onChoicePicked: index => carryFlow.pick(index)
        }
    }
}

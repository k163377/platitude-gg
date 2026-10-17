pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// Which remote branch a local one is measured against, from the row that asks to git's write (デザイン規約
// §ブランチが測られる相手を決める), asked in the one question bar (§立っている質問は 1 か所で聞く).
//
// Sized to the page: nothing is drawn here, and the bar belongs to the graph.
Item {
    id: upstreamFlow

    required property RepoTab repoTab
    /// Where the remote-tracking side is read: whether the name answered with is one this repository actually holds.
    required property NavSectionModel remotesModel
    /// The one bar this question stands in, and whose pill it dresses while it stands (`RepoPage.startRowAsk` raises
    /// it).
    required property GraphPane graphPane

    /// Raise the standing question with this form in it; the page owns the bar.
    signal askRequested(string oidHex, string label, string accept, var run, var form)

    /// Whether this question is the one standing, which is what its bindings on the bar hang off.
    property bool asking: false
    /// The branch the answer is about, and the two halves of what it will be measured against.
    property string branch: ""
    property string remote: ""
    property string branchName: ""
    /// Whether that pair names a remote-tracking branch held here. Not a refusal (a name not here is one the next
    /// push makes) — it only picks the bar's line.
    property bool targetIsThere: false
    /// Whether the branch has an upstream to take off — a remote reading, a branch here, or a gone one — which an
    /// emptied name asks to do.
    property bool unsettable: false
    /// The branch the worktree is on: the one name the unset heading draws in `textLink` (デザイン規約 §ref の種別).
    property string headBranch: ""
    /// The name box emptied on a branch with an upstream: the question turns into taking it off — its heading and
    /// pill change, and the line goes (デザイン規約 §ブランチが測られる相手を決める). The remote is only picked, so the
    /// name is the half that can be emptied.
    readonly property bool unsetting: upstreamFlow.unsettable && upstreamFlow.branchName === ""
    /// The unset heading with the branch's seat left in it: a plain delete's words keep the ordinary rules, so the
    /// branch wears its own colour there (`AskBar.labelSentence`, §回数押しの色).
    //: %1 is a branch name. Asks whether it stops being measured against a remote branch.
    readonly property string unsetSentence: qsTr("Unset the upstream of %1?")
    readonly property string heading: upstreamFlow.unsetting
        ? upstreamFlow.unsetSentence.arg(upstreamFlow.branch)
        //: %1 is a branch name. Asks which remote branch it is measured against.
        : qsTr("Upstream of %1?").arg(upstreamFlow.branch)
    readonly property string pill: upstreamFlow.unsetting
        //: The pill that answers the question above: takes the upstream off.
        ? qsTr("Unset")
        //: The pill that answers the question above: records the upstream just named.
        : qsTr("Set")

    readonly property var remotes: upstreamFlow.repoTab.remoteNames
    /// What the chosen remote already carries here, the name box's suggestions. `total` is read so every refs read
    /// asks again (a binding does not follow a slot call).
    ///
    /// Only while the question stands: the binding re-runs on every refs read, and the list costs the number of refs
    /// (internal-docs/性能.md §設計規則). Landing a frame after the bar is fine (`AppCombo.wanted`).
    readonly property var branches: {
        upstreamFlow.remotesModel.total
        return upstreamFlow.asking
             ? upstreamFlow.remotesModel.branchesOn(upstreamFlow.remote, upstreamFlow.repoTab.remoteNames)
             : []
    }
    readonly property string target: upstreamFlow.remote + "/" + upstreamFlow.branchName
    readonly property bool filled: upstreamFlow.remote !== "" && upstreamFlow.branchName !== ""

    anchors.fill: parent

    /// Opens the question on this branch, at `counterpart` — the remote branch it already tracks, or empty.
    /// `unsettable`: it has an upstream an emptied name takes off.
    function startAsk(branch, oidHex, counterpart, unsettable) {
        upstreamFlow.branch = branch
        upstreamFlow.unsettable = unsettable
        // The cut is the configured remote name where one owns the ref — a remote's own name may contain `/`
        // (`GitFacts.remoteOfRef`).
        const from = counterpart !== ""
                   ? GitFacts.remoteOfRef(counterpart, upstreamFlow.repoTab.remoteNames) : ""
        upstreamFlow.remote = from !== "" ? from : upstreamFlow.repoTab.defaultRemote
        upstreamFlow.branchName = from !== ""
            ? GitFacts.branchOfRef(counterpart, upstreamFlow.repoTab.remoteNames) : branch
        upstreamFlow.settleTarget()
        upstreamFlow.askRequested(oidHex, upstreamFlow.heading, upstreamFlow.pill, upstreamFlow.answer, upstreamForm)
        // After the bar is up: raising it resets the properties the `asking` bindings below own, and a
        // binding whose value has not changed does not push back.
        upstreamFlow.asking = true
    }

    function pickRemote(index) {
        if (index < 0 || index >= upstreamFlow.remotes.length)
            return
        upstreamFlow.remote = upstreamFlow.remotes[index]
        upstreamFlow.settleTarget()
    }

    /// The name box's edits; automation writes here too (no injected key reaches the box offscreen).
    function setBranchName(name) {
        upstreamFlow.branchName = name
        upstreamFlow.settleTarget()
    }

    function settleTarget() {
        upstreamFlow.targetIsThere =
            upstreamFlow.filled
            && upstreamFlow.remotesModel.oidOfName(upstreamFlow.target) !== ""
    }

    function answer() {
        if (upstreamFlow.unsetting)
            upstreamFlow.repoTab.unsetUpstream(upstreamFlow.branch)
        else
            upstreamFlow.repoTab.setUpstream(upstreamFlow.branch, upstreamFlow.remote, upstreamFlow.branchName)
    }

    // Bindings, since the question changes while it stands. `RestoreNone`: letting go happens at the press that
    // walks away, with the bar still on screen (`PublishFlow`).
    Binding {
        target: upstreamFlow.graphPane
        property: "askAnswerable"
        value: upstreamFlow.filled || upstreamFlow.unsetting
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // Taking it off is the plain delete's two presses, as the card's row is (デザイン規約 §回数押しの色); a held pill is
    // answered by the pill alone, so Enter in the emptied box does nothing (`AskBar.answerFromForm`).
    Binding {
        target: upstreamFlow.graphPane
        property: "askHold"
        value: upstreamFlow.unsetting
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: upstreamFlow.graphPane
        property: "askPlainHold"
        value: upstreamFlow.unsetting
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: upstreamFlow.graphPane
        property: "askLabel"
        value: upstreamFlow.heading
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // The unset heading's branch in its own colour (`unsetSentence`); the set heading stays one run of words.
    Binding {
        target: upstreamFlow.graphPane
        property: "askLabelSentence"
        value: upstreamFlow.unsetting ? upstreamFlow.unsetSentence : ""
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: upstreamFlow.graphPane
        property: "askLabelRef"
        value: upstreamFlow.unsetting ? upstreamFlow.branch : ""
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: upstreamFlow.graphPane
        property: "askLabelRefTint"
        value: upstreamFlow.branch === upstreamFlow.headBranch ? Theme.textLink : Theme.textPrimary
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: upstreamFlow.graphPane
        property: "askAccept"
        value: upstreamFlow.pill
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // Asking for a name is the accent; taking the upstream off asks for nothing, so it is the plain delete's dress
    // instead — the heading in the plain ink, the warning on the rule, the frame and the pill's mark
    // (デザイン規約 §ブランチが測られる相手を決める, §回数押しの色).
    Binding {
        target: upstreamFlow.graphPane
        property: "askNeutral"
        value: !upstreamFlow.unsetting
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: upstreamFlow.graphPane
        property: "askPlainWords"
        value: upstreamFlow.unsetting
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: upstreamFlow.graphPane
        property: "askDetail"
        // The unset question says it all in its heading: what an upstream decides goes with the word.
        value: upstreamFlow.unsetting ? ""
               : !upstreamFlow.filled
               ? qsTr("Pick the upstream.")
               : !upstreamFlow.targetIsThere
                 //: %1 is a remote branch, e.g. origin/main.
                 ? qsTr("%1 has not been fetched here; a push sends this branch there.")
                   .arg(upstreamFlow.target)
                 //: %1 is a remote branch, e.g. origin/main.
                 : qsTr("Ahead and behind are counted against %1.").arg(upstreamFlow.target)
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }

    /// Automation: opens the name box's list (no injected click reaches it) through a press's door
    /// (`AppCombo.pressField`).
    function openBranches() {
        const form = upstreamFlow.graphPane.askForm
        if (!form || !form.branchPick)
            return false
        form.branchPick.pressField()
        return true
    }
    /// Automation reads the popup itself: the call that requested it may not have put it on screen.
    function branchesOpen() {
        const form = upstreamFlow.graphPane.askForm
        return Boolean(form && form.branchPick && form.branchPick.popup.visible)
    }
    /// Automation: Enter in the name box. No key reaches the box offscreen, so this raises the box's own `accepted`,
    /// which `AppCombo.submitted` hangs off; that Enter raises it is fixed in `tst_appcombo`.
    function enterBranch() {
        const form = upstreamFlow.graphPane.askForm
        if (!form || !form.branchPick)
            return false
        form.branchPick.accepted()
        return true
    }

    Component {
        id: upstreamForm
        UpstreamForm {
            remotes: upstreamFlow.remotes
            branches: upstreamFlow.branches
            remote: upstreamFlow.remote
            branch: upstreamFlow.branchName
            onRemotePicked: index => upstreamFlow.pickRemote(index)
            onBranchEdited: name => upstreamFlow.setBranchName(name)
        }
    }
}

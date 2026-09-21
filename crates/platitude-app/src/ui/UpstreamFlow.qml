pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// Which remote branch a local one is measured against, from the row that asks to git's write (デザイン規約
// §ブランチが測られる相手を決める).
//
// **A question**, because nothing here knows the answer: a branch may belong with a name
// unlike its own, on any of several remotes, and the same-named one is a guess. The two halves are answered in the
// one bar every other question stands in (§立っている質問は 1 か所で聞く).
//
// **Information** — the setting is this repository's own config and takes nothing away, so the bar is
// the neutral colour and a click answers it (§はじめてリモートへ送る, the other question of this kind).
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

    /// Raise the standing question with this form in it. The page owns the bar — every other question in the window
    /// goes up the same way.
    signal askRequested(string oidHex, string label, string accept, var run, var form)

    /// Whether this question is the one standing, which is what its bindings on the bar hang off.
    property bool asking: false
    /// The branch the answer is about, and the two halves of what it will be measured against.
    property string branch: ""
    property string remote: ""
    property string branchName: ""
    /// Whether that pair names a remote-tracking branch this repository holds. **git refuses an upstream it cannot
    /// find** (`fatal: the requested upstream branch … does not exist`, measured), and the answer is here
    /// — the refs are already on this side — so the pill goes quiet.
    property bool targetIsThere: false

    readonly property var remotes: upstreamFlow.repoTab.remoteNames
    readonly property string target: upstreamFlow.remote + "/" + upstreamFlow.branchName
    readonly property bool filled: upstreamFlow.remote !== "" && upstreamFlow.branchName !== ""

    anchors.fill: parent

    /// Opens the question on this branch. `counterpart` is the remote branch it already speaks for, empty where it
    /// speaks for none — where it does, that is where the question opens, so changing one half is one gesture.
    function startAsk(branch, oidHex, counterpart) {
        upstreamFlow.branch = branch
        // The cut is the configured remote name where one owns the ref — a remote's own name may contain `/`
        // (`GitFacts.remoteOfRef`).
        const from = counterpart !== ""
                   ? GitFacts.remoteOfRef(counterpart, upstreamFlow.repoTab.remoteNames) : ""
        upstreamFlow.remote = from !== "" ? from : upstreamFlow.repoTab.defaultRemote
        upstreamFlow.branchName = from !== ""
            ? GitFacts.branchOfRef(counterpart, upstreamFlow.repoTab.remoteNames) : branch
        upstreamFlow.settleTarget()
        upstreamFlow.askRequested(
            oidHex,
            //: %1 is a branch name. Asks which remote branch it is measured against.
            qsTr("Upstream of %1?").arg(branch),
            //: The pill that answers the question above: records the upstream just named.
            qsTr("Set"),
            upstreamFlow.answer,
            upstreamForm)
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

    /// Typing into the name box, and where the automation writes too — no injected key reaches the box on the
    /// offscreen platform.
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
        upstreamFlow.repoTab.setUpstream(
            upstreamFlow.branch, upstreamFlow.remote, upstreamFlow.branchName)
    }

    // The bar's own state follows what has been answered into it, which is why these are bindings:
    // the question changes what it is saying while it stands. **Nothing is put back when they let go**, for
    // the reason the other question of this kind spells out (`PublishFlow`): letting go happens at the press that
    // walks away, and the bar is still on screen for the 200ms after it.
    Binding {
        target: upstreamFlow.graphPane
        property: "askAnswerable"
        value: upstreamFlow.targetIsThere
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: upstreamFlow.graphPane
        property: "askNeutral"
        value: true
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: upstreamFlow.graphPane
        property: "askDetail"
        value: !upstreamFlow.filled
               ? qsTr("Pick the upstream.")
               : !upstreamFlow.targetIsThere
                 //: %1 is a remote branch, e.g. origin/main.
                 ? qsTr("%1 has not been fetched here.").arg(upstreamFlow.target)
                 //: %1 is a remote branch, e.g. origin/main.
                 : qsTr("Ahead and behind are counted against %1.").arg(upstreamFlow.target)
        when: upstreamFlow.asking
        restoreMode: Binding.RestoreNone
    }

    Component {
        id: upstreamForm
        UpstreamForm {
            remotes: upstreamFlow.remotes
            remote: upstreamFlow.remote
            branch: upstreamFlow.branchName
            // An empty box is a question not yet answered (デザイン規約 §可否・警告の出し場所).
            refused: upstreamFlow.filled && !upstreamFlow.targetIsThere
            onRemotePicked: index => upstreamFlow.pickRemote(index)
            onBranchEdited: name => upstreamFlow.setBranchName(name)
        }
    }
}

pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// Everything between the toolbar's push button and git's answer: what the branch can do with its remote, the question
// the first push has to ask, and what a refusal leaves behind (デザイン規約 §リモートへ送る).
//
// The page keeps the toolbar's names — the band reads them through the active page — and hands the answers to git's
// writes back here.
//
// Sized to the page: the add-remote dialog centres itself in the item it was declared under.
Item {
    id: publishFlow

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    /// Where a remote-tracking branch's commit is read from.
    required property NavSectionModel remotesModel
    /// The one bar this question stands in, and whose pill it dresses while it stands (`RepoPage.startRowAsk` raises
    /// it).
    required property GraphPane graphPane

    /// Raise the standing question with this form in it. The page owns the bar — every other question in the window
    /// goes up the same way.
    signal askRequested(string label, var run, var form, string code, string refName)

    /// The add-remote dialog, which the automation types into.
    readonly property alias dialog: remoteDialog

    anchors.fill: parent

    // ---- push ------------------------------------------------------
    /// Where the button would send this branch. **A marked remote takes the push from the upstream** — every branch
    /// goes there under its own name, whatever it tracks (デザイン規約 §リモートを書き留める; git's own order, measured). Only where
    /// nothing is marked does the upstream answer.
    ///
    /// The order is core's (`platitude_core::remote::push_target`, through the pure `GitFacts` slot), the same table
    /// the send itself reads: **the branch's own mark first**, then the repository's, then what the branch tracks.
    /// Spelling it out here instead is how a fork's label named the repository it forked from.
    readonly property string pushTargetLabel:
        GitFacts.pushTarget(publishFlow.workTree.branch, publishFlow.workTree.upstream,
                            publishFlow.workTree.pushRemote, publishFlow.repoTab.pushDefault,
                            publishFlow.repoTab.defaultRemote, publishFlow.repoTab.remoteNames)
    /// What the branch can do with its remote, worked out before anything is sent (デザイン規約 §リモートへ送る):
    ///
    /// - `closed`   — there is no branch here to send
    /// - `publish`  — the branch has never been sent, so where it goes is
    ///                a question rather than something to look up. A
    ///                repository with no remote at all lands here too: the
    ///                question can make one
    /// - `elsewhere`— the push goes to the marked remote, which is not the
    ///                one this branch tracks, so the counts are about
    ///                somewhere else and say nothing at all
    /// - `ready`    — commits of ours to add, and nothing in the way
    /// - `clean`    — the remote already has them all
    /// - `behind`   — the remote moved on; we have nothing to add
    /// - `diverged` — both moved; only an overwrite can land
    ///
    /// The decision table itself lives in core (`platitude_core::remote::push_standing`, called through the pure
    /// `GitFacts` slot — every input is a property of this binding, so it re-reads when any of them moves). The
    /// tab-lifecycle half of `closed` is this side's own; whether the marked remote is the one the branch tracks is
    /// answered by the configured names, not the first slash, and the branch's own mark is weighed before the
    /// repository's — `elsewhere` is about wherever `pushTargetLabel` says the push is going. The counts behind this
    /// come from the last fetch, so they prove the negative only: a push may still be refused when they say it fits.
    readonly property string pushState:
        // Closed as well while the counts are not yet about the branch HEAD is on (`WorkTreeModel.countsSettled`):
        // between a move of HEAD and the status read behind it, the branch is named and its standing is not.
        publishFlow.repoTab.state !== "open" || !publishFlow.workTree.countsSettled ? "closed"
        : GitFacts.pushStanding(publishFlow.workTree.unborn,
                                publishFlow.workTree.detached, publishFlow.workTree.branch,
                                publishFlow.workTree.upstream, publishFlow.workTree.upstreamTracked,
                                publishFlow.workTree.ahead, publishFlow.workTree.behind,
                                publishFlow.workTree.pushRemote, publishFlow.repoTab.pushDefault,
                                publishFlow.repoTab.remoteNames)
    readonly property bool canPush: (publishFlow.pushState === "publish"
                                     || publishFlow.pushState === "elsewhere"
                                     || publishFlow.pushState === "ready")
                                    && publishFlow.repoTab.busyCount === 0
    /// `behind` is here for the same reason `diverged` is: the far side has commits this branch does not, and an
    /// overwrite is the only send that reaches from there (デザイン規約 §リモートへ送る). Both hold a tracking ref for
    /// the lease to pin to.
    readonly property bool canForcePush: (publishFlow.pushState === "ready"
                                          || publishFlow.pushState === "behind"
                                          || publishFlow.pushState === "diverged")
                                         && publishFlow.repoTab.busyCount === 0
    /// The branch a push of this button's is out for, from the send until the answer. What comes back names the
    /// operation and not what it was about, and `push` is also what a remote branch's rename and delete report as — so
    /// this says both which branch the answer belongs to and whether it belongs to this button at all.
    property string pushSentBranch: ""
    /// The branch whose last push git turned down, and what it said. Remembered per branch: most refusals are about
    /// that branch's standing with its remote and say nothing about the one beside it (デザイン規約 §リモートへ送る).
    property string pushFailBranch: ""
    property string pushFailReason: ""
    readonly property bool pushFailed:
        publishFlow.pushFailBranch !== "" && publishFlow.pushFailBranch === publishFlow.workTree.branch
    function pushNow() {
        if (publishFlow.pushState === "publish") {
            publishFlow.startPublishAsk()
            return
        }
        publishFlow.pushSentBranch = publishFlow.workTree.branch
        publishFlow.repoTab.pushCurrent("", "")
    }
    /// The lease is pinned to the commit this window has on screen rather than left to compare against the tracking
    /// ref: a background fetch must not turn this into a plain force. A remote that moved since is refused, and the
    /// refusal is answered by a fetch (core).
    function forcePush() {
        publishFlow.pushSentBranch = publishFlow.workTree.branch
        publishFlow.repoTab.pushCurrent("lease", publishFlow.upstreamOid())
    }
    /// Commit the remote-tracking branch points at, as shown here.
    function upstreamOid() {
        return publishFlow.workTree.upstream !== ""
               ? publishFlow.remotesModel.oidOfName(publishFlow.workTree.upstream)
               : ""
    }
    /// A push this button sent has come back (`writePushed` — the page calls this once per answer). Nothing in the
    /// answer says which branch it was for, and a remote branch's rename and delete answer under the same word — the
    /// slot filled at the send is what makes this the toolbar button's news (デザイン規約 §リモートへ送る).
    function noteWriteAnswer() {
        if (!publishFlow.repoTab.writePushed || publishFlow.pushSentBranch === "")
            return
        const sent = publishFlow.pushSentBranch
        publishFlow.pushSentBranch = ""
        if (publishFlow.repoTab.writeRefused) {
            publishFlow.pushFailBranch = sent
            publishFlow.pushFailReason = publishFlow.repoTab.lastWriteError
        } else if (publishFlow.pushFailBranch === sent) {
            // Landed. The button that went through must not go on saying that the go before it did not.
            publishFlow.pushFailBranch = ""
            publishFlow.pushFailReason = ""
        }
    }

    // ---- the first push: where does this branch go? -----------------
    // (デザイン規約 §はじめてリモートへ送る). Three things raise it — this button, a graph row, a REMOTES row — all through the one bar.
    property bool publishAsking: false
    /// The remote the answer picks.
    property string publishRemote: ""
    /// What the branch is to be called over there — its own name unless the answer says otherwise.
    property string publishBranch: ""

    readonly property var publishRemotes: {
        const packed = publishFlow.repoTab.remoteNames
        return packed === "" ? [] : packed.split(String.fromCharCode(31))
    }
    /// The chooser's last row: opens the add-remote dialog. The question stays standing behind it and picks the new
    /// remote up when it lands.
    readonly property string publishAddChoice: Words.addRemote
    readonly property var publishChoices:
        publishFlow.publishRemotes.concat([publishFlow.publishAddChoice])
    /// The `+` on the left menu's REMOTES band, with no question
    /// standing behind it. The same dialog the chooser's last row opens
    /// (規約 §リモートを書き留める) — a second way in would be a second form,
    /// and this one already knows which names are taken.
    function startAddRemote() {
        remoteDialog.start("", "", publishFlow.publishRemotes, false, true)
    }
    /// The same form with the half that is already known filled in: this remote's URL, and whether it is the one
    /// pushes go to. Raised from the remote's own row on the left menu (デザイン規約 §リモートを書き留める).
    function startEditRemote(name) {
        remoteDialog.start(name, publishFlow.repoTab.remoteUrl(name), publishFlow.publishRemotes,
                           name === publishFlow.repoTab.pushDefault,
                           publishFlow.repoTab.pushDefaultLocal)
    }
    function choosePublishRemote(index) {
        if (index >= publishFlow.publishRemotes.length) {
            publishFlow.startAddRemote()
            return
        }
        publishFlow.publishRemote = publishFlow.publishRemotes[index]
        publishFlow.refreshPublishCheck()
    }

    readonly property string publishTarget: publishFlow.publishRemote + "/" + publishFlow.publishBranch
    readonly property bool publishFilled: publishFlow.publishBranch !== "" && publishFlow.publishRemote !== ""
    /// The remote has answered for exactly what is typed now.
    readonly property bool publishChecked:
        publishFlow.repoTab.remoteBranchAsked === publishFlow.publishRemote
        + String.fromCharCode(31) + publishFlow.publishBranch
    /// …and what that answer says a push under this name would meet (`platitude_core::remote::RemoteBranchState`).
    /// Empty until the answer for exactly this name is in.
    readonly property string publishState:
        publishFlow.publishChecked ? publishFlow.repoTab.remoteBranchState : ""
    /// The far side cannot be settled from this end. Sending stays a click — a plain push can only fast-forward, so
    /// nothing over there can be lost by pressing — but the bar wears the frame and the mark (デザイン規約 §リモートへ送る).
    readonly property bool publishUnsure:
        publishFlow.publishState === "unknown" || publishFlow.publishState === "unreachable"
    /// The name is taken by commits this history does not have: a plain push cannot land at all (measured), only an
    /// overwrite can, so the pill becomes the diverged branch's `push -f` — held, warning-coloured (デザイン規約 §リモートへ送る).
    readonly property bool publishRefused: publishFlow.publishState === "refused"
    /// The commit the question showed on the far side, which is what an overwrite leases against: a remote that moved
    /// since is refused by git rather than flattened (§相手の履歴を置き換える).
    readonly property string publishLease:
        publishFlow.publishRefused ? publishFlow.repoTab.remoteBranchTip : ""

    /// Asked once the typing settles, not per keystroke: the check is a network round trip.
    Timer {
        id: publishCheckTimer
        interval: 350
        onTriggered: {
            if (publishFlow.publishAsking && publishFlow.publishRemote !== "" && publishFlow.publishBranch !== "")
                publishFlow.repoTab.checkRemoteBranch(publishFlow.publishRemote, publishFlow.publishBranch)
        }
    }
    function refreshPublishCheck() {
        publishCheckTimer.restart()
    }

    // The bar's own state follows what has been typed into it, which is why these are bindings rather than arguments:
    // the question changes what it is asking while it stands.
    //
    // **Nothing is put back when they let go** (`RestoreNone`). `publishAsking` drops at the press that walks away —
    // it has to, or the check timer would still be firing a round trip at a remote for a question nobody is asking any
    // more — and a restore there lands on a bar that is still on screen, retracting: the blue frame this question
    // wears would turn `warning` for the 200ms it takes to go, which is the flash that was reported (observed). Every
    // one of these is written again by `GraphPane.startAsking`, so the next question dresses the bar whole and there
    // is nothing for a restore to be right about.
    Binding {
        target: publishFlow.graphPane
        property: "askAnswerable"
        value: publishFlow.publishFilled && publishFlow.publishChecked
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: publishFlow.graphPane
        property: "askHold"
        value: publishFlow.publishRefused
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: publishFlow.graphPane
        property: "askNeutral"
        value: !publishFlow.publishRefused
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: publishFlow.graphPane
        property: "askCode"
        value: publishFlow.publishRefused ? "push -f" : "push"
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: publishFlow.graphPane
        property: "askAlert"
        value: publishFlow.publishUnsure
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: publishFlow.graphPane
        property: "askDetail"
        value: !publishFlow.publishFilled ? qsTr("Pick where it goes.")
               : !publishFlow.publishChecked
                 ? qsTr("Asking %1 what it has…").arg(publishFlow.publishRemote)
                 : publishFlow.publishState === "unreachable"
                   ? qsTr("%1 did not answer — it may already have that branch.")
                     .arg(publishFlow.publishRemote)
                   : publishFlow.publishState === "unknown"
                     ? qsTr("%1 already exists, and what it holds is not here.")
                       .arg(publishFlow.publishTarget)
                     : publishFlow.publishRefused
                       ? qsTr("Commits only %1 has stop being on it.")
                         .arg(publishFlow.publishTarget)
                       : publishFlow.publishState === "fast-forward"
                         ? qsTr("%1 already exists — your commits go on top of it.")
                           .arg(publishFlow.publishTarget)
                         : qsTr("%1 does not exist yet; this makes it.")
                           .arg(publishFlow.publishTarget)
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: publishFlow.graphPane
        property: "askTip"
        value: publishFlow.publishRefused
               ? qsTr("Hold to overwrite %1, dropping %n commit(s) it has and yours does not. A remote that moved since is refused.",
                      "", publishFlow.repoTab.remoteBranchTheirs)
                 .arg(publishFlow.publishTarget)
               : publishFlow.publishState === "unknown"
                 ? qsTr("%1 was never fetched here, so what it holds cannot be read — the push can only fast-forward it.")
                   .arg(publishFlow.publishTarget)
                 : publishFlow.publishState === "unreachable"
                   ? qsTr("%1 did not answer; the push can only fast-forward, and its answer will say what went wrong.")
                     .arg(publishFlow.publishRemote)
                   : ""
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }

    function startPublishAsk() {
        publishFlow.publishRemote = publishFlow.repoTab.defaultRemote
        publishFlow.publishBranch = publishFlow.workTree.branch
        // `push` goes untranslated — it is the command's spelling, not a word for it.
        // The name's seat stays open: the bar draws it in the colour it wears everywhere else (デザイン規約 §ref の種別).
        publishFlow.askRequested(
            qsTr("%1 where?"), publishFlow.answerPublish, publishForm, "push", publishFlow.workTree.branch)
        // After the bar is up, never before: raising it resets the properties the `publishAsking` bindings above own,
        // and a binding whose value has not changed does not push back.
        publishFlow.publishAsking = true
        publishFlow.refreshPublishCheck()
        // No remote at all: the chooser holds nothing but its last row, so the dialog that row opens comes up unasked
        // (デザイン規約 §はじめてリモートへ送る).
        if (publishFlow.publishRemotes.length === 0)
            publishFlow.choosePublishRemote(publishFlow.publishRemotes.length)
    }
    /// Typing into the name box. The automation writes through here too, since no injected key can reach the box on the
    /// offscreen platform.
    function setPublishBranch(name) {
        publishFlow.publishBranch = name
        publishFlow.refreshPublishCheck()
    }
    /// Automation: the chooser's last row, and what gets typed into the dialog it opens (`<name>|<url>`, either half
    /// may be empty).
    function startPublishAddRemote(spec) {
        const parts = spec === "" ? [] : spec.split("|")
        publishFlow.choosePublishRemote(publishFlow.publishRemotes.length)
        remoteDialog.setFields(parts.length > 0 ? parts[0] : "", parts.length > 1 ? parts[1] : "")
    }
    /// Automation: the chooser's list, which no injected click can reach.
    function openPublishRemotes() {
        const form = publishFlow.graphPane.askForm
        if (!form || !form.remotePick)
            return false
        form.remotePick.popup.open()
        return form.remotePick.popup.visible
    }
    /// Whether the list has a row to put the mark on — the field its rows read, not a row of its own. A picture of a
    /// list with no mark in it and a picture of a list whose mark was never plumbed frame the same way.
    function publishRemotesMarked() {
        const form = publishFlow.graphPane.askForm
        return Boolean(form && form.remotePick && form.remotePick.markedRow !== "")
    }
    /// Automation reads the popup itself, rather than assuming that the call which requested it also put it on screen.
    function publishRemotesOpen() {
        const form = publishFlow.graphPane.askForm
        return Boolean(form && form.remotePick
                       && form.remotePick.popup.visible)
    }

    function answerPublish() {
        // The same slot a plain push fills: a refused first push is still this button's news (デザイン規約 §リモートへ送る).
        publishFlow.pushSentBranch = publishFlow.workTree.branch
        publishFlow.repoTab.publishCurrent(publishFlow.publishRemote,
                                           publishFlow.publishBranch,
                                           publishFlow.publishLease)
    }

    RemoteDialog {
        id: remoteDialog
        onSubmitted: (name, url) => {
            if (remoteDialog.editing === "")
                publishFlow.repoTab.addRemote(name, url)
            else
                publishFlow.repoTab.setRemoteUrl(name, url)
            if (publishFlow.publishAsking) {
                publishFlow.publishRemote = name
                publishFlow.refreshPublishCheck()
            }
        }
        // Clearing goes through the same slot with nothing to point at. It reaches this repository's config only —
        // the box says so itself where the mark came from somewhere else, and is not offered there.
        onMarkChanged: (name, marked) => publishFlow.repoTab.setPushDefault(marked ? name : "")
    }

    Component {
        id: publishForm
        PublishForm {
            choices: publishFlow.publishChoices
            markedRemote: publishFlow.repoTab.pushDefault
            remote: publishFlow.publishRemote
            branch: publishFlow.publishBranch
            onRemotePicked: index => publishFlow.choosePublishRemote(index)
            onBranchEdited: name => publishFlow.setPublishBranch(name)
        }
    }
}

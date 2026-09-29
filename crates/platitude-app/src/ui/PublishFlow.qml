pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// Everything between the toolbar's push button and git's answer: what the branch can do with its remote, the first
// push's question, and what a refusal leaves behind (デザイン規約 §リモートへ送る). The band reads this through the
// active page, which hands git's answers back here.
//
// Sized to the page: the add-remote dialog centres itself in the item it was declared under.
Item {
    id: publishFlow

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    /// Where a remote-tracking branch's commit is read from.
    required property NavSectionModel remotesModel
    /// Holds the bar this question stands in and dresses (`RepoPage.startRowAsk` raises it).
    required property GraphPane graphPane

    /// Raise the standing question with this form — the page owns the bar.
    signal askRequested(string label, var run, var form, string code, string refName)

    /// The add-remote dialog, which the automation types into.
    readonly property alias dialog: remoteDialog

    anchors.fill: parent

    // ---- push ------------------------------------------------------
    /// Where the button would send this branch: the branch's own mark, then the repository's, then the upstream
    /// (デザイン規約 §リモートを書き留める). **The order is core's** (`platitude_core::remote::push_target`), the same the send
    /// reads — never re-derive it here, or the label and the send part ways.
    readonly property string pushTargetLabel:
        GitFacts.pushTarget(publishFlow.workTree.branch, publishFlow.workTree.upstream,
                            publishFlow.workTree.pushRemote, publishFlow.repoTab.pushDefault,
                            publishFlow.repoTab.defaultRemote, publishFlow.repoTab.remoteNames)
    /// What the branch can do with its remote (デザイン規約 §リモートへ送る):
    ///
    /// - `closed`    — no branch to send
    /// - `publish`   — never sent: where it goes is a question (also with no remote — the question can make one)
    /// - `elsewhere` — the push goes to a marked remote this branch does not track (`pushTargetLabel`), and nothing
    ///                 here tracks the branch over there, so no counts speak
    /// - `ready`     — commits to add, nothing in the way
    /// - `clean`     — the remote has them all
    /// - `behind`    — the remote moved on; nothing to add
    /// - `diverged`  — both moved; only an overwrite can land
    ///
    /// The table is core's (`platitude_core::remote::push_standing`, through a pure `GitFacts` slot whose inputs are
    /// all bound here), read against the destination's own counts where the push goes elsewhere (`pushElsewhere`).
    /// The counts are the last fetch's, so they prove the negative only: a push may still be refused.
    readonly property string pushState:
        // `closed` too until the counts are about HEAD's branch (`WorkTreeModel.countsSettled`).
        publishFlow.repoTab.state !== "open" || !publishFlow.workTree.countsSettled ? "closed"
        : GitFacts.pushStanding(publishFlow.workTree.unborn,
                                publishFlow.workTree.detached, publishFlow.workTree.branch,
                                publishFlow.workTree.upstream, publishFlow.workTree.upstreamTracked,
                                publishFlow.workTree.ahead, publishFlow.workTree.behind,
                                publishFlow.workTree.pushRemote, publishFlow.repoTab.pushDefault,
                                publishFlow.repoTab.remoteNames, publishFlow.workTree.pushTracking,
                                publishFlow.workTree.pushAhead, publishFlow.workTree.pushBehind)
    /// A mark sends the push to another remote than the upstream's: the counts that speak for it are then the
    /// destination's own, never the upstream's (デザイン規約 §リモートへ送る). Core's rule, the one `pushState` weighs.
    readonly property bool pushElsewhere:
        GitFacts.pushGoesElsewhere(publishFlow.workTree.upstream, publishFlow.workTree.pushRemote,
                                   publishFlow.repoTab.pushDefault, publishFlow.repoTab.remoteNames)
    /// The counts the standing reads, for the words that say them (`BandPushButton.tip`), and the remote branch they
    /// are against, which an overwrite leases to.
    readonly property int pushAhead: publishFlow.pushElsewhere ? publishFlow.workTree.pushAhead
                                                               : publishFlow.workTree.ahead
    readonly property int pushBehind: publishFlow.pushElsewhere ? publishFlow.workTree.pushBehind
                                                                : publishFlow.workTree.behind
    readonly property string pushMeasured: publishFlow.pushElsewhere ? publishFlow.workTree.pushTracking
                                                                     : publishFlow.workTree.upstream
    readonly property bool canPush: (publishFlow.pushState === "publish"
                                     || publishFlow.pushState === "elsewhere"
                                     || publishFlow.pushState === "ready")
                                    && publishFlow.repoTab.busyCount === 0
    /// `behind` too: the far side has commits this branch lacks, so only an overwrite reaches, and both states have a
    /// tracking ref to lease against.
    readonly property bool canForcePush: (publishFlow.pushState === "ready"
                                          || publishFlow.pushState === "behind"
                                          || publishFlow.pushState === "diverged")
                                         && publishFlow.repoTab.busyCount === 0
    /// The branch whose last push git turned down, and what it said — per branch, since a refusal is about that
    /// branch's standing (デザイン規約 §リモートへ送る).
    property string pushFailBranch: ""
    property string pushFailReason: ""
    readonly property bool pushFailed:
        publishFlow.pushFailBranch !== "" && publishFlow.pushFailBranch === publishFlow.workTree.branch
    /// The branch travels with the press: the tab keeps it beside the push's id and hands it back with the answer
    /// (`RepoTab.pushAnswer` / `pushAnswerBranch`) — nothing here holds it across the round trip.
    function pushNow() {
        if (publishFlow.pushState === "publish") {
            publishFlow.startPublishAsk()
            return
        }
        publishFlow.repoTab.pushCurrent(publishFlow.workTree.branch, "", "")
    }
    /// Leased against the commit this window shows, not the tracking ref — a background fetch moves that and would
    /// make it a plain force. A remote that moved since is refused, and core answers with a fetch.
    function forcePush() {
        publishFlow.repoTab.pushCurrent(publishFlow.workTree.branch, "lease", publishFlow.leaseOid())
    }
    /// Commit the remote branch the counts are against (`pushMeasured`) points at, as shown here.
    function leaseOid() {
        return publishFlow.pushMeasured !== ""
               ? publishFlow.remotesModel.oidOfName(publishFlow.pushMeasured)
               : ""
    }
    /// A push this button sent has come back, found by its id (`RepoTab.pushAnswer`, -1 when this notify carries
    /// none; the page calls this once per notify). Reads that answer's own words
    /// (rules-refs/app-ui.md「ツールバーの push の答えは id で受け取る」).
    function noteWriteAnswer() {
        const at = publishFlow.repoTab.pushAnswer
        if (at < 0)
            return
        const sent = publishFlow.repoTab.pushAnswerBranch
        if (publishFlow.repoTab.writeAnswerFailed(at)) {
            publishFlow.pushFailBranch = sent
            publishFlow.pushFailReason = publishFlow.repoTab.writeAnswerError(at)
        } else if (publishFlow.pushFailBranch === sent) {
            // Landed: drop the last refusal.
            publishFlow.pushFailBranch = ""
            publishFlow.pushFailReason = ""
        }
    }

    // ---- the first push: where does this branch go? -----------------
    // (デザイン規約 §はじめてリモートへ送る). Raised by the push button, in the bar every question shares.
    property bool publishAsking: false
    /// The remote the answer picks.
    property string publishRemote: ""
    /// What the branch is to be called over there.
    property string publishBranch: ""

    readonly property var publishRemotes: publishFlow.repoTab.remoteNames
    /// The chooser's last row: opens the add-remote dialog. The question stays standing behind it and picks the new
    /// remote up when it lands.
    readonly property string publishAddChoice: Words.addRemote
    readonly property var publishChoices:
        publishFlow.publishRemotes.concat([publishFlow.publishAddChoice])
    /// The `+` on the left menu's REMOTES band: the chooser's own dialog, with no question behind it
    /// (規約 §リモートを書き留める).
    function startAddRemote() {
        remoteDialog.start("", "", publishFlow.publishRemotes, false, true)
    }
    /// The same form filled in for an existing remote (its URL, whether it is origin), from its row on the left menu.
    function startEditRemote(name) {
        remoteDialog.start(name, publishFlow.repoTab.remoteUrl(name), publishFlow.publishRemotes,
                           name === publishFlow.repoTab.markedOrigin,
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
    /// The pair the remote's last answer is about; nothing while a read is out. **`remoteBranchRevision` is read on
    /// purpose**: a binding does not follow a slot call, so the revision that moves with every answer re-asks. A slot
    /// because a Qt property cannot hold nothing (`encode::Optional`).
    readonly property var remoteBranchAsked: {
        publishFlow.repoTab.remoteBranchRevision
        return publishFlow.repoTab.remoteBranchAsked()
    }
    /// The remote has answered for exactly what is typed now.
    readonly property bool publishChecked: {
        const asked = publishFlow.remoteBranchAsked
        return asked ? asked.remote === publishFlow.publishRemote && asked.branch === publishFlow.publishBranch
                     : false
    }
    /// …and what a push under this name would meet (`platitude_core::remote::RemoteBranchState`); empty until the
    /// answer for exactly this name is in.
    readonly property string publishState:
        publishFlow.publishChecked ? publishFlow.repoTab.remoteBranchState : ""
    /// The far side cannot be read. Still a click — a plain push can only fast-forward, so nothing there can be lost —
    /// but the bar wears the alert (デザイン規約 §リモートへ送る).
    readonly property bool publishUnsure:
        publishFlow.publishState === "unknown" || publishFlow.publishState === "unreachable"
    /// The name holds commits this history lacks: only an overwrite can land, so the pill becomes a held `push -f`
    /// (デザイン規約 §リモートへ送る).
    readonly property bool publishRefused: publishFlow.publishState === "refused"
    /// The far tip the question showed, which an overwrite leases against (§相手の履歴を置き換える).
    readonly property string publishLease:
        publishFlow.publishRefused ? publishFlow.repoTab.remoteBranchTip : ""

    /// Asked once the typing settles: the check is a network round trip.
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

    // Bindings, because the question changes while it stands. **`RestoreNone`**: `publishAsking` drops while the bar
    // is still retracting on screen, and a restore would flash its frame `warning`; `GraphPane.startAsking` rewrites
    // all of these for the next question anyway.
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
        // A sentence for each count — `commit(s)` would reach the reader as written (デザイン規約 §タイポグラフィ).
        value: publishFlow.publishRefused
               ? (publishFlow.repoTab.remoteBranchTheirs === 1
                  ? qsTr("Hold to overwrite %1, dropping %n commit it has and yours does not. A remote that moved since is refused.",
                         "", publishFlow.repoTab.remoteBranchTheirs)
                  : qsTr("Hold to overwrite %1, dropping %n commits it has and yours does not. A remote that moved since is refused.",
                         "", publishFlow.repoTab.remoteBranchTheirs))
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
        // Open on the upstream where there is one, even with no tracking ref (デザイン規約 §ブランチが測られる相手を決める)
        // — but only on a remote still configured: `remoteOfRef` cuts a lost name at the first slash, and the chooser
        // cannot pick back a row it does not hold.
        const upstream = publishFlow.workTree.upstream
        const on = upstream !== ""
                 ? GitFacts.remoteOfRef(upstream, publishFlow.repoTab.remoteNames) : ""
        const tracked = on !== "" && publishFlow.publishRemotes.indexOf(on) >= 0
        publishFlow.publishRemote = tracked ? on : publishFlow.repoTab.defaultRemote
        publishFlow.publishBranch = tracked
            ? GitFacts.branchOfRef(upstream, publishFlow.repoTab.remoteNames)
            : publishFlow.workTree.branch
        // `push` untranslated (the command's spelling); `%1` is left for the bar to draw the name in its ref colour
        // (デザイン規約 §ref の種別).
        publishFlow.askRequested(
            qsTr("%1 where?"), publishFlow.answerPublish, publishForm, "push", publishFlow.workTree.branch)
        // After the bar is up: raising it resets what the bindings above own, and an unchanged binding does not push
        // back.
        publishFlow.publishAsking = true
        publishFlow.refreshPublishCheck()
        // No remote: the chooser holds only its last row, so its dialog opens straight away
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
    /// Automation: whether the list has a row to mark (`markedRow`) — a picture cannot tell no mark from an unplumbed
    /// one.
    function publishRemotesMarked() {
        const form = publishFlow.graphPane.askForm
        return Boolean(form && form.remotePick && form.remotePick.markedRow !== "")
    }
    /// Automation: Enter in the name box, raised as the box's own `accepted` (no key reaches it offscreen). That Enter
    /// raises `accepted` is fixed in `tst_askanswer`.
    function enterBranch() {
        const form = publishFlow.graphPane.askForm
        if (!form || !form.branchField)
            return false
        form.branchField.accepted()
        return true
    }
    /// Automation reads the popup itself: the call that requested it may not have put it on screen.
    function publishRemotesOpen() {
        const form = publishFlow.graphPane.askForm
        return Boolean(form && form.remotePick
                       && form.remotePick.popup.visible)
    }

    function answerPublish() {
        // The same owner a plain push writes to: a refused first push is still this button's news
        // (デザイン規約 §リモートへ送る).
        publishFlow.repoTab.publishCurrent(publishFlow.workTree.branch,
                                           publishFlow.publishRemote,
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
        // Clearing is the same slot with no name; it reaches this repository's config only.
        onMarkChanged: (name, marked) => publishFlow.repoTab.markOrigin(marked ? name : "")
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

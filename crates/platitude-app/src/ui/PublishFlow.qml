pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui
import platitude.ui.words

// Everything between the toolbar's push button and git's answer: what the branch can do with its remote, the first
// push's question, and what a refusal leaves behind (デザイン規約 §リモートへ送る). The band reads this through the
// active page, which hands git's answers back here.
//
// Sized to the page: the add-remote dialog centres itself in the item it was declared under.
Item {
    id: publishFlow

    required property RepoTab repoTab
    required property WorktreeModel worktree
    /// Where a remote-tracking branch's commit is read from.
    required property NavSectionModel remotesModel
    /// Holds the bar this question stands in and dresses (`RepoPage.startRowAsk` raises it).
    required property GraphPane graphPane

    /// Raise the standing question with this form — the page owns the bar —, marked on the row `oidHex` ("" for none),
    /// naming `refName` in its own colour `refTint`.
    signal askRequested(string oidHex, string label, var run, var form, string code, string refName, color refTint)

    /// The add-remote dialog, which the automation types into: null until first opened, or up front with
    /// [`keepBuilt`].
    readonly property var dialog: remoteSeat.item
    /// The dialog built up front, for a caller that reads it before it opens (the harness, `RepoPage.keepBuilt`).
    /// False in every page a person opens (rules-refs/app-ui.md — the `WindowDialogSeat` line).
    property bool keepBuilt: false
    /// The one way to the dialog: built here, then called in the same turn — the build is synchronous.
    function builtDialog() {
        remoteSeat.active = true
        return remoteSeat.item
    }

    anchors.fill: parent

    // ---- push ------------------------------------------------------
    /// The branch's push, read whole by core off one destination (`NavSectionModel.headPush`,
    /// `RefsSnapshot::head_push`) — the one the send resolves: the standing, where the label says it goes, the tip's
    /// counts, the commit an overwrite leases to, the first push's upstream and what a press does there are all its
    /// fields (`encode::PushRecord`, a branch's row's too), never re-derived here, or the label and the send part ways.
    /// The branch's push is the model's record, handed on as read (`WorktreeModel.headPush`: whether the upstream is a
    /// branch here is the status's word, never read off its name, and both marks ride with the status — not the tab's
    /// `pushDefault`, which lands before or after the counts read under it). Read against the remotes' listing — their
    /// names, and the commit the lease pins, as the remote branches list it — so it follows that listing
    /// (`refsRevision`): a fetch that moves the far side moves the lease with what is drawn.
    ///
    /// Where a mark sends the push to another remote than the upstream's — a branch here being on none — the counts
    /// that speak for it are the destination's own, never the upstream's (デザイン規約 §リモートへ送る; user decision —
    /// ask before changing): git refuses a non-fast-forward against the destination's ref, so read against the upstream
    /// a plain `push` would stand over a diverged destination and dead-end. The status tick reads the destination's
    /// counts only in this case (`remote::push_track`). The tip's counts and the overwrite's lease come off that same
    /// destination: one of them reading the upstream would say the diverged counts and lease on origin's tip, refused
    /// as `stale info`. Headless: `push-target` / `push-hover` / `force-push` on `--preset forkdiverged`, and
    /// `push-target forkmark --preset forkmark` for no tracking ref.
    ///
    /// `state` is the standing's word, which only the words the button says branch on (`BandPushButton.sentenceTo`):
    /// `closed` (no branch to send, a status still another branch's, or a tab not open), `unborn`, `publish`,
    /// `stays-here`, `elsewhere`, `ready`, `clean`, `behind`, `diverged` — the table is core's
    /// (`remote::PushStanding`). The counts are the last fetch's, so they prove the negative only: a push may still be
    /// refused.
    readonly property var pushFacts: {
        publishFlow.remotesModel.refsRevision
        return publishFlow.remotesModel.headPush(publishFlow.worktree.headPush, publishFlow.repoTab.state === "open",
                                                 publishFlow.repoTab.defaultRemote)
    }
    /// Nothing this window asked git for is out — from the press, not from when the queue starts it (`settling` rises
    /// at the ask, `busyCount` only once the write runs). A push is answered by its one owner (`RepoTab.pushAnswer`),
    /// so the doors that send one beside another — the toolbar's button, the question's pill — wait on this, and only
    /// one is ever out.
    readonly property bool pushDoorsOpen: publishFlow.repoTab.busyCount === 0 && !publishFlow.repoTab.settling
    /// A plain press sends (`remote::PushStanding::sends_plain`) and the doors are open.
    readonly property bool canPush: publishFlow.pushFacts.sendsPlain && publishFlow.pushDoorsOpen
    /// The held overwrite reaches (`remote::PushStanding::overwrites`) and the doors are open.
    readonly property bool canForcePush: publishFlow.pushFacts.overwrites && publishFlow.pushDoorsOpen
    /// What git said to the last refused push of each branch, by branch — a refusal is about that branch's standing, so
    /// whichever door sent it (the toolbar, the branch's row) it is that branch's, until a push of it lands or the
    /// branch is gone (`forgetUnlisted`) (デザイン規約 §リモートへ送る). Most refusals (`fetch first`) are about that
    /// branch and its far side and say nothing of the next branch; a push that lands must not keep saying the last one
    /// failed. Standing on the branch makes it the toolbar's `!` (`BandPushButton.failed`). A `Map`, so no branch name
    /// is taken for something an object already has, and replaced whole: a binding does not see a key change.
    property var pushFails: new Map()
    readonly property bool pushFailed: publishFlow.pushFails.has(publishFlow.worktree.branch)
    readonly property string pushFailReason:
        publishFlow.pushFailed ? publishFlow.pushFails.get(publishFlow.worktree.branch) : ""
    /// The branch travels with the press: the tab keeps it beside the push's id and hands it back with the answer
    /// (`RepoTab.pushAnswer` / `pushAnswerBranch`) — nothing here holds it across the round trip.
    function pushNow() {
        if (publishFlow.pushFacts.asks) {
            // A gone branch here names no remote, whatever its name reads like (`startPublishAsk`).
            publishFlow.startPublishAsk(publishFlow.worktree.branch, publishFlow.pushFacts.upstream, "")
            return
        }
        publishFlow.repoTab.pushCurrent(publishFlow.worktree.branch, "", "")
    }
    /// Leased against the commit this window shows (`pushFacts.lease`), not the tracking ref — a background fetch
    /// moves that and would make it a plain force. A remote that moved since is refused, and core answers with a
    /// fetch. With no commit of the far side on screen (`lease` empty: an upstream fetched outside `refs/remotes/` is
    /// counted by git and never listed, and a count can land one read ahead of its row) the lease is git's own, to its
    /// tracking ref (`remote::PushForce::WithTrackingLease`): the press is offered as what git would do, not held back
    /// for what the list does not draw (user decision — ask before changing).
    function forcePush() {
        publishFlow.repoTab.pushCurrent(publishFlow.worktree.branch, "lease", publishFlow.pushFacts.lease)
    }
    /// A branch's push has come back — the toolbar's or a branch row's —, found by its id (`RepoTab.pushAnswer`, -1
    /// when this notify carries none; the page calls this once per notify). Reads that answer's own words
    /// (`writeAnswerFailed(at)` / `writeAnswerError(at)`; why by id: `ops::PushOut`).
    function noteWriteAnswer() {
        const at = publishFlow.repoTab.pushAnswer
        if (at < 0)
            return
        const sent = publishFlow.repoTab.pushAnswerBranch
        const fails = new Map(publishFlow.pushFails)
        if (publishFlow.repoTab.writeAnswerFailed(at))
            fails.set(sent, publishFlow.repoTab.writeAnswerError(at))
        else
            // Landed: that branch's last refusal goes.
            fails.delete(sent)
        publishFlow.pushFails = fails
    }
    /// The refs moved: a branch the listing no longer holds takes its refusal with it, so one made again under the
    /// name does not wear the old one's `!`. `listed(name)` answers whether the listing holds that branch.
    function forgetUnlisted(listed) {
        const kept = new Map()
        for (const [branch, why] of publishFlow.pushFails) {
            if (listed(branch))
                kept.set(branch, why)
        }
        if (kept.size !== publishFlow.pushFails.size)
            publishFlow.pushFails = kept
    }

    // ---- the first push: where does this branch go? -----------------
    // A branch with no upstream has its destination written nowhere. Sending it to the default remote under the same
    // name is a guess, and a wrong guess has no way back — so the press asks. Raised by the push button and a branch
    // row's `push` to… (the menu's first level), in the bar every question shares (デザイン規約 §立っている質問は
    // 1 か所で聞く): a question that stood somewhere else depending on who raised it would not read.
    // The word pressed is the word that answers: the bar is raised by `push` and its pill is `push` (code spelling).
    property bool publishAsking: false
    /// The branch the question is about: the one the door that raised it names — the toolbar's is the worktree's, a
    /// row's (left-menu row, graph chip) is its own, and the question marks that row's commit; the heading names that
    /// branch (user decision — ask before changing). A question opened on a row that named another branch would be
    /// answered for the branch nobody asked about. The same-name check weighs this branch's history too.
    property string publishLocal: ""
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
        publishFlow.builtDialog().start("", "", publishFlow.publishRemotes, false, true)
    }
    /// The same form filled in for an existing remote (its URL, whether it is origin), from its row on the left menu.
    function startEditRemote(name) {
        publishFlow.builtDialog().start(name, publishFlow.repoTab.remoteUrl(name), publishFlow.publishRemotes,
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

    readonly property string publishTarget: GitFacts.remoteBranchName(publishFlow.publishRemote,
                                                                      publishFlow.publishBranch)
    readonly property bool publishFilled: publishFlow.publishBranch !== "" && publishFlow.publishRemote !== ""
    /// What the remote's last answer is about — the remote, the name, and the branch here it was weighed against;
    /// nothing while a read is out. **`remoteBranchRevision` is read on
    /// purpose**: a binding does not follow a slot call, so the revision that moves with every answer re-asks. A slot
    /// because a Qt property cannot hold nothing (`encode::Optional`).
    readonly property var remoteBranchAsked: {
        publishFlow.repoTab.remoteBranchRevision
        return publishFlow.repoTab.remoteBranchAsked()
    }
    /// The remote has answered for exactly what is typed now, weighed against the branch this question is about — an
    /// answer for the last question's branch can land under this one.
    readonly property bool publishChecked: {
        const asked = publishFlow.remoteBranchAsked
        return asked ? asked.remote === publishFlow.publishRemote && asked.branch === publishFlow.publishBranch
                       && asked.local === publishFlow.publishLocal
                     : false
    }
    /// …and what a push under this name would meet (`platitude_core::remote::RemoteBranchState`); empty until the
    /// answer for exactly this name is in. "Exists" alone decides nothing: git fast-forwards a name this history
    /// contains and refuses one it does not, and the two lose different things, so they wear different forms. A name
    /// that exists is still let through: adding to an existing branch is legitimate (the same branch from another
    /// machine, riding someone else's); the remote-branch replace refuses taken names only because it says outright
    /// that it makes a new one. `fast-forward` keeps the plain form — nothing is lost, and the surprise ("you moved
    /// somebody's branch") is the line's to say.
    readonly property string publishState:
        publishFlow.publishChecked ? publishFlow.repoTab.remoteBranchState : ""
    /// The far side cannot be read (`unreachable`: bad URL, credentials, offline; `unknown`: its commit is not here).
    /// Unknown stays unknown — reading a failed ask as "free" would assume git refuses what it does not. Still a click,
    /// word and gesture unchanged — a plain push can only fast-forward, so the one thing to say is "not confirmed": the
    /// bar wears the alert, the frame and the `!` the toolbar's refusal memory wears, same size and placement
    /// (デザイン規約 §リモートへ送る). No overwrite form here: there is no tip to lease against, and a bare lease reads the
    /// tracking ref, which a background fetch moves — the push lets git answer, and the force form comes once a fetch
    /// has made the tip readable. The failed read leaves the log closed: the bar already says it.
    readonly property bool publishUnsure:
        publishFlow.publishState === "unknown" || publishFlow.publishState === "unreachable"
    /// The name holds commits this history lacks: only an overwrite can land, so the pill becomes a held `push -f` —
    /// the toolbar's force form, so one pill never carries two meanings (デザイン規約 §リモートへ送る). The commits lost
    /// are in the tip (`tipOf`).
    readonly property bool publishRefused: publishFlow.publishState === "refused"
    /// The far tip the question showed (from `ls-remote`), which an overwrite leases against: a remote that moved
    /// since is refused as `stale info` (§相手の履歴を置き換える).
    readonly property string publishLease:
        publishFlow.publishRefused ? publishFlow.repoTab.remoteBranchTip : ""

    /// Asked once the typing settles: the check is a network round trip, and of the remote itself (not the tracking
    /// ref) — the answer has to be today's. Settled is the platform's own measure of it, the gap that tells two
    /// keystrokes of one input from two inputs (`QStyleHints::keyboardInputInterval`: macOS's key repeat delay, Qt's
    /// 400 elsewhere) — how fast a hand types is the person's and the machine's, not this window's.
    Timer {
        id: publishCheckTimer
        interval: Application.styleHints.keyboardInputInterval
        onTriggered: {
            if (publishFlow.publishAsking && publishFlow.publishRemote !== "" && publishFlow.publishBranch !== "")
                publishFlow.repoTab.checkRemoteBranch(publishFlow.publishRemote, publishFlow.publishBranch,
                                                      publishFlow.publishLocal)
        }
    }
    function refreshPublishCheck() {
        publishCheckTimer.restart()
    }

    // Bindings, because the question changes while it stands. **`RestoreNone`**: `publishAsking` drops at the press
    // that walks away (it must — standing, the check timer keeps going out to the remote) while the bar is still
    // retracting on screen, and a restore would put that bar back to its plain values: the heading and the line gone,
    // `neutral` dropped, the frame jumping from the accent to `warning`. `GraphPane.startAsking` rewrites all of these
    // for the next question anyway. Headless: `publish-dismiss` — the retracting bar is in no picture, so its must_say
    // is the whole check.
    // Shut until the remote has answered for this name (the word drops, the frame stays — §無効), then words, colour
    // and gesture change at once. Shut while a write is out too, as the toolbar's button is (`pushDoorsOpen`): a
    // question raised from a branch's row can stand while the toolbar sends another, and only one send is ever out.
    // Dressing: an information question wears the accent (`askNeutral`); the word is `textPrimary` in every form and
    // only the held `push -f` takes colour — the toolbar's `push` is white in every clickable state, so a press with
    // one question in between has no reason to change its dress (§立っている質問は 1 か所で聞く).
    Binding {
        target: publishFlow.graphPane
        property: "askAnswerable"
        value: publishFlow.publishFilled && publishFlow.publishChecked && publishFlow.pushDoorsOpen
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
        value: publishFlow.detailOf(publishFlow.publishRemote, publishFlow.publishTarget)
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    // The remote and the branch over there it names, each behind its mark (デザイン規約 §ref の種別「名前の印」).
    Binding {
        target: publishFlow.graphPane
        property: "askDetailMarks"
        value: Words.marksOf(publishFlow.detailOf, [[publishFlow.publishRemote, "remote"],
                                                    [publishFlow.publishTarget, "remote"]])
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: publishFlow.graphPane
        property: "askTip"
        value: publishFlow.tipOf(publishFlow.publishRemote, publishFlow.publishTarget)
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: publishFlow.graphPane
        property: "askTipMarks"
        value: Words.marksOf(publishFlow.tipOf, [[publishFlow.publishRemote, "remote"],
                                                 [publishFlow.publishTarget, "remote"]])
        when: publishFlow.publishAsking
        restoreMode: Binding.RestoreNone
    }
    /// The line under the question, with `remote` and `target` (the branch over there) where it names them.
    function detailOf(remote, target) {
        return !publishFlow.publishFilled ? qsTr("Enter where it goes.")
               : !publishFlow.publishChecked
                 ? qsTr("Asking %1 what it has…").arg(remote)
                 // Any failure of the ask, an answer of a refusal included: git's words are in the log.
                 : publishFlow.publishState === "unreachable"
                   ? qsTr("Asking %1 failed — it may already have that branch.").arg(remote)
                   : publishFlow.publishState === "unknown"
                     ? qsTr("%1 already exists, and what it holds is not here.").arg(target)
                     : publishFlow.publishRefused
                       ? Words.losesCommits(target, publishFlow.repoTab.remoteBranchTheirs)
                       : publishFlow.publishState === "fast-forward"
                         ? qsTr("%1 already exists — your commits go on top of it.").arg(target)
                         : qsTr("%1 does not exist yet; this makes it.").arg(target)
    }
    /// The pill's tip, the same way.
    function tipOf(remote, target) {
        // A sentence for each count — `commit(s)` would reach the reader as written (デザイン規約 §タイポグラフィ).
        return publishFlow.publishRefused
               ? (publishFlow.repoTab.remoteBranchTheirs === 1
                  ? qsTr("%1 to overwrite %2, dropping %n commit it has and yours does not. A remote that moved since is refused.",
                         "", publishFlow.repoTab.remoteBranchTheirs)
                  : qsTr("%1 to overwrite %2, dropping %n commits it has and yours does not. A remote that moved since is refused.",
                         "", publishFlow.repoTab.remoteBranchTheirs))
                 .arg(Words.pressCount(Metrics.holdPressesWarned)).arg(target)
               : publishFlow.publishState === "unknown"
                 ? qsTr("%1 was never fetched here, so what it holds cannot be read — the push can only fast-forward it.")
                   .arg(target)
                 : publishFlow.publishState === "unreachable"
                   ? qsTr("Asking %1 failed; the push can only fast-forward, and its answer will say what went wrong.")
                     .arg(remote)
                   : ""
    }

    /// Asks where `branch` goes, opening on `upstream` — the remote branch it is configured for, fetched or not; "" for
    /// none, or for a branch here, which names no remote — and marking the row `oidHex` ("" for none: the toolbar's is
    /// the worktree's own). The toolbar hands `pushFacts`' `upstream`, a branch row `branchPush`'s — the same field off
    /// the same reading (`remote::Upstream::remote_branch`). A press that asks (`publish`) is both "no upstream" and
    /// "no tracking ref"; the second has its destination written already, and the question opens on it (user decision —
    /// ask before changing): opening on the default remote and the branch's own name would ask again for the answer
    /// just given, and throw it away. Right after `UpstreamFlow` took a name not here, this opens on it. Otherwise the
    /// field defaults to the branch's own name. Net: `publish-upstream`.
    function startPublishAsk(branch, upstream, oidHex) {
        // Open on the upstream where there is one, even with no tracking ref — but only on a remote still configured:
        // `remoteBranchOf` cuts a lost name at the first slash, and the chooser cannot pick back a row it does not hold.
        // A gone branch here (`remote = .`) names no remote either and falls back to the guess: a plain `git push`
        // refuses that shape too, so the row's `to…` says "not in one go; decide where it goes again".
        const far = GitFacts.remoteBranchOf(upstream, publishFlow.repoTab.remoteNames)
        const tracked = far.remote !== "" && publishFlow.publishRemotes.indexOf(far.remote) >= 0
        publishFlow.publishLocal = branch
        publishFlow.publishRemote = tracked ? far.remote : publishFlow.repoTab.defaultRemote
        publishFlow.publishBranch = tracked ? far.branch : branch
        // `push` untranslated (the command's spelling); `%1` is left for the bar to draw the name in its ref colour —
        // the worktree's own branch's blue, any other's plain ink (デザイン規約 §ref の種別).
        publishFlow.askRequested(
            oidHex, qsTr("%1 where?"), publishFlow.answerPublish, publishForm, "push", branch,
            GitFacts.isCurrent(publishFlow.worktree.standing, branch) ? Theme.textLink : Theme.textPrimary)
        // After the bar is up: raising it resets what the bindings above own, and an unchanged binding does not push
        // back — an assignment under a live `Binding` wins and kills that binding's hold for good (turning `when` back
        // restores nothing), so armed first, `askAnswerable` / `askHold` / `askNeutral` would stand dead.
        publishFlow.publishAsking = true
        publishFlow.refreshPublishCheck()
        // No remote: the chooser holds only its last row — a bar alone would ask where with nothing to pick — so its
        // dialog opens straight away, after the bar is up (the reason above). One path with pressing the last row:
        // the question stands behind as usual (heading, name box), because writing a remote down is not a push and
        // which branch goes under what name is still unanswered; Add returns to it with that remote picked, and
        // Cancel / Escape returns to a bar with an empty destination whose last row opens the dialog again.
        // Headless: `publish` / `publish-taken` / `publish-remotes` / `publish-add` / `publish-go` / `publish-enter` /
        // `publish-new-go`; no remote: `--preset noremote`.
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
        publishFlow.builtDialog().setFields(parts.length > 0 ? parts[0] : "", parts.length > 1 ? parts[1] : "")
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
        // The same owner a plain push writes to: a refused first push is still news about that branch
        // (デザイン規約 §リモートへ送る).
        publishFlow.repoTab.publishBranch(publishFlow.publishLocal,
                                          publishFlow.publishRemote,
                                          publishFlow.publishBranch,
                                          publishFlow.publishLease)
    }

    // Filling the flow, which fills the page: the dialog centres itself in its parent.
    Loader {
        id: remoteSeat
        anchors.fill: parent
        active: publishFlow.keepBuilt
        sourceComponent: RemoteDialog {
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

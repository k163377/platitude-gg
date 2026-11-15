pragma ComponentBehavior: Bound

import QtQuick
// For the attached types alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The sidebar's row verbs: the name boxes a row opens (renaming a branch, a tag or a stash, replacing a remote
/// branch), the tooltip a row puts out, and the lines a row opens under itself.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var` because naming its type would be a cycle: the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var remotesModel: driver.remotesModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var tagsModel: driver.tagsModel
    readonly property var graphPane: driver.graphPane
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var navProbe: driver.navProbe
    readonly property var refMenu: driver.refMenu
    readonly property var writeBarrier: driver.barrierWrite

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn, so a verb
    /// belongs to one.
    function run(act, arg) {
        if (act === "nav-rename-far") {
            // A box on a row the list had scrolled away from: the list has to bring it back (デザイン規約 §左メニューの所作).
            // On the folded rail's section: the one list a run can scroll and read back through one handle. The TAGS of
            // `--preset manytags`: a section with more rows than the list shows.
            page.foldByHand(true)
            farTimer.start()
        } else if (act === "nav-rename-drop") {
            // The box walked away from untyped: "fold" takes the list down to the rail, "away" is a press elsewhere
            // (Main's `FocusRelease` enters here).
            sidebarPane.beginRename("branch", worktree.branch, worktree.branch)
            if (arg === "fold")
                page.foldByHand(true)
            else
                page.releasePressedAway(null)
            driver.reportAtCompletion(() => "nav_drop how=" + Harness.autoActArg
                                            + " collapsed=" + page.sidebarCollapsed
                                            + " box=" + (sidebarPane.editKey !== "")
                                            + " editing=" + sidebarPane.editKey)
        } else if (act === "nav-rename" || act === "rename-branch" || act === "rename-tag" || act === "rename-stash") {
            const kind = act === "rename-tag" ? "tag" : act === "rename-stash" ? "stash" : "branch"
            const id = kind === "branch" ? worktree.branch
                     : kind === "tag" ? tagsModel.nameAt(0) : stashesModel.fullAt(0)
            const shown = kind === "stash" ? stashesModel.nameAt(0) : id
            sidebarPane.beginRename(kind, id, shown)
            if (act !== "nav-rename")
                sidebarPane.submitEdit(arg)
        } else if (act === "rename-taken") {
            // A rename git turns down for a reason the box cannot see coming — another git holding the ref's lock
            // (`--preset reflock`, `held`) — sent through the box's Enter, behind its refusal gate as a hand's is: the
            // box stays open holding the name, and the bar says who turned it down (デザイン規約 §答えの要らない報せ).
            // A name the box itself refuses is not sent, and the run says nothing. Not by making the same name first
            // through the product: the page takes that write's landing for the rename's and closes the box (both are
            // `OperationKind::Branch`, and the rename's answer is not awaited by id).
            sidebarPane.beginRename("branch", worktree.branch, arg)
            if (!sidebarPane.editRefused)
                sidebarPane.submitEdit(arg)
            renameTakenTimer.start()
        } else if (act === "rename-tag-box" || act === "rename-branch-box") {
            // For a name the box itself refuses (デザイン規約 §答えの要らない報せ「押す前に断れるものは、名前の箱で断る」).
            // `was=` names the row, so a run that opened the box on the wrong one says so.
            tagNameBoxTimer.begin(arg, act === "rename-branch-box" ? "branch" : "tag")
        } else if (act === "replace-remote" || act === "replace-remote-box"
                   || act === "replace-remote-go" || act === "replace-remote-tip") {
            // Named outright (`origin/billing:billing-v2`) because the remote's rows are behind a fold. "-box" leaves
            // the box standing, the plain act stops at the question, "-go" holds the pill to the end, "-tip" stops at
            // the question with a hand on the pill. "-go" cannot take the branch the far side stands on: on
            // `origin/main` git answers `refusing to delete the current branch` and the run fails on `write-failures 1`
            // — aim at another row (`origin/feature/topic-a`).
            const parts = arg.split(":")
            const ref = parts[0]
            const was = GitFacts.branchOfRef(ref, repoTab.remoteNames)
            // "-box" types the argument in, to photograph a name the remote already carries being refused, and opens
            // the remote's fold (a remote root starts closed) so the row is there at all.
            if (act === "replace-remote-box")
                remotesModel.toggleFolder(GitFacts.remoteOfRef(ref, repoTab.remoteNames))
            sidebarPane.beginRename("remote", ref, act === "replace-remote-box" ? parts[1] : was)
            // The dispatch completes "-box". A known verb answers true even on its early way out — falsy would send the
            // dispatch on asking every other family.
            if (act === "replace-remote-box")
                return true
            sidebarPane.submitEdit(parts[1])
            if (act === "replace-remote-go") {
                driver.holdToEnd(graphPane)
            } else {
                replaceAskTimer.points = act === "replace-remote-tip"
                replaceAskTimer.order = replaceAskTimer.points ? ["pointed"] : []
                replaceAskTimer.start()
            }
        } else if (act === "rename-tag-remote" || act === "rename-tag-remote-go" || act === "rename-tag-remote-tip") {
            // `<tag>:<新しい名前>[:<選ぶ答え>]` (`replace` / `add` / `leave`; named, not numbered — `nav-open-tag`
            // の同じ理由). Without an answer the bar stands as it comes down; "-go" answers it (`replace` if none is
            // named); "-tip" puts a hand on the pill and always picks `replace`, the one answer whose pill has words.
            tagRemoteRenameTimer.begin(arg, act === "rename-tag-remote-go", act === "rename-tag-remote-tip")
        } else if (act === "nav-branch-box" || act === "nav-rename-box" || act === "nav-tag-box") {
            // The boxes the left menu opens on a row, left standing. `<section>:<ref>[:<幅>][:away]`: the width is what
            // a hand would drag the pane's bar to — the box gets the row's share of it, less a folder's indent.
            navNameBoxTimer.begin(act === "nav-rename-box" ? "rename"
                                : act === "nav-tag-box" ? "tag" : "branch", arg)
        } else if (act === "name-branch") {
            // The argument is the name, and the run needs `--preset deep`: it names the top row, which on a preset
            // with a worktree row (`basic`) git refuses with `unable to read tree`. With no name nothing is written and
            // the run is silent until the watchdog.
            graphPane.view.namingSubmitted(graphModel.oidAt(0), arg, "branch")
        } else if (act === "rename-box-out") {
            // On the first row, which every preset with a chip on it can answer.
            boxOutTimer.route = arg
            boxOutTimer.start()
        } else {
            return false
        }
        return true
    }
    // Reported off the box's and the bar's own state: a box closed and one left open holding the name, and the bar's
    // place word, are a few pixels apart.
    PageStage {
        id: renameTakenTimer
        name: "rename_taken"
        onTriggered: {
            // git's answer is the bar coming down — the box keeps the name either way — and the bar settled: its height
            // animates in after the words, so a shot on the answer alone catches it still on its way down
            // (`NoticeBar.settled`).
            renameTakenTimer.complete({
                "name": "rename_taken_ready",
                "when": () => ({
                    "idle": repoTab.busyCount === 0,
                    "bar": page.noticeCard.open && page.noticeCard.settled }),
                "report": () => "rename_taken open=" + (sidebarPane.editKey !== "")
                                + " refused=" + sidebarPane.editRefused
                                + " tone=" + page.noticeCard.tone
                                // The log's opening line after its place word — last, since it is a sentence.
                                + " place=" + page.noticeCard.detailPlace
                                + " says=" + page.noticeCard.detail })
        }
    }
    // PGG_AUTO_ACT=rename-tag-box / rename-branch-box: the box on the first tag or on the branch HEAD is on, the
    // argument typed in, and whether the reason for refusing the name reached the reader. A branch's argument may name
    // the row too, `<branch>:<typed>` (a branch name holds no `:`) — a branch a stopped rebase holds is never HEAD's.
    //
    // `editKey` is only the ask: the box is built by a `Loader` in a row not yet laid out, so `box=` reads the box as
    // drawn and holding the keyboard (`NavList.rowBoxShown` / `rowFocused`). `refused=` alone passes a box refusing in
    // silence, and Qt can drop the tooltip the box's binding raises (`NavNameBox`) as a binding loop without a word,
    // so `tip=` is taken through the box (`NavList.rowTipShown` = the box's own `TipAsk.shown`). `text=` is read off
    // the window's box (`Tips.box.text`, bound to the target's `TipAsk.words`) — the sentence that reached the reader,
    // not `editRefusedWhy` inside the model. The picture holds the red frame alone — the reason is the tooltip's — so
    // the line is the judgement; a box refusing in silence wears the same frame, never raises the tip this waits on,
    // and the run goes quiet until the watchdog.
    PageStage {
        id: tagNameBoxTimer
        name: "tag_name_box"
        property string typed: ""
        property string kind: "tag"
        /// The branch named before the `:`, `""` for HEAD's.
        property string row: ""
        function begin(arg, kind) {
            const named = kind === "branch" ? ("" + arg).indexOf(":") : -1
            tagNameBoxTimer.row = named >= 0 ? ("" + arg).substring(0, named) : ""
            tagNameBoxTimer.typed = named >= 0 ? ("" + arg).substring(named + 1) : "" + arg
            tagNameBoxTimer.kind = kind
            sidebarPane.beginRename(kind, tagNameBoxTimer.was(), tagNameBoxTimer.typed)
            tagNameBoxTimer.start()
        }
        function was() {
            return tagNameBoxTimer.kind === "tag" ? tagsModel.nameAt(0)
                 : tagNameBoxTimer.row !== "" ? tagNameBoxTimer.row : worktree.branch
        }
        /// The row the box stands on, -1 while the model has none. Asked each time: a binding on a call would hold the
        /// empty model's answer (rules/app-ui.md「QML バインディングはプロパティにしか反応しない」).
        function rowOfBox() {
            const model = tagNameBoxTimer.kind === "tag" ? tagsModel : branchesModel
            return model.rowOfName(tagNameBoxTimer.was())
        }
        function open(row) {
            const list = navProbe.listOf(tagNameBoxTimer.kind)
            return row >= 0 && list.rowBoxShown(row) && list.rowFocused(row)
        }
        function tipped(row) {
            return navProbe.listOf(tagNameBoxTimer.kind).rowTipShown(row)
        }
        /// The box drawn and holding the keyboard; only a refused name raises a tip to wait for.
        function ready() {
            const row = tagNameBoxTimer.rowOfBox()
            const open = tagNameBoxTimer.open(row)
            return { "ready": open, "tip": !sidebarPane.editRefused || (open && tagNameBoxTimer.tipped(row)) }
        }
        onTriggered: {
            tagNameBoxTimer.complete({
                "name": "tag_name_box_ready",
                "when": () => tagNameBoxTimer.ready(),
                "report": () => {
                    const at = tagNameBoxTimer.rowOfBox()
                    return tagNameBoxTimer.kind + "_name_box was=" + tagNameBoxTimer.was()
                           + " typed=" + tagNameBoxTimer.typed
                           + " box=" + tagNameBoxTimer.open(at)
                           + " refused=" + sidebarPane.editRefused
                           + " tip=" + tagNameBoxTimer.tipped(at)
                           + " text=" + Tips.box.text
                } })
        }
    }
    // PGG_AUTO_ACT=nav-rename-far. Each stage waits for the one before to land — a list still building has no height to
    // scroll by.
    PageStage {
        id: farTimer
        name: "nav_far"
        order: ["peeked", "scrolled", "renamed"]
        onTriggered: {
            const section = sidebarPane.peekSection
            if (farTimer.at("start")) {
                // Rested on the cell each beat until the section stands.
                farTimer.input("peeked", () => {
                    if (sidebarPane.peekKind !== "")
                        return true
                    navProbe.peekAt("tag")
                    return false
                })
                return
            }
            if (farTimer.at("peeked")) {
                if (!Awaited.all("nav_far_peek", { "peek": sidebarPane.peekKind !== "" }))
                    return
                farTimer.input("scrolled", () => {
                    section.scrollToEnd()
                })
                return
            }
            if (farTimer.at("scrolled")) {
                if (!Awaited.all("nav_far_scrolled", { "away": !section.rowInView(0) }))
                    return
                farTimer.input("renamed", () => {
                    sidebarPane.beginRename("tag", tagsModel.nameAt(0), tagsModel.nameAt(0))
                })
                return
            }
            farTimer.complete({
                "name": "nav_far_box",
                "when": () => ({ "box": sidebarPane.editKey !== "" }),
                "report": () => "nav_far row=" + tagsModel.nameAt(0)
                                + " shown=" + sidebarPane.peekSection.rowInView(0)
                                + " box=" + (sidebarPane.editKey !== "")
                                + " collapsed=" + page.sidebarCollapsed
                                + " editing=" + sidebarPane.editKey })
        }
    }
    // The replace's question, waited on until settled: a bar photographed before its words arrive is a red line with
    // nothing on it. The plain verb ends here — the write is "-go"'s half.
    PageStage {
        id: replaceAskTimer
        name: "replace_ask"
        /// A hand goes on the pill once the bar has settled (`replace-remote-tip`): the tip places itself against the
        /// pill as it opens, so a pill still moving would leave it behind.
        property bool points: false
        onTriggered: {
            if (!Awaited.all("replace_ask_ready", { "ready": graphPane.askCard.settled }))
                return
            if (replaceAskTimer.points && replaceAskTimer.at("start")) {
                // `pill=` is the word the hand is on, `words=` whether the bar has a tip to open at all. A run that
                // ends on the ceiling with this as its last line stopped at the tip.
                replaceAskTimer.input("pointed", () => {
                    graphPane.askCard.pointedAt = true
                }, () => "replace_ask step=pointed pill=" + graphPane.askCard.accept
                         + " words=" + (graphPane.askCard.tip !== ""))
                return
            }
            // `code=` empty is part of the claim: a push plus a delete is no one command, so the pill is in the
            // ordinary voice (規約 §git 用語のコード表記「どれもコマンドと 1 対 1 でない」).
            replaceAskTimer.complete({
                "name": "replace_ask_tip",
                "when": () => ({ "tip": !replaceAskTimer.points || graphPane.askCard.tipStanding }),
                "report": () => "replace_ask hold=" + graphPane.askHold
                                + " code=" + graphPane.askCode
                                + (replaceAskTimer.points ? " tip=" + graphPane.askCard.tipStanding : "") })
        }
    }
    // PGG_AUTO_ACT=rename-tag-remote / -go / -tip: the question that carries a tag's new name over to the remote
    // holding the old one.
    //
    // The rename waits for this run's fetch (`nav-open-tag` の同じ待ち): renamed before `ls-remote` answered, it arms
    // nothing and the bar never comes. The fetch is waited on by its ask's id (`AutoActDriver.pressWrite`): its start
    // and end can be drained together, so a beat watching the busy count rise can miss it altogether.
    PageStage {
        id: tagRemoteRenameTimer
        name: "tag_remote_rename"
        property string tag: ""
        /// The tag's new name.
        property string naming: ""
        /// Whether the pill is pressed through, or the run stops at the question standing.
        property bool holds: false
        /// Which of the three answers the run picks, empty for the bar as it comes down.
        property string choice: ""
        /// Whether a hand goes on the pill once the pick has dressed the bar (`rename-tag-remote-tip`).
        property bool points: false
        function begin(arg, holds, points) {
            const parts = ("" + arg).split(":")
            tagRemoteRenameTimer.tag = parts[0]
            tagRemoteRenameTimer.naming = parts.length > 1 ? parts[1] : ""
            tagRemoteRenameTimer.choice = points ? "replace"
                : parts.length > 2 ? parts[2] : (holds ? "replace" : "")
            tagRemoteRenameTimer.holds = holds
            tagRemoteRenameTimer.points = points
            // The pick and the hand only where asked; a run that answers says the bar before the answer goes in.
            tagRemoteRenameTimer.order = ["renamed"].concat(tagRemoteRenameTimer.choice !== "" ? ["picked"] : [],
                                                            points ? ["pointed"] : [],
                                                            holds ? ["said", "answered"] : [])
            driver.pressFetch()
            tagRemoteRenameTimer.start()
        }
        /// What the bar says. `here=` is the wait for the new name's row said out loud; `mark=` weighs the commit the
        /// bar marked (taken before the write) against the one the tags section came back with for the new name.
        function line() {
            return driver.carryWords("tag", tagRemoteRenameTimer.choice)
                   + " here=" + (tagsModel.rowOfName(tagRemoteRenameTimer.naming) >= 0)
                   + " mark=" + (graphPane.view.askOid !== ""
                       && graphPane.view.askOid === tagsModel.oidOfName(tagRemoteRenameTimer.naming))
                   + (tagRemoteRenameTimer.points ? " tip=" + graphPane.askCard.tipStanding : "")
        }
        onTriggered: {
            const op = tagRemoteRenameTimer
            // Past the answer only `leave` is still running: it writes nothing, so the bar going back up is its end.
            if (op.at("answered")) {
                op.complete({
                    "name": "tag_remote_rename_shut",
                    "when": () => ({ "shut": graphPane.askCard.shut }),
                    "report": () => op.witnessed("bar") })
                return
            }
            if (op.at("start")) {
                // The arming needs both sides holding the name (`RepoPage.armRenameTagRemote`), read once this run's
                // fetch is through.
                if (!Awaited.all("tag_remote_rename_fetched", {
                        "idle": repoTab.busyCount === 0,
                        "fetched": driver.wroteAndSettled(),
                        "both": tagsModel.tagSides(op.tag) === "both" }))
                    return
                // Through the row's own road, so a build where the gesture stopped reaching the rename waits here.
                op.input("renamed", () => {
                    sidebarPane.beginRename("tag", op.tag, op.tag)
                    sidebarPane.submitEdit(op.naming)
                })
                return
            }
            // And the new name's row, which can land after the bar: the write's answer raises the question before the
            // read that re-lists the tags (rules-refs/app-ui.md「その書き込みが無効化した読み直しより先に来る」).
            if (!Awaited.all("tag_remote_rename_asked", {
                    "idle": repoTab.busyCount === 0,
                    "bar": graphPane.askCard.settled,
                    "row": tagsModel.rowOfName(op.naming) >= 0 }))
                return
            if (op.choice !== "" && op.at("renamed")) {
                // Through the field's own door, since no injected click opens a popup offscreen
                // (`RenameCarryFlow.pickChoice`); asked each beat until it takes, the form's `Loader` building a frame
                // behind the bar.
                op.input("picked", () => page.pickCarryChoice(driver.carryChoiceIndex(op.choice)))
                return
            }
            // A beat after the pick, so the tip places itself against the pill wearing the picked answer's word.
            if (op.points && !op.at("pointed")) {
                // `pill=` / `words=` as in `replaceAskTimer`.
                op.input("pointed", () => {
                    graphPane.askCard.pointedAt = true
                }, () => "rename_carry step=pointed pill=" + graphPane.askCard.accept
                         + " words=" + (graphPane.askCard.tip !== ""))
                return
            }
            if (!op.holds) {
                op.complete({
                    "name": "tag_remote_rename_tip",
                    "when": () => ({ "tip": !op.points || graphPane.askCard.tipStanding }),
                    "report": () => op.line() })
                return
            }
            if (!Awaited.all("tag_remote_rename_tip", { "tip": !op.points || graphPane.askCard.tipStanding }))
                return
            // The bar as the answer finds it, kept before the answer goes in — the answer takes it down — and claimed
            // by the completion: the write's barrier's (`reportAtCompletion`), or for `leave`, this operation's. The
            // barrier is this operation's to start: the dispatch hands none to a verb that defers its completion
            // (`AutoActDriver.dispatchFinished`). Only the answer that takes a name off the remote asks for a hold.
            op.witness("bar", op.line())
            op.advance("said")
            if (!graphPane.askHold && op.choice === "leave") {
                // Writes nothing: see the head of `onTriggered`.
                op.input("answered", () => {
                    page.answerRowAsk()
                })
                return
            }
            driver.reportAtCompletion(() => tagRemoteRenameTimer.witnessed("bar"))
            if (graphPane.askHold) {
                op.input("answered", () => {
                    driver.holdToEnd(graphPane)
                })
            } else {
                op.input("answered", () => {
                    driver.pressWrite("answer-ask", () => { page.answerRowAsk(); return true })
                })
            }
            op.stop()
            writeBarrier.start()
        }
    }
    // PGG_AUTO_ACT=nav-branch-box / nav-rename-box / nav-tag-box: how wide the box comes out on a row of that depth in
    // a pane of that width — one question, so one timer for all of them. The field's modes (`NavNameBox`) are read side
    // by side: `Branch name` for a new branch, `Tag name` for a new tag, the rename (emptied, it shows its kind's
    // placeholder). A width under the floor (180) is taken at the floor by the splitter; none keeps the width it opened
    // at (260 by default). `whole=` is the wider of the full placeholder and the name the box opened with — latched at
    // the open, so typing does not move it. A box drawn outside the pane has two ways to vanish that a box inside its
    // row has not (cut by a layer, its row scrolled away), hence `shown=` beside `open=` / `focused=`; `:away` takes
    // the second, on a section that overflows (TAGS of `--preset manytags`). On `basic`, `branch:main` /
    // `branch:feature/topic-a` / `remote:origin/feature/topic-a` keep a `Branch name` box in its seat even at the floor
    // (seat 118 / 146 / 140, whole 95 with the default font); the rename side's wall, outside the pane at every width,
    // is `--preset worktrees`' `release/2026-08-candidate-with-a-very-long-name`.
    PageStage {
        id: navNameBoxTimer
        name: "nav_name_box"
        property string mode: "branch"
        property string kind: "branch"
        property string ref: ""
        /// The width the splitter settles at.
        property real want: 0
        readonly property NavSectionModel section:
            navNameBoxTimer.kind === "tag" ? tagsModel
            : navNameBoxTimer.kind === "remote" ? remotesModel : branchesModel
        /// Walks the list past the row once the box stands: a box drawn outside the list has to go when its row does.
        property bool away: false
        function begin(mode, arg) {
            const parts = ("" + arg).split(":")
            navNameBoxTimer.mode = mode
            navNameBoxTimer.away = parts[parts.length - 1] === "away"
            navNameBoxTimer.kind = parts[0]
            navNameBoxTimer.ref = parts[1]
            const asked = parts.length > 2 && parts[2] !== "away" ? Number(parts[2]) : NaN
            navNameBoxTimer.want = isNaN(asked) ? sidebarPane.width
                                                  : Math.max(asked, sidebarPane.minOpenWidth)
            if (!isNaN(asked))
                page.setSidebarWidth(asked)
            // A remote's root folder starts closed (`models::nav::tree`), leaving its rows in no list.
            if (navNameBoxTimer.kind === "remote")
                remotesModel.toggleFolder(GitFacts.remoteOfRef(navNameBoxTimer.ref, repoTab.remoteNames))
            navNameBoxTimer.order = navNameBoxTimer.away ? ["opened", "walked"] : ["opened"]
            navNameBoxTimer.start()
        }
        /// On the row and built: a row not laid out yet answers 0, like a row with no box.
        function drawn() {
            const row = navNameBoxTimer.section.rowOfName(navNameBoxTimer.ref)
            return { "open": sidebarPane.editKey === navNameBoxTimer.kind + ":" + navNameBoxTimer.ref,
                     "drawn": navProbe.listOf(navNameBoxTimer.kind).rowBoxWidth(row) > 0 }
        }
        /// The box as drawn; `focused=` since a box that cannot be typed into looks the same in the picture.
        function line() {
            const at = navNameBoxTimer.section.rowOfName(navNameBoxTimer.ref)
            const view = navProbe.listOf(navNameBoxTimer.kind)
            const drawn = view.rowBoxWidth(at)
            const whole = view.rowBoxWhole(at)
            return "nav_name_box mode=" + navNameBoxTimer.mode
                   + " ref=" + navNameBoxTimer.ref
                   + " row=" + at
                   + " pane=" + Math.round(sidebarPane.width)
                   // The room the row holds, the drawn width, and what reading it whole takes: whether it needed room
                   // outside the pane and got it.
                   + " seat=" + Math.round(view.rowBoxSeat(at))
                   + " box=" + Math.round(drawn)
                   + " whole=" + Math.ceil(whole)
                   // Cut, a placeholder no longer says what goes in and a name cannot be read back
                   // (デザイン規約 §グラフ行のダブルクリック).
                   + " cut=" + (Math.ceil(whole) > Math.round(drawn))
                   + " open=" + (sidebarPane.editKey === navNameBoxTimer.kind + ":" + navNameBoxTimer.ref)
                   + " focused=" + view.rowFocused(at)
                   + " shown=" + view.rowBoxShown(at)
                   + " at=" + view.rowBoxAt(at)
        }
        onTriggered: {
            const op = navNameBoxTimer
            // -1 for a row behind a closed folder or in a section not read yet.
            const row = op.section.rowOfName(op.ref)
            if (op.at("start")) {
                // The width first: the box is laid out in what the row has left, so one opened earlier settles into a
                // width nobody asked about.
                if (!Awaited.all("nav_name_box_row", {
                        "row": row >= 0,
                        "width": Math.round(sidebarPane.width) === Math.round(op.want) }))
                    return
                op.input("opened", () => {
                    if (op.mode === "rename") {
                        // What a second click puts in the box (`NavList`): a remote branch without its remote.
                        const shown = op.kind === "remote"
                            ? GitFacts.branchOfRef(op.ref, repoTab.remoteNames)
                            : op.ref
                        sidebarPane.beginRename(op.kind, op.ref, shown)
                        return
                    }
                    const oid = op.section.oidOfName(op.ref)
                    // Through the menu — the box's only door on this side, and the one that picks this box over the
                    // graph's (`RepoPage.refMenuInSidebar`). Both names are the whole ref: only `full` reaches git.
                    page.openRefMenu(op.kind, op.ref, op.ref, oid, "sidebar")
                    if (op.mode === "tag")
                        page.startTagAt(oid)
                    else
                        page.startBranchAt(oid)
                    // Closed as choosing a row closes it: a menu left standing is drawn over the rows beside the box.
                    refMenu.close()
                })
                return
            }
            const list = navProbe.listOf(op.kind)
            if (op.at("opened")) {
                if (!op.away) {
                    op.complete({ "name": "nav_name_box_drawn", "when": () => op.drawn(), "report": () => op.line() })
                    return
                }
                if (!Awaited.all("nav_name_box_drawn", op.drawn()))
                    return
                // Just out of the view from where the row stands, near enough that the view still holds its delegate
                // — otherwise what went is the row and not the box. A list too short to scroll the row out is the
                // fixture's shortfall, named here instead of waited out.
                if (!op.input("walked", () => list.scrollRowOut(row)))
                    op.note("nav_name_box_away waiting=room")
                return
            }
            // Walked past: wait for the box to go, read off `shown` — a released delegate answers 0 for everything.
            op.complete({
                "name": "nav_name_box_gone",
                "when": () => ({ "gone": !navProbe.listOf(op.kind).rowBoxShown(op.section.rowOfName(op.ref)) }),
                "report": () => op.line() })
        }
    }
    // PGG_AUTO_ACT=rename-box-out: one way out of the name box per run. The claim is what the box and the gesture are
    // left holding: a wait still running is how a box comes back by itself, so `armed=false` says it will not — the
    // picture cannot tell.
    PageStage {
        id: boxOutTimer
        name: "box_out"
        order: ["first_click", "second_click", "out"]
        /// Which way out this run takes: `same-row` / `other-row` (a click there only closes the box) / `escape` /
        /// `away` (an untouched box goes with a press elsewhere) / `typed-away` (a box holding a half-typed name
        /// stays — the one way out that leaves it).
        property string route: ""
        onTriggered: {
            const op = boxOutTimer
            const item = graphPane.view.itemAtIndex(0)
            if (op.at("start")) {
                // Opened by the gesture itself, two clicks: a box put up any other way leaves the gesture with no
                // memory of the row, and every way out passes for free.
                if (!Awaited.all("box_out_row", { "row": !!item }))
                    return
                op.input("first_click", () => {
                    item.leftClick(Qt.NoModifier)
                })
            } else if (op.at("first_click")) {
                // Inside the double-click window a second click would be half a double-click.
                if (!Awaited.all("box_out_second", { "row": !!item, "window": !graphPane.view.clickGuarded }))
                    return
                op.input("second_click", () => {
                    item.leftClick(Qt.NoModifier)
                })
            } else if (op.at("second_click")) {
                if (!Awaited.all("box_out_box", { "row": !!item, "box": graphPane.namingOid !== "" }))
                    return
                op.input("out", () => {
                    const route = op.route
                    if (route === "same-row" || route === "other-row") {
                        // A click is a press then a release, answered in different places: the press reaches
                        // `FocusRelease` first and takes the box down, and only then does the row see the click — a
                        // click put in alone skips the press.
                        page.releasePressedAway(null)
                        const row = route === "same-row" ? item : graphPane.view.itemAtIndex(1)
                        if (!row)
                            return false
                        row.leftClick(Qt.NoModifier)
                    } else if (route === "escape") {
                        graphPane.view.namingCancelled()
                    } else {
                        if (route === "typed-away") {
                            graphPane.view.namingText = "half-written"
                            item.takeNamingFocus()
                        }
                        page.releasePressedAway(null)
                    }
                })
            } else {
                // The beat after the way out, on the page it leaves.
                op.complete({
                    "name": "box_out_left",
                    "when": () => ({ "page": PageSettled.settled(page) }),
                    "report": () => "rename_box_out route=" + op.route
                                    + " box=" + (graphPane.namingOid !== "")
                                    + " armed=" + graphPane.view.renameWaiting
                                    + " guarded=" + graphPane.view.clickGuarded
                                    + " typed=" + graphPane.namingText })
            }
        }
    }
}

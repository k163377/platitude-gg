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

    // The driver's own names, read once so the verbs can name them bare.
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
    readonly property var refTagCard: driver.refTagCard
    readonly property var writeBarrier: driver.barrierWrite

    /// Runs `act` if it is this family's and says whether it was; the driver asks the families in turn, so each verb
    /// belongs to one (`AutoActDriver`).
    function run(act, arg) {
        if (act === "nav-rename-far") {
            // A box on a row the list had scrolled away from: the list has to bring it back (デザイン規約 §左メニューの所作).
            // On the folded rail's section: the one list a run can scroll and read back through one handle.
            page.foldByHand(true)
            farTimer.start()
        } else if (act === "nav-rename-drop") {
            // The box walked away from untyped: "fold" takes the list down to the rail, "away" is a press elsewhere
            // (Main's `FocusRelease` enters here).
            sidebarPane.beginRename("branch", worktree.branch,
                                    worktree.branch)
            if (arg === "fold")
                page.foldByHand(true)
            else
                page.releasePressedAway(null)
            driver.reportAtCompletion(() => "nav_drop how=" + arg
                                            + " collapsed=" + page.sidebarCollapsed
                                            + " box=" + (sidebarPane.editKey !== "")
                                            + " editing=" + sidebarPane.editKey)
        } else if (act === "nav-tip" || act === "nav-open") {
            // `<section>:<row>`, or `head` for the current branch's sticky stand-in. `nav-open` reads the lines a row
            // opens under itself (`NavRowFacts`) instead of the shared tooltip.
            navTipTimer.opens = act === "nav-open"
            navTipTimer.then = ""
            navTipTimer.begin(arg)
        } else if (act === "nav-open-foot") {
            // The same rest on the last row of an overflowing section scrolled to its end, with nowhere to open into:
            // the list has to move (`NavList.revealOpenRow`). `away` then leaves, and the list gives back what it took;
            // `bar` takes the list's bar instead, which closes the row and gives it back the same way.
            navTipTimer.opens = true
            navTipTimer.then = arg
            navTipTimer.foot = true
            navTipTimer.begin("branch")
        } else if (act === "nav-drag-open") {
            // A press on a closed row's line that starts to move: the lines come out at once and the drag carries on
            // into them (デザイン規約 §左メニューの所作).
            const parts = ("" + arg).split(":")
            dragOpenTimer.kind = parts[0] === "" ? "branch" : parts[0]
            dragOpenTimer.row = parts.length > 1 ? Number(parts[1]) : 2
            dragOpenTimer.held = parts.length > 2 ? parts[2] : ""
            dragOpenTimer.start()
        } else if (act === "nav-open-tip") {
            // The rest that opens a row, held on: the row opens, and a rest later the supplement it keeps (a worktree's
            // path) follows from the same unmoved hand (デザイン規約 §左メニューの所作).
            const parts = ("" + arg).split(":")
            openTipTimer.kind = parts[0] === "" ? "worktree" : parts[0]
            openTipTimer.row = parts.length > 1 ? Number(parts[1]) : 3
            openTipTimer.start()
        } else if (act === "nav-open-held") {
            // The name box first, then a hand on a row: nothing may open — it would move the rows under the box.
            sidebarPane.beginRename("branch", navProbe.tipNameAt("branch", 2),
                                    navProbe.tipNameAt("branch", 2))
            openHeldTimer.start()
        } else if (act === "nav-open-then") {
            // The open row, then the hand's next move: `away` leaves it, `edit` opens the name box on that row, `menu`
            // opens a menu elsewhere, `filter` types a filter that takes the row out, `rightclick-name` raises the
            // row's menu on its own line, and `rightclick` / `sweep` / `tap` act on the open lines themselves.
            navTipTimer.opens = true
            navTipTimer.then = arg
            navTipTimer.begin("branch:2")
        } else if (act === "nav-follow" || act === "nav-follow-lit") {
            // `<section>:<row>[:<filter>]`: the open row's first line that goes somewhere, pressed (the graph goes
            // there and the panel stays — デザイン規約 §左メニューの所作), or with `-lit` rested on.
            followTimer.begin(arg, act === "nav-follow-lit")
        } else if (act === "nav-open-tag") {
            // A TAGS row opening on the remotes carrying its name. Named, not numbered: tags sort newest-first with
            // remote-only names last, so the row a name lands on is the preset's business.
            navOpenTagTimer.begin(arg, -1)
        } else if (act === "tag-line-menu") {
            // `<tag>:<line>`: the same row opened, then the menu asked for on that line of it — a carrier's line names
            // its remote on its own, so the TAG card acts on it (`NavRowFacts.lineMenu`).
            const at = arg.lastIndexOf(":")
            navOpenTagTimer.begin(arg.slice(0, at), Number(arg.slice(at + 1)))
        } else if (act === "nav-rename" || act === "rename-branch"
                   || act === "rename-tag" || act === "rename-stash") {
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
            // A name the box itself refuses is not sent, and the run says nothing.
            sidebarPane.beginRename("branch", worktree.branch, arg)
            if (!sidebarPane.editRefused)
                sidebarPane.submitEdit(arg)
            renameTakenTimer.start()
        } else if (act === "rename-tag-box" || act === "rename-branch-box") {
            // The box opened with the argument typed in, so a name the box itself refuses can be photographed
            // (デザイン規約 §答えの要らない報せ「押す前に断れるものは、名前の箱で断る」). `was=` names the row, so a
            // run that opened the box on the wrong one says so.
            tagNameBoxTimer.begin(arg, act === "rename-branch-box" ? "branch" : "tag")
        } else if (act === "replace-remote" || act === "replace-remote-box"
                   || act === "replace-remote-go" || act === "replace-remote-tip") {
            // Named outright (`origin/billing:billing-v2`) because the remote's rows are behind a fold. "-box" leaves
            // the box standing, the plain act stops at the question, "-go" holds the pill to the end, "-tip" stops at
            // the question with a hand on the pill.
            const parts = arg.split(":")
            const ref = parts[0]
            const was = GitFacts.branchOfRef(ref, repoTab.remoteNames)
            // "-box" types the argument in, to photograph a name the remote already carries being refused, and opens
            // the remote's fold (a remote root starts closed) so the row is there at all.
            if (act === "replace-remote-box")
                remotesModel.toggleFolder(GitFacts.remoteOfRef(ref, repoTab.remoteNames))
            sidebarPane.beginRename("remote", ref,
                                    act === "replace-remote-box" ? parts[1] : was)
            // The box standing is the whole of "-box": the dispatch completes it. A known verb answers true even on
            // its early way out — falsy would send the dispatch on asking every other family.
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
            // A tag a remote carries too, renamed here, and the question that comes back for the copy over there.
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
            if (!Awaited.all("rename_taken_ready", {
                "idle": repoTab.busyCount === 0,
                "bar": page.noticeCard.open && page.noticeCard.settled }))
                return
            renameTakenTimer.complete(() => "rename_taken open=" + (sidebarPane.editKey !== "")
                                            + " refused=" + sidebarPane.editRefused
                                            + " tone=" + page.noticeCard.tone
                                            // The word of the line that opens the log, then the line. Last, because
                                            // it is a sentence.
                                            + " place=" + page.noticeCard.detailPlace
                                            + " says=" + page.noticeCard.detail)
        }
    }
    // PGG_AUTO_ACT=rename-tag-box / rename-branch-box: the box on the first tag or on the branch HEAD is on, the
    // argument typed in, and whether the reason for refusing the name reached the reader. A branch's argument may name
    // the row too, `<branch>:<typed>` (a branch name holds no `:`) — a branch a stopped rebase holds is never HEAD's.
    //
    // `editKey` is only the ask: the box is built by a `Loader` in a row not yet laid out, so `box=` reads the box as
    // drawn and holding the keyboard (`NavList.rowBoxShown` / `rowFocused`). `refused=` alone passes a box refusing in
    // silence, and Qt can drop the tooltip the box's binding raises (`NavNameBox`) as a binding loop without a word,
    // so `tip=` is taken through the box (`NavList.rowTipShown` = the box's own `TipAsk.shown`).
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
        onTriggered: {
            const row = tagNameBoxTimer.rowOfBox()
            const open = tagNameBoxTimer.open(row)
            // Only a refused name raises a tip to wait for.
            if (!Awaited.all("tag_name_box_ready", {
                    "ready": open,
                    "tip": !sidebarPane.editRefused || (open && tagNameBoxTimer.tipped(row)) }))
                return
            tagNameBoxTimer.complete(() => {
                const at = tagNameBoxTimer.rowOfBox()
                return tagNameBoxTimer.kind + "_name_box was=" + tagNameBoxTimer.was()
                       + " typed=" + tagNameBoxTimer.typed
                       + " box=" + tagNameBoxTimer.open(at)
                       + " refused=" + sidebarPane.editRefused
                       + " tip=" + tagNameBoxTimer.tipped(at)
                       + " text=" + Tips.box.text
            })
        }
    }
    // PGG_AUTO_ACT=nav-rename-far. Each stage waits for the one before to land — a list still building has no height to
    // scroll by.
    PageStage {
        id: farTimer
        name: "nav_far"
        order: ["scrolled", "renamed"]
        onTriggered: {
            const section = sidebarPane.peekSection
            if (farTimer.at("start")) {
                if (!Awaited.all("nav_far_peek", { "peek": sidebarPane.peekKind !== "" })) {
                    navProbe.peekAt("tag")
                    return
                }
                farTimer.input("scrolled", () => {
                    section.scrollToEnd()
                })
                return
            }
            if (farTimer.at("scrolled")) {
                if (!Awaited.all("nav_far_scrolled", { "away": !section.rowInView(0) }))
                    return
                farTimer.input("renamed", () => {
                    sidebarPane.beginRename("tag", tagsModel.nameAt(0),
                                            tagsModel.nameAt(0))
                })
                return
            }
            if (!Awaited.all("nav_far_box", { "box": sidebarPane.editKey !== "" }))
                return
            farTimer.complete(() => "nav_far row=" + tagsModel.nameAt(0)
                                    + " shown=" + sidebarPane.peekSection.rowInView(0)
                                    + " box=" + (sidebarPane.editKey !== "")
                                    + " collapsed=" + page.sidebarCollapsed
                                    + " editing=" + sidebarPane.editKey)
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
                // `pill=` is the word the hand is on, `words=` whether the bar has a tip to open at all.
                replaceAskTimer.input("pointed", () => {
                    graphPane.askCard.pointedAt = true
                }, () => "replace_ask step=pointed pill=" + graphPane.askCard.accept
                         + " words=" + (graphPane.askCard.tip !== ""))
                return
            }
            if (!Awaited.all("replace_ask_tip", { "tip": !replaceAskTimer.points || graphPane.askCard.tipStanding }))
                return
            // `code=` empty is part of the claim: a push plus a delete is no one command, so the pill is in the
            // ordinary voice (規約 §git 用語のコード表記「どれもコマンドと 1 対 1 でない」).
            replaceAskTimer.complete(() => "replace_ask hold=" + graphPane.askHold
                                           + " code=" + graphPane.askCode
                                           + (replaceAskTimer.points ? " tip=" + graphPane.askCard.tipStanding : ""))
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
            driver.pressWrite("fetch", () => {
                repoTab.fetch("")
                return true
            })
            tagRemoteRenameTimer.start()
        }
        /// What the bar says. `here=` is the wait for the new name's row said out loud; `mark=` weighs the commit the
        /// bar marked (taken before the write) against the one the tags section came back with for the new name.
        function line() {
            return driver.carryWords("tag", tagRemoteRenameTimer.choice)
                   + " here=" + (tagsModel.rowOfName(tagRemoteRenameTimer.naming) >= 0)
                   + " mark=" + (graphPane.view.askOid !== ""
                       && graphPane.view.askOid
                          === tagsModel.oidOfName(tagRemoteRenameTimer.naming))
                   + (tagRemoteRenameTimer.points ? " tip=" + graphPane.askCard.tipStanding : "")
        }
        onTriggered: {
            const op = tagRemoteRenameTimer
            // Past the answer only `leave` is still running: it writes nothing, so the bar going back up is its end.
            if (op.at("answered")) {
                if (!Awaited.all("tag_remote_rename_shut", { "shut": graphPane.askCard.shut }))
                    return
                op.complete(() => [])
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
            if (!Awaited.all("tag_remote_rename_tip", { "tip": !op.points || graphPane.askCard.tipStanding }))
                return
            if (!op.holds) {
                op.complete(() => op.line())
                return
            }
            // Said before the answer goes in. The barrier is this operation's to start: the dispatch hands none to a
            // verb that defers its completion (`AutoActDriver.dispatchFinished`). Only the answer that takes a name off
            // the remote asks for a hold.
            op.advance("said", () => op.line())
            if (graphPane.askHold) {
                op.input("answered", () => {
                    driver.holdToEnd(graphPane)
                })
            } else if (op.choice === "leave") {
                // Writes nothing: see the head of `onTriggered`.
                op.input("answered", () => {
                    page.answerRowAsk()
                })
                return
            } else {
                op.input("answered", () => {
                    driver.pressWrite("answer-ask", () => { page.answerRowAsk(); return true })
                })
            }
            op.stop()
            writeBarrier.start()
        }
    }
    // No rest first: the gesture skips it, and a run that rested on the row would be reading the other path.
    PageStage {
        id: dragOpenTimer
        name: "nav_drag_open"
        order: ["dragged"]
        property string kind: "branch"
        property int row: 2
        /// A filter typed before the hand arrives, for a row that is folded away without one.
        property string held: ""
        /// The row's name, read as the press went in.
        property string rowName: ""
        onTriggered: {
            const op = dragOpenTimer
            if (op.at("start")) {
                // The filter, typed again until it takes; then the row, pressed again until it is there to press: a
                // miss reads like a row that refused to open (`nav-open` の同じ歩き).
                const filtered = op.held === "" || navProbe.typeFilter(op.held) === op.held
                if (!Awaited.all("nav_drag_open_row", {
                        "filtered": filtered,
                        "row": filtered && navProbe.tipNameAt(op.kind, op.row) !== "" }))
                    return
                op.input("dragged", () => {
                    op.rowName = navProbe.tipNameAt(op.kind, op.row)
                    return navProbe.dragRow(op.kind, op.row)
                })
                return
            }
            // The beat after the press, on the page it leaves.
            if (!Awaited.all("nav_drag_open_dragged", { "page": PageSettled.settled(page) }))
                return
            op.complete(() => {
                const took = navProbe.rowNameTook(op.kind, op.row)
                return "nav_drag_open section=" + op.kind
                    + " row=" + op.row + " name=" + op.rowName
                    + " open=" + navProbe.rowFactsOpen
                    + " caret=" + navProbe.rowNameCaret(op.kind, op.row)
                    + " copied=" + (took !== "")
                    // A drag that took the name is not a click — a second click on the row opens a name box
                    // (`NavItemDelegate.lineClicked`).
                    + " clicked=" + (sidebarPane.activeKey !== "")
                    + " took=" + took
            })
        }
    }
    // PGG_AUTO_ACT=nav-open-tip: the worktree's path the open row keeps for the hand that opened it. The picture
    // shows the words; `says=` is the same path read off the row, so a tip drawn from anywhere else fails to match.
    PageStage {
        id: openTipTimer
        name: "nav_open_tip"
        property string kind: "worktree"
        property int row: 3
        onTriggered: {
            // Re-applied every beat (`nav-open` の同じ歩き).
            navProbe.pointTipAt(openTipTimer.kind, openTipTimer.row)
            if (!Awaited.all("nav_open_tip", { "open": navProbe.rowFactsOpen, "tip": Tips.box.visible }))
                return
            openTipTimer.complete(() => {
                const tip = Tips.box
                return "nav_open_tip section=" + openTipTimer.kind
                    + " row=" + openTipTimer.row
                    + " open=" + navProbe.rowFactsOpen
                    + " tip=" + tip.visible
                    // An empty tip answers `tip=true` as well as a full one (`NavRowFacts.said`); weighed in the run,
                    // since the path is this machine's own.
                    + " same=" + (tip.text === navProbe.factsSays())
                    // Last, since a path carries anything.
                    + " says=" + navProbe.rowFactsWords()
                    + " text=" + tip.text
            })
        }
    }
    // The row still has to be the one under the hand — a run that never reached it would answer the same way.
    PageStage {
        id: openHeldTimer
        name: "nav_open_held"
        onTriggered: {
            // The stand-in opens outright where a hand would sit out a rest, so a row that was going to open is open
            // by the time the completion reads it.
            navProbe.pointTipAt("branch", 2)
            if (!Awaited.all("nav_open_held", { "row": navProbe.tipNameAt("branch", 2) !== "",
                                                "box": sidebarPane.editKey !== "" }))
                return
            openHeldTimer.complete(() => "nav_open_held row=2 name=" + navProbe.tipNameAt("branch", 2)
                                         + " box=true open=" + navProbe.rowFactsOpen
                                         + " lit=" + navProbe.rowWashLit("branch", 2))
        }
    }
    // PGG_AUTO_ACT=nav-follow / nav-follow-lit. The landing is read off the graph, not the press: the page's selection
    // is only the click's bookkeeping, so `landed=` is the graph's row for that commit, lit and laid out.
    //
    // No `order`: a row whose lines go nowhere completes where it opened, one with a line that goes somewhere after
    // aiming at it — which of the two is the repository's to say.
    PageStage {
        id: followTimer
        name: "nav_follow"
        property string kind: "remote"
        property int row: 0
        property string filter: ""
        property bool lit: false
        /// The first of the open lines that goes somewhere, and where, read once the lines stand still.
        property int line: -1
        property var to: null
        /// The row's name, read as it opened.
        property string rowName: ""
        function begin(arg, lit) {
            const parts = ("" + arg).split(":")
            followTimer.kind = parts[0] === "" ? "remote" : parts[0]
            followTimer.row = parts.length > 1 ? Number(parts[1]) : 0
            followTimer.filter = parts.length > 2 ? parts[2] : ""
            followTimer.lit = lit
            followTimer.start()
        }
        function said() {
            return " section=" + followTimer.kind + " row=" + followTimer.row + " line=" + followTimer.line
        }
        /// The open row's supplement is up, where it keeps one (a WORKTREES row's path, `NavRowFacts.says`): it comes
        /// up a rest after the lines, and every end below waits it out, or the picture holds the box or not by the
        /// machine's pace. Said through `Awaited`, so a run stalled here names it.
        function tipRested() {
            const lines = sidebarPane.openFactsItem()
            return Awaited.all(followTimer.lit ? "nav_follow_lit" : "nav_follow",
                               { "tip": lines === null || lines.says === "" || lines.tipShown })
        }
        onTriggered: {
            const op = followTimer
            // A filter, once: the way a leaf under a folded folder is brought into the list.
            if (op.filter !== "" && op.at("start")) {
                // The page's rows in before the filter rebuilds them (a section's list stands only once peeked, so
                // the list itself is nothing to wait for here).
                if (!Awaited.all("nav_follow_filter", { "page": PageSettled.settled(page) }))
                    return
                op.input("filtered", () => {
                    navProbe.typeFilter(op.filter)
                })
                return
            }
            // The rest, every beat until the row is open (`nav-open` の同じ歩き).
            if (op.at("start") || op.at("filtered")) {
                navProbe.pointTipAt(op.kind, op.row)
                if (!Awaited.all("nav_follow_open", { "open": navProbe.rowFactsOpen }))
                    return
                op.rowName = navProbe.tipNameAt(op.kind, op.row)
                op.advance("opened")
                return
            }
            // Nothing is aimed until the lines stop moving: laid out a pass after they are built, a press worked out
            // from a line still at its origin lands on the row's mark.
            if (op.at("opened")) {
                if (!Awaited.all("nav_follow_geom", { "still": op.still("nav_follow_geom",
                                                                        sidebarPane.rowFactsGeom()) }))
                    return
                op.line = navProbe.factsFirstGoing()
                op.to = navProbe.factsGoesTo(op.line)
                if (op.to === null) {
                    // Lines that go nowhere are this row's answer — waiting on for a landing would spend the ceiling
                    // and take no picture.
                    if (!op.tipRested())
                        return
                    op.complete(() => (op.lit ? "nav_follow_lit" : "nav_follow") + op.said()
                                      + " followed=false name=" + op.rowName)
                    return
                }
                op.input("aimed", () => {
                    if (op.lit)
                        navProbe.pointFactsWords(op.line, true)
                    else
                        navProbe.followFactsLine(op.line)
                })
                return
            }
            if (op.lit) {
                if (!op.tipRested())
                    return
                // `aimed=` is the band as drawn, `hand=` the cursor the area took — two parts, so either alone splits.
                op.complete(() => "nav_follow_lit" + op.said()
                                  + " aimed=" + navProbe.factsLineAimed(op.line)
                                  + " hand=" + navProbe.factsHand()
                                  + " open=" + navProbe.rowFactsOpen
                                  + " to=" + op.to.key + " name=" + op.rowName)
                return
            }
            // A commit the walk never reached is an answer; a row the model has but the view has not laid out is
            // waited for.
            const want = op.to.oid
            const landing = graphModel.rowOf(want)
            if (!Awaited.all("nav_follow_landed", {
                    "selected": page.selectedOid === want,
                    "card": driver.cardSettled,
                    "laid": landing < 0 || !!graphPane.view.itemAtIndex(landing) }))
                return
            if (!op.tipRested())
                return
            op.complete(() => {
                const at = graphModel.rowOf(want)
                const item = at >= 0 ? graphPane.view.itemAtIndex(at) : null
                return "nav_follow" + op.said()
                       + " followed=true"
                       + " landed=" + (!!item && item.oid_hex === want && item.selected)
                       // The panel stays: the click mark is on the open row, not the one the line named
                       // (デザイン規約「行き先はグラフ」).
                       + " marked=" + (sidebarPane.activeKey === sidebarPane.openKey)
                       + " box=" + (sidebarPane.editKey !== "")
                       + " open=" + navProbe.rowFactsOpen
                       + " to=" + op.to.key + " at=" + at + " name=" + op.rowName
            })
        }
    }
    // PGG_AUTO_ACT=nav-tip / nav-open / nav-open-then / nav-open-foot. Every run first opens a control row — the first
    // WORKTREES row, the one section every repository has — so an empty overlay still says the pointer arrived
    // somewhere. The hand leaves it on the beat it opened, before its supplement could show, so it raises nothing.
    PageStage {
        id: navTipTimer
        name: "nav_tip"
        property string kind: "branch"
        property int row: 0
        property bool head: false
        /// Whether the run reads the lines a row opens (PGG_AUTO_ACT=nav-open) instead of the shared tooltip, and
        /// what the hand does once the row is open (PGG_AUTO_ACT=nav-open-then).
        property bool opens: false
        property string then: ""
        /// The name rested on, the last time before the hand moved on.
        property string rested: ""
        /// A filter the current branch does not answer to, putting the stand-in on screen with no row anywhere
        /// (`HeadPinRow.seated`). The presets this argument runs on never overflow a section, so the half where the
        /// list scrolled out from under the row is `nav-pin-edge`'s.
        property string hide: ""
        /// The other way in: the BRANCHES folder row to click shut over the current branch's row
        /// (`NavProbe.clickRow`); -1 for none.
        property int fold: -1
        /// Whether the row rested on is the last of its section, the list taken to its end first
        /// (PGG_AUTO_ACT=nav-open-foot). Scrolled once — not re-applied under a hand that has arrived.
        property bool foot: false
        /// Where the list stood when the hand arrived — the run's own note, held against what closing gives back
        /// rather than the list's (`NavList.openRestY`).
        property real restY: 0
        /// Whether the control row was open as the hand left it.
        property bool lit: false
        function begin(arg) {
            const parts = ("" + arg).split(":")
            navTipTimer.head = parts[0] === "head"
            navTipTimer.kind = navTipTimer.head || parts[0] === "" ? "branch" : parts[0]
            // The filter takes the stand-in's own row out of the list, or brings a leaf under a folded folder into it
            // (a filtered row stands under its full name — `SidebarFilterRow`).
            navTipTimer.hide = navTipTimer.head ? (parts.length > 1 ? parts[1] : "")
                             : (parts.length > 2 ? parts[2] : "")
            // `head:<filter>:<row>` — the folder to shut over the current branch; the filter is then usually empty.
            navTipTimer.fold = navTipTimer.head && parts.length > 2 ? Number(parts[2]) : -1
            navTipTimer.row = !navTipTimer.head && parts.length > 1 ? Number(parts[1]) : 0
            // The control row first; then the fold or the scroll asked for; then, where one is asked, the hand's move.
            navTipTimer.order = ["lit"].concat(navTipTimer.fold >= 0 ? ["folded"] : [],
                                               navTipTimer.foot ? ["footed"] : [],
                                               navTipTimer.opens && navTipTimer.then !== "" ? ["acted"] : [])
            navTipTimer.start()
        }
        function target() {
            return navTipTimer.head ? branchesModel.headRow : navTipTimer.row
        }
        /// The list is the subject at the foot, so it has a line of its own: `shown=` is the row inside what the list
        /// shows, `back=` the list's position against the run's note from before the hand arrived.
        function footLine() {
            const at = navProbe.listContentY(navTipTimer.kind)
            return "nav_open_foot what=" + navTipTimer.then
                + " row=" + navTipTimer.target() + " name=" + navTipTimer.rested
                + " open=" + navProbe.rowFactsOpen
                + " shown=" + navProbe.rowFactsShown()
                + " back=" + (Math.round(at) === Math.round(navTipTimer.restY))
                // `bar`: closed at the grab itself, not by the run's ask after it (`NavProbe.afterOpen`).
                + (navTipTimer.then === "bar" ? " shut=" + navProbe.shutByBar : "")
                + " rest=" + Math.round(navTipTimer.restY) + " at=" + Math.round(at)
                + " seat=" + sidebarPane.rowFactsGeom()
        }
        function thenLine() {
            const target = navTipTimer.target()
            return "nav_open_then what=" + navTipTimer.then
                + " row=" + target + " name=" + navTipTimer.rested
                + " open=" + navProbe.rowFactsOpen
                + " lit=" + navProbe.rowWashLit(navTipTimer.kind, target)
                + " box=" + (sidebarPane.editKey !== "")
                + " menu=" + sidebarPane.menuOpen
                + " rows=" + navProbe.listOf(navTipTimer.kind).count
                // A drag takes words and the row hears nothing; a press that never moved is the row's
                // (`NavRowFacts.handClicked`).
                + " caret=" + navProbe.factsCaret()
                + " copied=" + (navProbe.factsTook() !== "")
                + " clicked=" + (sidebarPane.activeKey !== "")
                // A press in a line's band leaves the row the click mark too (`SidebarRowGestures.followLine`),
                // so the commit the graph went to tells the two apart, not the key.
                + " own=" + (page.selectedOid === navProbe.listOf(navTipTimer.kind).rowOidAt(target))
                // The two above as words — the repository's business, not the claim's.
                + " took=" + navProbe.factsTook()
                + " active=" + sidebarPane.activeKey
        }
        function tipLine() {
            const target = navTipTimer.target()
            const tip = Tips.box
            const words = navTipTimer.head ? navProbe.headPinWords : navProbe.tipWordsAt(navTipTimer.kind, target)
            return "nav_tip section=" + (navTipTimer.head ? "head" : navTipTimer.kind)
                + " row=" + target + " name=" + navTipTimer.rested
                + " lit=" + navTipTimer.lit + " wants=" + (words !== "")
                + " tip=" + tip.visible + " open=" + navProbe.rowFactsOpen
                // Where the stand-in rested once a fold took its row: `sat=` weighs its own `y` against the holding
                // row's seat, so other arithmetic shows; `at=` / `gap=` are the raw pair. Its words at rest are
                // `nav-pin-seat`'s — this run lights it.
                + (navTipTimer.fold >= 0
                   ? " under=" + navProbe.headPinUnder
                     + " sat=" + (Math.round(navProbe.headPinY) === Math.round(navProbe.headPinSeatY()))
                     + " at=" + Math.round(navProbe.headPinY)
                     + " gap=" + Math.round(navProbe.headPinSeatY())
                   : "")
                // Whether what opened is inside what the list shows (true wherever there was room under the row).
                + (navTipTimer.opens ? " shown=" + navProbe.rowFactsShown() : "")
                // Off its rows (`AutoActDriver.tipAside`); after the fold and the facts, which read as one run each.
                + " aside=" + driver.tipAside(tip)
                + " text=" + (tip.visible ? tip.text : "")
                // After a text that may carry anything, so the judged four above stay one substring.
                + (navTipTimer.opens ? " seat=" + sidebarPane.rowFactsGeom() : "")
                + (navTipTimer.opens ? " says=" + navProbe.rowFactsWords() : "")
                + (navTipTimer.head ? " pin=" + navProbe.headPinLit : "")
        }
        onTriggered: {
            const op = navTipTimer
            const tip = Tips.box
            if (op.at("start")) {
                // Re-applied every beat: the delegate arrives on a later layout than the rows the model got, and a
                // miss reads exactly like a row that answers a rest with nothing (`NavList.clickRow`).
                navProbe.pointTipAt("worktree", 0)
                if (!Awaited.all("nav_tip_control", { "open": navProbe.rowFactsOpen }))
                    return
                op.input("lit", () => {
                    op.lit = navProbe.rowFactsOpen
                    navProbe.pointTipAt("worktree", -1)
                    // The filter goes in only now: it would take the control row out along with the stand-in's.
                    if (op.hide !== "")
                        navProbe.typeFilter(op.hide)
                    if (op.head)
                        sidebarPane.headPinPointed = true
                    op.restY = navProbe.listContentY(op.kind)
                })
                return
            }
            // The fold, once the view has built its row: BRANCHES arrives on a read of its own, and a click on row 0
            // of a still-empty list folds nothing while answering as an already-folded row. A beat is given back.
            if (op.fold >= 0 && op.at("lit")) {
                op.input("folded", () => navProbe.clickRow("branch", op.fold))
                return
            }
            // The list to its end, once, before the hand rests; a beat is given back so it stands still under the
            // pointer.
            if (op.foot && op.at("lit")) {
                // BRANCHES may still be empty — the control's WORKTREES is a read of its own — and its last row would
                // be -1, which is also "the pointer is on no row".
                const list = navProbe.listOf("branch")
                if (!Awaited.all("nav_tip_foot", { "rows": !!list && list.count > 0 }))
                    return
                op.input("footed", () => {
                    navProbe.scrollBranchesToEnd()
                    op.row = list.count - 1
                    op.restY = navProbe.listContentY("branch")
                })
                return
            }
            const target = op.target()
            const acted = op.at("acted")
            // Nothing is waited on after the hand moves on: what opened is inside the row and goes with the pointer.
            if (!acted) {
                // Not once the hand has moved on: re-applying would put the pointer back on the row it walked off
                // (`nav-open-then away`).
                if (!op.head)
                    navProbe.pointTipAt(op.kind, target)
                // The stand-in has no tip, so what is read there is that the pointer is on it.
                const name = op.head ? branchesModel.headName : navProbe.tipNameAt(op.kind, target)
                const words = op.head ? navProbe.headPinWords : navProbe.tipWordsAt(op.kind, target)
                // An opening row is read on its own lines; elsewhere a row with words waits for the shared tip to be
                // up, one without for the control's tip to have left.
                if (!Awaited.all("nav_tip_rested", {
                        "row": name !== "",
                        "pin": !op.head || navProbe.headPinLit,
                        "open": !op.opens || navProbe.rowFactsOpen,
                        "tip": op.opens || (words !== "") === tip.visible }))
                    return
                // After the hand moved on, a filter may have taken the row out, so the name is the one read now.
                op.rested = name
                if (op.opens && op.then !== "") {
                    op.input("acted", () => {
                        navProbe.afterOpen(op.then, op.kind, target)
                    })
                    return
                }
            }
            // Nothing is read until the layout stops moving: the lines are measured on a layout and the section's
            // height follows that, so on the opening pass neither has arrived and the lines read as out of view. The
            // geometry standing across a swapped frame is the list standing still (`StageTimer.still`). Nor until the
            // supplement (a worktree's path) has answered either way: it comes up a rest after the lines, and read in
            // between the picture has it half out on one run and not at all on the next. And the page: a row the
            // hand pressed selects its commit, whose read is the page's.
            if (!Awaited.all("nav_tip_landed", {
                "laid": !op.opens || op.still("nav_tip_geom", sidebarPane.rowFactsGeom()),
                "said": !op.opens || acted || (navProbe.factsSays() !== "") === tip.visible,
                "page": PageSettled.settled(page) }))
                return
            if (op.foot)
                op.complete(() => op.footLine())
            else if (op.then !== "")
                op.complete(() => op.thenLine())
            else
                op.complete(() => op.tipLine())
        }
    }
    // PGG_AUTO_ACT=nav-open-tag: the lines a TAGS row opens under itself, one per remote carrying the name.
    //
    // The readings must be in first: nothing local records a remote's `refs/tags/` (`remote::tags::list_tags`), so a
    // row rested on before `ls-remote` answered looks like nobody else's tag. Waits on the answer itself, as
    // `AutoActRefVerbs.tagMenuTimer` does.
    PageStage {
        id: navOpenTagTimer
        name: "nav_open_tag"
        property string tag: ""
        /// What has to be known first: `remote` waits until some remote carries the name, `drift` until one carries it
        /// somewhere else. Empty is a name nobody out there has — an empty row looks the same before the reading, so
        /// that run waits on its own fetch being read through (`AutoActDriver.wroteAndSettled`).
        property string wants: ""
        /// Where the hand goes on to rest once the row is open: `line` = the first of the lines it opened — the first
        /// on purpose: searching for the one with a supplement would ask the answer where to find itself. Lines are in
        /// name order, so which it is is the preset's (`--preset tagremotes`: `fork`) — / `name` = the row's own name,
        /// which keeps why the copy here wears the warning / empty = nowhere. On a place with nothing to add the run
        /// hits the ceiling.
        property string rest: ""
        /// PGG_AUTO_ACT=tag-line-menu: the line the menu is asked for on, -1 for a run that only opens the row.
        property int menuLine: -1
        function begin(arg, menuLine) {
            const parts = ("" + arg).split(":")
            const last = parts[parts.length - 1]
            const rest = parts.length > 1 && (last === "tip" || last === "name") ? last : ""
            navOpenTagTimer.tag = parts[0]
            navOpenTagTimer.wants = parts.length > 1 && parts[1] !== rest ? parts[1] : ""
            navOpenTagTimer.rest = rest === "tip" ? "line" : rest
            navOpenTagTimer.menuLine = menuLine
            // The second rest where one is asked, or the menu on a line and its card.
            navOpenTagTimer.order = menuLine >= 0 ? ["rested", "card"]
                                  : navOpenTagTimer.rest !== "" ? ["rested"] : []
            // Waited on by its ask's id (`rename-tag-remote` の同じ待ち).
            driver.pressWrite("fetch", () => {
                repoTab.fetch("")
                return true
            })
            navOpenTagTimer.start()
        }
        /// Whether what this run waits on has arrived, asked of the lookups the lines are drawn from
        /// (`NavSectionModel`) — no count of fetches says whether the readings are in.
        function ready() {
            if (navOpenTagTimer.wants === "drift")
                return tagsModel.remoteTagDrift(navOpenTagTimer.tag, repoTab.defaultRemote) !== ""
            if (navOpenTagTimer.wants === "remote") {
                const sides = tagsModel.tagSides(navOpenTagTimer.tag)
                return sides === "remote" || sides === "both"
            }
            return driver.wroteAndSettled()
        }
        onTriggered: {
            const op = navOpenTagTimer
            const idle = !(repoTab.busyCount !== 0)
            const read = idle && op.ready()
            // Asked and re-applied every beat: the fetch rebuilds the list under the hand (`nav-open` の同じ歩き).
            const row = read ? tagsModel.rowOfName(op.tag) : -1
            if (row >= 0)
                navProbe.pointTipAt("tag", row)
            const open = row >= 0 && navProbe.rowFactsOpen
            // Nothing is read until the layout stops moving (`nav-open` の同じ待ち).
            if (!Awaited.all("nav_open_tag_ready", {
                    "ready": idle,
                    "read": read,
                    "row": row >= 0,
                    "open": open,
                    "laid": open && op.still("nav_open_tag_geom", sidebarPane.rowFactsGeom()) }))
                return
            // Or the menu on one line, asked once — its answer is the card, not the line.
            if (op.menuLine >= 0) {
                if (op.at("start")) {
                    op.input("rested", () => navProbe.factsLineMenu(op.menuLine))
                    return
                }
                if (!Awaited.all("nav_open_tag_menu", { "menu": refMenu.opened }))
                    return
                op.input("card", () => {
                    refMenu.openSub(refTagCard)
                })
                op.complete(() => driver.tagReachWords(refTagCard))
                return
            }
            // Then the second rest, on the first line or the name, asked again until it takes: the lines are built a
            // pass after the row that opened them.
            if (op.rest !== "") {
                if (op.at("start")) {
                    op.input("rested", () => op.rest === "name" ? navProbe.pointFactsName(true)
                                                                 : navProbe.pointFactsLine(0, true))
                    return
                }
                if (!Awaited.all("nav_open_tag_tip", { "tip": Tips.box.visible }))
                    return
            }
            // `sides=` (what the row's cloud is drawn from) and `says=` (the carriers the lines name) are the claim as
            // a pair: a cloud over no lines and lines naming an unheard-of remote frame alike.
            op.complete(() => {
                const tip = Tips.box
                return "nav_open_tag tag=" + op.tag
                    + " row=" + tagsModel.rowOfName(op.tag)
                    + " sides=" + tagsModel.tagSides(op.tag)
                    + " open=" + navProbe.rowFactsOpen
                    + " shown=" + navProbe.rowFactsShown()
                    + " seat=" + sidebarPane.rowFactsGeom()
                    + " says=" + navProbe.rowFactsWords()
                    // The line's supplement, read off the shared instance — last, after a sentence with free fields.
                    + " tip=" + tip.visible + " text=" + (tip.visible ? tip.text : "")
            })
        }
    }
    // PGG_AUTO_ACT=nav-branch-box / nav-rename-box / nav-tag-box: how wide the box comes out on a row of that depth in
    // a pane of that width — one question, so one timer for all of them.
    PageStage {
        id: navNameBoxTimer
        name: "nav_name_box"
        property string mode: "branch"
        property string kind: "branch"
        property string ref: ""
        /// The width the splitter settles at: the ask, held to the pane's floor.
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
                remotesModel.toggleFolder(
                    GitFacts.remoteOfRef(navNameBoxTimer.ref, repoTab.remoteNames))
            navNameBoxTimer.order = navNameBoxTimer.away ? ["opened", "walked"] : ["opened"]
            navNameBoxTimer.start()
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
            const key = op.kind + ":" + op.ref
            const list = navProbe.listOf(op.kind)
            if (op.at("opened")) {
                // On the row and built: a row not laid out yet answers 0, like a row with no box. `focused=` is
                // reported since a box that cannot be typed into looks the same in the picture.
                if (!Awaited.all("nav_name_box_drawn", { "open": sidebarPane.editKey === key,
                                                         "drawn": list.rowBoxWidth(row) > 0 }))
                    return
                if (op.away) {
                    // Just out of the view from where the row stands, near enough that the view still holds its
                    // delegate — otherwise what went is the row and not the box. A list too short to scroll the row
                    // out is the fixture's shortfall, named here instead of waited out.
                    if (!op.input("walked", () => list.scrollRowOut(row)))
                        Awaited.note("nav_name_box_away waiting=room")
                    return
                }
            } else if (!Awaited.all("nav_name_box_gone", { "gone": !list.rowBoxShown(row) })) {
                // Walked past: wait for the box to go, read off `shown` — a released delegate answers 0 for everything.
                return
            }
            op.complete(() => {
                const at = op.section.rowOfName(op.ref)
                const view = navProbe.listOf(op.kind)
                const drawn = view.rowBoxWidth(at)
                const whole = view.rowBoxWhole(at)
                return "nav_name_box mode=" + op.mode
                       + " ref=" + op.ref
                       + " row=" + at
                       + " pane=" + Math.round(sidebarPane.width)
                       // The room the row holds, the drawn width, and what reading it whole takes: whether it
                       // needed room outside the pane and got it.
                       + " seat=" + Math.round(view.rowBoxSeat(at))
                       + " box=" + Math.round(drawn)
                       + " whole=" + Math.ceil(whole)
                       // Cut, a placeholder no longer says what goes in and a name cannot be read back
                       // (デザイン規約 §グラフ行のダブルクリック).
                       + " cut=" + (Math.ceil(whole) > Math.round(drawn))
                       + " open=" + (sidebarPane.editKey === op.kind + ":" + op.ref)
                       + " focused=" + view.rowFocused(at)
                       + " shown=" + view.rowBoxShown(at)
                       + " at=" + view.rowBoxAt(at)
            })
        }
    }
    // PGG_AUTO_ACT=rename-box-out: one way out of the name box per run. The claim is what the box and the gesture are
    // left holding: a wait still running is how a box comes back by itself, so `armed=false` says it will not.
    PageStage {
        id: boxOutTimer
        name: "box_out"
        order: ["first_click", "second_click", "out"]
        /// Which way out this run takes: `same-row` / `other-row` / `escape` / `away` / `typed-away`.
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
                if (!Awaited.all("box_out_left", { "page": PageSettled.settled(page) }))
                    return
                op.complete(() => "rename_box_out route=" + op.route
                                  + " box=" + (graphPane.namingOid !== "")
                                  + " armed=" + graphPane.view.renameWaiting
                                  + " guarded=" + graphPane.view.clickGuarded
                                  + " typed=" + graphPane.namingText)
            }
        }
    }
}

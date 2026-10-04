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
    readonly property var workTree: driver.workTree
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
    readonly property var renderedBarrier: driver.barrierRendered
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
            sidebarPane.beginRename("branch", workTree.branch,
                                    workTree.branch)
            if (arg === "fold")
                page.foldByHand(true)
            else
                page.releasePressedAway(null)
            Harness.report("nav_drop how=" + arg
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
            dragOpenTimer.acted = false
            dragOpenTimer.start()
        } else if (act === "nav-open-tip") {
            // The rest that opens a row, held on: the row opens, and a rest later the supplement it keeps (a working
            // copy's path) follows from the same unmoved hand (デザイン規約 §左メニューの所作).
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
            // opens a menu elsewhere, `filter` types a filter that takes the row out, and `rightclick` / `sweep` /
            // `tap` act on the open lines themselves.
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
            navOpenTagTimer.begin(arg)
        } else if (act === "tag-line-menu") {
            // `<tag>:<line>`: the same row opened, then the menu asked for on that line of it — a carrier's line names
            // its remote on its own, so the TAG card acts on it (`NavRowFacts.lineMenu`).
            const at = arg.lastIndexOf(":")
            navOpenTagTimer.begin(arg.slice(0, at))
            navOpenTagTimer.menuLine = Number(arg.slice(at + 1))
        } else if (act === "nav-rename" || act === "rename-branch"
                   || act === "rename-tag" || act === "rename-stash") {
            const kind = act === "rename-tag" ? "tag" : act === "rename-stash" ? "stash" : "branch"
            const id = kind === "branch" ? workTree.branch
                     : kind === "tag" ? tagsModel.nameAt(0) : stashesModel.fullAt(0)
            const shown = kind === "stash" ? stashesModel.nameAt(0) : id
            sidebarPane.beginRename(kind, id, shown)
            if (act !== "nav-rename")
                sidebarPane.submitEdit(arg)
        } else if (act === "rename-taken") {
            // A name git refuses, submitted for real: the box stays open holding it, with git's words under it and in
            // the bar (デザイン規約 §答えの要らない報せ).
            sidebarPane.beginRename("branch", workTree.branch, arg)
            sidebarPane.submitEdit(arg)
            renameTakenTimer.start()
        } else if (act === "rename-tag-box") {
            // The box opened with the argument typed in, so a name the box itself refuses can be photographed
            // (デザイン規約 §答えの要らない報せ「押す前に断れるものは、名前の箱で断る」). `was=` names the row, so a
            // run that opened the box on the wrong one says so.
            tagNameBoxTimer.begin(arg)
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
            if (act === "replace-remote-box") {
                renderedBarrier.begin()
                // A known verb answers true even on its early way out — falsy would send the dispatch on asking
                // every other family.
                return true
            }
            sidebarPane.submitEdit(parts[1])
            if (act === "replace-remote-go") {
                driver.holdToEnd(graphPane)
            } else {
                replaceAskTimer.points = act === "replace-remote-tip"
                replaceAskTimer.pointed = false
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
    // Reported off the box's own state: a closed box and one left open with a warning frame are two pixels apart.
    SampleTimer {
        id: renameTakenTimer
        onTriggered: {
            // And the bar settled: its height animates in after the words, so a shot on the refusal alone catches it
            // still on its way down (`NoticeBar.settled`).
            if (repoTab.busyCount !== 0 || !sidebarPane.editRefused || !page.noticeCard.settled)
                return
            renameTakenTimer.stop()
            Harness.report("rename_taken open=" + (sidebarPane.editKey !== "")
                              + " refused=" + sidebarPane.editRefused
                              + " bar=" + page.noticeCard.open
                              + " tone=" + page.noticeCard.tone
                              + " why=" + sidebarPane.editRefusedWhy)
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=rename-tag-box: the box, and whether the reason for refusing the name reached the reader.
    //
    // `editKey` is only the ask: the box is built by a `Loader` in a row not yet laid out, so `box=` reads the box as
    // drawn and holding the keyboard (`NavList.rowBoxShown` / `rowFocused`). `refused=` alone passes a box refusing in
    // silence, and Qt can drop the tooltip the box's binding raises (`NavNameBox`) as a binding loop without a word,
    // so `tip=` is taken through the box (`NavList.rowTipShown`, whose tip it is — `tests/qml/tst_tipowner.qml`).
    SampleTimer {
        id: tagNameBoxTimer
        property string typed: ""
        function begin(arg) {
            tagNameBoxTimer.typed = "" + arg
            sidebarPane.beginRename("tag", tagsModel.nameAt(0), tagNameBoxTimer.typed)
            tagNameBoxTimer.start()
        }
        onTriggered: {
            // Asked every beat: a binding on a call would hold the empty model's answer
            // (rules/app-ui.md「QML バインディングはプロパティにしか反応しない」).
            const was = tagsModel.nameAt(0)
            const row = tagsModel.rowOfName(was)
            const list = navProbe.listOf("tag")
            const open = row >= 0 && list.rowBoxShown(row) && list.rowFocused(row)
            if (!open)
                return
            // Only a refused name raises a tip to wait for.
            const tipped = list.rowTipShown(row)
            if (sidebarPane.editRefused && !tipped)
                return
            tagNameBoxTimer.stop()
            Harness.report("tag_name_box was=" + was
                              + " typed=" + tagNameBoxTimer.typed
                              + " box=" + open
                              + " refused=" + sidebarPane.editRefused
                              + " tip=" + tipped
                              + " text=" + page.ToolTip.toolTip.text)
            renderedBarrier.begin()
        }
    }
    // PGG_AUTO_ACT=nav-rename-far. Each step waits for the one before to land — a list still building has no height to
    // scroll by.
    property int farStep: 0
    SampleTimer {
        id: farTimer
        onTriggered: {
            const section = sidebarPane.peekSection
            if (acts.farStep === 0) {
                if (sidebarPane.peekKind === "") {
                    navProbe.peekAt("tag")
                    return
                }
                section.scrollToEnd()
                acts.farStep = 1
            } else if (acts.farStep === 1) {
                if (section.rowInView(0))
                    return
                sidebarPane.beginRename("tag", tagsModel.nameAt(0),
                                        tagsModel.nameAt(0))
                acts.farStep = 2
            } else if (acts.farStep === 2) {
                if (sidebarPane.editKey === "")
                    return
                farTimer.stop()
                Harness.report(
                "nav_far row=" + tagsModel.nameAt(0)
                + " shown=" + section.rowInView(0)
                + " box=" + (sidebarPane.editKey !== "")
                + " collapsed=" + page.sidebarCollapsed
                + " editing=" + sidebarPane.editKey)
                driver.complete()
            }
        }
    }
    // The replace's question, waited on until settled: a bar photographed before its words arrive is a red line with
    // nothing on it. The plain verb ends here — the write is "-go"'s half.
    SampleTimer {
        id: replaceAskTimer
        /// A hand goes on the pill once the bar has settled (`replace-remote-tip`): the tip places itself against the
        /// pill as it opens, so a pill still moving would leave it behind.
        property bool points: false
        property bool pointed: false
        onTriggered: {
            if (!graphPane.askCard.settled)
                return
            if (replaceAskTimer.points && !graphPane.askCard.tipStanding) {
                graphPane.askCard.pointedAt = true
                if (!replaceAskTimer.pointed) {
                    replaceAskTimer.pointed = true
                    // `pill=` is the word the hand is on, `words=` whether the bar has a tip to open at all.
                    Harness.report("replace_ask step=pointed pill=" + graphPane.askCard.accept
                                   + " words=" + (graphPane.askCard.tip !== ""))
                }
                return
            }
            replaceAskTimer.stop()
            // `code=` empty is part of the claim: a push plus a delete is no one command, so the pill is in the
            // ordinary voice (規約 §git 用語のコード表記「どれもコマンドと 1 対 1 でない」).
            Harness.report("replace_ask hold=" + graphPane.askHold
                              + " code=" + graphPane.askCode
                              + (replaceAskTimer.points ? " tip=" + graphPane.askCard.tipStanding : ""))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=rename-tag-remote / -go / -tip: the question that carries a tag's new name over to the remote
    // holding the old one.
    //
    // The rename waits for this run's fetch (`nav-open-tag` の同じ待ち): renamed before `ls-remote` answered, it arms
    // nothing and the bar never comes. The fetch is waited on by its ask's id (`AutoActDriver.pressWrite`): its start
    // and end can be drained together, so a beat watching the busy count rise can miss it altogether.
    SampleTimer {
        id: tagRemoteRenameTimer
        property string tag: ""
        property string name: ""
        /// Whether the pill is held to the end, or the run stops at the question standing.
        property bool holds: false
        /// Whether the rename has gone in: before it the run waits on the reading, after it on the bar.
        property bool sent: false
        /// Which of the three answers the run picks, empty for the bar as it comes down.
        property string choice: ""
        property bool picked: false
        /// Whether the line has gone out — see the head of `onTriggered`.
        property bool said: false
        /// Whether a hand goes on the pill once the pick has dressed the bar (`rename-tag-remote-tip`).
        property bool points: false
        property bool pointed: false
        function begin(arg, holds, points) {
            const parts = ("" + arg).split(":")
            tagRemoteRenameTimer.tag = parts[0]
            tagRemoteRenameTimer.name = parts.length > 1 ? parts[1] : ""
            tagRemoteRenameTimer.choice = points ? "replace"
                : parts.length > 2 ? parts[2] : (holds ? "replace" : "")
            tagRemoteRenameTimer.holds = holds
            tagRemoteRenameTimer.points = points
            tagRemoteRenameTimer.pointed = false
            tagRemoteRenameTimer.sent = false
            tagRemoteRenameTimer.picked = false
            tagRemoteRenameTimer.said = false
            driver.pressWrite("fetch", () => {
                repoTab.fetch("")
                return true
            })
            tagRemoteRenameTimer.start()
        }
        onTriggered: {
            // Past the line only `leave` is still running: it writes nothing, so the bar going back up is its end.
            if (tagRemoteRenameTimer.said) {
                if (!graphPane.askCard.shut)
                    return
                tagRemoteRenameTimer.stop()
                driver.complete()
                return
            }
            if (repoTab.busyCount !== 0)
                return
            if (!tagRemoteRenameTimer.sent) {
                // The arming needs both sides holding the name (`RepoPage.armRenameTagRemote`), read once this run's
                // fetch is through.
                if (!driver.wroteAndSettled()
                    || tagsModel.tagSides(tagRemoteRenameTimer.tag) !== "both")
                    return
                tagRemoteRenameTimer.sent = true
                Harness.report("rename_carry step=renamed")
                // Through the row's own road, so a build where the gesture stopped reaching the rename waits here.
                sidebarPane.beginRename("tag", tagRemoteRenameTimer.tag, tagRemoteRenameTimer.tag)
                sidebarPane.submitEdit(tagRemoteRenameTimer.name)
                return
            }
            if (!graphPane.askCard.settled)
                return
            // And the new name's row, which can land after the bar: the write's answer raises the question before the
            // read that re-lists the tags (rules-refs/app-ui.md「その書き込みが無効化した読み直しより先に来る」).
            if (tagsModel.rowOfName(tagRemoteRenameTimer.name) < 0)
                return
            if (tagRemoteRenameTimer.choice !== "" && !tagRemoteRenameTimer.picked) {
                // Through the field's own door, since no injected click opens a popup offscreen
                // (`RenameCarryFlow.pickChoice`); re-applied until it takes, the form's `Loader` building a frame
                // behind the bar.
                if (!page.pickCarryChoice(driver.carryChoiceIndex(tagRemoteRenameTimer.choice)))
                    return
                tagRemoteRenameTimer.picked = true
                Harness.report("rename_carry step=picked")
                return
            }
            // A beat after the pick, so the tip places itself against the pill wearing the picked answer's word.
            if (tagRemoteRenameTimer.points && !graphPane.askCard.tipStanding) {
                graphPane.askCard.pointedAt = true
                if (!tagRemoteRenameTimer.pointed) {
                    tagRemoteRenameTimer.pointed = true
                    // `pill=` / `words=` as in `replaceAskTimer`.
                    Harness.report("rename_carry step=pointed pill=" + graphPane.askCard.accept
                                   + " words=" + (graphPane.askCard.tip !== ""))
                }
                return
            }
            tagRemoteRenameTimer.said = true
            // `here=` is the wait above said out loud; `mark=` weighs the commit the bar marked (taken before the
            // write) against the one the tags section came back with for the new name.
            Harness.report(driver.carryWords("tag", tagRemoteRenameTimer.choice)
                              + " here=" + (tagsModel.rowOfName(tagRemoteRenameTimer.name) >= 0)
                              + " mark=" + (graphPane.view.askOid !== ""
                                  && graphPane.view.askOid
                                     === tagsModel.oidOfName(tagRemoteRenameTimer.name))
                              + (tagRemoteRenameTimer.points ? " tip=" + graphPane.askCard.tipStanding : ""))
            if (!tagRemoteRenameTimer.holds) {
                tagRemoteRenameTimer.stop()
                driver.complete()
                return
            }
            // The barrier is this timer's to start: the dispatch hands none to a verb that defers its completion
            // (`AutoActDriver.dispatchFinished`). Only the answer that takes a name off the remote asks for a hold.
            if (graphPane.askHold) {
                driver.holdToEnd(graphPane)
            } else if (tagRemoteRenameTimer.choice === "leave") {
                // Writes nothing: see the head of `onTriggered`.
                page.answerRowAsk()
                return
            } else {
                driver.pressWrite("answer-ask", () => { page.answerRowAsk(); return true })
            }
            tagRemoteRenameTimer.stop()
            writeBarrier.start()
        }
    }
    // No rest first: the gesture skips it, and a run that rested on the row would be reading the other path.
    SampleTimer {
        id: dragOpenTimer
        property string kind: "branch"
        property int row: 2
        /// A filter typed before the hand arrives, for a row that is folded away without one.
        property string held: ""
        property bool acted: false
        property string name: ""
        onTriggered: {
            if (!dragOpenTimer.acted) {
                if (dragOpenTimer.held !== "" && navProbe.typeFilter(dragOpenTimer.held) !== dragOpenTimer.held)
                    return
                // Re-tried until the row is there to press: a miss reads like a row that refused to open
                // (`nav-open` の同じ歩き).
                dragOpenTimer.name = navProbe.tipNameAt(dragOpenTimer.kind, dragOpenTimer.row)
                if (dragOpenTimer.name === "" || !navProbe.dragRow(dragOpenTimer.kind, dragOpenTimer.row))
                    return
                dragOpenTimer.acted = true
                return
            }
            dragOpenTimer.stop()
            const took = navProbe.rowNameTook(dragOpenTimer.kind, dragOpenTimer.row)
            Harness.report("nav_drag_open section=" + dragOpenTimer.kind
                + " row=" + dragOpenTimer.row + " name=" + dragOpenTimer.name
                + " open=" + navProbe.rowFactsOpen
                + " caret=" + navProbe.rowNameCaret(dragOpenTimer.kind, dragOpenTimer.row)
                + " copied=" + (took !== "")
                // A drag that took the name is not a click — a second click on the row opens a name box
                // (`NavItemDelegate.lineClicked`).
                + " clicked=" + (sidebarPane.activeKey !== "")
                + " took=" + took)
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-open-tip: the working copy's path the open row keeps for the hand that opened it. The picture
    // shows the words; `says=` is the same path read off the row, so a tip drawn from anywhere else fails to match.
    SampleTimer {
        id: openTipTimer
        property string kind: "worktree"
        property int row: 3
        onTriggered: {
            const tip = page.ToolTip.toolTip
            // Re-applied every beat (`nav-open` の同じ歩き).
            navProbe.pointTipAt(openTipTimer.kind, openTipTimer.row)
            if (!navProbe.rowFactsOpen)
                return
            if (!tip.visible)
                return
            openTipTimer.stop()
            Harness.report("nav_open_tip section=" + openTipTimer.kind
                + " row=" + openTipTimer.row
                + " open=" + navProbe.rowFactsOpen
                + " tip=" + tip.visible
                // An empty tip answers `tip=true` as well as a full one (`NavRowFacts.said`); weighed in the run,
                // since the path is this machine's own.
                + " same=" + (tip.text === navProbe.factsSays())
                // Last, since a path carries anything.
                + " says=" + navProbe.rowFactsWords()
                + " text=" + tip.text)
            driver.complete()
        }
    }
    // The row still has to be the one under the hand — a run that never reached it would answer the same way.
    SampleTimer {
        id: openHeldTimer
        onTriggered: {
            // Read in the same beat: the stand-in opens outright where a hand would sit out a rest, so a row that was
            // going to open is open already.
            navProbe.pointTipAt("branch", 2)
            const name = navProbe.tipNameAt("branch", 2)
            if (name === "" || sidebarPane.editKey === "")
                return
            openHeldTimer.stop()
            Harness.report("nav_open_held row=2 name=" + name
                + " box=true open=" + navProbe.rowFactsOpen
                + " lit=" + navProbe.rowWashLit("branch", 2))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-follow / nav-follow-lit. The landing is read off the graph, not the press: the page's selection
    // is only the click's bookkeeping, so `landed=` is the graph's row for that commit, lit and laid out.
    SampleTimer {
        id: followTimer
        property string kind: "remote"
        property int row: 0
        property string filter: ""
        property bool lit: false
        /// 0 the filter, 1 the rest that opens the row, 2 the lines standing still, 3 the answer.
        property int step: 0
        property string stood: ""
        property int line: -1
        property var to: null
        property string name: ""
        function begin(arg, lit) {
            const parts = ("" + arg).split(":")
            followTimer.kind = parts[0] === "" ? "remote" : parts[0]
            followTimer.row = parts.length > 1 ? Number(parts[1]) : 0
            followTimer.filter = parts.length > 2 ? parts[2] : ""
            followTimer.lit = lit
            followTimer.step = followTimer.filter === "" ? 1 : 0
            followTimer.stood = ""
            followTimer.line = -1
            followTimer.to = null
            followTimer.start()
        }
        function said() {
            return " section=" + followTimer.kind + " row=" + followTimer.row + " line=" + followTimer.line
        }
        onTriggered: {
            // A filter, once: the way a leaf under a folded folder is brought into the list.
            if (followTimer.step === 0) {
                navProbe.typeFilter(followTimer.filter)
                followTimer.step = 1
                return
            }
            // The rest, every beat until the row is open (`nav-open` の同じ歩き).
            if (followTimer.step === 1) {
                navProbe.pointTipAt(followTimer.kind, followTimer.row)
                if (!navProbe.rowFactsOpen)
                    return
                followTimer.name = navProbe.tipNameAt(followTimer.kind, followTimer.row)
                followTimer.step = 2
                return
            }
            // Nothing is aimed until the lines stop moving: laid out a pass after they are built, a press worked out
            // from a line still at its origin lands on the row's mark.
            if (followTimer.step === 2) {
                const geom = sidebarPane.rowFactsGeom()
                if (geom !== followTimer.stood) {
                    followTimer.stood = geom
                    return
                }
                followTimer.line = navProbe.factsFirstGoing()
                followTimer.to = navProbe.factsGoesTo(followTimer.line)
                if (followTimer.to === null) {
                    // Lines that go nowhere are this row's answer — waiting on would spend the ceiling and take no
                    // picture.
                    followTimer.stop()
                    Harness.report((followTimer.lit ? "nav_follow_lit" : "nav_follow") + followTimer.said()
                                      + " followed=false name=" + followTimer.name)
                    driver.complete()
                    return
                }
                if (followTimer.lit)
                    navProbe.pointFactsWords(followTimer.line, true)
                else
                    navProbe.followFactsLine(followTimer.line)
                followTimer.step = 3
                return
            }
            if (followTimer.lit) {
                followTimer.stop()
                // `aimed=` is the band as drawn, `hand=` the cursor the area took — two parts, so either alone splits.
                Harness.report("nav_follow_lit" + followTimer.said()
                                  + " aimed=" + navProbe.factsLineAimed(followTimer.line)
                                  + " hand=" + navProbe.factsHand()
                                  + " open=" + navProbe.rowFactsOpen
                                  + " to=" + followTimer.to.key + " name=" + followTimer.name)
                driver.complete()
                return
            }
            const want = followTimer.to.oid
            if (page.selectedOid !== want || !driver.cardSettled)
                return
            // A commit the walk never reached is an answer; a row the model has but the view has not laid out is
            // waited for.
            const at = graphModel.rowOf(want)
            const item = at >= 0 ? graphPane.view.itemAtIndex(at) : null
            if (at >= 0 && !item)
                return
            followTimer.stop()
            Harness.report("nav_follow" + followTimer.said()
                              + " followed=true"
                              + " landed=" + (!!item && item.oid_hex === want && item.selected)
                              // The panel stays: the click mark is on the open row, not the one the line named
                              // (デザイン規約「行き先はグラフ」).
                              + " marked=" + (sidebarPane.activeKey === sidebarPane.openKey)
                              + " box=" + (sidebarPane.editKey !== "")
                              + " open=" + navProbe.rowFactsOpen
                              + " to=" + followTimer.to.key + " at=" + at + " name=" + followTimer.name)
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-tip / nav-open / nav-open-then / nav-open-foot. Every run first opens a control row — the first
    // WORKTREES row, the one section every repository has — so an empty overlay still says the pointer arrived
    // somewhere. The hand leaves it on the beat it opened, before its supplement could show, so it raises nothing.
    SampleTimer {
        id: navTipTimer
        property string kind: "branch"
        property int row: 0
        property bool head: false
        /// Whether the run reads the lines a row opens (PGG_AUTO_ACT=nav-open) instead of the shared tooltip, and
        /// what the hand does once the row is open (PGG_AUTO_ACT=nav-open-then).
        property bool opens: false
        property string then: ""
        /// Whether that next thing has been done (once: the beat after it is the answer), and the name rested on then.
        property bool acted: false
        property string rested: ""
        /// A filter the current branch does not answer to, putting the stand-in on screen with no row anywhere
        /// (`HeadPinRow.seated`). The presets this argument runs on never overflow a section, so the half where the
        /// list scrolled out from under the row is `nav-pin-edge`'s.
        property string hide: ""
        /// The other way in: the BRANCHES folder row to click shut over the current branch's row
        /// (`NavProbe.clickRow`); -1 for none.
        property int fold: -1
        property bool folded: false
        /// Whether the row rested on is the last of its section, the list taken to its end first
        /// (PGG_AUTO_ACT=nav-open-foot). Scrolled once — not re-applied under a hand that has arrived.
        property bool foot: false
        property bool footDone: false
        /// Where the list stood when the hand arrived — the run's own note, held against what closing gives back
        /// rather than the list's (`NavList.openRestY`).
        property real restY: 0
        /// The geometry the last beat read, so a beat that reads the same one knows the layout has come to rest.
        property string stood: ""
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
            navTipTimer.lit = false
            navTipTimer.folded = false
            navTipTimer.acted = false
            navTipTimer.footDone = false
            navTipTimer.stood = ""
            navTipTimer.start()
        }
        onTriggered: {
            const tip = page.ToolTip.toolTip
            if (!navTipTimer.lit) {
                // Re-applied every beat: the delegate arrives on a later layout than the rows the model got, and a
                // miss reads exactly like a row that answers a rest with nothing (`NavList.clickRow`).
                navProbe.pointTipAt("worktree", 0)
                if (!navProbe.rowFactsOpen)
                    return
                navTipTimer.lit = true
                navProbe.pointTipAt("worktree", -1)
                // The filter goes in only now: it would take the control row out along with the stand-in's.
                if (navTipTimer.hide !== "")
                    navProbe.typeFilter(navTipTimer.hide)
                if (navTipTimer.head)
                    sidebarPane.headPinPointed = true
                navTipTimer.restY = navProbe.listContentY(navTipTimer.kind)
                return
            }
            // The fold, once the view has built its row: BRANCHES arrives on a read of its own, and a click on row 0
            // of a still-empty list folds nothing while answering as an already-folded row. A beat is given back.
            if (navTipTimer.fold >= 0 && !navTipTimer.folded) {
                if (!navProbe.clickRow("branch", navTipTimer.fold))
                    return
                navTipTimer.folded = true
                return
            }
            // The list to its end, once, before the hand rests; a beat is given back so it stands still under the
            // pointer.
            if (navTipTimer.foot && !navTipTimer.footDone) {
                // BRANCHES may still be empty — the control's WORKTREES is a read of its own — and its last row would
                // be -1, which is also "the pointer is on no row".
                const list = navProbe.listOf("branch")
                if (!list || list.count <= 0)
                    return
                navProbe.scrollBranchesToEnd()
                navTipTimer.row = list.count - 1
                navTipTimer.footDone = true
                navTipTimer.restY = navProbe.listContentY("branch")
                return
            }
            const target = navTipTimer.head ? branchesModel.headRow : navTipTimer.row
            // Not once the hand has moved on: re-applying would put the pointer back on the row it walked off
            // (`nav-open-then away`).
            if (!navTipTimer.head && !navTipTimer.acted)
                navProbe.pointTipAt(navTipTimer.kind, target)
            // The stand-in has no tip, so what is read there is that the pointer is on it. After the hand moved on,
            // a filter may have taken the row out, so the name is the one read then.
            const name = navTipTimer.acted ? navTipTimer.rested
                       : navTipTimer.head ? branchesModel.headName
                       : navProbe.tipNameAt(navTipTimer.kind, target)
            if (name === "" || (!navTipTimer.acted && navTipTimer.head && !navProbe.headPinLit))
                return
            navTipTimer.rested = name
            const words = navTipTimer.head ? navProbe.headPinWords
                        : navProbe.tipWordsAt(navTipTimer.kind, target)
            // An opening row is read on its own lines; elsewhere a row with words waits for the shared tip to be up,
            // one without for the control's tip to have left. Nothing is waited on after the hand moves on: what
            // opened is inside the row and goes with the pointer, so the tick after the order is the answer.
            if (!navTipTimer.acted) {
                if (navTipTimer.opens) {
                    if (!navProbe.rowFactsOpen)
                        return
                    if (navTipTimer.then !== "") {
                        navTipTimer.acted = true
                        navProbe.afterOpen(navTipTimer.then, navTipTimer.kind, target)
                        return
                    }
                } else if ((words !== "") !== tip.visible) {
                    return
                }
            }
            // Nothing is read until the layout stops moving: the lines are measured on a layout and the section's
            // height follows that, so on the opening pass neither has arrived and the lines read as out of view. Two
            // beats with the same geometry is the list standing still.
            if (navTipTimer.opens) {
                const geom = sidebarPane.rowFactsGeom()
                if (geom !== navTipTimer.stood) {
                    navTipTimer.stood = geom
                    return
                }
                // Nor until the supplement (a working copy's path) has answered either way: it comes up a rest after
                // the lines, and read in between the picture has it half out on one run and not at all on the next.
                if (!navTipTimer.acted && (navProbe.factsSays() !== "") !== tip.visible)
                    return
            }
            navTipTimer.stop()
            // The list is the subject at the foot, so it has a line of its own: `shown=` is the row inside what the
            // list shows, `back=` the list's position against the run's note from before the hand arrived.
            if (navTipTimer.foot) {
                const at = navProbe.listContentY(navTipTimer.kind)
                Harness.report("nav_open_foot what=" + navTipTimer.then
                    + " row=" + target + " name=" + name
                    + " open=" + navProbe.rowFactsOpen
                    + " shown=" + navProbe.rowFactsShown()
                    + " back=" + (Math.round(at) === Math.round(navTipTimer.restY))
                    // `bar`: closed at the grab itself, not by the run's ask after it (`NavProbe.afterOpen`).
                    + (navTipTimer.then === "bar" ? " shut=" + navProbe.shutByBar : "")
                    + " rest=" + Math.round(navTipTimer.restY) + " at=" + Math.round(at)
                    + " seat=" + sidebarPane.rowFactsGeom())
                driver.complete()
                return
            }
            if (navTipTimer.then !== "") {
                Harness.report("nav_open_then what=" + navTipTimer.then
                    + " row=" + target + " name=" + name
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
                    + " active=" + sidebarPane.activeKey)
                driver.complete()
                return
            }
            Harness.report("nav_tip section=" + (navTipTimer.head ? "head" : navTipTimer.kind)
                + " row=" + target + " name=" + name
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
                + (navTipTimer.head ? " pin=" + navProbe.headPinLit : ""))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-open-tag: the lines a TAGS row opens under itself, one per remote carrying the name.
    //
    // The readings must be in first: nothing local records a remote's `refs/tags/` (`remote::tags::list_tags`), so a
    // row rested on before `ls-remote` answered looks like nobody else's tag. Waits on the answer itself, as
    // `AutoActRefVerbs.tagMenuTimer` does.
    SampleTimer {
        id: navOpenTagTimer
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
        property bool rested: false
        /// PGG_AUTO_ACT=tag-line-menu: the line the menu is asked for on, -1 for a run that only opens the row.
        property int menuLine: -1
        /// As `navTipTimer.stood`.
        property string stood: ""
        function begin(arg) {
            const parts = ("" + arg).split(":")
            const last = parts[parts.length - 1]
            const rest = parts.length > 1 && (last === "tip" || last === "name") ? last : ""
            navOpenTagTimer.tag = parts[0]
            navOpenTagTimer.wants = parts.length > 1 && parts[1] !== rest ? parts[1] : ""
            navOpenTagTimer.rest = rest === "tip" ? "line" : rest
            navOpenTagTimer.stood = ""
            navOpenTagTimer.rested = false
            navOpenTagTimer.menuLine = -1
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
            if (repoTab.busyCount !== 0)
                return
            if (!navOpenTagTimer.ready())
                return
            // Asked and re-applied every beat: the fetch rebuilds the list under the hand (`nav-open` の同じ歩き).
            const row = tagsModel.rowOfName(navOpenTagTimer.tag)
            if (row < 0)
                return
            navProbe.pointTipAt("tag", row)
            if (!navProbe.rowFactsOpen)
                return
            // Nothing is read until the layout stops moving (`nav-open` の同じ待ち).
            const geom = sidebarPane.rowFactsGeom()
            if (geom !== navOpenTagTimer.stood) {
                navOpenTagTimer.stood = geom
                return
            }
            // Or the menu on one line, asked once — its answer is the card, not the line.
            if (navOpenTagTimer.menuLine >= 0) {
                if (!navOpenTagTimer.rested) {
                    navOpenTagTimer.rested = navProbe.factsLineMenu(navOpenTagTimer.menuLine)
                    return
                }
                if (!refMenu.opened)
                    return
                navOpenTagTimer.stop()
                refMenu.openSub(refTagCard)
                Harness.report(driver.tagReachWords(refTagCard))
                renderedBarrier.begin()
                return
            }
            // Then the second rest, on the first line or the name, re-applied until it takes: the lines are built a
            // pass after the row that opened them.
            const tip = page.ToolTip.toolTip
            if (navOpenTagTimer.rest !== "") {
                if (!navOpenTagTimer.rested) {
                    navOpenTagTimer.rested = navOpenTagTimer.rest === "name" ? navProbe.pointFactsName(true)
                                                                             : navProbe.pointFactsLine(0, true)
                    return
                }
                if (!tip.visible)
                    return
            }
            navOpenTagTimer.stop()
            // `sides=` (what the row's cloud is drawn from) and `says=` (the carriers the lines name) are the claim as
            // a pair: a cloud over no lines and lines naming an unheard-of remote frame alike.
            Harness.report("nav_open_tag tag=" + navOpenTagTimer.tag
                + " row=" + row
                + " sides=" + tagsModel.tagSides(navOpenTagTimer.tag)
                + " open=" + navProbe.rowFactsOpen
                + " shown=" + navProbe.rowFactsShown()
                + " seat=" + sidebarPane.rowFactsGeom()
                + " says=" + navProbe.rowFactsWords()
                // The line's supplement, read off the shared instance — last, after a sentence with free fields.
                + " tip=" + tip.visible + " text=" + (tip.visible ? tip.text : ""))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=nav-branch-box / nav-rename-box / nav-tag-box: how wide the box comes out on a row of that depth in
    // a pane of that width — one question, so one timer for all of them.
    SampleTimer {
        id: navNameBoxTimer
        property string mode: "branch"
        property string kind: "branch"
        property string ref: ""
        /// The width the splitter settles at: the ask, held to the pane's floor.
        property real want: 0
        property int step: 0
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
            navNameBoxTimer.step = 0
            navNameBoxTimer.start()
        }
        onTriggered: {
            // -1 for a row behind a closed folder or in a section not read yet.
            const row = navNameBoxTimer.section.rowOfName(navNameBoxTimer.ref)
            if (navNameBoxTimer.step === 0) {
                // The width first: the box is laid out in what the row has left, so one opened earlier settles into a
                // width nobody asked about.
                if (row < 0 || Math.round(sidebarPane.width) !== Math.round(navNameBoxTimer.want))
                    return
                if (navNameBoxTimer.mode === "rename") {
                    // What a second click puts in the box (`NavList`): a remote branch without its remote.
                    const shown = navNameBoxTimer.kind === "remote"
                        ? GitFacts.branchOfRef(navNameBoxTimer.ref, repoTab.remoteNames)
                        : navNameBoxTimer.ref
                    sidebarPane.beginRename(navNameBoxTimer.kind, navNameBoxTimer.ref, shown)
                    navNameBoxTimer.step = 1
                    return
                }
                const oid = navNameBoxTimer.section.oidOfName(navNameBoxTimer.ref)
                // Through the menu — the box's only door on this side, and the one that picks this box over the
                // graph's (`RepoPage.refMenuInSidebar`). Both names are the whole ref: only `full` reaches git.
                page.openRefMenu(navNameBoxTimer.kind, navNameBoxTimer.ref,
                                 navNameBoxTimer.ref, oid, true)
                if (navNameBoxTimer.mode === "tag")
                    page.startTagAt(oid)
                else
                    page.startBranchAt(oid)
                // Closed as choosing a row closes it: a menu left standing is drawn over the rows beside the box.
                refMenu.close()
                navNameBoxTimer.step = 1
                return
            }
            const key = navNameBoxTimer.kind + ":" + navNameBoxTimer.ref
            const list = navProbe.listOf(navNameBoxTimer.kind)
            const drawn = list.rowBoxWidth(row)
            if (navNameBoxTimer.step === 1) {
                // On the row and built: a row not laid out yet answers 0, like a row with no box. `focused=` is
                // reported since a box that cannot be typed into looks the same in the picture.
                if (sidebarPane.editKey !== key || drawn <= 0)
                    return
                if (navNameBoxTimer.away) {
                    // Far enough that the row is out of the list, near enough that the view still holds its delegate —
                    // otherwise what went is the row and not the box.
                    list.scrollRows(4)
                    navNameBoxTimer.step = 2
                    return
                }
            } else if (list.rowBoxShown(row)) {
                // Walked past: wait for the box to go, read off `shown` — a released delegate answers 0 for everything.
                return
            }
            navNameBoxTimer.stop()
            const whole = list.rowBoxWhole(row)
            Harness.report("nav_name_box mode=" + navNameBoxTimer.mode
                              + " ref=" + navNameBoxTimer.ref
                              + " row=" + row
                              + " pane=" + Math.round(sidebarPane.width)
                              // The room the row holds, the drawn width, and what reading it whole takes: whether it
                              // needed room outside the pane and got it.
                              + " seat=" + Math.round(list.rowBoxSeat(row))
                              + " box=" + Math.round(drawn)
                              + " whole=" + Math.ceil(whole)
                              // A cut placeholder asks nothing (デザイン規約 §グラフ行のダブルクリック).
                              + " cut=" + (Math.ceil(whole) > Math.round(drawn))
                              + " open=" + (sidebarPane.editKey === key)
                              + " focused=" + list.rowFocused(row)
                              + " shown=" + list.rowBoxShown(row)
                              + " at=" + list.rowBoxAt(row))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=rename-box-out: one way out of the name box per run. The claim is what the box and the gesture are
    // left holding: a wait still running is how a box comes back by itself, so `armed=false` says it will not.
    property int boxOutStep: 0
    SampleTimer {
        id: boxOutTimer
        /// Which way out this run takes: `same-row` / `other-row` / `escape` / `away` / `typed-away`.
        property string route: ""
        onTriggered: {
            const item = graphPane.view.itemAtIndex(0)
            if (!item)
                return
            if (acts.boxOutStep === 0) {
                // Opened by the gesture itself, two clicks: a box put up any other way leaves the gesture with no
                // memory of the row, and every way out passes for free.
                item.leftClick(Qt.NoModifier)
                acts.boxOutStep = 1
            } else if (acts.boxOutStep === 1) {
                if (graphPane.view.clickGuarded)
                    return
                item.leftClick(Qt.NoModifier)
                acts.boxOutStep = 2
            } else if (acts.boxOutStep === 2) {
                if (graphPane.namingOid === "")
                    return
                const route = boxOutTimer.route
                if (route === "same-row" || route === "other-row") {
                    // A click is a press then a release, answered in different places: the press reaches
                    // `FocusRelease` first and takes the box down, and only then does the row see the click — a click
                    // put in alone skips the press.
                    page.releasePressedAway(null)
                    const row = route === "same-row" ? item : graphPane.view.itemAtIndex(1)
                    if (!row)
                        return
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
                acts.boxOutStep = 3
            } else if (acts.boxOutStep === 3) {
                boxOutTimer.stop()
                Harness.report(
                "rename_box_out route=" + boxOutTimer.route
                + " box=" + (graphPane.namingOid !== "")
                + " armed=" + graphPane.view.renameWaiting
                + " guarded=" + graphPane.view.clickGuarded
                + " typed=" + graphPane.namingText)
                driver.complete()
            }
        }
    }
}

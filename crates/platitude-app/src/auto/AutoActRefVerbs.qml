pragma ComponentBehavior: Bound

import QtQuick
// For the attached types alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// What a ref's own menu offers: deleting a branch, a tag, a stash or a remote, making a tag here, and the
/// refusals git answers those with.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing
// and is never given a size.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
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
    readonly property var refMenu: driver.refMenu
    readonly property var refBranchCard: driver.refBranchCard
    readonly property var refTagCard: driver.refTagCard
    readonly property var refDeleteItem: driver.refDeleteItem
    readonly property var refStashDropItem: driver.refStashDropItem
    readonly property var refPushTagItem: driver.refPushTagItem
    readonly property var refTagHereItem: driver.refTagHereItem
    readonly property var refTagDeleteItem: driver.refTagDeleteItem
    readonly property var refRemoteTagDeleteItem: driver.refRemoteTagDeleteItem
    readonly property var refTagBothDeleteItem: driver.refTagBothDeleteItem
    readonly property var commitMenu: driver.commitMenu
    readonly property var tagHereCommitItem: driver.tagHereCommitItem
    readonly property var commitTagCard: driver.commitTagCard
    readonly property var commitBranchCard: driver.commitBranchCard
    readonly property var commitDeleteItem: driver.commitDeleteItem
    readonly property var switchCommitItem: driver.switchCommitItem
    readonly property var renderedBarrier: driver.barrierRendered
    readonly property var writeBarrier: driver.barrierWrite

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "delete-branch" || act === "delete-branch-go" || act === "delete-branch-refused") {
            // On a branch git refuses, the row turns into the held force-delete, which "-go" then runs to its end and
            // "-refused" stands still and reads. **The card goes up and the row is pressed from the sampler rather
            // than from here** (`deleteRowTimer`): nothing is deleted while a write is running, and the page drops a
            // request made then instead of queueing it (`RepoPage.deleteRow`) — so a press put in a tick early goes
            // nowhere, and every one of these waits out the watchdog on a write nobody made.
            //
            // The plain one is finished by the write barrier `dispatchFinished` puts up, which reads nothing until
            // the press arms it; the other two own their own completion (`AutoActCompletion.defersCompletion`) and
            // their tail goes on off the press itself.
            driver.expectWriteAtPress()
            deleteRowTimer.after = act === "delete-branch-go" ? "hold"
                                 : act === "delete-branch-refused" ? "refused" : ""
            deleteRowTimer.start()
        } else if (act === "delete-gone") {
            // The row and its chip leave at the press, and git is asked behind them (デザイン規約 §消す操作は先に画面から
            // 消す). **A tag, because git refuses no tag delete** — the branch's own half of the rule is the row coming
            // *back* from a refusal, which `delete-branch-refused` photographs. The page is asked to hold the
            // in-between open for the shot before the press rather than off it (`RepoPage.holdGoneRows`); a demo
            // repository answers before a picture can be grabbed.
            page.holdGoneRows = true
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            refMenu.openSub(refTagCard)
            refTagDeleteItem.completeHold()
            goneRowTimer.start()
        } else if (act === "delete-tag" || act === "delete-tag-go") {
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            refMenu.openSub(refTagCard)
            if (act === "delete-tag-go")
                refTagDeleteItem.completeHold()
        } else if (act === "delete-stash" || act === "delete-stash-go") {
            const dropOid = stashesModel.oidOfName(stashesModel.nameAt(0))
            page.openRefMenu("stash", stashesModel.nameAt(0),
                             stashesModel.fullAt(0), dropOid)
            if (act === "delete-stash-go") {
                // The drop takes the entry's row off the graph; the write
                // barrier alone would photograph the graph still holding
                // it (the same edge stash-pop-row waits on).
                driver.graphGoneOid = dropOid
                refStashDropItem.completeHold()
            }
        } else if (act === "delete-remote" || act === "delete-remote-go" || act === "remote-refused") {
            // Named outright (`origin/feature/x`) because those rows sit behind a fold — opened here so the row is
            // under the menu. The remote's own name may hold `/`, so the cut is the configured one
            // (`GitFacts.remoteOfRef`), the same as the rows the verbs drive.
            remotesModel.toggleFolder(GitFacts.remoteOfRef(arg, repoTab.remoteNames))
            page.openRefMenu("remote", arg, arg, remotesModel.oidOfName(arg))
            refMenu.openSub(refBranchCard)
            Harness.report("ref_menu kind=remote delete=" + refDeleteItem.code
                              + " " + refDeleteItem.text)
            if (act === "delete-remote-go" || act === "remote-refused")
                refDeleteItem.completeHold()
            // The far side keeps the branch (`--preset protected`): what comes back is a report rather than a
            // failure, and the bar it comes down in is what this one photographs.
            if (act === "remote-refused")
                driver.barrierNotice.start()
        } else if (act === "delete-force") {
            repoTab.deleteBranch(arg, true)
        } else if (act === "delete-branch-chip") {
            // The same delete, asked from the graph row's entrance — the very card the sidebar's row opens
            // (`RefBranchMenu`). **The subject is the card going.** The row is left standing for a refusal
            // (`AppMenuItem.staysOpen`), so a delete git takes has to take the card down behind it, and a card that
            // closed frames exactly like one that was never opened — only the report can say which.
            //
            // The menu goes up and the row is pressed from the sampler, on the same terms as the sidebar's three
            // above: both entrances end at the one page function, and it drops what it is asked while a write runs.
            driver.expectWriteAtPress()
            chipDeleteTimer.start()
        } else if (act === "chip-menu") {
            // The chip's own entrance. **It raises the row's menu, aimed at that name** — there is no second menu on
            // the other side of the chip column any more (デザイン規約 §グラフ行の右クリック). Only the kind letter
            // and the name of the record are read.
            page.openRowMenu(branchesModel.oidOfName(arg), "L00000" + arg)
            chipMenuTimer.start()
        } else if (act === "chip-menu-current") {
            page.openRowMenu(branchesModel.oidOfName(workTree.branch),
                             "L10010" + workTree.branch)
            chipMenuTimer.start()
        } else if (act === "delete-blocked-tip") {
            // Forced rather than hovered: the pointer cannot be put on a row from here, and this writes to the property
            // the real hover writes to. The argument names the branch, because the delete row is out for more than one
            // reason: without one it is the branch you are standing on, with one it is a branch another working copy
            // has checked out (the flags say which, and the current branch is the only chip that carries them).
            //
            // `<branch>:remote` aims at the row that reaches the remote reading instead. That one is out for a
            // reason of its own — the two names standing on different commits — and the local row above it can be
            // pressable at the same time (デザイン規約 §左メニューの所作 の削除の表).
            const wantsRemote = arg.endsWith(":remote")
            const named = wantsRemote ? arg.slice(0, -7) : arg
            const blockedOn = named === "" ? workTree.branch : named
            page.openRefMenu("branch", blockedOn, blockedOn, branchesModel.oidOfName(blockedOn))
            refMenu.openSub(refBranchCard)
            acts.blockedTipRow = wantsRemote ? refBranchCard.deleteRemoteItem : refDeleteItem
            acts.blockedTipRow.tipForced = true
            blockedTipTimer.start()
        } else if (act === "menu-highlight") {
            // The keyboard's road to `highlighted` — the only one that can be driven from here.
            const litOn = arg === "" ? workTree.branch : arg
            page.openRefMenu("branch", litOn, litOn, branchesModel.oidOfName(litOn))
            refMenu.currentIndex = 1
            Harness.report("menu_highlight index=" + refMenu.currentIndex)
        } else if (act === "delete-branch-early") {
            // The early answer dresses the delete row before any click; the argument picks which half is on show.
            // **The card goes up from the sampler rather than from here** — what it is about has to be true at the
            // moment it opens, and this is the one tick nobody chose (`earlyDeleteTimer`).
            earlyDeleteTimer.start()
        } else if (act === "branch-at-tag") {
            sidebarPane.beginBranchAt("tag", tagsModel.nameAt(0),
                                      tagsModel.oidOfName(tagsModel.nameAt(0)))
            sidebarPane.submitEdit(arg)
        } else if (act === "create-tag") {
            // The graph's road, all the way through: the commit menu's row opens the box in the chip column, and what
            // is typed there is what git is finally spawned with. Nothing about it is a shortcut — the row is the one
            // the pointer would press, and the submit is the field's own.
            //
            // The row under HEAD's, counted the way `commit-menu` counts it: the rows above HEAD are whatever else the
            // graph is showing (the working tree, a stash), and neither of them opens this menu at all.
            acts.createTagOid = graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(acts.createTagOid)
            commitMenu.openSub(commitTagCard)
            tagHereCommitItem.triggered()
            // `<name>:box` stops at the box the row opened, which is the other half of what this verb wires up: the
            // chip column asking the second of its two questions (`GraphRowChips`).
            if (arg.endsWith(":box")) {
                graphPane.view.namingText = arg.slice(0, -4)
                Harness.report("create_tag box=" + (graphPane.view.namingOid !== "")
                                  + " mode=" + graphPane.view.namingMode)
                renderedBarrier.begin()
            } else {
                graphPane.view.namingSubmitted(acts.createTagOid, arg, "tag")
                createTagTimer.start()
            }
        } else if (act === "tag-refused") {
            // The same row as `delete-remote-tag`, against a remote that keeps its tags (`--preset protected`).
            // The far side turns a tag down the way it turns a branch down, and the two used to reach the screen
            // differently — one as a report, one as an error over the log (デザイン規約 §答えの要らない報せ).
            tagMenuTimer.begin(arg, "remote-refuse")
        } else if (act === "tag-menu" || act === "push-tag"
                   || act === "delete-remote-tag" || act === "delete-tag-both") {
            // The rows a tag's menu grew, and the press that runs one of them. The suffix on the argument says what
            // has to be known before the card is worth reading — `:drift` for the forced push, `:remote` for the
            // delete rows — and both of those are answers only a fetch brings.
            tagMenuTimer.begin(arg,
                               act === "push-tag" ? "push"
                             : act === "delete-remote-tag" ? "remote-delete"
                             : act === "delete-tag-both" ? "both-delete" : "")
        } else if (act === "move-branch") {
            // Past the question, for the write it guards.
            page.switchTo("force", repoTab.localNameFor(arg),
                          repoTab.localNameFor(arg), arg)
        } else {
            return false
        }
        return true
    }
    /// Says whether the delete this run asked for went out, and reports it either way. **The input's own answer**
    /// (`RepoPage.deleteRowAsked`) rather than a second reading of what would have stopped it: a run that latched on
    /// a press the page dropped waits on a write nobody made, which is the watchdog's whole ceiling in silence. The
    /// caller finishes the run where it stands on a false answer, and this line is what fails it
    /// (`verify/verbs/nav.rs`). `busy=` is read after the press rather than before, so it says whether the count rose
    /// under it; the row's own two say whether a reader would have had that press at all.
    function deleteRowLanded(row) {
        Harness.report("delete_row asked=" + page.deleteRowAsked
                          + " busy=" + repoTab.busyCount
                          + " offered=" + row.offered
                          + " blocked=" + (row.blockedReason !== ""))
        return page.deleteRowAsked
    }
    // The delete asked of the left pane's card. **The card goes up and the row is pressed here rather than at the
    // dispatch**: the delete row is only in the card while nothing is running (`offers::ref_menu`) and the page drops
    // a request made then on the same terms (`RepoPage.deleteRow`), so a tick early is a press that goes nowhere.
    SampleTimer {
        id: deleteRowTimer
        /// What goes on after the press: `hold` runs the held `-D` git's refusal leaves behind, `refused` stands
        /// still and reads the row it turned into, and "" is the plain delete, which the write barrier finishes.
        property string after: ""
        onTriggered: {
            // **The precondition is read here and nowhere else** (app-ui.md §UI 自動化の因果性).
            if (repoTab.busyCount !== 0)
                return
            deleteRowTimer.stop()
            const arg = Harness.autoActArg
            const oid = branchesModel.oidOfName(arg)
            page.openRefMenu("branch", arg, arg, oid)
            page.deleteRow("branch", arg, arg, oid)
            refMenu.openSub(refBranchCard)
            if (!acts.deleteRowLanded(refDeleteItem)) {
                // Nothing went out, so nothing is coming to wait for: the tail is never started and the run says
                // where it stands rather than spending the ceiling on a write nobody made.
                driver.complete()
                return
            }
            driver.pressedWrite()
            if (deleteRowTimer.after === "hold")
                forceDeleteTimer.start()
            else if (deleteRowTimer.after === "refused")
                refusedRowTimer.start()
        }
    }
    // git's refusal has to come back before the row it turns into a held one can be held — or photographed.
    SampleTimer {
        id: forceDeleteTimer
        onTriggered: {
            if (repoTab.writeSeq <= driver.writeSeqBefore || repoTab.busyCount !== 0 || refDeleteItem.holdMs <= 0)
                return
            forceDeleteTimer.stop()
            Harness.report("ref_menu delete=" + refDeleteItem.text
                              + " note=" + refDeleteItem.note)
            driver.writeSeqBefore = repoTab.writeSeq
            refDeleteItem.completeHold()
            writeBarrier.start()
        }
    }
    // The blocked row's line, worn where the pointer would put it. Past `tipDelayMs`, like the other forced tooltips:
    // read any sooner and the attached ToolTip has not opened yet, so the line reports false while the picture taken at
    // quit holds it.
    SampleTimer {
        id: blockedTipTimer
        onTriggered: {
            if (acts.blockedTipRow === null || !acts.blockedTipRow.ToolTip.visible)
                return
            blockedTipTimer.stop()
            Harness.report(
            "delete_blocked code=" + acts.blockedTipRow.code
            + " tip=" + acts.blockedTipRow.ToolTip.visible
            + " reason=" + acts.blockedTipRow.blockedReason)
            driver.complete()
        }
    }
    SampleTimer {
        id: chipMenuTimer
        onTriggered: {
            if (!refMenu.opened && !commitMenu.opened)
                return
            chipMenuTimer.stop()
            // `ref=` is the menu that must **not** come up: the chip and the rest of the row are one target now, and a
            // second menu on the chip's side is the very split this entrance was joined to end.
            Harness.report("chip_menu ref=" + refMenu.opened
                                       + " commit=" + commitMenu.opened
                                       + " switch=" + switchCommitItem.offered
                                       + " delete=" + commitDeleteItem.code
                                       + " " + commitDeleteItem.text)
            driver.complete()
        }
    }
    // A tag that has been made is a row in TAGS, and **the write answers before the read that puts it there** (core
    // `AfterWrite::Graph` — verify-ui §壊れない動詞). Stopping at the write barrier photographs the sidebar as it was a
    // moment before, which is a picture of nothing having happened; what this waits for is the name itself.
    SampleTimer {
        id: createTagTimer
        onTriggered: {
            const oid = tagsModel.oidOfName(Harness.autoActArg)
            if (oid === "" || repoTab.busyCount !== 0)
                return
            createTagTimer.stop()
            Harness.report("create_tag tag=" + Harness.autoActArg
                              + " row=" + tagsModel.rowOfName(Harness.autoActArg)
                              + " total=" + tagsModel.total
                              + " at=" + (oid === acts.createTagOid))
            renderedBarrier.begin()
        }
    }
    /// The commit the run asked for the tag on, so the report can say the tag landed on that one rather than on
    /// wherever HEAD happened to be.
    property string createTagOid: ""
    /// Which row of the delete table `delete-blocked-tip` is holding a tooltip open on.
    property var blockedTipRow: null

    // The tag menu's push row, which is the one row in this application whose whole shape — chip, hold, colour — is
    // decided by what a remote was last heard to carry (`RefRowMenu`). Two states to photograph and they are told
    // apart by nothing but that reading, so the report spells it out: a picture of `push` and a picture of
    // `push --force` differ by five glyphs in a card that is otherwise identical.
    //
    // **`want` is what the run is waiting for, not what it asserts.** The drifted side needs the remotes read
    // (`ls-remote --tags` is the only carrier — core.md タグのリモート状態), and that read lands well after the fetch it
    // rides out with; waiting on the fetch alone photographs the plain row and calls it the forced one.
    SampleTimer {
        id: tagMenuTimer
        /// The tag the menu is to stand on, and what the run needs known about it before the card is worth
        /// photographing: `drift` (a remote has the name on another commit) or `remote` (a remote has it at all).
        /// Both are answers only `ls-remote --tags` carries, so either one fetches first.
        property string tag: ""
        property string wants: ""
        /// Which row this run presses, empty for the ones that only stand the card up.
        property string press: ""
        function begin(arg, pressing) {
            const parts = arg.split(":")
            tagMenuTimer.tag = parts[0]
            tagMenuTimer.wants = parts.length > 1 ? parts[1] : ""
            tagMenuTimer.press = pressing
            if (tagMenuTimer.wants !== "")
                repoTab.fetch("")
            tagMenuTimer.start()
        }
        /// Whether what this run is waiting on has arrived, asked of the same lookups the menu asks
        /// (`NavSectionModel`): the readings are in or they are not, and no count of fetches says which.
        function ready() {
            if (tagMenuTimer.wants === "drift")
                return tagsModel.remoteTagDrift(tagMenuTimer.tag, repoTab.defaultRemote) !== ""
            if (tagMenuTimer.wants === "remote") {
                const sides = tagsModel.tagSides(tagMenuTimer.tag)
                return sides === "remote" || sides === "both"
            }
            return true
        }
        onTriggered: {
            if (!tagMenuTimer.ready() || repoTab.busyCount !== 0)
                return
            tagMenuTimer.stop()
            page.openRefMenu("tag", tagMenuTimer.tag, tagMenuTimer.tag,
                             tagsModel.oidOfName(tagMenuTimer.tag), true)
            // Every row this verb is about is one card in (デザイン規約 §メニュー の入れ子), so the run opens it: the
            // report reads the rows either way, but a picture of the outer card proves nothing about them.
            refMenu.openSub(refTagCard)
            // Every row this menu grew, in one line. The delete rows are told apart by which of them is drawn and
            // which of them can be pressed — a card missing one frames exactly like a card that never offered it,
            // and a row greyed for the drifted reading frames exactly like one that is simply out (デザイン規約
            // §左メニューの所作 の削除の表).
            // **The order is the judging order.** `must_say` matches a run of this line, so what one run has to
            // assert together has to sit together: the sides and the four rows they decide first, the push row's
            // own shape after them (`verify/verbs/remote.rs`).
            // A row with no seat has no state to read: `blockedReason` is a binding and answers whether or not the
            // row is drawn, so the offer is asked first.
            const outRows = [refRemoteTagDeleteItem.offered && refRemoteTagDeleteItem.blocked ? "remote" : "",
                             refTagBothDeleteItem.offered && refTagBothDeleteItem.blocked ? "both" : ""]
                            .filter(w => w !== "")
            Harness.report("tag_menu tag=" + tagMenuTimer.tag
                              + " sides=" + tagsModel.tagSides(tagMenuTimer.tag)
                              + " local_del=" + refTagDeleteItem.offered
                              + " remote_del=" + refRemoteTagDeleteItem.offered
                              + " both_del=" + refTagBothDeleteItem.offered
                              // Which of the two that reach the remote are standing but out. A word rather than a
                              // flag apiece: `none` is a claim of its own, and a run asserting it cannot be matched
                              // by the line that names them.
                              + " blocked=" + (outRows.length === 0 ? "none" : outRows.join(","))
                              + " tag_here=" + refTagHereItem.offered
                              + " push=" + refPushTagItem.offered
                              + " code=" + refPushTagItem.code
                              + " held=" + (refPushTagItem.holdMs > 0)
                              // The model's own reading (the same one ready() polls) — not a page id: the driver's
                              // property list is this family's whole reach (`AutoActDriver`).
                              + " lease=" + tagsModel.remoteTagDrift(tagMenuTimer.tag, repoTab.defaultRemote)
                              + " text=" + refPushTagItem.text)
            if (tagMenuTimer.press === "") {
                driver.complete()
                return
            }
            driver.writeSeqBefore = repoTab.writeSeq
            if (tagMenuTimer.press === "remote-refuse") {
                // Nothing moves, so there is no reading of this name to wait for: what the run waits on is the bar.
                refRemoteTagDeleteItem.completeHold()
                driver.barrierNotice.start()
                return
            }
            if (tagMenuTimer.press === "remote-delete" || tagMenuTimer.press === "both-delete") {
                // What a delete is judged on is the sidebar afterwards, and **the write answers before the read that
                // rebuilds it** (core `AfterWrite::Graph`) — stopping at the write barrier photographs the list as it
                // was and calls it the list as it is. What the run waits for is this name's own reading changing.
                tagGoneTimer.was = tagsModel.tagSides(tagMenuTimer.tag)
                tagGoneTimer.tag = tagMenuTimer.tag
                if (tagMenuTimer.press === "remote-delete")
                    refRemoteTagDeleteItem.completeHold()
                else
                    refTagBothDeleteItem.completeHold()
                tagGoneTimer.start()
                return
            }
            if (refPushTagItem.holdMs > 0)
                refPushTagItem.completeHold()
            else
                refPushTagItem.triggered()
            writeBarrier.start()
        }
    }
    // A tag delete, judged on the sidebar it leaves rather than on the write that made it. The name's own reading is
    // the edge: gone from both sides it answers nothing at all, gone from the remote alone it drops back to `here`.
    // **A count would not do** — the remote half of a name held on both sides takes no row away.
    SampleTimer {
        id: tagGoneTimer
        property string tag: ""
        property string was: ""
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            const now = tagsModel.tagSides(tagGoneTimer.tag)
            if (now === tagGoneTimer.was)
                return
            tagGoneTimer.stop()
            Harness.report("tag_gone tag=" + tagGoneTimer.tag
                              + " was=" + tagGoneTimer.was
                              + " sides=" + now
                              + " row=" + tagsModel.rowOfName(tagGoneTimer.tag)
                              + " total=" + tagsModel.total)
            renderedBarrier.begin()
        }
    }
    // Waits on the early answer, not on a refusal: nothing here writes.
    SampleTimer {
        id: earlyDeleteTimer
        /// Whether the card is up on the branch yet. While it is not, every tick is the input's own branch and
        /// nothing below is read.
        property bool cardUp: false
        onTriggered: {
            const arg = Harness.autoActArg
            if (!earlyDeleteTimer.cardUp) {
                // **The precondition is read here and nowhere else** (app-ui.md §UI 自動化の因果性): the delete row is
                // only in the card while nothing is running (`offers::ref_menu`), and the card works its answers out
                // once as it opens — put up a tick early it asks nobody and is never asked again, which spends the
                // watchdog's 120 seconds in silence instead of failing on a line anyone can read.
                if (repoTab.busyCount !== 0)
                    return
                earlyDeleteTimer.cardUp = true
                page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
                refMenu.openSub(refBranchCard)
                if (refBranchCard.deleteAsked)
                    return
                // The input says it did not land, so nothing is coming to latch on: the branch names no commit, or
                // its delete is out (the branch the tree is on, one another working copy holds). Reported in facts
                // rather than in the row's own sentence — `must_say` never matches a non-ASCII line on Windows
                // (verify-ui §Windows での実行・デバッグの罠), and the em dash is in every one of them.
                earlyDeleteTimer.stop()
                Harness.report("delete_early asked=false"
                                  + " oid=" + (branchesModel.oidOfName(arg) !== "")
                                  + " offered=" + refDeleteItem.offered
                                  + " blocked=" + (refDeleteItem.blockedReason !== ""))
                driver.complete()
                return
            }
            // The row's `code` is never empty on a branch, so it cannot tell "not answered yet" from "answered merged"
            // — both wear `branch --delete`. The card says whether an answer is in hand: the graph's, in the frame
            // the card opened (`RefBranchMenu.deleteAnswered`), or git's by the echo of the branch asked about, which
            // the asking clears before the question goes out (app-ui.md §UI 自動化の因果性).
            const fromRows = refBranchCard.deleteAnswered
            if (!fromRows && repoTab.branchDeleteAsked !== arg)
                return
            earlyDeleteTimer.stop()
            Harness.report("delete_early asked=true"
                                       + " from=" + (fromRows ? "rows" : "git")
                                       + " merged=" + (fromRows ? refBranchCard.deleteMerged
                                                                : repoTab.branchDeleteMerged)
                                       + " code=" + refDeleteItem.code
                                       + " held=" + (refDeleteItem.holdMs > 0)
                                       + " note=" + refDeleteItem.note)
            driver.complete()
        }
    }
    // The delete git took, asked from the graph row. Judged rather than photographed: the menu that stayed up for a
    // refusal has to go once none is coming, and a closed card is the same picture as a card nobody opened. `code=`
    // is what tells a landing from a refusal — a turned-down delete leaves the row wearing `branch -D`.
    SampleTimer {
        id: chipDeleteTimer
        /// Whether the row has been pressed yet. While it has not, every tick is the input's own branch and nothing
        /// below is read.
        property bool pressed: false
        onTriggered: {
            if (!chipDeleteTimer.pressed) {
                // **The precondition is read here and nowhere else** (app-ui.md §UI 自動化の因果性), the same one the
                // sidebar's three read: the page drops the request while a write is running.
                if (repoTab.busyCount !== 0)
                    return
                const arg = Harness.autoActArg
                page.openRowMenu(branchesModel.oidOfName(arg), "L00000" + arg)
                commitMenu.openSub(commitBranchCard)
                // The row's own press: a stays-open row is picked rather than triggered (`AppMenuItem.picked`).
                commitDeleteItem.picked()
                if (!acts.deleteRowLanded(commitDeleteItem)) {
                    chipDeleteTimer.stop()
                    driver.complete()
                    return
                }
                chipDeleteTimer.pressed = true
                driver.pressedWrite()
                return
            }
            if (repoTab.writeSeq <= driver.writeSeqBefore || repoTab.busyCount !== 0)
                return
            chipDeleteTimer.stop()
            Harness.report("chip_delete branch=" + Harness.autoActArg
                              + " row=" + branchesModel.rowOfName(Harness.autoActArg)
                              + " code=" + commitDeleteItem.code
                              + " menu=" + commitMenu.opened
                              + " card=" + commitBranchCard.opened)
            driver.complete()
        }
    }
    SampleTimer {
        id: refusedRowTimer
        onTriggered: {
            if (repoTab.writeSeq <= driver.writeSeqBefore || repoTab.busyCount !== 0)
                return
            refusedRowTimer.stop()
            Harness.report("ref_menu delete=" + refDeleteItem.code
                                       + " " + refDeleteItem.text
                                       + " note=" + refDeleteItem.note)
            driver.complete()
        }
    }
    // The row taken away ahead of git's answer. **Waited on the list, not on the write** — being ahead of the write
    // is the whole of what this photographs, so a barrier here would wait out the very state it is about.
    SampleTimer {
        id: goneRowTimer
        onTriggered: {
            if (tagsModel.rowOfName(Harness.autoActArg) >= 0)
                return
            goneRowTimer.stop()
            Harness.report("gone_row tag=" + Harness.autoActArg
                              + " row=" + tagsModel.rowOfName(Harness.autoActArg)
                              + " total=" + tagsModel.total
                              + " chips=" + (graphModel.goneChips !== ""))
            driver.complete()
        }
    }
}

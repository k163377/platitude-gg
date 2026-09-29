pragma ComponentBehavior: Bound

import QtQuick
// For the attached `ToolTip` alone (rules-refs/app-ui.md「attached 型は宣言元モジュールを import しないと」).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// What a ref's own menu offers: deleting a branch, a tag, a stash or a remote, making a tag here, and the
/// refusals git answers those with.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: naming its type would be a cycle — the driver is the file that builds this one.
    required property var driver

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
    readonly property var refPullItem: driver.refPullItem
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
    readonly property var pullCommitItem: driver.pullCommitItem
    readonly property var renderedBarrier: driver.barrierRendered
    readonly property var writeBarrier: driver.barrierWrite

    /// Where a TAG card's rows reach, as one line: the remote the deletes go to, whether that row is up and out, its
    /// words, where the push goes, and why the row greys — what the picture cannot say is which remote a row that
    /// names none would have hit. The free words last: a claim is one substring.
    function reachWords(card) {
        const row = card.deleteRemoteTagItem
        return "tag_reach reach=" + card.reach
            + " offered=" + row.offered + " blocked=" + (row.offered && row.blocked)
            + " push=" + card.pushTagItem.text
            + " del=" + row.text + " why=" + row.blockedReason
    }

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "delete-branch" || act === "delete-branch-go" || act === "delete-branch-refused") {
            // The plain one is finished by the write barrier `dispatchFinished` puts up, armed by the press; `-go` and
            // `-refused` own their completion (`AutoActCompletion.defersCompletion`) and go on off the press.
            driver.beginWrite(act)
            deleteRowTimer.after = act === "delete-branch-go" ? "hold"
                                 : act === "delete-branch-refused" ? "refused" : ""
            deleteRowTimer.start()
        } else if (act === "delete-gone") {
            // The row and its chip leave at the press, git asked behind them (デザイン規約 §消す操作は先に画面から消す).
            // A tag, because git refuses no tag delete. The page holds the in-between open for the shot
            // (`RepoPage.holdGoneRows`) — a demo repository answers before a picture can be grabbed.
            page.holdGoneRows = true
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            refMenu.openSub(refTagCard)
            refTagDeleteItem.completeHold()
            goneRowTimer.start()
        } else if (act === "delete-stood-down") {
            // The other end of `delete-gone`: waits for the in-between to be over. The screen cannot tell a row the
            // list dropped from one the window hides, so the claim is `stood=` / `chips=` read either side of the
            // wait. It catches a stand-in that never lets go (`ops::StandIn`). A tag, as in `delete-gone`.
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            refMenu.openSub(refTagCard)
            // The latch is read at the press (the hold's end): on a demo repository the stand-in can be let go inside
            // one tick (rules-refs/app-ui.md「一瞬だけ立つ状態は signal で観測して latch する」).
            driver.beginWrite(act)
            refTagDeleteItem.held.connect(stoodDownTimer.pressed)
            refTagDeleteItem.completeHold()
            stoodDownTimer.start()
        } else if (act === "delete-tag" || act === "delete-tag-go") {
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            refMenu.openSub(refTagCard)
            // The barrier is armed at the hold's end (`holdToEnd`): armed at the dispatch, it passes on the opening
            // fetch's answer and photographs the card still up.
            if (act === "delete-tag-go")
                driver.holdToEnd(refTagDeleteItem)
        } else if (act === "delete-stash" || act === "delete-stash-go") {
            const dropOid = stashesModel.oidOfName(stashesModel.nameAt(0))
            page.openRefMenu("stash", stashesModel.nameAt(0),
                             stashesModel.fullAt(0), dropOid)
            if (act === "delete-stash-go") {
                // The drop takes the entry's row off the graph; the write barrier alone would photograph the graph
                // still holding it (the same edge `stash-pop-row` waits on).
                driver.graphGoneOid = dropOid
                refStashDropItem.completeHold()
            }
        } else if (act === "delete-remote" || act === "delete-remote-go" || act === "remote-refused") {
            // Named outright (`origin/feature/x`); its fold is opened so the row is under the menu. A remote's name
            // may hold `/`, so the cut is the configured one (`GitFacts.remoteOfRef`).
            remotesModel.toggleFolder(GitFacts.remoteOfRef(arg, repoTab.remoteNames))
            page.openRefMenu("remote", arg, arg, remotesModel.oidOfName(arg))
            refMenu.openSub(refBranchCard)
            Harness.report("ref_menu kind=remote delete=" + refDeleteItem.code
                              + " " + refDeleteItem.text)
            if (act === "delete-remote-go" || act === "remote-refused")
                driver.holdToEnd(refDeleteItem)
            // The far side keeps the branch (`--preset protected`): the refusal comes back as a report, in a bar.
            if (act === "remote-refused")
                driver.barrierNotice.start()
        } else if (act === "delete-upstream-go" || act === "delete-both-go") {
            // A local branch's card and a row that reaches its reading: that alone, or the pair. What reaches the
            // remote is leased to the reading's commit as the card read it (`RefRowMenu.branchFacts`), so a card
            // holding the wrong one is a refused write. The branch has to be tracked, merged and in step
            // (`--preset stack`'s `side`).
            page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
            refMenu.openSub(refBranchCard)
            driver.holdToEnd(act === "delete-both-go" ? refBranchCard.deleteBothItem : refBranchCard.deleteRemoteItem)
        } else if (act === "delete-force") {
            repoTab.deleteBranch(arg, true)
        } else if (act === "delete-branch-chip") {
            // The same delete from the graph row's card (`RefBranchMenu`), pressed from the sampler on the sidebar's
            // terms: both entrances end at `RepoPage.deleteRow`.
            driver.beginWrite(act)
            chipDeleteTimer.start()
        } else if (act === "pull-menu") {
            // Bare: the current branch's row. An argument: a remote-tracking row, its fold opened as `delete-remote`
            // does. Two remote rows tell the offer apart — only the upstream this branch is measured against carries
            // it (`offers::ref_menu`).
            const onRemote = arg !== ""
            const name = onRemote ? arg : workTree.branch
            if (onRemote)
                remotesModel.toggleFolder(GitFacts.remoteOfRef(arg, repoTab.remoteNames))
            const oid = onRemote ? remotesModel.oidOfName(arg) : branchesModel.oidOfName(name)
            pullMenuTimer.onRemote = onRemote
            page.openRefMenu(onRemote ? "remote" : "branch", name, name, oid)
            pullMenuTimer.start()
        } else if (act === "pull-blocked") {
            // The row out where both sides have moved, its reason under the pointer (規約 §取り込んで合流させる). Bare
            // is the sidebar's entrance, `chip` the graph's: the same answer from each needs a run through each.
            // Forced, because a pointer cannot be put on a row from here (verify-ui §hover の絵の撮り方).
            const onBranch = workTree.branch
            if (arg === "chip")
                page.openRowMenu(branchesModel.oidOfName(onBranch), { "kind": "branch", "name": onBranch })
            else
                page.openRefMenu("branch", onBranch, onBranch, branchesModel.oidOfName(onBranch))
            pullBlockedTimer.onChip = arg === "chip"
            pullBlockedTimer.row = arg === "chip" ? pullCommitItem : refPullItem
            pullBlockedTimer.row.tipForced = true
            pullBlockedTimer.start()
        } else if (act === "tag-chip-menu") {
            // `<tag>@<row>`: the graph row's menu aimed at that tag's chip on that row — a reading one remote alone
            // draws names that remote on its own (`CommitMenuState.tagFacts`). Every remote is read first.
            const at = arg.split("@")
            tagChipTimer.tag = at[0]
            tagChipTimer.row = at.length > 1 ? Number(at[1]) : 0
            driver.pressWrite("fetch", () => {
                repoTab.fetch("")
                return true
            })
            tagChipTimer.start()
        } else if (act === "chip-menu") {
            // The chip raises the row's menu aimed at its name (デザイン規約 §グラフ行の右クリック).
            page.openRowMenu(branchesModel.oidOfName(arg), { "kind": "branch", "name": arg })
            chipMenuTimer.start()
        } else if (act === "chip-menu-current") {
            page.openRowMenu(branchesModel.oidOfName(workTree.branch),
                             { "kind": "branch", "name": workTree.branch })
            chipMenuTimer.start()
        } else if (act === "delete-blocked-tip") {
            // Forced through the property the real hover writes. The argument picks why the delete row is out: none
            // is the current branch, a name is one another working copy has checked out. `<branch>:remote` aims at
            // the row reaching the remote reading, out when the two names stand on different commits while the
            // local row may be pressable (デザイン規約 §左メニューの所作 の削除の表).
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
        } else if (act === "delete-branch-early" || act === "delete-branch-early-far") {
            // The early answer dresses the delete row before any click; the card goes up from the sampler
            // (`earlyDeleteTimer`). `-far` differs only in the repository: its branch's tip is outside the drawn rows
            // (`--preset deep-parked`), so git answers every time, where the plain one takes whichever road the refs'
            // timing hands it.
            earlyDeleteTimer.start()
        } else if (act === "branch-at-tag") {
            sidebarPane.beginBranchAt("tag", tagsModel.nameAt(0),
                                      tagsModel.oidOfName(tagsModel.nameAt(0)))
            sidebarPane.submitEdit(arg)
        } else if (act === "create-tag") {
            // The graph's road: the commit menu's row opens the box in the chip column, and the field's own submit
            // spawns git. The row under HEAD's, counted as `commit-menu` counts: the rows above HEAD (the working tree,
            // a stash) open no such menu.
            acts.createTagOid = graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(acts.createTagOid)
            commitMenu.openSub(commitTagCard)
            tagHereCommitItem.triggered()
            // `<name>:box` stops at the box the row opened — the chip column's second question (`GraphRowChips`).
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
            // `delete-remote-tag` against a remote that keeps its tags (`--preset protected`): the refusal arrives as a
            // report, as a branch's does (デザイン規約 §答えの要らない報せ).
            tagMenuTimer.begin(arg, "remote-refuse")
        } else if (act === "tag-menu" || act === "push-tag"
                   || act === "delete-remote-tag" || act === "delete-tag-both") {
            // `<tag>[:drift|:remote][:tip]` — `:drift` for the forced push, `:remote` for the delete rows, `:tip` for
            // the reason a greyed delete row gives.
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
    /// Whether the delete went out, reported either way — the page's own answer (`RepoPage.deleteRowAsked`); latching
    /// on a press the page dropped would wait on a write nobody made. On false the caller finishes the run and this
    /// line fails it (`verify/verbs/nav.rs`). `busy=` is read after the press; `offered=` / `blocked=` say whether a
    /// reader had the press at all.
    function deleteRowLanded(row) {
        Harness.report("delete_row asked=" + page.deleteRowAsked
                          + " busy=" + repoTab.busyCount
                          + " offered=" + row.offered
                          + " blocked=" + (row.blockedReason !== ""))
        return page.deleteRowAsked
    }
    // The delete asked of the left pane's card. The card goes up and the row is pressed here, not in `run`: while a
    // write runs the card has no delete row (`offers::ref_menu`) and the page drops the request (`RepoPage.deleteRow`).
    SampleTimer {
        id: deleteRowTimer
        /// After the press: `hold` runs the held `-D` git's refusal leaves, `refused` reads the row it turned into,
        /// and "" is the plain delete, which the write barrier finishes.
        property string after: ""
        onTriggered: {
            // The precondition is read here and nowhere else (app-ui.md §UI 自動化).
            if (repoTab.busyCount !== 0)
                return
            deleteRowTimer.stop()
            const arg = Harness.autoActArg
            const oid = branchesModel.oidOfName(arg)
            page.openRefMenu("branch", arg, arg, oid)
            page.deleteRow("branch", arg, arg, oid)
            refMenu.openSub(refBranchCard)
            if (!acts.deleteRowLanded(refDeleteItem)) {
                driver.complete()
                return
            }
            driver.inputWent(true)
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
            if (!driver.wroteAndSettled() || refDeleteItem.holdMs <= 0)
                return
            forceDeleteTimer.stop()
            Harness.report("ref_menu delete=" + refDeleteItem.text
                              + " note=" + refDeleteItem.note)
            driver.holdToEnd(refDeleteItem)
            writeBarrier.start()
        }
    }
    // PGG_AUTO_ACT=tag-chip-menu: once this run's fetch is read through, the graph row's menu on that tag's chip, its
    // TAG card open.
    SampleTimer {
        id: tagChipTimer
        property string tag: ""
        property int row: 0
        onTriggered: {
            if (!driver.wroteAndSettled() || repoTab.busyCount !== 0)
                return
            const oid = graphModel.oidAt(tagChipTimer.row)
            if (oid === "")
                return
            tagChipTimer.stop()
            page.openRowMenu(oid, { "kind": "tag", "name": tagChipTimer.tag })
            commitMenu.openSub(commitTagCard)
            Harness.report(acts.reachWords(commitTagCard))
            renderedBarrier.begin()
        }
    }
    // Waits on the attached ToolTip itself (it opens after `tipDelayMs`): read sooner, the line says false while the
    // shot holds it.
    SampleTimer {
        id: blockedTipTimer
        onTriggered: {
            if (acts.blockedTipRow === null || !acts.blockedTipRow.ToolTip.visible)
                return
            blockedTipTimer.stop()
            Harness.report(
            "delete_blocked code=" + acts.blockedTipRow.code
            + " tip=" + acts.blockedTipRow.ToolTip.visible
            // The holding copy's folder (`RefBranchMenu.holderLeaf`), empty on other rows — the sentence itself is
            // tst_branchcard's to judge.
            + " holder=" + refBranchCard.holderLeaf
            + " reason=" + acts.blockedTipRow.blockedReason)
            driver.complete()
        }
    }
    /// The greyed `pull` row and its tip. The line claims what the picture cannot: offered, out rather than pale
    /// (`AppMenuItem.blocked`, which decides colour and press), and the sentence about the divergence.
    SampleTimer {
        id: pullBlockedTimer
        /// Which entrance this run came in by, and the row it left standing there.
        property bool onChip: false
        property var row: null
        onTriggered: {
            const card = pullBlockedTimer.onChip ? commitMenu : refMenu
            if (!card.opened || !pullBlockedTimer.row.ToolTip.visible)
                return
            pullBlockedTimer.stop()
            Harness.report("pull_blocked where=" + (pullBlockedTimer.onChip ? "chip" : "row")
                              + " offered=" + pullBlockedTimer.row.offered
                              + " blocked=" + pullBlockedTimer.row.blocked
                              + " tip=" + pullBlockedTimer.row.ToolTip.visible
                              + " says=" + pullBlockedTimer.row.blockedReason)
            driver.complete()
        }
    }
    /// The `pull` row as the card opens. Whether it was offered is the claim (`offers::ref_menu`): a row wired to
    /// nothing frames like one that reaches git.
    SampleTimer {
        id: pullMenuTimer
        /// Whether the card went up on a remote-tracking row — the half that names the branch it lands in.
        property bool onRemote: false
        onTriggered: {
            if (!refMenu.opened)
                return
            pullMenuTimer.stop()
            // `sentence=`: both rows run the same `git pull`, so both are the word alone (デザイン規約 §取り込んで合流させる).
            Harness.report("pull_menu open=" + refMenu.opened
                              + " kind=" + (pullMenuTimer.onRemote ? "remote" : "branch")
                              + " pull=" + refPullItem.offered
                              + " code=" + refPullItem.code
                              + " sentence=" + (refPullItem.refSentence !== ""))
            driver.complete()
        }
    }
    SampleTimer {
        id: chipMenuTimer
        onTriggered: {
            if (!refMenu.opened && !commitMenu.opened)
                return
            chipMenuTimer.stop()
            // `ref=` must stay down: a second menu on the chip's side is the split this entrance is joined to prevent.
            Harness.report("chip_menu ref=" + refMenu.opened
                                       + " commit=" + commitMenu.opened
                                       + " switch=" + switchCommitItem.offered
                                       + " delete=" + commitDeleteItem.code
                                       + " " + commitDeleteItem.text)
            driver.complete()
        }
    }
    // Waits for the name in TAGS, not the write barrier, which photographs the sidebar from before
    // (rules-refs/app-ui.md「その書き込みが無効化した読み直しより先に来る」).
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
    property string createTagOid: ""
    /// Which row of the delete table `delete-blocked-tip` is holding a tooltip open on.
    property var blockedTipRow: null

    // The tag menu's rows. The push row's whole shape (chip, hold, colour) is decided by what a remote was last heard
    // to carry (`RefRowMenu`), and `push` / `push --force` differ by five glyphs in an otherwise identical card — so
    // the report spells it out. `wants` is what the run waits for: that reading lands well after the fetch it rides
    // out with (rules-refs/core.md「タグのリモート状態は `ls-remote --tags` で訊くしかない」), so waiting on the fetch
    // alone photographs the plain row as the forced one.
    SampleTimer {
        id: tagMenuTimer
        /// The tag, and what must be known before the card is worth photographing: `drift` (a remote has the name on
        /// another commit) or `remote` (a remote has it at all). Either fetches first.
        property string tag: ""
        property string wants: ""
        /// Which row this run presses, empty for the ones that only stand the card up.
        property string press: ""
        /// `:tip` last: the card stood up, the `push --delete` row's reason is forced out, as `delete-blocked-tip` does
        /// on a branch's card — the one place that row says why it is out.
        property bool tip: false
        function begin(arg, pressing) {
            const parts = arg.split(":")
            tagMenuTimer.tag = parts[0]
            tagMenuTimer.tip = parts.length > 1 && parts[parts.length - 1] === "tip"
            tagMenuTimer.wants = parts.length > 1 && parts[1] !== "tip" ? parts[1] : ""
            tagMenuTimer.press = pressing
            // `read` waits for every remote's reading, so it waits on its own fetch by its id (`nav-open-tag`'s wait):
            // a name several remotes carry frames as one only one of them does while the others are still unread.
            if (tagMenuTimer.wants === "read")
                driver.pressWrite("fetch", () => {
                    repoTab.fetch("")
                    return true
                })
            else if (tagMenuTimer.wants !== "")
                repoTab.fetch("")
            tagMenuTimer.start()
        }
        /// Asked of the lookups the menu asks (`NavSectionModel`); no count of fetches says the readings are in.
        function ready() {
            if (tagMenuTimer.wants === "read")
                return driver.wroteAndSettled()
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
            // The rows are one card in (デザイン規約 §メニュー の入れ子); opened so the picture shows them.
            refMenu.openSub(refTagCard)
            // Every row the menu grew, in one line: a card missing a row, or a row greyed for drift, frames like one
            // never offered or simply out (デザイン規約 §左メニューの所作 の削除の表). The order is the judging order —
            // `must_say` matches one stretch of the line: the sides and the rows they decide first, the push row's
            // shape after (`verify/verbs/remote.rs`). The offer is asked first: `blocked` is a binding and answers
            // whether or not the row is drawn.
            const outRows = [refRemoteTagDeleteItem.offered && refRemoteTagDeleteItem.blocked ? "remote" : "",
                             refTagBothDeleteItem.offered && refTagBothDeleteItem.blocked ? "both" : ""]
                            .filter(w => w !== "")
            Harness.report("tag_menu tag=" + tagMenuTimer.tag
                              + " sides=" + tagsModel.tagSides(tagMenuTimer.tag)
                              + " local_del=" + refTagDeleteItem.offered
                              + " remote_del=" + refRemoteTagDeleteItem.offered
                              + " both_del=" + refTagBothDeleteItem.offered
                              // The remote-reaching rows standing but out; `none` is a claim of its own.
                              + " blocked=" + (outRows.length === 0 ? "none" : outRows.join(","))
                              + " tag_here=" + refTagHereItem.offered
                              + " push=" + refPushTagItem.offered
                              + " code=" + refPushTagItem.code
                              + " held=" + (refPushTagItem.holdMs > 0)
                              // The model's reading, as `ready()` polls: this family reaches only the
                              // driver's properties.
                              + " lease=" + tagsModel.remoteTagDrift(tagMenuTimer.tag, repoTab.defaultRemote)
                              + " text=" + refPushTagItem.text)
            Harness.report(acts.reachWords(refTagCard))
            if (tagMenuTimer.press === "" && tagMenuTimer.tip) {
                // Forced through the property the real hover writes; the branch card's wait answers for it.
                acts.blockedTipRow = refRemoteTagDeleteItem
                acts.blockedTipRow.tipForced = true
                blockedTipTimer.start()
                return
            }
            if (tagMenuTimer.press === "") {
                driver.complete()
                return
            }
            if (tagMenuTimer.press === "remote-refuse") {
                // Nothing moves, so there is no reading of this name to wait for: what the run waits on is the bar.
                driver.holdToEnd(refRemoteTagDeleteItem)
                driver.barrierNotice.start()
                return
            }
            if (tagMenuTimer.press === "remote-delete" || tagMenuTimer.press === "both-delete") {
                // Judged on the sidebar after: waits for this name's reading to change, as `createTagTimer` does.
                tagGoneTimer.was = tagsModel.tagSides(tagMenuTimer.tag)
                tagGoneTimer.tag = tagMenuTimer.tag
                driver.holdToEnd(tagMenuTimer.press === "remote-delete" ? refRemoteTagDeleteItem
                                                                         : refTagBothDeleteItem)
                tagGoneTimer.start()
                return
            }
            // The forced push is held, and the barrier is armed at the hold's end (`holdToEnd`): this run's own fetch
            // answers during the hold, and a barrier armed here would pass on it with nothing pushed.
            if (refPushTagItem.holdMs > 0)
                driver.holdToEnd(refPushTagItem)
            else
                driver.pressWrite("push-tag", () => {
                    refPushTagItem.triggered()
                    return true
                })
            writeBarrier.start()
        }
    }
    // A tag delete, judged on the sidebar it leaves. The edge is the name's own reading: gone from both sides it
    // answers nothing, gone from the remote alone it drops back to `here` (no row goes).
    SampleTimer {
        id: tagGoneTimer
        property string tag: ""
        property string was: ""
        onTriggered: {
            if (!driver.wroteAndSettled())
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
    // Waits on the early answer: nothing here writes.
    SampleTimer {
        id: earlyDeleteTimer
        /// Whether the card is up yet; until then every tick is the input's branch only.
        property bool cardUp: false
        onTriggered: {
            const arg = Harness.autoActArg
            if (!earlyDeleteTimer.cardUp) {
                // The precondition is read here and nowhere else (app-ui.md §UI 自動化): the delete row exists only
                // while nothing runs (`offers::ref_menu`), and the card works its answers out once as it opens — put up
                // a tick early, it asks nobody and the run waits out the watchdog.
                if (repoTab.busyCount !== 0)
                    return
                earlyDeleteTimer.cardUp = true
                page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
                refMenu.openSub(refBranchCard)
                if (refBranchCard.deleteAsked)
                    return
                // Not landed (the name has no commit, or its delete is out), so nothing will come to latch on. Reported
                // as facts, not the reason's sentence.
                earlyDeleteTimer.stop()
                Harness.report("delete_early asked=false"
                                  + " oid=" + (branchesModel.oidOfName(arg) !== "")
                                  + " offered=" + refDeleteItem.offered
                                  + " blocked=" + (refDeleteItem.blockedReason !== ""))
                driver.complete()
                return
            }
            // `code` cannot tell "not answered yet" from "answered merged" — both wear `branch --delete`. The answer is
            // in hand when the graph gave it as the card opened (`RefBranchMenu.deleteAnswered`) or git echoes the
            // branch asked about, which the asking clears before the question goes out (app-ui.md §UI 自動化).
            const fromRows = refBranchCard.deleteAnswered
            if (!fromRows && repoTab.branchDeleteAsked !== arg)
                return
            earlyDeleteTimer.stop()
            // Both sides say the same three words; `unknown` is git's alone, what a run whose reads fell over says.
            Harness.report("delete_early asked=true"
                                       + " from=" + (fromRows ? "rows" : "git")
                                       + " merged=" + (fromRows ? (refBranchCard.deleteMerged ? "yes" : "no")
                                                                : repoTab.branchDeleteMerged)
                                       + " code=" + refDeleteItem.code
                                       + " held=" + (refDeleteItem.holdMs > 0)
                                       + " note=" + refDeleteItem.note)
            driver.complete()
        }
    }
    // The delete asked from the graph row. The subject is the card going: the row stays up for a refusal
    // (`AppMenuItem.staysOpen`), so a delete git takes has to take the card down — and a closed card frames like one
    // nobody opened. `code=` tells a landing from a refusal, which leaves the row wearing `branch -D`.
    SampleTimer {
        id: chipDeleteTimer
        /// Whether the row has been pressed; until then every tick is the input's branch only.
        property bool pressed: false
        onTriggered: {
            if (!chipDeleteTimer.pressed) {
                // The precondition is read here and nowhere else (app-ui.md §UI 自動化): the page drops the request
                // while a write runs.
                if (repoTab.busyCount !== 0)
                    return
                const arg = Harness.autoActArg
                page.openRowMenu(branchesModel.oidOfName(arg), { "kind": "branch", "name": arg })
                commitMenu.openSub(commitBranchCard)
                // The row's own press: a stays-open row is picked (`AppMenuItem.picked`).
                commitDeleteItem.picked()
                if (!acts.deleteRowLanded(commitDeleteItem)) {
                    chipDeleteTimer.stop()
                    driver.complete()
                    return
                }
                chipDeleteTimer.pressed = true
                driver.inputWent(true)
                return
            }
            if (!driver.wroteAndSettled())
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
            if (!driver.wroteAndSettled())
                return
            refusedRowTimer.stop()
            Harness.report("ref_menu delete=" + refDeleteItem.code
                                       + " " + refDeleteItem.text
                                       + " note=" + refDeleteItem.note)
            driver.complete()
        }
    }
    // The row taken away ahead of git's answer, waited on the list: a barrier would wait out the state photographed.
    SampleTimer {
        id: goneRowTimer
        onTriggered: {
            if (tagsModel.rowOfName(Harness.autoActArg) >= 0)
                return
            goneRowTimer.stop()
            Harness.report("gone_row tag=" + Harness.autoActArg
                              + " row=" + tagsModel.rowOfName(Harness.autoActArg)
                              + " total=" + tagsModel.total
                              + " chips=" + (graphModel.goneChips.length > 0))
            driver.complete()
        }
    }
    // …and the row let go of again, the half no picture holds. Waited on the gone set: the write answers before the
    // listing that takes the row away for good is asked for, so a barrier would report the middle as the end.
    SampleTimer {
        id: stoodDownTimer
        /// Whether the row was stood in for, read off the gone set at the press — the slot fills it before the press
        /// returns, and sampling could miss the window. Without it a press that took nothing away passes the wait.
        property bool stood: false
        function pressed() {
            stoodDownTimer.stood = graphModel.goneChips.length > 0
            driver.inputWent(true)
        }
        onTriggered: {
            if (!driver.wroteAndSettled())
                return
            // The rows stay off screen until a listing taken after the write is drawn; this is it arriving
            // (`ops::StandIn`).
            if (graphModel.goneChips.length > 0)
                return
            stoodDownTimer.stop()
            Harness.report("stood_down tag=" + Harness.autoActArg
                              + " stood=" + stoodDownTimer.stood
                              + " row=" + tagsModel.rowOfName(Harness.autoActArg)
                              + " total=" + tagsModel.total
                              + " chips=" + (graphModel.goneChips.length > 0))
            renderedBarrier.begin()
        }
    }
}

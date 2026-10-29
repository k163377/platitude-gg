pragma ComponentBehavior: Bound

import QtQuick
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
    readonly property var worktree: driver.worktree
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
    readonly property var commitDeleteItem: driver.commitDeleteItem
    readonly property var switchCommitItem: driver.switchCommitItem
    readonly property var pullCommitItem: driver.pullCommitItem
    readonly property var writeBarrier: driver.barrierWrite

    /// Where a TAG card's rows reach, as one line: the remote the deletes go to, whether that row is up and out, its
    /// words, where the push goes, and why the row greys — what the picture cannot say is which remote a row that
    /// names none would have hit. The free words last: a claim is one substring.
    function reachWords(card) {
        const row = card.deleteRemoteTagItem
        return "tag_reach reach=" + card.reach
            + " offered=" + row.offered + " blocked=" + (row.offered && row.blocked)
            + " push=" + card.pushTagItem.text
            + " del=" + [row.text, row.codeName].filter(words => words !== "").join(" ")
            + " why=" + row.blockedReason
    }

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "delete-branch" || act === "delete-branch-go" || act === "delete-branch-refused") {
            // All three own their completion (`AutoActCompletion.defersCompletion`) and go on off the press.
            driver.beginWrite(act)
            deleteRowTimer.afterPress = act === "delete-branch-go" ? "hold"
                                      : act === "delete-branch-refused" ? "refused" : "menu"
            deleteRowTimer.start()
        } else if (act === "delete-gone") {
            // The row and its chip leave at the press, git asked behind them (デザイン規約 §消す操作は先に画面から消す).
            // A tag, because git refuses no tag delete. The page holds the in-between open for the shot
            // (`RepoPage.holdGoneRows`) — a demo repository answers before a picture can be grabbed.
            page.holdGoneRows = true
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            refMenu.openSub(refTagCard)
            // Pressed as a hold, so the run ends on the write's settle with the rows still held away: the display is
            // the hold's, the reads behind it are the write's, and a picture between the two is of a page still
            // arriving.
            driver.holdToEnd(refTagDeleteItem)
            goneRowTimer.start()
        } else if (act === "delete-stood-down") {
            // The other end of `delete-gone`: waits for the in-between to be over. The screen cannot tell a row the
            // list dropped from one the window hides, so the claim is `stood=` / `chips=` read either side of the
            // wait. It catches a stand-in that never lets go (`ops::StandIn`). A tag, as in `delete-gone`.
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            refMenu.openSub(refTagCard)
            // The latch is read at the press (the hold's end), in the turn its handler ran — the row's `held` and the
            // stand-in it raised are both inside `completeHold` — and kept as the dispatch's witness (`stood`): on a
            // demo repository the stand-in can be let go inside one tick (rules-refs/app-ui.md「一瞬だけ立つ状態は
            // signal で観測して latch する」). Without it a press that took nothing away passes the wait.
            driver.beginWrite(act)
            if (refTagDeleteItem.completeHold() !== false) {
                driver.act.witness("stood", graphModel.goneChips.length > 0)
                driver.inputWent(true)
            }
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
                driver.awaitGraphGone(dropOid)
                refStashDropItem.completeHold()
            }
        } else if (act === "delete-remote" || act === "delete-remote-go" || act === "remote-refused") {
            // Named outright (`origin/feature/x`); its fold is opened so the row is under the menu. A remote's name
            // may hold `/`, so the cut is the configured one (`GitFacts.remoteOfRef`).
            remotesModel.toggleFolder(GitFacts.remoteOfRef(arg, repoTab.remoteNames))
            page.openRefMenu("remote", arg, arg, remotesModel.oidOfName(arg))
            // The row as the menu offered it, read here, ahead of the press below, and kept (`offered`): the
            // completion reads its report past the write.
            driver.act.witness("offered", "ref_menu kind=remote delete=" + refDeleteItem.code
                                          + " " + refDeleteItem.text)
            driver.reportAtCompletion(() => driver.act.witnessed("offered"))
            if (act === "delete-remote-go" || act === "remote-refused")
                driver.holdToEnd(refDeleteItem)
            // The far side keeps the branch (`--preset protected`): the refusal comes back as a report, in a bar.
            if (act === "remote-refused")
                driver.barrierNotice.start()
        } else if (act === "delete-upstream-go" || act === "delete-both-go") {
            // A local branch's menu and a row of its delete table that reaches its reading: that alone, or the pair.
            // What reaches the remote is leased to the reading's commit as the BRANCH card froze it
            // (`RefRowMenu.branchFacts`), so a card holding the wrong one is a refused write. The branch has to be
            // tracked, merged and in step (`--preset stack`'s `side`).
            page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
            driver.holdToEnd(act === "delete-both-go" ? driver.refRowMenu.deleteBothItem
                                                      : driver.refRowMenu.deleteRemoteItem)
        } else if (act === "delete-force") {
            repoTab.deleteBranch(arg, true)
        } else if (act === "delete-branch-chip") {
            // The same delete from the graph row's menu (`BranchDeleteRow`), pressed from the sampler on the
            // sidebar's terms: both entrances end at `RepoPage.deleteRow`.
            driver.beginWrite(act)
            chipDeleteTimer.start()
        } else if (act === "pull-menu") {
            // Bare: the current branch's row. A local branch's name: that branch's own row, the worktree being on
            // another. Anything else: a remote-tracking row, its fold opened as `delete-remote` does. Two remote rows
            // tell the offer apart — only the upstream this branch is measured against carries it
            // (`offers::pull_row`).
            const onRemote = arg !== "" && branchesModel.oidOfName(arg) === ""
            const name = arg === "" ? worktree.branch : arg
            if (onRemote)
                remotesModel.toggleFolder(GitFacts.remoteOfRef(arg, repoTab.remoteNames))
            const oid = onRemote ? remotesModel.oidOfName(arg) : branchesModel.oidOfName(name)
            pullMenuTimer.onRemote = onRemote
            page.openRefMenu(onRemote ? "remote" : "branch", name, name, oid)
            pullMenuTimer.start()
        } else if (act === "pull-blocked") {
            // The row out where git would turn the press down, its reason under the pointer (規約 §取り込んで合流させる).
            // `[chip][:<branch>]`: bare is the sidebar's entrance, `chip` the graph's — the same answer from each needs
            // a run through each —; a branch names the row of one the worktree is not on (the current one otherwise).
            // Forced, because a pointer cannot be put on a row from here (verify-ui §hover の絵の撮り方).
            const onChip = arg === "chip" || arg.startsWith("chip:")
            const named = onChip ? arg.slice("chip".length).replace(/^:/, "") : arg
            const onBranch = named === "" ? worktree.branch : named
            if (onChip)
                page.openRowMenu(branchesModel.oidOfName(onBranch), { "kind": "branch", "name": onBranch })
            else
                page.openRefMenu("branch", onBranch, onBranch, branchesModel.oidOfName(onBranch))
            pullBlockedTimer.onChip = onChip
            pullBlockedTimer.row = onChip ? pullCommitItem : refPullItem
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
            page.openRowMenu(branchesModel.oidOfName(worktree.branch),
                             { "kind": "branch", "name": worktree.branch })
            chipMenuTimer.start()
        } else if (act === "delete-blocked-tip") {
            // Forced through the property the real hover writes. The argument picks why the delete row is out: none
            // is the current branch, a name is one another worktree has checked out. `<branch>:remote` aims at
            // the row reaching the remote reading, out when the two names stand on different commits while the
            // local row may be pressable (デザイン規約 §左メニューの所作 の削除の表).
            const named = arg.endsWith(":remote") ? arg.slice(0, -7) : arg
            const blockedOn = named === "" ? worktree.branch : named
            page.openRefMenu("branch", blockedOn, blockedOn, branchesModel.oidOfName(blockedOn))
            acts.blockedTipRow.tipForced = true
            blockedTipTimer.start()
        } else if (act === "menu-highlight") {
            // The keyboard's road to `highlighted` — the only one that can be driven from here.
            const litOn = arg === "" ? worktree.branch : arg
            page.openRefMenu("branch", litOn, litOn, branchesModel.oidOfName(litOn))
            refMenu.currentIndex = 1
            driver.reportAtCompletion(() => "menu_highlight index=" + refMenu.currentIndex)
        } else if (act === "delete-branch-early" || act === "delete-branch-early-far") {
            // The early answer dresses the delete row before any click; the menu goes up from the sampler
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
            // spawns git. The row under HEAD's, counted as `commit-menu` counts: the rows above HEAD (the worktree,
            // a stash) open no such menu. The commit asked for is the dispatch's witness (`createTagOid`).
            const tagOid = driver.act.witness("createTagOid", graphModel.oidAt(graphModel.rowOf(worktree.headOid) + 1))
            page.openRowMenu(tagOid)
            commitMenu.openSub(commitTagCard)
            tagHereCommitItem.triggered()
            // `<name>:box` stops at the box the row opened — the chip column's name box, as a tag's (`GraphRowChips`).
            // The verb owns its completion (the named one ends on TAGS), so the box completes from an operation of
            // its own.
            if (arg.endsWith(":box")) {
                graphPane.view.namingText = arg.slice(0, -4)
                createTagBoxTimer.start()
            } else {
                graphPane.view.namingSubmitted(tagOid, arg, "tag")
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
    /// Whether the delete went out, said either way — the page's own answer (`RepoPage.deleteRowAsked`); latching on a
    /// press the page dropped would wait on a write nobody made. On false the run completes, which fails on this line
    /// where it is the claim, and elsewhere on the line after git's answer never coming (`verify/verbs/nav.rs`,
    /// `graph.rs`). Read at the press: `busy=` is the moment after it; `offered=` / `blocked=` say whether a reader had
    /// the press at all.
    function deleteRowLine(row) {
        return "delete_row asked=" + page.deleteRowAsked
               + " busy=" + repoTab.busyCount
               + " offered=" + row.offered
               + " blocked=" + (row.blockedReason !== "")
    }
    // The delete asked of the left pane's menu. The menu goes up and the row is pressed here, not in `run`: a menu
    // opened while a write runs greys its delete row (`AppMenu.runningWhy`) and the page drops the request
    // (`RepoPage.deleteRow`).
    PageStage {
        id: deleteRowTimer
        name: "delete_row"
        order: ["pressed"]
        /// After the press: `hold` runs the held `-D` git's refusal leaves, `refused` reads the row it turned into,
        /// and `menu` reads the menu once git has answered the plain delete.
        property string afterPress: "menu"
        // The `delete_row` line, read at the press, is its witness (`said`): the run's claim, said by whichever
        // operation completes it. `open` is whether the menu stood as the delete was pressed.
        onTriggered: {
            const op = deleteRowTimer
            // The precondition is read here and nowhere else (app-ui.md §UI 自動化).
            if (!Awaited.all("delete_row_ready", { "ready": driver.menusLive() }))
                return
            if (!op.input("pressed", () => {
                    const arg = Harness.autoActArg
                    const oid = branchesModel.oidOfName(arg)
                    page.openRefMenu("branch", arg, arg, oid)
                    op.witness("open", refMenu.visible)
                    page.deleteRow("branch", arg, arg, oid)
                    op.witness("said", acts.deleteRowLine(refDeleteItem))
                    if (page.deleteRowAsked)
                        driver.inputWent(true)
                }))
                return
            // The page dropped the press: nothing will be written, and the line read at the press is the run's end.
            if (!page.deleteRowAsked) {
                op.complete({ "name": "delete_row_dropped", "when": () => ({ "dropped": !page.deleteRowAsked }),
                              "report": () => op.witnessed("said") })
                return
            }
            // `-go` completes on the write barrier behind the `-D` it presses, so its claim — the row as that press
            // found it (`forceDeleteTimer`'s `row`) — rides on the line handed here.
            driver.reportAtCompletion(() => op.afterPress === "hold"
                                             ? [deleteRowTimer.witnessed("said"), forceDeleteTimer.witnessed("row")]
                                             : deleteRowTimer.witnessed("said"))
            op.stop()
            if (op.afterPress === "hold")
                forceDeleteTimer.start()
            else if (op.afterPress === "refused")
                refusedRowTimer.start()
            else if (op.afterPress === "menu")
                plainDeleteTimer.start()
        }
    }
    // The plain delete, judged on the menu: the menu stays up for a refusal (`BranchDeleteRow`'s plain delete,
    // `onHeld`), so a delete git takes has to take it down — through the answer `RefRowMenu` binds on its card
    // (`RefBranchMenu.landedDelete`), the graph row's being `chipDeleteTimer`'s. A closed menu frames like one nobody
    // opened, so `was=` says it stood at the press. `code=` tells a landing from a refusal, which leaves the row
    // wearing `branch -D`.
    PageStage {
        id: plainDeleteTimer
        name: "plain_delete"
        onTriggered: plainDeleteTimer.complete({
            "name": "plain_delete_landed",
            "when": () => ({ "written": driver.wroteAndSettled() }),
            "report": () => driver.verbReport().concat(["ref_delete branch=" + Harness.autoActArg
                                                        + " row=" + branchesModel.rowOfName(Harness.autoActArg)
                                                        + " code=" + refDeleteItem.code
                                                        + " was=" + deleteRowTimer.witnessed("open")
                                                        + " menu=" + refMenu.opened]) })
    }
    // git's refusal has to come back before the row it turns into `-D` (the warning's three presses) can be pressed
    // through — or photographed. The plain delete is pressed too, so a count is no sign of the refusal; this one is.
    // `forced=` is the refusal as the card took it (`RefBranchMenu.forceDeleteBranch`, off the `refusedDelete`
    // `RefRowMenu` binds on it): the row wears `-D` from the early check either way, so nothing else tells a refusal
    // that reached the card from one the left menu dropped.
    // The menu went up before that delete, so the write greys none of its rows (`AppMenu.waiting`); a row out of
    // reach still takes no press (`AppMenuItem.completeHold`), and the sampler presses again on its next beat.
    PageStage {
        id: forceDeleteTimer
        name: "force_delete"
        order: ["refused", "pressed"]
        onTriggered: {
            const op = forceDeleteTimer
            // The refusal in hand is a stage: a press that did not go has armed the watch again (`beginWrite`), so
            // the first delete no longer reads as written through.
            if (op.at("start")) {
                if (!Awaited.all("force_delete_refused", {
                        "written": driver.wroteAndSettled(),
                        "warned": refDeleteItem.holdPresses >= Metrics.holdPressesWarned }))
                    return
                op.advance("refused")
            }
            // The row as it stood when pressed, read before the press — its handler runs inside it (`holdToEnd`) —
            // and answered by the press that went in, as the stage's line and the run's claim (`row`).
            if (!op.input("pressed", () => {
                    const said = op.witness("row", "ref_menu forced="
                                                   + (refBranchCard.forceDeleteBranch === Harness.autoActArg)
                                                   + " delete=" + refDeleteItem.text + " note=" + refDeleteItem.note)
                    return driver.holdToEnd(refDeleteItem) ? said : false
                }, (said) => said))
                return
            op.stop()
            writeBarrier.start()
        }
    }
    // PGG_AUTO_ACT=tag-chip-menu: once this run's fetch is read through, the graph row's menu on that tag's chip, its
    // TAG card open.
    PageStage {
        id: tagChipTimer
        name: "tag_chip"
        order: ["opened"]
        property string tag: ""
        property int row: 0
        onTriggered: {
            const oid = graphModel.oidAt(tagChipTimer.row)
            if (!Awaited.all("tag_chip_ready", { "ready": !(!driver.wroteAndSettled() || !driver.menusLive()),
                                                  "page": PageSettled.settled(page),
                                                  "oid": oid !== "" }))
                return
            if (!tagChipTimer.input("opened", () => {
                    page.openRowMenu(oid, { "kind": "tag", "name": tagChipTimer.tag })
                    commitMenu.openSub(commitTagCard)
                }))
                return
            tagChipTimer.complete({ "when": () => ({}), "report": () => acts.reachWords(commitTagCard) })
        }
    }
    // PGG_AUTO_ACT=create-tag `<name>:box`: the box the row opened, holding the name, read where the dispatch left it.
    PageStage {
        id: createTagBoxTimer
        name: "create_tag_box"
        onTriggered: createTagBoxTimer.complete({
            "when": () => ({}),
            "report": () => "create_tag box=" + (graphPane.view.namingOid !== "")
                            + " mode=" + graphPane.view.namingMode })
    }
    // Waits on the window's tip itself (it opens after `tipDelayMs`): read sooner, the line says false while the shot
    // holds it.
    PageStage {
        id: blockedTipTimer
        name: "blocked_tip"
        onTriggered: blockedTipTimer.complete({
            "name": "blocked_tip_ready",
            "when": () => ({ "ready": !(acts.blockedTipRow === null || !acts.blockedTipRow.tip.shown) }),
            "report": () => "delete_blocked code=" + acts.blockedTipRow.code
                + " tip=" + acts.blockedTipRow.tip.shown
                + " aside=" + driver.tipAside(Tips.box)
                // The name the holding worktree goes by (`RefBranchMenu.holderLeaf`), empty on other rows — the
                // sentence is tst_branchcard's to judge.
                + " holder=" + refBranchCard.holderLeaf
                + " reason=" + acts.blockedTipRow.blockedReason })
    }
    /// The greyed `pull` row and its tip. The line claims what the picture cannot: offered, out rather than pale
    /// (`AppMenuItem.blocked`, which decides colour and press), and the sentence about the divergence.
    PageStage {
        id: pullBlockedTimer
        name: "pull_blocked"
        /// Which entrance this run came in by, and the row it left standing there.
        property bool onChip: false
        property var row: null
        onTriggered: pullBlockedTimer.complete({
            "name": "pull_blocked_ready",
            "when": () => ({ "ready": !(!(pullBlockedTimer.onChip ? commitMenu : refMenu).opened
                                        || !pullBlockedTimer.row.tip.shown) }),
            "report": () => "pull_blocked where=" + (pullBlockedTimer.onChip ? "chip" : "row")
                            + " offered=" + pullBlockedTimer.row.offered
                            + " blocked=" + pullBlockedTimer.row.blocked
                            + " tip=" + pullBlockedTimer.row.tip.shown
                            + " says=" + pullBlockedTimer.row.blockedReason })
    }
    /// The `pull` row as the card opens. Whether it was offered is the claim (`offers::pull_row`): a row wired to
    /// nothing frames like one that reaches git.
    PageStage {
        id: pullMenuTimer
        name: "pull_menu"
        /// Whether the card went up on a remote-tracking row — the half that names the branch it lands in.
        property bool onRemote: false
        // `sentence=` / `into=`: every row names the branch that moves (デザイン規約 §取り込んで合流させる) — on the
        // upstream's row, one the row does not name. `runs=`: what the press would run — `here` (`git pull`) or
        // `aside` (the fast-forward). The name last: it is free text.
        onTriggered: pullMenuTimer.complete({
            "name": "pull_menu_ready",
            "when": () => ({ "ready": refMenu.opened }),
            "report": () => "pull_menu open=" + refMenu.opened
                            + " kind=" + (pullMenuTimer.onRemote ? "remote" : "branch")
                            + " pull=" + refPullItem.offered
                            + " code=" + refPullItem.code
                            + " sentence=" + (refPullItem.refSentence !== "")
                            + " runs=" + driver.refRowMenu.pullRuns
                            + " into=" + (refPullItem.offered ? refPullItem.refName : "") })
    }
    PageStage {
        id: chipMenuTimer
        name: "chip_menu"
        // `ref=` must stay down: a second menu on the chip's side is the split this entrance is joined to prevent.
        onTriggered: chipMenuTimer.complete({
            "name": "chip_menu_ready",
            "when": () => ({ "ready": !(!refMenu.opened && !commitMenu.opened) }),
            "report": () => "chip_menu ref=" + refMenu.opened
                            + " commit=" + commitMenu.opened
                            + " switch=" + switchCommitItem.offered
                            + " delete=" + commitDeleteItem.code
                            + " " + commitDeleteItem.text })
    }
    // Waits for the name in TAGS, not the write barrier, which photographs the sidebar from before
    // (rules-refs/app-ui.md「その書き込みが無効化した読み直しより先に来る」). `at=` is against the commit the dispatch
    // asked for (`createTagOid`).
    PageStage {
        id: createTagTimer
        name: "create_tag"
        onTriggered: createTagTimer.complete({
            "name": "create_tag_ready",
            "when": () => ({ "ready": !(tagsModel.oidOfName(Harness.autoActArg) === "" || repoTab.busyCount !== 0) }),
            "report": () => "create_tag tag=" + Harness.autoActArg
                            + " row=" + tagsModel.rowOfName(Harness.autoActArg)
                            + " total=" + tagsModel.total
                            + " at=" + (tagsModel.oidOfName(Harness.autoActArg)
                                        === driver.act.witnessed("createTagOid")) })
    }
    /// Which row a run holds a tooltip open on: `delete-blocked-tip`'s row of the delete table (`<branch>:remote` the
    /// one reaching the remote reading), or the TAG card's `push --delete` (`tag-menu …:tip`).
    readonly property var blockedTipRow: {
        if (Harness.autoAct !== "delete-blocked-tip")
            return refRemoteTagDeleteItem
        if (!Harness.autoActArg.endsWith(":remote"))
            return refDeleteItem
        return driver.refRowMenu ? driver.refRowMenu.deleteRemoteItem : null
    }

    // The tag menu's rows. The push row's whole shape (chip, hold, colour) is decided by what a remote was last heard
    // to carry (`RefRowMenu`), and `push` / `push --force` differ by five glyphs in an otherwise identical card — so
    // the report spells it out. `wants` is what the run waits for: that reading lands well after the fetch it rides
    // out with (rules-refs/core.md「タグのリモート状態は `ls-remote --tags` で訊くしかない」), so waiting on the fetch
    // alone photographs the plain row as the forced one.
    PageStage {
        id: tagMenuTimer
        name: "tag_menu"
        order: ["opened"]
        /// The tag, and what must be known before the card is worth photographing: `drift` (a remote has the name on
        /// another commit) or `remote` (a remote has it at all). Either fetches first.
        property string tag: ""
        property string wants: ""
        /// Which row this run presses, empty for the ones that only stand the card up.
        property string press: ""
        /// `:tip` last: the card stood up, the `push --delete` row's reason is forced out, as `delete-blocked-tip` does
        /// on a branch's delete table — the one place that row says why it is out.
        property bool tip: false
        function begin(arg, pressing) {
            const parts = arg.split(":")
            tagMenuTimer.tag = parts[0]
            tagMenuTimer.tip = parts.length > 1 && parts[parts.length - 1] === "tip"
            tagMenuTimer.wants = parts.length > 1 && parts[1] !== "tip" ? parts[1] : ""
            tagMenuTimer.press = pressing
            // A run that presses presses one row of the card it stood up.
            tagMenuTimer.order = pressing === "" ? ["opened"] : ["opened", "pressed"]
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
        /// Every row the menu grew, in one line: a card missing a row, or a row greyed for drift, frames like one never
        /// offered or simply out (デザイン規約 §左メニューの所作 の削除の表). The order is the judging order — `must_say`
        /// matches one stretch of the line: the sides and the rows they decide first, the push row's shape after
        /// (`verify/verbs/remote.rs`). The offer is asked first: `blocked` is a binding and answers whether or not the
        /// row is drawn. Then where the card's rows reach.
        function menuLines() {
            const outRows = [refRemoteTagDeleteItem.offered && refRemoteTagDeleteItem.blocked ? "remote" : "",
                             refTagBothDeleteItem.offered && refTagBothDeleteItem.blocked ? "both" : ""]
                            .filter(w => w !== "")
            return ["tag_menu tag=" + tagMenuTimer.tag
                    + " sides=" + tagsModel.tagSides(tagMenuTimer.tag)
                    + " local_del=" + refTagDeleteItem.offered
                    + " remote_del=" + refRemoteTagDeleteItem.offered
                    + " both_del=" + refTagBothDeleteItem.offered
                    // The remote-reaching rows standing but out; `none` is a claim of its own.
                    + " blocked=" + (outRows.length === 0 ? "none" : outRows.join(","))
                    + " tag_here=" + refTagHereItem.offered
                    + " push=" + refPushTagItem.offered
                    + " code=" + refPushTagItem.code
                    + " held=" + (refPushTagItem.holdPresses > 0)
                    // The model's reading, as `ready()` polls: this family reaches only the driver's properties.
                    + " lease=" + tagsModel.remoteTagDrift(tagMenuTimer.tag, repoTab.defaultRemote)
                    + " text=" + refPushTagItem.text,
                    acts.reachWords(refTagCard)]
        }
        onTriggered: {
            const op = tagMenuTimer
            if (op.at("start")) {
                if (!Awaited.all("tag_menu_ready", { "ready": !(!op.ready() || !driver.menusLive()) }))
                    return
                if (!op.input("opened", () => {
                        page.openRefMenu("tag", op.tag, op.tag, tagsModel.oidOfName(op.tag), true)
                        // The rows are one card in (デザイン規約 §メニュー の入れ子); opened so the picture shows them.
                        refMenu.openSub(refTagCard)
                        // `:tip`: forced through the property the real hover writes; the branch card's wait answers
                        // for it (`blockedTipRow`).
                        if (op.press === "" && op.tip)
                            acts.blockedTipRow.tipForced = true
                    }))
                    return
                if (op.press === "" && !op.tip) {
                    op.complete({ "when": () => ({}), "report": () => op.menuLines() })
                    return
                }
                // The card as it opened, before anything is pressed on it. The push's run claims it: kept (`card`)
                // for the write barrier that completes it.
                op.say(() => op.menuLines())
                if (op.press === "push") {
                    op.witness("card", op.menuLines())
                    driver.reportAtCompletion(() => tagMenuTimer.witnessed("card"))
                }
                if (op.press === "") {
                    op.stop()
                    blockedTipTimer.start()
                    return
                }
            }
            // A row out of reach takes no press and answers false (`holdToEnd`): pressed again on the next beat.
            if (op.press === "remote-refuse") {
                // Nothing moves, so there is no reading of this name to wait for: what the run waits on is the bar.
                if (!op.input("pressed", () => driver.holdToEnd(refRemoteTagDeleteItem)))
                    return
                op.stop()
                driver.barrierNotice.start()
                return
            }
            if (op.press === "remote-delete" || op.press === "both-delete") {
                // Judged on the sidebar after: waits for this name's reading to change, as `createTagTimer` does — what
                // it is held against is read at the press, and kept where the press went in (`was`, `errorBefore`,
                // `noticeBefore`).
                if (!op.input("pressed", () => {
                        const was = tagsModel.tagSides(op.tag)
                        const errorBefore = repoTab.lastError
                        const noticeBefore = page.noticeCard.open ? page.noticeCard.label : ""
                        if (!driver.holdToEnd(op.press === "remote-delete" ? refRemoteTagDeleteItem
                                                                           : refTagBothDeleteItem))
                            return false
                        op.witness("was", was)
                        op.witness("errorBefore", errorBefore)
                        op.witness("noticeBefore", noticeBefore)
                        return true
                    }))
                    return
                op.stop()
                tagGoneTimer.start()
                return
            }
            // The forced push is held, and the barrier is armed at the hold's end (`holdToEnd`): this run's own fetch
            // answers during the hold, and a barrier armed here would pass on it with nothing pushed.
            if (!op.input("pressed", () => {
                    if (refPushTagItem.holdPresses > 0)
                        return driver.holdToEnd(refPushTagItem)
                    return driver.pressWrite("push-tag", () => {
                        refPushTagItem.triggered()
                        return true
                    })
                }))
                return
            op.stop()
            writeBarrier.start()
        }
    }
    // A tag delete, judged on the sidebar it leaves. The edge is the name's own reading: gone from both sides it
    // answers nothing, gone from the remote alone it drops back to `here` (no row goes).
    // The tag is the menu's (`tagMenuTimer.tag`), and what it is held against is the press's witnesses: the name's
    // reading (`was`), and what the page already said (`errorBefore`, `noticeBefore`) — only an answer that moved
    // these is this write's refusal.
    PageStage {
        id: tagGoneTimer
        name: "tag_gone"
        /// Whether this write came back refused — a bar the press raised (a lease the remote had left), or an error.
        function refused() {
            return (page.noticeCard.open && page.noticeCard.label !== tagMenuTimer.witnessed("noticeBefore"))
                || repoTab.lastError !== tagMenuTimer.witnessed("errorBefore")
        }
        function moved() {
            return tagsModel.tagSides(tagMenuTimer.tag) !== tagMenuTimer.witnessed("was")
        }
        /// The reading moved, or the write came back refused with nothing moved.
        function answered() {
            return { "ready": driver.wroteAndSettled(),
                     "page": PageSettled.settled(page),
                     "answered": tagGoneTimer.moved() || tagGoneTimer.refused() }
        }
        function keptLine() {
            return "tag_kept tag=" + tagMenuTimer.tag + " sides=" + tagsModel.tagSides(tagMenuTimer.tag)
        }
        onTriggered: {
            const op = tagGoneTimer
            if (!Awaited.all("tag_gone_ready", op.answered()))
                return
            if (op.moved()) {
                op.complete({ "name": "tag_gone_ready", "when": () => op.answered(),
                              "report": () => "tag_gone tag=" + tagMenuTimer.tag
                                              + " was=" + tagMenuTimer.witnessed("was")
                                              + " sides=" + tagsModel.tagSides(tagMenuTimer.tag)
                                              + " row=" + tagsModel.rowOfName(tagMenuTimer.tag)
                                              + " total=" + tagsModel.total })
                return
            }
            // Refused, nothing moves: the answer is what is shot — a bar it raised, read by the bar's barrier.
            if (page.noticeCard.open) {
                driver.reportAtCompletion(() => tagGoneTimer.keptLine())
                op.stop()
                driver.barrierNotice.start()
                return
            }
            op.complete({ "name": "tag_gone_ready", "when": () => op.answered(), "report": () => op.keptLine() })
        }
    }
    // Waits on the early answer: nothing here writes.
    PageStage {
        id: earlyDeleteTimer
        name: "delete_early"
        order: ["menu_up"]
        onTriggered: {
            const op = earlyDeleteTimer
            const arg = Harness.autoActArg
            if (op.at("start")) {
                // The precondition is read here and nowhere else (app-ui.md §UI 自動化): the delete row is live only
                // on a menu raised while nothing runs (`AppMenu.runningWhy`), and the BRANCH card it reads works its
                // answers out once as the menu opens — put up a tick early, it asks nobody and the run waits out the
                // watchdog.
                if (!Awaited.all("delete_early_live", { "live": driver.menusLive() }))
                    return
                if (!op.input("menu_up", () => {
                        page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
                    }))
                    return
                if (refBranchCard.deleteAsked)
                    return
                // Not landed (the name has no commit, or its delete is out), so nothing will come to latch on. Reported
                // as facts, not the reason's sentence.
                op.complete({
                    "name": "delete_early_unasked",
                    "when": () => ({ "unasked": !refBranchCard.deleteAsked }),
                    "report": () => "delete_early asked=false"
                                    + " oid=" + (branchesModel.oidOfName(Harness.autoActArg) !== "")
                                    + " offered=" + refDeleteItem.offered
                                    + " blocked=" + (refDeleteItem.blockedReason !== "") })
                return
            }
            // `code` cannot tell "not answered yet" from "answered merged" — both wear `branch --delete`. The answer is
            // in hand when the graph gave it as the card opened (`RefBranchMenu.deleteAnswered`) or git echoes the
            // branch asked about, which the asking clears before the question goes out (app-ui.md §UI 自動化).
            // Both sides say the same three words; `unknown` is git's alone, what a run whose reads fell over says.
            op.complete({
                "name": "delete_early_answered",
                "when": () => ({ "answered": refBranchCard.deleteAnswered
                                             || repoTab.branchDeleteAsked === Harness.autoActArg }),
                "report": () => {
                    const fromRows = refBranchCard.deleteAnswered
                    return "delete_early asked=true"
                           + " from=" + (fromRows ? "rows" : "git")
                           + " merged=" + (fromRows ? (refBranchCard.deleteMerged ? "yes" : "no")
                                                    : repoTab.branchDeleteMerged)
                           + " code=" + refDeleteItem.code
                           + " presses=" + refDeleteItem.holdPresses
                           + " note=" + refDeleteItem.note
                } })
        }
    }
    // The delete asked from the graph row. The subject is the menu going: the menu stays up for a refusal
    // (`BranchDeleteRow`'s plain delete, `onHeld`), so a delete git takes has to take it down — and a closed menu
    // frames like one nobody opened, so `was=` says it stood at the press. `code=` tells a landing from a refusal,
    // which leaves the row wearing `branch -D`.
    PageStage {
        id: chipDeleteTimer
        name: "chip_delete"
        order: ["pressed"]
        // The `delete_row` line, read at the press, is its witness (`said`); `open` is whether the menu stood then.
        onTriggered: {
            const op = chipDeleteTimer
            if (op.at("start")) {
                // The precondition is read here and nowhere else (app-ui.md §UI 自動化): a menu raised while a write
                // runs greys its rows, and the page drops the request.
                if (!Awaited.all("chip_delete_live", { "live": driver.menusLive() }))
                    return
                if (!op.input("pressed", () => {
                        const arg = Harness.autoActArg
                        page.openRowMenu(branchesModel.oidOfName(arg), { "kind": "branch", "name": arg })
                        op.witness("open", commitMenu.visible)
                        // The row's own presses: the plain delete's two, which keep the menu up (`BranchDeleteRow`'s
                        // `onHeld`).
                        commitDeleteItem.completeHold()
                        op.witness("said", acts.deleteRowLine(commitDeleteItem))
                        if (page.deleteRowAsked)
                            driver.inputWent(true)
                    }))
                    return
                // The page dropped the press: nothing will be written, and the line read at the press is the run's end.
                if (!page.deleteRowAsked) {
                    op.complete({ "name": "chip_delete_dropped", "when": () => ({ "dropped": !page.deleteRowAsked }),
                                  "report": () => op.witnessed("said") })
                    return
                }
                op.say(() => op.witnessed("said"))
                return
            }
            op.complete({
                "name": "chip_delete_landed",
                "when": () => ({ "written": driver.wroteAndSettled() }),
                "report": () => "chip_delete branch=" + Harness.autoActArg
                                + " row=" + branchesModel.rowOfName(Harness.autoActArg)
                                + " code=" + commitDeleteItem.code
                                + " was=" + op.witnessed("open")
                                + " menu=" + commitMenu.opened })
        }
    }
    // The row git's refusal turned the plain delete into, said beside the press's own line (`deleteRowTimer`).
    PageStage {
        id: refusedRowTimer
        name: "refused_row"
        onTriggered: refusedRowTimer.complete({
            "name": "refused_row_ready",
            "when": () => ({ "ready": driver.wroteAndSettled(), "page": PageSettled.settled(page) }),
            "report": () => driver.verbReport().concat(["ref_menu delete=" + refDeleteItem.code
                                                        + " " + refDeleteItem.text
                                                        + " note=" + refDeleteItem.note]) })
    }
    // The row taken away at the press, waited off the list, and the write behind it settled: the page holds the
    // rows away past the answer (`holdGoneRows`), so the in-between is what the picture is of.
    PageStage {
        id: goneRowTimer
        name: "gone_row"
        onTriggered: goneRowTimer.complete({
            "name": "gone_row_ready",
            "when": () => ({ "ready": !(tagsModel.rowOfName(Harness.autoActArg) >= 0),
                             "written": driver.wroteAndSettled(),
                             "page": PageSettled.settled(page) }),
            "report": () => "gone_row tag=" + Harness.autoActArg
                            + " row=" + tagsModel.rowOfName(Harness.autoActArg)
                            + " total=" + tagsModel.total
                            + " chips=" + (graphModel.goneChips.length > 0) })
    }
    // …and the row let go of again, the half no picture holds. Waited on the gone set: the write answers before the
    // listing that takes the row away for good is asked for, so a barrier would report the middle as the end.
    // `stood=` is whether the row was stood in for, read off the gone set at the press (the dispatch's `stood`) — the
    // slot fills it before the press returns, and sampling could miss the window.
    PageStage {
        id: stoodDownTimer
        name: "stood_down"
        // The rows stay off screen until a listing taken after the write is drawn; `let_go` is it arriving
        // (`ops::StandIn`).
        onTriggered: stoodDownTimer.complete({
            "name": "stood_down_ready",
            "when": () => ({ "ready": driver.wroteAndSettled(),
                             "page": PageSettled.settled(page),
                             "let_go": graphModel.goneChips.length === 0 }),
            "report": () => "stood_down tag=" + Harness.autoActArg
                            + " stood=" + driver.act.witnessed("stood")
                            + " row=" + tagsModel.rowOfName(Harness.autoActArg)
                            + " total=" + tagsModel.total
                            + " chips=" + (graphModel.goneChips.length > 0) })
    }
}

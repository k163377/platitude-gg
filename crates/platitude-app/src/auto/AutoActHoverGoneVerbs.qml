pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// What a hover display does when what it stands for changes under it, read stage by stage through the paths the change
/// comes in by — the app's own writes landing and the lists rebuilt from them: a chip's list over a row rewritten or a
/// chip renamed (`list-reread`), the line-ending card over a warning that changed (`eol-change`), the worktree's rows
/// read again under the hand (`wip-reread`), and a row's tip over rows that moved under a still hand (`tip-slide`). A
/// run stops at the moment its argument names (verify-ui スキル §board).
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var wipPane: driver.wipPane
    readonly property var diffPane: driver.diffPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var detailsModel: driver.detailsModel
    readonly property var refList: driver.refList

    function run(act, arg) {
        const cut = arg.indexOf(":")
        const head = cut < 0 ? arg : arg.substring(0, cut)
        const tail = cut < 0 ? "" : arg.substring(cut + 1)
        if (act === "list-reread") {
            // `<row|pin>:<how>` — the list out of HEAD's row's chip (`row`) or of the current branch's stand-in's
            // (`pin`), then: `commit` (the row rewritten to the commit this run makes, `--preset dirty`), `amend`
            // (HEAD rewritten in place, the stand-in's), `branch` (a branch made on HEAD, the hand inside the list),
            // `branch-chip` (the same, the hand still on the chip), `pending` (the list asked for under a menu, a
            // branch made meanwhile, the menu gone).
            listRereadTimer.on = head === "" ? "row" : head
            listRereadTimer.how = tail === "" ? "branch" : tail
            listRereadTimer.step = 0
            listRereadTimer.namesBefore = -1
            listRereadTimer.headBefore = ""
            listRereadTimer.start()
        } else if (act === "eol-change") {
            // `<unstaged|staged>` — the card on `src/flipped.kt` in that bucket, held by the hand inside it, and a line
            // of it staged under it: the warning stays and says another count (`--preset eol`).
            page.showWip()
            eolChangeTimer.bucket = head === "" ? "unstaged" : head
            eolChangeTimer.step = 0
            eolChangeTimer.start()
        } else if (act === "wip-reread") {
            // `<same|above|inside|staged|gone|fixed>` — the pane's hand on a marked row of the worktree's lists with
            // its card out, and a file written from outside so the status reads the rows again (`--preset eol`); at
            // `wipRereadTimer`.
            page.showWip()
            wipRereadTimer.how = head === "" ? "same" : head
            wipRereadTimer.step = 0
            wipRereadTimer.reads = 0
            wipRereadTimer.left = ""
            wipRereadTimer.start()
        } else if (act === "tip-slide") {
            // `files[:<stage>]` — a changed file's tip in the commit's list (`--preset long`), the hand still on its
            // row, and a folder above it shut so the rows slide under the hand; stages at `tipSlideTimer`.
            tipSlideTimer.stage = tail === "" ? "moved" : tail
            tipSlideTimer.step = 0
            tipSlideTimer.nudges = 0
            tipSlideTimer.stood = false
            tipSlideTimer.start()
        } else {
            return false
        }
        return true
    }

    // PGG_AUTO_ACT=list-reread. The list out of a chip with the hand inside it (or still on the chip), and what it
    // stands for changed by one of this tab's own writes landing — the path a change from outside comes in by as well,
    // the graph read again: a row rewritten to another commit, the stand-in standing for another, the chip's names
    // changed. None of it is the reader's doing (デザイン規約 §hover のツールチップ「外乱では閉じない」): the list stays
    // and says the names as they are now — a branch made on its commit (`RowHoverHost.chipChanged`) — and goes with a
    // row whose commit lost its names or left the graph, and with the stand-in standing for another commit
    // (`RowHoverHost.reseat` / `rowGone`). A list asked for under a menu opens on the names as they are
    // (`RowHoverHost.openRefList` reads them off the chip).
    SampleTimer {
        id: listRereadTimer
        property string on: "row"
        property string how: "branch"
        property int step: 0
        property int namesBefore: -1
        property string headBefore: ""
        /// The chip the list is asked from, and where a hand on it stands.
        function anchor() {
            if (listRereadTimer.on === "pin")
                return graphPane.headPin.visible ? graphPane.headPin.chipItem : null
            const row = graphModel.headRow < 0 ? null : graphPane.view.itemAtIndex(graphModel.headRow)
            return row !== null && row.labelRecords.length > 0 ? row : null
        }
        function handOn(item) {
            if (listRereadTimer.on === "pin") {
                const front = item.chipItem
                return front.mapToItem(null, front.width / 2, front.height / 2)
            }
            return item.mapToItem(null, item.labelsW / 2, item.height / 2)
        }
        /// A branch made on HEAD's commit, through the driver's door for a write.
        function pressBranch() {
            return driver.pressWrite("branch", () => {
                repoTab.createBranch("list-reread", graphModel.headRowOid, false)
                return true
            })
        }
        function done(stage) {
            listRereadTimer.stop()
            Harness.report("list_reread on=" + listRereadTimer.on + " how=" + listRereadTimer.how + " stage=" + stage
                           + " list=" + refList.opened
                           + " names=" + (refList.opened ? refList.records.length - listRereadTimer.namesBefore : "-")
                           + " moved=" + (graphModel.headRowOid !== listRereadTimer.headBefore))
            driver.complete()
        }
        onTriggered: {
            if (listRereadTimer.step === 0) {
                const read = !graphModel.loading && graphModel.finishCount > 0 && graphModel.headRow >= 0
                // The stand-in rides the edge HEAD's row went out of: the view sent to the end puts it at the top,
                // sent each beat until it stands.
                if (read && listRereadTimer.on === "pin" && !graphPane.headPin.visible)
                    graphPane.view.positionViewAtEnd()
                const item = read ? listRereadTimer.anchor() : null
                if (!Awaited.all("list_reread_chip", { "read": read, "chip": item !== null }))
                    return
                listRereadTimer.headBefore = graphModel.headRowOid
                if (listRereadTimer.how === "pending") {
                    // A menu of the page's up first, on another row: the hand on the chip is an ask kept behind it.
                    page.openRowMenu(graphModel.oidAt(graphModel.rowTotal - 1))
                }
                graphPane.hand.stir(listRereadTimer.handOn(item))
                listRereadTimer.step = 1
                return
            }
            if (listRereadTimer.step === 1) {
                if (listRereadTimer.how === "pending") {
                    if (!Awaited.all("list_reread_held_back", { "menu": page.menuStanding, "list": !refList.opened }))
                        return
                    listRereadTimer.namesBefore = graphModel.headLabels.length
                    if (!Awaited.all("list_reread_branch", { "pressed": listRereadTimer.pressBranch() }))
                        return
                    listRereadTimer.step = 2
                    return
                }
                if (!Awaited.all("list_reread_list", { "list": refList.opened }))
                    return
                listRereadTimer.namesBefore = refList.records.length
                if (listRereadTimer.how !== "branch-chip") {
                    // Into the list: the chip let go, the card's own hold on (`AppCard.contentPointed`).
                    graphPane.hand.stir(Qt.point(-1, -1))
                    refList.contentPointed = true
                }
                if (listRereadTimer.how === "commit") {
                    wipPane.setMessage("feat: something", "")
                } else if (listRereadTimer.how === "amend") {
                    wipPane.setAmendChecked(true)
                    page.amending = true
                    wipPane.setMessage("feat: said again", "")
                }
                listRereadTimer.step = 2
                return
            }
            if (listRereadTimer.step === 2 && listRereadTimer.how !== "pending") {
                // Through the driver's one door for a write (`AutoActDriver.pressWrite`), asked each beat until it
                // takes: the commit button once it is live, the amend, the branch.
                const pressed = listRereadTimer.how === "commit"
                              ? driver.pressWrite("commit", () => wipPane.pressCommit())
                              : listRereadTimer.how === "amend"
                              ? driver.pressWrite("amend", () => { page.commitNow(); return true })
                              : listRereadTimer.pressBranch()
                if (!Awaited.all("list_reread_write", { "pressed": pressed }))
                    return
                listRereadTimer.step = 3
                return
            }
            if (listRereadTimer.how === "pending") {
                if (listRereadTimer.step === 2) {
                    const named = graphModel.headLabels.length > listRereadTimer.namesBefore
                    if (!Awaited.all("list_reread_named", { "names": named }))
                        return
                    driver.commitMenu.close()
                    // The names the list opens with are counted from what the chip had before the branch.
                    listRereadTimer.namesBefore = graphModel.headLabels.length - 1
                    listRereadTimer.step = 3
                    return
                }
                if (!Awaited.all("list_reread_back", { "menu": !page.menuStanding, "list": refList.opened }))
                    return
                listRereadTimer.done("after")
                return
            }
            // Landed — the commit on HEAD's row, or the branch on HEAD's chip — and the list as it ends: gone with the
            // row's commit that lost its names or left the graph, and with the stand-in standing for HEAD's new commit;
            // still out with a branch made on its commit, saying the names as they are now.
            const rewritten = listRereadTimer.how === "commit" || listRereadTimer.how === "amend"
            if (!Awaited.all("list_reread_landed", {
                    "landed": rewritten ? graphModel.headRowOid !== listRereadTimer.headBefore
                                        : graphModel.headLabels.length > listRereadTimer.namesBefore,
                    "list": rewritten ? !refList.opened
                                      : refList.opened && refList.records.length > listRereadTimer.namesBefore,
                    "page": PageSettled.settled(page)
                }))
                return
            listRereadTimer.done("after")
        }
    }

    // PGG_AUTO_ACT=eol-change. The line-ending card held by the hand inside it, and the file under it changed so that
    // it stays marked in that bucket and its mark says another count — the card says the new sentence, or goes; never
    // the old one (`WipPane.weighEol`). `unstaged`: `src/mixed.kt` (one added line with the other ending), saved over
    // from outside with a second such line — the app hears it by reading the tree again (`RepoTab.pollNow`, as the poll
    // does). `staged`: `src/flipped.kt` (every line's ending changed in the tree), one line's new ending staged, the
    // card asked for, then a second line's staged under it through the diff's own marks (`DiffPane.stageLine`) — a line
    // whose ending changed is two lines of the diff, the old one taken out and the new one put in: two strokes a line.
    SampleTimer {
        id: eolChangeTimer
        property string bucket: "unstaged"
        property int step: 0
        property string before: ""
        /// Strokes gone in.
        property int strokes: 0
        readonly property string path: eolChangeTimer.bucket === "staged" ? "src/flipped.kt" : "src/mixed.kt"
        /// The first line of the hunk that is `kind` (`del` / `add`), or -1. The hunk lists every line taken out before
        /// every line put in, so the old line of the first ending and its new one are the first of each.
        function firstOf(kind) {
            const view = diffPane.view
            for (let i = 0; i < view.count; i++) {
                const row = view.itemAtIndex(i)
                if (row && row.hunk === 0 && row.kind === kind)
                    return row.line
            }
            return -1
        }
        /// One stroke — the old line taken out on an odd stroke, the new one put in on an even one — once the last has
        /// landed and the diff stands read again; says whether it went in.
        function stroke(name) {
            const kind = eolChangeTimer.strokes % 2 === 0 ? "del" : "add"
            const line = eolChangeTimer.firstOf(kind)
            if (!Awaited.all(name, {
                    // The first stroke has no write before it to wait out.
                    "wrote": eolChangeTimer.strokes === 0 || driver.wroteAndSettled(),
                    "diff": !page.diffSettling && diffPane.diffSettled()
                             && page.diffKey === "unstaged:" + eolChangeTimer.path,
                    "line": line >= 0
                }))
                return false
            if (!driver.pressWrite("stage-line", () => {
                    diffPane.stageLine(0, line)
                    return true
                }))
                return false
            eolChangeTimer.strokes++
            return true
        }
        /// The card asked for on the row and held by the hand inside it; says whether it is.
        function cardHeld() {
            wipPane.pointEol(eolChangeTimer.bucket + ":" + eolChangeTimer.path)
            if (!Awaited.all("eol_change_card", { "card": wipPane.eolCardOpen }))
                return false
            eolChangeTimer.before = wipPane.eolHoverCard.notice
            wipPane.eolHoverCard.contentPointed = true
            return true
        }
        onTriggered: {
            const marked = wipPane.modelOf(eolChangeTimer.bucket)
            const staged = eolChangeTimer.bucket === "staged"
            if (eolChangeTimer.step === 0) {
                if (!Awaited.all("eol_change_row", { "row": wipPane.rowIn("unstaged", eolChangeTimer.path) !== null }))
                    return
                eolChangeTimer.strokes = 0
                if (staged)
                    page.openDiff("unstaged", eolChangeTimer.path, "")
                eolChangeTimer.step = 1
                return
            }
            if (eolChangeTimer.step === 1) {
                if (staged && eolChangeTimer.strokes < 2) {
                    eolChangeTimer.stroke("eol_change_first")
                    return
                }
                if (staged && !Awaited.all("eol_change_staged", {
                        "wrote": driver.wroteAndSettled(),
                        "marked": marked.eolMarked(eolChangeTimer.path)
                    }))
                    return
                if (!eolChangeTimer.cardHeld())
                    return
                if (!staged) {
                    // Saved over from outside: a second line lands with the other ending.
                    const why = Harness.writeFile(repoTab.repoPath + "/" + eolChangeTimer.path,
                                                  "fun a() = 1\nfun b() = 2\r\nfun c() = 3\r\n"
                                                  + "fun d() = 4\nfun e() = 5\n")
                    if (why !== "")
                        Harness.report("eol_change write " + why)
                    repoTab.pollNow()
                }
                eolChangeTimer.step = 2
                return
            }
            if (eolChangeTimer.step === 2 && staged && eolChangeTimer.strokes < 4) {
                eolChangeTimer.stroke("eol_change_second")
                return
            }
            if (!Awaited.all("eol_change_landed", {
                    "wrote": !staged || driver.wroteAndSettled(),
                    "diff": !page.diffSettling,
                    "marked": marked.eolMarked(eolChangeTimer.path)
                }))
                return
            const now = wipPane.eolCardOpen ? wipPane.eolHoverCard.notice : ""
            const said = !wipPane.eolCardOpen || now !== eolChangeTimer.before
            if (!Awaited.all("eol_change_said", { "new_or_gone": said }))
                return
            eolChangeTimer.stop()
            Harness.report("eol_change bucket=" + eolChangeTimer.bucket + " card=" + wipPane.eolCardOpen
                           + " changed=" + (now !== eolChangeTimer.before)
                           + " stale=" + (wipPane.eolCardOpen && now === eolChangeTimer.before)
                           + " was=\"" + eolChangeTimer.before + "\" now=\"" + now + "\"")
            driver.complete()
        }
    }

    // PGG_AUTO_ACT=wip-reread. The pane's hand on a marked row of the worktree's lists, its line-ending card out, and a
    // file written from outside — the poll reads the tree again and the lists are told the rows that went, came or
    // changed (`NavSectionModel.reshape`): the row under the hand keeps its delegate, and the pane puts it back under
    // the hand as far as its list scrolls (`WipPane.rowKeep`). Where the rows land decides:
    //   `same`   — `src/mixed.kt` saved over in place: the row stays under the hand — lit, its card up with the new
    //              count;
    //   `fixed`  — `src/mixed.kt` saved over with its endings put right and a change kept: the row stays under the
    //              hand, lit, and only its mark goes — the card goes with it (`WipPane.weighEol`);
    //   `above`  — a new file written above it, in a list too short to scroll: another row comes under the still hand
    //              as the read's — lit as the row under it, the card gone (the hand was on the row, not in the card),
    //              the hand held until it moves; moved two pixels, that row is still the lit one;
    //   `inside` — the same file written above with the hand inside the card: the card stays where it opened, and
    //              the hand is held (デザイン規約 §hover のツールチップ「外乱では閉じない」);
    //   `staged` — the hand on the staged `src/twice.kt` and a new file in the unstaged list: the staged bucket steps
    //              down the pane under the hand — the card gone, the hand held;
    //   `gone`   — the hand inside `src/flipped.kt`'s card and the file staged by the app's own write: the card goes
    //              with its row, and the row that stepped up under the hand — heard again where it stood, which is no
    //              move — opens nothing.
    SampleTimer {
        id: wipRereadTimer
        property string how: "same"
        property int step: 0
        /// The unstaged list's reads since the write (`NavSectionModel.rowsArrived`).
        property int reads: 0
        property string before: ""
        property real bucketYBefore: 0
        /// Where the card stood before the write (`inside`).
        property point cardBefore: Qt.point(0, 0)
        property point at: Qt.point(-1, -1)
        readonly property string bucket: wipRereadTimer.how === "staged" ? "staged" : "unstaged"
        readonly property string path: wipRereadTimer.how === "staged" ? "src/twice.kt"
            : wipRereadTimer.how === "gone" ? "src/flipped.kt" : "src/mixed.kt"
        /// The file written: the same one saved over in place — with a second line of the other ending (`same`), or
        /// with its endings put right and a change kept, so the row stays and its mark goes (`fixed`) — or a new one
        /// sorted above every row.
        readonly property string written: wipRereadTimer.how === "same" || wipRereadTimer.how === "fixed"
            ? "src/mixed.kt" : "src/aaa.kt"
        readonly property string text: wipRereadTimer.how === "same"
            ? "fun a() = 1\nfun b() = 2\r\nfun c() = 3\r\nfun d() = 4\nfun e() = 5\n"
            : wipRereadTimer.how === "fixed" ? "fun a() = 1\nfun b() = 2\nfun c() = 33\nfun d() = 4\nfun e() = 5\n"
            : "fun aaa() = 0\n"
        function paneOf(bucket) {
            const standing = wipPane.bucketPanes
            for (let i = 0; i < standing.length; i++) {
                if (standing[i].section === bucket)
                    return standing[i]
            }
            return null
        }
        /// The row under the pane's hand in the run's bucket, or null.
        function under() {
            const pane = wipRereadTimer.paneOf(wipRereadTimer.bucket)
            return pane !== null ? pane.rowUnderHand() : null
        }
        /// What the read left, read once the pane has weighed it: the card, the hand's hold, the row under it.
        property string left: ""
        function read() {
            const row = wipRereadTimer.under()
            const card = wipPane.eolHoverCard
            return "card=" + wipPane.eolCardOpen + " still=" + wipPane.hand.stillHeld
                + " lit=" + (row !== null && row.washLit)
                + " changed=" + (wipPane.eolCardOpen && card.notice !== wipRereadTimer.before)
                + (wipRereadTimer.how === "inside"
                   ? " placed=" + (card.x === wipRereadTimer.cardBefore.x && card.y === wipRereadTimer.cardBefore.y)
                   : "")
        }
        function done() {
            const row = wipRereadTimer.under()
            wipRereadTimer.stop()
            Harness.report("wip_reread how=" + wipRereadTimer.how + " " + wipRereadTimer.left
                           + (wipRereadTimer.how === "above"
                              ? " moved_lit=" + (row !== null && row.washLit) + " after_card=" + wipPane.eolCardOpen
                              : "")
                           + " under=" + (row !== null ? row.fullName : "") + " reads=" + wipRereadTimer.reads)
            driver.complete()
        }
        property Connections reading: Connections {
            target: wipPane.unstagedModel
            function onRowsArrived() { wipRereadTimer.reads++ }
        }
        onTriggered: {
            if (wipRereadTimer.step === 0) {
                const row = wipPane.rowIn(wipRereadTimer.bucket, wipRereadTimer.path)
                if (!Awaited.all("wip_reread_row", { "row": row !== null }))
                    return
                wipRereadTimer.at = row.mapToItem(null, row.width / 2, row.height / 2)
                // The pane's hand on the row, as it hears a hand (`HandWatch.stir`): the row's own ask brings the card.
                wipPane.hand.stir(wipRereadTimer.at)
                wipRereadTimer.step = 1
                return
            }
            if (wipRereadTimer.step === 1) {
                if (!Awaited.all("wip_reread_card", { "card": wipPane.eolCardOpen }))
                    return
                wipRereadTimer.before = wipPane.eolHoverCard.notice
                wipRereadTimer.bucketYBefore = wipRereadTimer.paneOf("staged").y
                wipRereadTimer.reads = 0
                if (wipRereadTimer.how === "gone") {
                    // The hand inside the card, which the pane no longer hears; the file staged by the app's own
                    // write — its row leaves the list, and the rows under the card step up.
                    wipPane.eolHoverCard.contentPointed = true
                    wipPane.hand.stir(Qt.point(-1, -1))
                    driver.pressWrite("stage-file", () => {
                        wipPane.moveStage("unstaged", wipRereadTimer.path)
                        return true
                    })
                    wipRereadTimer.step = 2
                    return
                }
                if (wipRereadTimer.how === "inside") {
                    // The hand inside the card, which the pane no longer hears; the rows move under the card.
                    wipRereadTimer.cardBefore = Qt.point(wipPane.eolHoverCard.x, wipPane.eolHoverCard.y)
                    wipPane.eolHoverCard.contentPointed = true
                    wipPane.hand.stir(Qt.point(-1, -1))
                }
                // Saved over in place with a second line of the other ending, or a new file sorted above every row.
                const why = Harness.writeFile(repoTab.repoPath + "/" + wipRereadTimer.written, wipRereadTimer.text)
                if (why !== "")
                    Harness.report("wip_reread write " + why)
                repoTab.pollNow()
                wipRereadTimer.step = 2
                return
            }
            if (wipRereadTimer.step === 2) {
                // The read in; read on the next beat, once the pane has weighed it (`WipPane.rowKeep`).
                if (!Awaited.all("wip_reread_read", {
                        "read": wipRereadTimer.reads > 0 && !wipPane.rowKeep.busy,
                        "moved": wipRereadTimer.how !== "staged"
                                 || wipRereadTimer.paneOf("staged").y !== wipRereadTimer.bucketYBefore,
                        "gone": wipRereadTimer.how !== "gone"
                                || (driver.wroteAndSettled() && wipPane.rowIn("unstaged", wipRereadTimer.path) === null
                                    && !wipPane.eolCardOpen),
                        // The mark went with the row still there: the card goes as the pane weighs its target.
                        "down": wipRereadTimer.how !== "fixed" || !wipPane.eolCardOpen
                    }))
                    return
                if (wipRereadTimer.how === "gone") {
                    // The card went with its row, and the hand that was inside it is heard again where it stood — in
                    // Qt's order: the hover handed to what is under the hand first, then taken from the card that went
                    // (`QQuickDeliveryAgentPrivate::deliverHoverEvent`).
                    wipPane.hand.stir(wipRereadTimer.at)
                    wipPane.eolHoverCard.contentPointed = false
                    wipRereadTimer.step = 5
                    return
                }
                wipRereadTimer.step = 3
                return
            }
            if (wipRereadTimer.step === 5) {
                // A card that must not open: the hover Qt hands what is under the hand comes with the next frame, and
                // a seat it asks is deciding until its rest runs out (`PageSettled`'s `hover:` term) — read after
                // both, what has not opened by then was never asked for.
                if (!Awaited.all("wip_reread_still", { "drawn": Awaited.drawnSince("wip_reread_stirred"),
                                                       "page": PageSettled.settled(page) }))
                    return
                wipRereadTimer.left = wipRereadTimer.read()
                wipRereadTimer.done()
                return
            }
            if (wipRereadTimer.step === 3) {
                // The row the read left under the hand asks for its tip or card after the rest: the forms that end
                // here end on that decision; `above` reads first, then moves the hand (below).
                if (wipRereadTimer.how !== "above"
                        && !Awaited.all("wip_reread_landed", { "page": PageSettled.settled(page) }))
                    return
                wipRereadTimer.left = wipRereadTimer.read()
                if (wipRereadTimer.how !== "above") {
                    wipRereadTimer.done()
                    return
                }
                // The hand moved two pixels: the row the read brought under it is the row it is on now.
                wipPane.hand.stir(Qt.point(wipRereadTimer.at.x + 2, wipRereadTimer.at.y))
                wipRereadTimer.step = 4
                return
            }
            // The two pixels were a tremor: the hold lifts (`HandWatch`), and the row under the hand asks for its own
            // card, which comes after the rest — the run ends on it, so the picture is of the state the tremor leaves
            // and not of the rest half spent.
            if (!Awaited.all("wip_reread_moved", { "card": wipPane.eolCardOpen, "page": PageSettled.settled(page) }))
                return
            wipRereadTimer.done()
        }
    }

    // PGG_AUTO_ACT=tip-slide. A changed file's tip out on a row with the pane's hand still on that row, then a folder
    // above it shut — the rows below slide up under the hand (`AppListView.rowsHand` told by the list's count) — read
    // stage by stage:
    //   `shown` — the tip out;
    //   `slid`  — the folder shut: the tip goes in that turn, no beat;
    //   `still` — the hand not moved, past the rest: nothing comes out on the row that slid under it;
    //   `moved` — the hand moved two pixels: the tip comes out after the rest.
    // The row's own hover is stood in for by the pane's `pointedTipRow`, a row number: after the fold the number is
    // another file's, the one that slid under the hand.
    SampleTimer {
        id: tipSlideTimer
        property string stage: "moved"
        property int step: 0
        property bool stood: false
        property point at: Qt.point(-1, -1)
        /// How often the run put the hand on the row again (step 1).
        property int nudges: 0
        /// The folder struck and the row tipped below it.
        property int folderAt: -1
        property int rowAt: -1
        readonly property var pane: detailsPane
        readonly property var view: detailsPane.filesWalk.view
        function tipOut() {
            return Tips.current !== null && Tips.box !== null && Tips.box.visible
        }
        function done(stage) {
            tipSlideTimer.stop()
            Harness.report("tip_slide on=files stage=" + stage + " tip=" + tipSlideTimer.tipOut()
                           + " still=" + tipSlideTimer.pane.hand.stillHeld)
            driver.complete()
        }
        /// The first folder with a file row two rows below it, and that row: shutting the folder slides the row.
        function pick() {
            const view = tipSlideTimer.view
            for (let i = 0; i + 2 < view.count; i++) {
                const folder = view.itemAtIndex(i)
                const below = view.itemAtIndex(i + 2)
                if (folder && folder.isFolder && !folder.isFolded && below && !below.isFolder) {
                    tipSlideTimer.folderAt = i
                    tipSlideTimer.rowAt = i + 2
                    return true
                }
            }
            return false
        }
        /// Stands the run on the commit's list, in the tree view; says whether its rows are there.
        function standOn() {
            if (!tipSlideTimer.stood) {
                page.activateRow(worktree.branchOid !== "" ? worktree.branchOid : graphModel.oidAt(0))
                detailsModel.setTreeView(true)
                tipSlideTimer.stood = true
                return false
            }
            return driver.cardSettled && !detailsModel.loading
        }
        onTriggered: {
            const pane = tipSlideTimer.pane
            if (tipSlideTimer.step === 0) {
                const stood = tipSlideTimer.standOn()
                if (!Awaited.all("tip_slide_rows", { "stood": stood, "row": stood && tipSlideTimer.pick() }))
                    return
                const row = tipSlideTimer.view.itemAtIndex(tipSlideTimer.rowAt)
                tipSlideTimer.at = row.mapToItem(null, row.width / 2, row.height / 2)
                // The pane's hand on the row, as it hears a hand (`HandWatch.stir`).
                pane.hand.stir(tipSlideTimer.at)
                pane.pointedTipRow = tipSlideTimer.rowAt
                tipSlideTimer.step = 1
                return
            }
            if (tipSlideTimer.step === 1) {
                // The pane can still lay its rows out after the run stood on them — the list stepping down as the
                // block over it grows — which is the rows moving under the hand the run put there
                // (`HandWatch.rowsMoved`): that hand waits for a move of its own before the tip comes, so the run puts
                // it on the row again, a pixel aside, as a hand arriving does.
                const held = pane.hand.stillHeld
                if (held) {
                    const row = tipSlideTimer.view.itemAtIndex(tipSlideTimer.rowAt)
                    if (row) {
                        tipSlideTimer.at = row.mapToItem(null, row.width / 2 + (++tipSlideTimer.nudges % 2),
                                                         row.height / 2)
                        pane.hand.stir(tipSlideTimer.at)
                    }
                }
                if (!Awaited.all("tip_slide_tip", { "tip": tipSlideTimer.tipOut(), "free": !held }))
                    return
                if (tipSlideTimer.stage === "shown") {
                    tipSlideTimer.done("shown")
                    return
                }
                const folder = tipSlideTimer.view.itemAtIndex(tipSlideTimer.folderAt)
                folder.folderToggled(folder.foldKey)
                if (tipSlideTimer.stage === "slid") {
                    tipSlideTimer.done("slid")
                    return
                }
                tipSlideTimer.step = 2
                return
            }
            if (tipSlideTimer.step === 2) {
                // The tip that must not come: the rows rebuilt under the hand are a polish away, and a seat one of them
                // asks is deciding until its rest runs out (`PageSettled`'s `rows:` and `hover:` terms) — read after
                // both, a tip not out by then was never asked for.
                if (!Awaited.all("tip_slide_rested", { "drawn": Awaited.drawnSince("tip_slide_folded"),
                                                       "page": PageSettled.settled(page) }))
                    return
                if (tipSlideTimer.stage === "still") {
                    tipSlideTimer.done("still")
                    return
                }
                pane.hand.stir(Qt.point(tipSlideTimer.at.x + 2, tipSlideTimer.at.y))
                tipSlideTimer.step = 3
                return
            }
            if (!Awaited.all("tip_slide_back", { "tip": tipSlideTimer.tipOut() }))
                return
            tipSlideTimer.done("moved")
        }
    }
}

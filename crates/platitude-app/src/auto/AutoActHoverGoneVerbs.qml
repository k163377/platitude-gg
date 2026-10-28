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
            // The stand-in is brought to the top first. Under a menu the branch is made before the list is out;
            // otherwise the list is out first.
            listRereadTimer.order = (listRereadTimer.on === "pin" ? ["pinned"] : [])
                .concat(listRereadTimer.how === "pending" ? ["stirred", "branched", "menu_closed"]
                                                          : ["stirred", "listed", "written"])
            listRereadTimer.start()
        } else if (act === "eol-change") {
            // `<unstaged|staged>` — the card on `src/flipped.kt` in that bucket, held by the hand inside it, and a line
            // of it staged under it: the warning stays and says another count (`--preset eol`).
            page.showWip()
            eolChangeTimer.bucket = head === "" ? "unstaged" : head
            // Staged, two strokes a line: the first line's before the card is held, the second's under it.
            eolChangeTimer.order = eolChangeTimer.bucket === "staged"
                ? ["diff_opened", "first_del", "first_add", "pointed", "held", "second_del", "second_add"]
                : ["row", "pointed", "held"]
            eolChangeTimer.start()
        } else if (act === "wip-reread") {
            // `<same|above|inside|staged|gone|fixed>` — the pane's hand on a marked row of the worktree's lists with
            // its card out, and a file written from outside so the status reads the rows again (`--preset eol`); at
            // `wipRereadTimer`.
            page.showWip()
            wipRereadTimer.how = head === "" ? "same" : head
            // `gone` hears the hand again where it stood; `above` moves it once the read is in.
            wipRereadTimer.order = wipRereadTimer.how === "gone" ? ["stirred", "written", "restirred"]
                                 : wipRereadTimer.how === "above" ? ["stirred", "written", "read", "moved"]
                                 : ["stirred", "written", "read"]
            wipRereadTimer.start()
        } else if (act === "tip-slide") {
            // `files[:<stage>]` — a changed file's tip in the commit's list (`--preset long`), the hand still on its
            // row, and a folder above it shut so the rows slide under the hand; stages at `tipSlideTimer`.
            tipSlideTimer.stage = tail === "" ? "moved" : tail
            tipSlideTimer.order = tipSlideTimer.stage === "shown" ? ["stood", "pointed", "tipped"]
                                : tipSlideTimer.stage === "slid" || tipSlideTimer.stage === "still"
                                ? ["stood", "pointed", "tipped", "folded"]
                                : ["stood", "pointed", "tipped", "folded", "moved"]
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
    PageStage {
        id: listRereadTimer
        name: "list_reread"
        property string on: "row"
        property string how: "branch"
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
        /// The run's line: `names=` counts the list's names past the ones it held before the branch (`namesBefore`),
        /// `moved=` HEAD's row off the commit it stood on as the hand came (`headBefore`).
        function line(stage) {
            const op = listRereadTimer
            return "list_reread on=" + op.on + " how=" + op.how + " stage=" + stage
                   + " list=" + refList.opened
                   + " names=" + (refList.opened ? refList.records.length - op.witnessed("namesBefore") : "-")
                   + " moved=" + (graphModel.headRowOid !== op.witnessed("headBefore"))
        }
        /// Landed — the commit on HEAD's row, or the branch on HEAD's chip — and the list as it ends: gone with the
        /// row's commit that lost its names or left the graph, and with the stand-in standing for HEAD's new commit;
        /// still out with a branch made on its commit, saying the names as they are now.
        function landed() {
            const op = listRereadTimer
            const rewritten = op.how === "commit" || op.how === "amend"
            return {
                "landed": rewritten ? graphModel.headRowOid !== op.witnessed("headBefore")
                                    : graphModel.headLabels.length > op.witnessed("namesBefore"),
                "list": rewritten ? !refList.opened
                                  : refList.opened && refList.records.length > op.witnessed("namesBefore"),
                "page": PageSettled.settled(page)
            }
        }
        onTriggered: {
            const op = listRereadTimer
            const pending = op.how === "pending"
            if (op.at("start") || op.at("pinned")) {
                const read = !graphModel.loading && graphModel.finishCount > 0 && graphModel.headRow >= 0
                // The stand-in rides the edge HEAD's row went out of: the view sent to the end puts it at the top,
                // sent each beat until it stands.
                if (read && op.on === "pin" && op.at("start"))
                    op.input("pinned", () => {
                        if (!graphPane.headPin.visible)
                            graphPane.view.positionViewAtEnd()
                        return graphPane.headPin.visible
                    })
                const item = read ? op.anchor() : null
                if (!Awaited.all("list_reread_chip", { "read": read, "chip": item !== null }))
                    return
                op.input("stirred", () => {
                    op.witness("headBefore", graphModel.headRowOid)
                    // A menu of the page's up first, on another row: the hand on the chip is an ask kept behind it.
                    if (pending)
                        page.openRowMenu(graphModel.oidAt(graphModel.rowTotal - 1))
                    graphPane.hand.stir(op.handOn(item))
                })
                return
            }
            if (pending) {
                if (op.at("stirred")) {
                    if (!Awaited.all("list_reread_held_back", { "menu": page.menuStanding, "list": !refList.opened }))
                        return
                    // Asked each beat until it takes.
                    op.input("branched", () => {
                        op.witness("labelsBefore", graphModel.headLabels.length)
                        return op.pressBranch()
                    })
                    return
                }
                if (op.at("branched")) {
                    const named = graphModel.headLabels.length > op.witnessed("labelsBefore")
                    if (!Awaited.all("list_reread_named", { "names": named }))
                        return
                    op.input("menu_closed", () => {
                        driver.commitMenu.close()
                        // The names the list opens with are counted from what the chip had before the branch.
                        op.witness("namesBefore", graphModel.headLabels.length - 1)
                    })
                    return
                }
                op.complete({
                    "name": "list_reread_back",
                    "when": () => ({ "menu": !page.menuStanding, "list": refList.opened }),
                    "report": () => op.line("after") })
                return
            }
            if (op.at("stirred")) {
                if (!Awaited.all("list_reread_list", { "list": refList.opened }))
                    return
                op.input("listed", () => {
                    op.witness("namesBefore", refList.records.length)
                    if (op.how !== "branch-chip") {
                        // Into the list: the chip let go, the card's own hold on (`AppCard.contentPointed`).
                        graphPane.hand.stir(Qt.point(-1, -1))
                        refList.contentPointed = true
                    }
                    if (op.how === "commit") {
                        wipPane.setMessage("feat: something", "")
                    } else if (op.how === "amend") {
                        wipPane.setAmendChecked(true)
                        page.amending = true
                        wipPane.setMessage("feat: said again", "")
                    }
                })
                return
            }
            if (op.at("listed")) {
                // Through the driver's one door for a write (`AutoActDriver.pressWrite`), asked each beat until it
                // takes: the commit button once it is live, the amend, the branch.
                op.input("written", () => op.how === "commit"
                                          ? driver.pressWrite("commit", () => wipPane.pressCommit())
                                          : op.how === "amend"
                                          ? driver.pressWrite("amend", () => { page.commitNow(); return true })
                                          : op.pressBranch())
                return
            }
            op.complete({ "name": "list_reread_landed", "when": () => op.landed(), "report": () => op.line("after") })
        }
    }

    // PGG_AUTO_ACT=eol-change. The line-ending card held by the hand inside it, and the file under it changed so that
    // it stays marked in that bucket and its mark says another count — the card says the new sentence, or goes; never
    // the old one (`WipPane.weighEol`). `unstaged`: `src/mixed.kt` (one added line with the other ending), saved over
    // from outside with a second such line — the app hears it by reading the tree again (`RepoTab.pollNow`, as the poll
    // does). `staged`: `src/flipped.kt` (every line's ending changed in the tree), one line's new ending staged, the
    // card asked for, then a second line's staged under it through the diff's own marks (`DiffPane.stageLine`) — a line
    // whose ending changed is two lines of the diff, the old one taken out and the new one put in: two strokes a line.
    PageStage {
        id: eolChangeTimer
        name: "eol_change"
        property string bucket: "unstaged"
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
        /// One stroke as the move to `stage` — the old line taken out (`del`) or the new one put in (`add`) — once the
        /// last has landed and the diff stands read again; `name` names the wait.
        function stroke(stage, kind, name) {
            const line = eolChangeTimer.firstOf(kind)
            if (!Awaited.all(name, {
                    // The first stroke has no write before it to wait out.
                    "wrote": stage === "first_del" || driver.wroteAndSettled(),
                    "diff": !page.diffSettling && diffPane.diffSettled()
                             && page.diffKey === "unstaged:" + eolChangeTimer.path,
                    "line": line >= 0
                }))
                return
            eolChangeTimer.input(stage, () => driver.pressWrite("stage-line", () => {
                diffPane.stageLine(0, line)
                return true
            }))
        }
        onTriggered: {
            const op = eolChangeTimer
            const marked = wipPane.modelOf(op.bucket)
            const staged = op.bucket === "staged"
            if (op.at("start")) {
                if (!Awaited.all("eol_change_row", { "row": wipPane.rowIn("unstaged", op.path) !== null }))
                    return
                if (staged)
                    op.input("diff_opened", () => {
                        page.openDiff("unstaged", op.path, "")
                    })
                else
                    op.advance("row")
                return
            }
            if (op.at("diff_opened")) {
                op.stroke("first_del", "del", "eol_change_first")
                return
            }
            if (op.at("first_del")) {
                op.stroke("first_add", "add", "eol_change_first")
                return
            }
            if (op.at("row") || op.at("first_add") || op.at("pointed")) {
                // The card held with the hand inside it — after the stage's write and the mark, where the file was
                // staged — asked for on the row each beat until it is out.
                const written = !staged || (driver.wroteAndSettled() && marked.eolMarked(op.path))
                if (written && !op.at("pointed"))
                    op.input("pointed", () => {
                        wipPane.pointEol(op.bucket + ":" + op.path)
                        return wipPane.eolCardOpen
                    })
                if (!Awaited.all("eol_change_held", { "written": written, "held": written && wipPane.eolCardOpen }))
                    return
                // The card's sentence as the hand goes in (`before`). A failed save over the file is said with the
                // stage.
                op.input("held", () => {
                    op.witness("before", wipPane.eolHoverCard.notice)
                    wipPane.eolHoverCard.contentPointed = true
                    if (staged)
                        return ""
                    // Saved over from outside: a second line lands with the other ending.
                    const why = Harness.writeFile(repoTab.repoPath + "/" + op.path,
                                                  "fun a() = 1\nfun b() = 2\r\nfun c() = 3\r\n"
                                                  + "fun d() = 4\nfun e() = 5\n")
                    repoTab.pollNow()
                    return why
                }, why => why === "" ? "" : "eol_change write " + why)
                return
            }
            if (staged && op.at("held")) {
                op.stroke("second_del", "del", "eol_change_second")
                return
            }
            if (staged && op.at("second_del")) {
                op.stroke("second_add", "add", "eol_change_second")
                return
            }
            if (!Awaited.all("eol_change_landed", {
                    "wrote": !staged || driver.wroteAndSettled(),
                    "diff": !page.diffSettling,
                    "marked": marked.eolMarked(op.path)
                }))
                return
            op.complete({
                "name": "eol_change_said",
                "when": () => ({ "new_or_gone": !wipPane.eolCardOpen
                                                || wipPane.eolHoverCard.notice !== op.witnessed("before") }),
                "report": () => {
                    const card = wipPane.eolCardOpen ? wipPane.eolHoverCard.notice : ""
                    const before = op.witnessed("before")
                    return "eol_change bucket=" + op.bucket + " card=" + wipPane.eolCardOpen
                           + " changed=" + (card !== before)
                           + " stale=" + (wipPane.eolCardOpen && card === before)
                           + " was=\"" + before + "\" now=\"" + card + "\""
                } })
        }
    }

    /// The unstaged list's reads (`NavSectionModel.rowsArrived`), counted for `wip-reread`, which witnesses the count
    /// as its write went in (`readsBefore`).
    property int unstagedReads: 0
    Connections {
        target: acts.wipPane.unstagedModel
        function onRowsArrived() {
            acts.unstagedReads++
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
    PageStage {
        id: wipRereadTimer
        name: "wip_reread"
        property string how: "same"
        // Witnessed on the way: where the pane's hand stands on the row (`handAt`), and as the write went in the
        // card's sentence (`before`), the staged bucket's place (`bucketYBefore`), the unstaged list's reads
        // (`readsBefore`, of `acts.unstagedReads`) and where the card stood (`cardBefore`, `inside`).
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
        function read() {
            const op = wipRereadTimer
            const row = op.under()
            const card = wipPane.eolHoverCard
            const before = op.how === "inside" ? op.witnessed("cardBefore") : null
            return "card=" + wipPane.eolCardOpen + " still=" + wipPane.hand.stillHeld
                + " lit=" + (row !== null && row.washLit)
                + " changed=" + (wipPane.eolCardOpen && card.notice !== op.witnessed("before"))
                + (op.how === "inside" ? " placed=" + (card.x === before.x && card.y === before.y) : "")
        }
        /// The run's line; `above` says what the read left as it found it before the hand moved (`left`, witnessed
        /// with the move).
        function line() {
            const op = wipRereadTimer
            const row = op.under()
            const above = op.how === "above"
            return "wip_reread how=" + op.how + " " + (above ? op.witnessed("left") : op.read())
                   + (above ? " moved_lit=" + (row !== null && row.washLit) + " after_card=" + wipPane.eolCardOpen : "")
                   + " under=" + (row !== null ? row.fullName : "")
                   + " reads=" + (acts.unstagedReads - op.witnessed("readsBefore"))
        }
        onTriggered: {
            const op = wipRereadTimer
            if (op.at("start")) {
                const row = wipPane.rowIn(op.bucket, op.path)
                if (!Awaited.all("wip_reread_row", { "row": row !== null }))
                    return
                // The pane's hand on the row, as it hears a hand (`HandWatch.stir`): the row's own ask brings the card.
                op.input("stirred", () => {
                    wipPane.hand.stir(op.witness("handAt", row.mapToItem(null, row.width / 2, row.height / 2)))
                })
                return
            }
            if (op.at("stirred")) {
                if (!Awaited.all("wip_reread_card", { "card": wipPane.eolCardOpen }))
                    return
                // A failed save over the file is said with the stage.
                op.input("written", () => {
                    op.witness("before", wipPane.eolHoverCard.notice)
                    op.witness("bucketYBefore", op.paneOf("staged").y)
                    op.witness("readsBefore", acts.unstagedReads)
                    if (op.how === "gone") {
                        // The hand inside the card, which the pane no longer hears; the file staged by the app's own
                        // write — its row leaves the list, and the rows under the card step up.
                        wipPane.eolHoverCard.contentPointed = true
                        wipPane.hand.stir(Qt.point(-1, -1))
                        driver.pressWrite("stage-file", () => {
                            wipPane.moveStage("unstaged", op.path)
                            return true
                        })
                        return ""
                    }
                    if (op.how === "inside") {
                        // The hand inside the card, which the pane no longer hears; the rows move under the card.
                        op.witness("cardBefore", Qt.point(wipPane.eolHoverCard.x, wipPane.eolHoverCard.y))
                        wipPane.eolHoverCard.contentPointed = true
                        wipPane.hand.stir(Qt.point(-1, -1))
                    }
                    // Saved over in place with a second line of the other ending, or a new file sorted above every row.
                    const why = Harness.writeFile(repoTab.repoPath + "/" + op.written, op.text)
                    repoTab.pollNow()
                    return why
                }, why => why === "" ? "" : "wip_reread write " + why)
                return
            }
            if (op.at("written")) {
                // The read in; read on the next beat, once the pane has weighed it (`WipPane.rowKeep`).
                if (!Awaited.all("wip_reread_read", {
                        "read": acts.unstagedReads > op.witnessed("readsBefore") && !wipPane.rowKeep.busy,
                        "moved": op.how !== "staged" || op.paneOf("staged").y !== op.witnessed("bucketYBefore"),
                        "gone": op.how !== "gone"
                                || (driver.wroteAndSettled() && wipPane.rowIn("unstaged", op.path) === null
                                    && !wipPane.eolCardOpen),
                        // The mark went with the row still there: the card goes as the pane weighs its target.
                        "down": op.how !== "fixed" || !wipPane.eolCardOpen
                    }))
                    return
                if (op.how === "gone") {
                    // The card went with its row, and the hand that was inside it is heard again where it stood — in
                    // Qt's order: the hover handed to what is under the hand first, then taken from the card that went
                    // (`QQuickDeliveryAgentPrivate::deliverHoverEvent`).
                    op.input("restirred", () => {
                        wipPane.hand.stir(op.witnessed("handAt"))
                        wipPane.eolHoverCard.contentPointed = false
                    })
                    return
                }
                op.advance("read")
                return
            }
            if (op.at("restirred")) {
                // A card that must not open: the hover Qt hands what is under the hand comes with the next frame, and
                // a seat it asks is deciding until its rest runs out (`PageSettled`'s `hover:` term) — read after
                // both, what has not opened by then was never asked for.
                if (!Awaited.all("wip_reread_still", { "drawn": op.drawnSince("wip_reread_stirred"),
                                                       "page": PageSettled.settled(page) }))
                    return
                op.complete({ "name": "wip_reread_still", "when": () => ({ "page": PageSettled.settled(page) }),
                              "report": () => op.line() })
                return
            }
            if (op.at("read")) {
                // The row the read left under the hand asks for its tip or card after the rest: the forms that end
                // here end on that decision; `above` reads first, then moves the hand (below).
                if (op.how !== "above") {
                    op.complete({ "name": "wip_reread_landed", "when": () => ({ "page": PageSettled.settled(page) }),
                                  "report": () => op.line() })
                    return
                }
                // The hand moved two pixels: the row the read brought under it is the row it is on now. What the read
                // left is kept as the hand moves.
                op.input("moved", () => {
                    op.witness("left", op.read())
                    const hand = op.witnessed("handAt")
                    wipPane.hand.stir(Qt.point(hand.x + 2, hand.y))
                })
                return
            }
            // The two pixels were a tremor: the hold lifts (`HandWatch`), and the row under the hand asks for its own
            // card, which comes after the rest — the run ends on it, so the picture is of the state the tremor leaves
            // and not of the rest half spent.
            op.complete({
                "name": "wip_reread_moved",
                "when": () => ({ "card": wipPane.eolCardOpen, "page": PageSettled.settled(page) }),
                "report": () => op.line() })
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
    PageStage {
        id: tipSlideTimer
        name: "tip_slide"
        property string stage: "moved"
        readonly property var pane: detailsPane
        readonly property var view: detailsPane.filesWalk.view
        function tipOut() {
            return Tips.current !== null && Tips.box !== null && Tips.box.visible
        }
        function line(stage) {
            return "tip_slide on=files stage=" + stage + " tip=" + tipSlideTimer.tipOut()
                   + " still=" + tipSlideTimer.pane.hand.stillHeld
        }
        /// The first folder with a file row two rows below it, whose shutting slides that row — its index, or -1.
        function pick() {
            const view = tipSlideTimer.view
            for (let i = 0; i + 2 < view.count; i++) {
                const folder = view.itemAtIndex(i)
                const below = view.itemAtIndex(i + 2)
                if (folder && folder.isFolder && !folder.isFolded && below && !below.isFolder)
                    return i
            }
            return -1
        }
        onTriggered: {
            const op = tipSlideTimer
            const pane = op.pane
            if (op.at("start")) {
                // The run stands on the commit's list, in the tree view.
                op.input("stood", () => {
                    page.activateRow(worktree.branchOid !== "" ? worktree.branchOid : graphModel.oidAt(0))
                    detailsModel.setTreeView(true)
                })
                return
            }
            if (op.at("stood")) {
                const stood = driver.cardSettled && !detailsModel.loading
                const folderAt = stood ? op.pick() : -1
                if (!Awaited.all("tip_slide_rows", { "stood": stood, "row": folderAt >= 0 }))
                    return
                // The pane's hand on the row, as it hears a hand (`HandWatch.stir`); the folder struck and the row
                // tipped below it are kept.
                op.input("pointed", () => {
                    op.witness("folderAt", folderAt)
                    const rowAt = op.witness("rowAt", folderAt + 2)
                    const row = op.view.itemAtIndex(rowAt)
                    pane.hand.stir(row.mapToItem(null, row.width / 2, row.height / 2))
                    pane.pointedTipRow = rowAt
                })
                return
            }
            if (op.at("pointed")) {
                // The pane can still lay its rows out after the run stood on them — the list stepping down as the
                // block over it grows — which is the rows moving under the hand the run put there
                // (`HandWatch.rowsMoved`): that hand waits for a move of its own before the tip comes, so the run puts
                // it on the row again, a pixel aside of where it stands (`HandWatch.at`), as a hand arriving does —
                // each beat until the tip is out over a hand that is free.
                if (!op.input("tipped", () => {
                        const held = pane.hand.stillHeld
                        if (held) {
                            const row = op.view.itemAtIndex(op.witnessed("rowAt"))
                            if (row) {
                                const centre = row.mapToItem(null, row.width / 2, row.height / 2)
                                pane.hand.stir(Qt.point(centre.x + (pane.hand.at.x === centre.x ? 1 : 0), centre.y))
                            }
                        }
                        return !held && op.tipOut()
                    }))
                    return
            }
            if (op.at("tipped")) {
                if (op.stage === "shown") {
                    op.complete({ "name": "tip_slide_tip",
                                  "when": () => ({ "tip": op.tipOut(), "free": !op.pane.hand.stillHeld }),
                                  "report": () => op.line("shown") })
                    return
                }
                op.input("folded", () => {
                    const folder = op.view.itemAtIndex(op.witnessed("folderAt"))
                    folder.folderToggled(folder.foldKey)
                })
                // Read in the turn of the fold where the page owes nothing.
                if (op.stage === "slid")
                    op.complete({ "when": () => ({}), "report": () => op.line("slid") })
                return
            }
            if (op.at("folded")) {
                // The tip that must not come: the rows rebuilt under the hand are a polish away, and a seat one of them
                // asks is deciding until its rest runs out (`PageSettled`'s `rows:` and `hover:` terms) — read after
                // both, a tip not out by then was never asked for.
                if (!Awaited.all("tip_slide_rested", { "drawn": op.drawnSince("tip_slide_folded"),
                                                       "page": PageSettled.settled(page) }))
                    return
                if (op.stage === "still") {
                    op.complete({ "name": "tip_slide_rested", "when": () => ({ "page": PageSettled.settled(page) }),
                                  "report": () => op.line("still") })
                    return
                }
                // Two pixels from where the hand stands.
                op.input("moved", () => {
                    pane.hand.stir(Qt.point(pane.hand.at.x + 2, pane.hand.at.y))
                })
                return
            }
            op.complete({ "name": "tip_slide_back", "when": () => ({ "tip": op.tipOut() }),
                          "report": () => op.line("moved") })
        }
    }
}

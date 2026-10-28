pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// What a read does to the row under the hand and to what the hand is in, list by list (デザイン規約 §hover のツール
/// チップ「外乱では閉じない」): the left menu's open row (`nav-reread`), the discard log's card (`recover-reread`) and
/// the diff's rows (`diff-reread`). Each puts the pane's hand where a hand would be (`HandWatch.stir`), makes the read
/// with a write of this window's or an edit from outside, and reports once the pane has weighed it (`RowKeep`).
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var sidebar: driver.sidebarPane
    readonly property var diffPane: driver.diffPane
    readonly property var graphModel: driver.graphModel

    function run(act, arg) {
        if (act === "nav-reread") {
            // `<above|short|gone>` — a row of the left menu open under the hand, the hand in its lines, and then:
            // `above` a tag written in between the top of the TAGS list and the open row (`--preset manytags`, the list
            // long enough to scroll back), `short` a branch written in above every branch in a list too short to scroll
            // back (`--preset basic`), `gone` the open row's own branch deleted (`--preset basic`) — here, which takes
            // the row off the list at the press, ahead of the read (デザイン規約 §消す操作は先に画面から消す).
            navTimer.how = arg === "" ? "above" : arg
            // `above` scrolls the TAGS list part way down before the hand comes onto a row.
            navTimer.order = (navTimer.how === "above" ? ["scrolled"] : []).concat(["pointed", "into_lines", "written"])
            navTimer.start()
            return true
        }
        if (act === "recover-reread") {
            // `<inside|row>` — the discard log's card out on an entry, the hand inside it (`inside`) or on its row
            // (`row`), and a tag made and deleted here, which the log reads in at the top (`--preset discards`).
            recoverTimer.how = arg === "" ? "inside" : arg
            recoverTimer.start()
            return true
        }
        if (act === "diff-reread") {
            // `<above|gone>` — the hand on a changed line of `notes.txt` read part way down, and the file saved over
            // from outside: `above` with a line written in the hunk above the hand's (`--preset manyhunks`), `gone`
            // with the hand's own line put back as it was.
            diffTimer.how = arg === "" ? "above" : arg
            diffTimer.start()
            return true
        }
        return false
    }

    /// The delegate of `list` showing `name`, null where none is built.
    function rowNamed(list, name) {
        for (let i = 0; i < list.count; i++) {
            const row = list.itemAtIndex(i)
            if (row !== null && row.fullName === name)
                return row
        }
        return null
    }
    /// Where a hand on `item`'s middle stands, in the window.
    function middleOf(item) {
        return item.mapToItem(null, item.width / 2, Math.min(item.height, Theme.rowHeight) / 2)
    }
    /// Whether the delegate `row` still stands for its row with its top where it was in the window (`at`) — not
    /// pooled, and not destroyed with a row the read took out (null).
    function standsAt(row, at) {
        return row !== null && row.pooled !== true && Math.abs(row.mapToItem(null, 0, 0).y - at) < 1
    }

    // nav-reread. The open row's seat (`OpenRowSeat`) and the panel's keeper (`SidebarPane.rowKeep`).
    PageStage {
        id: navTimer
        name: "nav_reread"
        order: ["pointed", "into_lines", "written"]
        property string how: "above"
        // Witnessed as the hand came onto the open row: its delegate (`row`) — held on purpose: `kept=` asks whether
        // this very delegate still stands where it stood (`standsAt`) — its full name (`rowName`) and key (`key`);
        // and as the write went in, where that row stood (`rowAt`) and the reads the list's model had (`readsBefore`).
        readonly property string kind: navTimer.how === "above" ? "tag" : "branch"
        readonly property var list: acts.sidebar.autoSections.listOf(navTimer.kind)
        readonly property var model: navTimer.how === "above" ? acts.driver.tagsModel : acts.driver.branchesModel
        /// How many reads the list's model has taken in (counted by the connection after this).
        property int reads: 0
        onTriggered: {
            const op = navTimer
            const list = op.list
            if (op.at("start") || op.at("scrolled")) {
                const enough = op.how === "above" ? 30 : 0
                if (!Awaited.all("nav_reread_rows", { "rows": list !== null && list.count > enough }))
                    return
                // The TAGS list part way down, so a tag written in between its top and the open row is one the
                // scroll can take back — sent each beat until the row the hand comes onto is built; BRANCHES as it
                // opens.
                if (op.how === "above" && op.at("start") && !op.input("scrolled", () => {
                        list.contentY = list.originY + 20 * Theme.rowHeight
                        list.forceLayout()
                        return list.itemAtIndex(24) !== null
                    }))
                    return
                const pointed = op.how === "above" ? list.itemAtIndex(24) : acts.rowNamed(list, "feature/topic-a")
                if (!Awaited.all("nav_reread_row", { "row": pointed !== null }))
                    return
                op.input("pointed", () => {
                    op.witness("row", pointed)
                    op.witness("rowName", pointed.fullName)
                    op.witness("key", pointed.rowKey)
                    acts.sidebar.hand.stir(acts.middleOf(pointed))
                })
                return
            }
            if (op.at("pointed")) {
                const row = op.witnessed("row")
                if (!Awaited.all("nav_reread_open", { "open": acts.sidebar.openKey === op.witnessed("key"),
                                                      "built": row.factsItem !== null }))
                    return
                // Down into its lines, as a hand reads them — a tag nobody carries opens its name alone, and the hand
                // stays on it.
                op.input("into_lines", () => {
                    if (row.factsItem.height > 0)
                        acts.sidebar.hand.stir(row.factsItem.mapToItem(null, row.factsItem.width / 2,
                                                                       row.factsItem.height / 2))
                })
                op.input("written", () => {
                    op.witness("rowAt", row.mapToItem(null, 0, 0).y)
                    op.witness("readsBefore", op.reads)
                    const head = acts.graphModel.headRowOid
                    if (op.how === "gone") {
                        driver.pressWrite("delete-branch", () => {
                            acts.repoTab.deleteBranch(op.witnessed("rowName"), true)
                            return true
                        })
                    } else if (op.how === "short") {
                        driver.pressWrite("create-branch", () => {
                            acts.repoTab.createBranch("aaa-nav-reread", head, false)
                            return true
                        })
                    } else {
                        // Named to stand two rows above the open one.
                        const above = list.itemAtIndex(22).fullName + "-a"
                        driver.pressWrite("create-tag", () => {
                            acts.repoTab.createTag(above, head)
                            return true
                        })
                    }
                })
                return
            }
            op.complete({
                "name": "nav_reread_read",
                "when": () => ({ "settled": driver.wroteAndSettled(),
                                 "read": op.reads > op.witnessed("readsBefore"),
                                 "weighed": !acts.sidebar.rowKeep.busy,
                                 "page": PageSettled.settled(page) }),
                "report": () => "nav_reread how=" + op.how
                                + " open=" + (acts.sidebar.openKey === op.witnessed("key"))
                                + " kept=" + acts.standsAt(op.witnessed("row"), op.witnessed("rowAt"))
                                + " still=" + acts.sidebar.hand.stillHeld
                                + " name=" + op.witnessed("rowName") })
        }
    }
    Connections {
        target: navTimer.model
        function onRowsArrived() {
            navTimer.reads++
        }
    }

    // recover-reread. The log's card seat (`RecoverPane.entryKeep`) and its keeper (`RecoverPane.rowKeep`).
    PageStage {
        id: recoverTimer
        name: "recover_reread"
        order: ["opened", "pointed", "made", "deleted"]
        property string how: "inside"
        // Witnessed as the tag is made: the entry the card stood for (`atBefore`) and where its content stood in the
        // window (`cardAt`).
        readonly property var pane: acts.sidebar.autoRecover
        onTriggered: {
            const op = recoverTimer
            const pane = op.pane
            if (op.at("start")) {
                op.input("opened", () => {
                    if (!acts.page.recoverOpen)
                        acts.page.toggleRecover()
                })
            }
            if (op.at("opened")) {
                if (!Awaited.all("recover_reread_rows", { "answered": acts.page.recoverAnswered,
                                                          "row": pane.entryList.itemAtIndex(2) !== null }))
                    return
                op.input("pointed", () => {
                    acts.sidebar.hand.stir(Qt.point(-1, -1))
                    pane.hand.stir(acts.middleOf(pane.entryList.itemAtIndex(2)))
                })
                return
            }
            if (op.at("pointed")) {
                if (!Awaited.all("recover_reread_card", { "card": pane.cardOpen }))
                    return
                op.input("made", () => {
                    op.witness("atBefore", pane.cardAt)
                    const card = pane.entryHoverCard
                    op.witness("cardAt", card.contentItem.mapToItem(null, 0, 0))
                    if (op.how === "inside") {
                        // The hand inside the card, which the list no longer hears.
                        card.contentPointed = true
                        pane.hand.stir(Qt.point(-1, -1))
                    }
                    // Something thrown away here: a tag made and taken away again, which the log reads in at its top.
                    driver.pressWrite("create-tag", () => {
                        acts.repoTab.createTag("recover-reread", acts.graphModel.headRowOid)
                        return true
                    })
                })
                return
            }
            if (op.at("made")) {
                if (!Awaited.all("recover_reread_made", { "settled": driver.wroteAndSettled() }))
                    return
                op.input("deleted", () => {
                    driver.pressWrite("delete-tag", () => {
                        acts.repoTab.deleteTag("recover-reread")
                        return true
                    })
                })
                return
            }
            op.complete({
                "name": "recover_reread_read",
                "when": () => ({ "settled": driver.wroteAndSettled(),
                                 "answered": acts.page.recoverAnswered,
                                 "moved": !op.pane.cardOpen || op.pane.cardAt !== op.witnessed("atBefore"),
                                 "weighed": !op.pane.rowKeep.busy,
                                 "page": PageSettled.settled(acts.page) }),
                "report": () => {
                    const log = recoverTimer.pane
                    const at = log.entryHoverCard.contentItem.mapToItem(null, 0, 0)
                    const was = op.witnessed("cardAt")
                    return "recover_reread how=" + op.how
                           + " card=" + log.cardOpen
                           + " placed=" + (log.cardOpen && at.x === was.x && at.y === was.y)
                           + " still=" + log.hand.stillHeld
                           + " at=" + op.witnessed("atBefore") + "->" + log.cardAt
                } })
        }
    }

    // diff-reread. The diff's keeper (`DiffPane.rowKeep`), over the rows the model told row by row
    // (`DiffModel.layOutKept`).
    PageStage {
        id: diffTimer
        name: "diff_reread"
        order: ["opened", "scrolled", "pointed", "written"]
        property string how: "above"
        // Witnessed as the hand came onto its line: the line's delegate (`row`) — held on purpose: `kept=` / `same=`
        // ask whether this very delegate still stands for that line where it stood — and its text (`said`); and as
        // the file was saved over, where that row stood (`rowAt`) and the reads the diff's model had (`readsBefore`).
        /// How many reads the diff's model has taken in (counted by the connection after this).
        property int reads: 0
        /// The row of the reworded line 185 among those the view has built from row 84 on, or -1.
        function lineRow() {
            const view = acts.diffPane.view
            for (let i = 84; i < Math.min(view.count, 120); i++) {
                const line = view.itemAtIndex(i)
                if (line !== null && line.kind === "add" && line.new_no === 185)
                    return i
            }
            return -1
        }
        /// `notes.txt` as `--preset manyhunks` leaves it, every sixteenth line reworded — and the edit the run
        /// saves over it: a line written in the hunk above the hand's (`above`), or the hand's own line put back
        /// (`gone`).
        function notes(edit) {
            let text = ""
            for (let i = 0; i < 320; i++) {
                const n = String(i).padStart(3, "0")
                if (edit === "above" && i === 150)
                    text += "150½: a line written in from outside\n"
                const reworded = i % 16 === 8 && !(edit === "gone" && i === 184)
                text += n + (reworded ? ": reworded while reading down the file\n"
                                      : ": a line of notes nobody has touched\n")
            }
            return text
        }
        onTriggered: {
            const op = diffTimer
            const view = acts.diffPane.view
            if (op.at("start")) {
                op.input("opened", () => {
                    if (!acts.page.diffShown || acts.page.diffKey !== "unstaged:notes.txt")
                        acts.page.openDiff("unstaged", "notes.txt", "")
                })
            }
            if (op.at("opened")) {
                if (!Awaited.all("diff_reread_rows", { "rows": acts.diffPane.firstChangedLine(0) >= 0 }))
                    return
                // The reworded line 185 (hunk 11, nine rows a hunk), with the hunk above line 150's a screen above it:
                // a line written in there moves the hand's row down the view. Sent each beat until that row is built.
                if (!op.input("scrolled", () => {
                        view.positionViewAtIndex(84, ListView.Beginning)
                        view.forceLayout()
                        return op.lineRow() >= 0
                    }))
                    return
            }
            if (op.at("scrolled")) {
                const at = op.lineRow()
                if (!Awaited.all("diff_reread_row", { "row": at >= 0 }))
                    return
                op.input("pointed", () => {
                    const pointed = view.itemAtIndex(at)
                    op.witness("row", pointed)
                    op.witness("said", pointed.text)
                    acts.diffPane.hand.stir(acts.middleOf(pointed))
                })
                return
            }
            if (op.at("pointed")) {
                const row = op.witnessed("row")
                // The row the hand is on laid out and standing, before the write moves the rows under it.
                if (!op.still("diff_reread_stood", row.mapToItem(null, 0, 0).y))
                    return
                // A failed save over the file is said with the stage.
                op.input("written", () => {
                    op.witness("rowAt", row.mapToItem(null, 0, 0).y)
                    op.witness("readsBefore", op.reads)
                    const why = Harness.writeFile(acts.repoTab.repoPath + "/notes.txt", op.notes(op.how))
                    acts.repoTab.pollNow()
                    return why
                }, why => why === "" ? "" : "diff_reread write " + why)
                return
            }
            op.complete({
                "name": "diff_reread_read",
                "when": () => ({ "read": op.reads > op.witnessed("readsBefore"),
                                 "weighed": !acts.diffPane.rowKeep.busy }),
                "report": () => {
                    const row = op.witnessed("row")
                    return "diff_reread how=" + op.how
                           + " kept=" + acts.standsAt(row, op.witnessed("rowAt"))
                           + " same=" + (row !== null && row.pooled !== true && row.text === op.witnessed("said"))
                           + " still=" + acts.diffPane.hand.stillHeld
                } })
        }
    }
    Connections {
        target: acts.diffPane.diffModel
        function onRowsArrived() {
            diffTimer.reads++
        }
    }
}

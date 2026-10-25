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
            navTimer.step = 0
            navTimer.start()
            return true
        }
        if (act === "recover-reread") {
            // `<inside|row>` — the discard log's card out on an entry, the hand inside it (`inside`) or on its row
            // (`row`), and a tag made and deleted here, which the log reads in at the top (`--preset discards`).
            recoverTimer.how = arg === "" ? "inside" : arg
            recoverTimer.step = 0
            recoverTimer.start()
            return true
        }
        if (act === "diff-reread") {
            // `<above|gone>` — the hand on a changed line of `notes.txt` read part way down, and the file saved over
            // from outside: `above` with a line written in the hunk above the hand's (`--preset manyhunks`), `gone`
            // with the hand's own line put back as it was.
            diffTimer.how = arg === "" ? "above" : arg
            diffTimer.step = 0
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
    SampleTimer {
        id: navTimer
        property string how: "above"
        property int step: 0
        property string key: ""
        property string name: ""
        property real rowAt: 0
        property var row: null
        readonly property string kind: navTimer.how === "above" ? "tag" : "branch"
        readonly property var list: acts.sidebar.autoSections.listOf(navTimer.kind)
        readonly property var model: navTimer.how === "above" ? acts.driver.tagsModel : acts.driver.branchesModel
        property int reads: 0
        property Connections reading: Connections {
            target: navTimer.model
            function onRowsArrived() { navTimer.reads++ }
        }
        onTriggered: {
            const list = navTimer.list
            if (navTimer.step === 0) {
                const enough = navTimer.how === "above" ? 30 : 0
                if (!Awaited.all("nav_reread_rows", { "rows": list !== null && list.count > enough }))
                    return
                // The TAGS list part way down, so a tag written in between its top and the open row is one the
                // scroll can take back; BRANCHES as it opens.
                if (navTimer.how === "above") {
                    list.contentY = list.originY + 20 * Theme.rowHeight
                    list.forceLayout()
                }
                const at = navTimer.how === "above" ? 24 : -1
                const row = at >= 0 ? list.itemAtIndex(at) : acts.rowNamed(list, "feature/topic-a")
                if (!Awaited.all("nav_reread_row", { "row": row !== null }))
                    return
                navTimer.row = row
                navTimer.name = row.fullName
                navTimer.key = row.rowKey
                acts.sidebar.hand.stir(acts.middleOf(row))
                navTimer.step = 1
                return
            }
            if (navTimer.step === 1) {
                const row = navTimer.row
                if (!Awaited.all("nav_reread_open", { "open": acts.sidebar.openKey === navTimer.key,
                                                      "built": row.factsItem !== null }))
                    return
                // Down into its lines, as a hand reads them — a tag nobody carries opens its name alone, and the hand
                // stays on it.
                if (row.factsItem.height > 0)
                    acts.sidebar.hand.stir(row.factsItem.mapToItem(null, row.factsItem.width / 2,
                                                                   row.factsItem.height / 2))
                navTimer.rowAt = row.mapToItem(null, 0, 0).y
                navTimer.reads = 0
                const head = acts.graphModel.headRowOid
                if (navTimer.how === "gone") {
                    driver.pressWrite("delete-branch", () => {
                        acts.repoTab.deleteBranch(navTimer.name, true)
                        return true
                    })
                } else if (navTimer.how === "short") {
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
                navTimer.step = 2
                return
            }
            if (!Awaited.all("nav_reread_read", { "settled": driver.wroteAndSettled(), "read": navTimer.reads > 0,
                                                  "weighed": !acts.sidebar.rowKeep.busy }))
                return
            navTimer.stop()
            const row = navTimer.row
            Harness.report("nav_reread how=" + navTimer.how
                           + " open=" + (acts.sidebar.openKey === navTimer.key)
                           + " kept=" + acts.standsAt(row, navTimer.rowAt)
                           + " still=" + acts.sidebar.hand.stillHeld
                           + " name=" + navTimer.name)
            driver.complete()
        }
    }

    // recover-reread. The log's card seat (`RecoverPane.entryKeep`) and its keeper (`RecoverPane.rowKeep`).
    SampleTimer {
        id: recoverTimer
        property string how: "inside"
        property int step: 0
        property point cardAt: Qt.point(0, 0)
        property int atBefore: -1
        readonly property var pane: acts.sidebar.autoRecover
        onTriggered: {
            const pane = recoverTimer.pane
            if (recoverTimer.step === 0) {
                if (!acts.page.recoverOpen)
                    acts.page.toggleRecover()
                if (!Awaited.all("recover_reread_rows", { "answered": acts.page.recoverAnswered,
                                                          "row": pane.entryList.itemAtIndex(2) !== null }))
                    return
                const row = pane.entryList.itemAtIndex(2)
                acts.sidebar.hand.stir(Qt.point(-1, -1))
                pane.hand.stir(acts.middleOf(row))
                recoverTimer.step = 1
                return
            }
            if (recoverTimer.step === 1) {
                if (!Awaited.all("recover_reread_card", { "card": pane.cardOpen }))
                    return
                recoverTimer.atBefore = pane.cardAt
                const card = pane.entryHoverCard
                recoverTimer.cardAt = card.contentItem.mapToItem(null, 0, 0)
                if (recoverTimer.how === "inside") {
                    // The hand inside the card, which the list no longer hears.
                    card.contentPointed = true
                    pane.hand.stir(Qt.point(-1, -1))
                }
                // Something thrown away here: a tag made and taken away again, which the log reads in at its top.
                driver.pressWrite("create-tag", () => {
                    acts.repoTab.createTag("recover-reread", acts.graphModel.headRowOid)
                    return true
                })
                recoverTimer.step = 2
                return
            }
            if (recoverTimer.step === 2) {
                if (!Awaited.all("recover_reread_made", { "settled": driver.wroteAndSettled() }))
                    return
                driver.pressWrite("delete-tag", () => {
                    acts.repoTab.deleteTag("recover-reread")
                    return true
                })
                recoverTimer.step = 3
                return
            }
            if (!Awaited.all("recover_reread_read", { "settled": driver.wroteAndSettled(),
                                                      "answered": acts.page.recoverAnswered,
                                                      "moved": !pane.cardOpen || pane.cardAt !== recoverTimer.atBefore,
                                                      "weighed": !pane.rowKeep.busy }))
                return
            recoverTimer.stop()
            const card = pane.entryHoverCard
            const at = card.contentItem.mapToItem(null, 0, 0)
            Harness.report("recover_reread how=" + recoverTimer.how
                           + " card=" + pane.cardOpen
                           + " placed=" + (pane.cardOpen && at.x === recoverTimer.cardAt.x
                                           && at.y === recoverTimer.cardAt.y)
                           + " still=" + pane.hand.stillHeld
                           + " at=" + recoverTimer.atBefore + "->" + pane.cardAt)
            driver.complete()
        }
    }

    // diff-reread. The diff's keeper (`DiffPane.rowKeep`), over the rows the model told row by row
    // (`DiffModel.layOutKept`).
    SampleTimer {
        id: diffTimer
        property string how: "above"
        property int step: 0
        property var row: null
        property real rowAt: 0
        property string said: ""
        property int reads: 0
        property Connections reading: Connections {
            target: acts.diffPane.diffModel
            function onRowsArrived() { diffTimer.reads++ }
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
            const view = acts.diffPane.view
            if (diffTimer.step === 0) {
                if (!acts.page.diffShown || acts.page.diffKey !== "unstaged:notes.txt")
                    acts.page.openDiff("unstaged", "notes.txt", "")
                if (!Awaited.all("diff_reread_rows", { "rows": acts.diffPane.firstChangedLine(0) >= 0 }))
                    return
                // The reworded line 185 (hunk 11, nine rows a hunk), with the hunk above line 150's a screen above it:
                // a line written in there moves the hand's row down the view.
                view.positionViewAtIndex(84, ListView.Beginning)
                view.forceLayout()
                let at = -1
                for (let i = 84; i < Math.min(view.count, 120) && at < 0; i++) {
                    const row = view.itemAtIndex(i)
                    if (row !== null && row.kind === "add" && row.new_no === 185)
                        at = i
                }
                if (!Awaited.all("diff_reread_row", { "row": at >= 0 }))
                    return
                const row = view.itemAtIndex(at)
                diffTimer.row = row
                diffTimer.said = row.text
                acts.diffPane.hand.stir(acts.middleOf(row))
                diffTimer.step = 1
                return
            }
            if (diffTimer.step === 1) {
                diffTimer.rowAt = diffTimer.row.mapToItem(null, 0, 0).y
                diffTimer.reads = 0
                const why = Harness.writeFile(acts.repoTab.repoPath + "/notes.txt", diffTimer.notes(diffTimer.how))
                if (why !== "")
                    Harness.report("diff_reread write " + why)
                acts.repoTab.pollNow()
                diffTimer.step = 2
                return
            }
            if (!Awaited.all("diff_reread_read", { "read": diffTimer.reads > 0,
                                                   "weighed": !acts.diffPane.rowKeep.busy }))
                return
            diffTimer.stop()
            const row = diffTimer.row
            const hand = acts.diffPane.hand
            Harness.report("diff_reread how=" + diffTimer.how
                           + " kept=" + acts.standsAt(row, diffTimer.rowAt)
                           + " same=" + (row !== null && row.pooled !== true && row.text === diffTimer.said)
                           + " still=" + hand.stillHeld)
            driver.complete()
        }
    }
}

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
        property string key: ""
        /// The open row's full name, read off it as the hand came onto it.
        property string rowName: ""
        property real rowAt: 0
        /// The open row's delegate as the hand found it, held on purpose: `kept=` asks whether this very delegate
        /// still stands where it stood (`standsAt`).
        property var row: null
        readonly property string kind: navTimer.how === "above" ? "tag" : "branch"
        readonly property var list: acts.sidebar.autoSections.listOf(navTimer.kind)
        readonly property var model: navTimer.how === "above" ? acts.driver.tagsModel : acts.driver.branchesModel
        /// How many reads the list's model has taken in (counted by the connection after this), and how many it had
        /// as the write went in.
        property int reads: 0
        property int readsBefore: 0
        onTriggered: {
            const op = navTimer
            const list = op.list
            if (op.at("start")) {
                const enough = op.how === "above" ? 30 : 0
                if (!Awaited.all("nav_reread_rows", { "rows": list !== null && list.count > enough }))
                    return
                // The TAGS list part way down, so a tag written in between its top and the open row is one the
                // scroll can take back; BRANCHES as it opens.
                if (op.how === "above") {
                    list.contentY = list.originY + 20 * Theme.rowHeight
                    list.forceLayout()
                }
                const at = op.how === "above" ? 24 : -1
                const pointed = at >= 0 ? list.itemAtIndex(at) : acts.rowNamed(list, "feature/topic-a")
                if (!Awaited.all("nav_reread_row", { "row": pointed !== null }))
                    return
                op.input("pointed", () => {
                    op.row = pointed
                    op.rowName = pointed.fullName
                    op.key = pointed.rowKey
                    acts.sidebar.hand.stir(acts.middleOf(pointed))
                })
                return
            }
            if (op.at("pointed")) {
                const row = op.row
                if (!Awaited.all("nav_reread_open", { "open": acts.sidebar.openKey === op.key,
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
                    op.rowAt = row.mapToItem(null, 0, 0).y
                    op.readsBefore = op.reads
                    const head = acts.graphModel.headRowOid
                    if (op.how === "gone") {
                        driver.pressWrite("delete-branch", () => {
                            acts.repoTab.deleteBranch(op.rowName, true)
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
            if (!Awaited.all("nav_reread_read", { "settled": driver.wroteAndSettled(),
                                                  "read": op.reads > op.readsBefore,
                                                  "weighed": !acts.sidebar.rowKeep.busy,
                                                  "page": PageSettled.settled(page) }))
                return
            op.complete(() => "nav_reread how=" + op.how
                              + " open=" + (acts.sidebar.openKey === op.key)
                              + " kept=" + acts.standsAt(op.row, op.rowAt)
                              + " still=" + acts.sidebar.hand.stillHeld
                              + " name=" + op.rowName)
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
        property point cardAt: Qt.point(0, 0)
        property int atBefore: -1
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
                    op.atBefore = pane.cardAt
                    const card = pane.entryHoverCard
                    op.cardAt = card.contentItem.mapToItem(null, 0, 0)
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
            if (!Awaited.all("recover_reread_read", { "settled": driver.wroteAndSettled(),
                                                      "answered": acts.page.recoverAnswered,
                                                      "moved": !pane.cardOpen || pane.cardAt !== op.atBefore,
                                                      "weighed": !pane.rowKeep.busy,
                                                      "page": PageSettled.settled(acts.page) }))
                return
            op.complete(() => {
                const log = recoverTimer.pane
                const at = log.entryHoverCard.contentItem.mapToItem(null, 0, 0)
                return "recover_reread how=" + op.how
                       + " card=" + log.cardOpen
                       + " placed=" + (log.cardOpen && at.x === op.cardAt.x && at.y === op.cardAt.y)
                       + " still=" + log.hand.stillHeld
                       + " at=" + op.atBefore + "->" + log.cardAt
            })
        }
    }

    // diff-reread. The diff's keeper (`DiffPane.rowKeep`), over the rows the model told row by row
    // (`DiffModel.layOutKept`).
    PageStage {
        id: diffTimer
        name: "diff_reread"
        order: ["opened", "pointed", "written"]
        property string how: "above"
        /// The delegate of the hand's line as the hand came onto it, held on purpose: `kept=` / `same=` ask whether
        /// this very delegate still stands for that line where it stood.
        property var row: null
        property real rowAt: 0
        property string said: ""
        /// How many reads the diff's model has taken in (counted by the connection after this), and how many it had
        /// as the file was saved over.
        property int reads: 0
        property int readsBefore: 0
        /// Why saving over the file failed, empty where it went through.
        property string wrote: ""
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
                // a line written in there moves the hand's row down the view.
                view.positionViewAtIndex(84, ListView.Beginning)
                view.forceLayout()
                let at = -1
                for (let i = 84; i < Math.min(view.count, 120) && at < 0; i++) {
                    const line = view.itemAtIndex(i)
                    if (line !== null && line.kind === "add" && line.new_no === 185)
                        at = i
                }
                if (!Awaited.all("diff_reread_row", { "row": at >= 0 }))
                    return
                op.input("pointed", () => {
                    const pointed = view.itemAtIndex(at)
                    op.row = pointed
                    op.said = pointed.text
                    acts.diffPane.hand.stir(acts.middleOf(pointed))
                })
                return
            }
            if (op.at("pointed")) {
                // The row the hand is on laid out and standing, before the write moves the rows under it.
                if (!op.still("diff_reread_stood", op.row.mapToItem(null, 0, 0).y))
                    return
                op.input("written", () => {
                    op.rowAt = op.row.mapToItem(null, 0, 0).y
                    op.readsBefore = op.reads
                    op.wrote = Harness.writeFile(acts.repoTab.repoPath + "/notes.txt", op.notes(op.how))
                    acts.repoTab.pollNow()
                }, () => op.wrote === "" ? "" : "diff_reread write " + op.wrote)
                return
            }
            if (!Awaited.all("diff_reread_read", { "read": op.reads > op.readsBefore,
                                                   "weighed": !acts.diffPane.rowKeep.busy }))
                return
            op.complete(() => {
                const row = op.row
                return "diff_reread how=" + op.how
                       + " kept=" + acts.standsAt(row, op.rowAt)
                       + " same=" + (row !== null && row.pooled !== true && row.text === op.said)
                       + " still=" + acts.diffPane.hand.stillHeld
            })
        }
    }
    Connections {
        target: acts.diffPane.diffModel
        function onRowsArrived() {
            diffTimer.reads++
        }
    }
}

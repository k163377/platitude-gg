pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// A graph row's menu standing with what hovers over the rows: the row's card asked under the menu, and the chip's
/// stacked list behind, under or around it. The list's own rows and their menu are `AutoActGraphRowVerbs`'.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var commitMenu: driver.commitMenu
    readonly property var refList: driver.refList
    readonly property var rowCard: driver.rowCard
    readonly property var rowHost: driver.rowHost

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "menu-hover") {
            // Same row and default as `commit-menu`; the row under the menu is then asked for its card.
            let hoverOid = arg
            if (hoverOid === "")
                hoverOid = graphModel.oidAt(graphModel.rowOf(worktree.headOid) + 1)
            page.openRowMenu(hoverOid)
            menuHoverTimer.oidHex = hoverOid
            menuHoverTimer.start()
        } else if (act === "menu-list") {
            // The chip's stacked list and a standing menu (デザイン規約 §メニュー): `behind` asks for HEAD's list with
            // a graph row's menu up, `under` raises that menu over the open list; `own` raises the menu on the list's
            // first row and takes the hand off the chip — the one menu it stays under (`list-menu` reads that menu's
            // cards, at once).
            menuListTimer.how = arg === "" ? "behind" : arg
            // `swap` has no order: with no other chipped row in view it ends where it would have moved the hand.
            menuListTimer.order = menuListTimer.how === "behind" ? ["row_menu", "list_asked"]
                                : menuListTimer.how === "under" ? ["list_opened", "row_menu"]
                                : menuListTimer.how === "own" ? ["list_opened", "list_menu", "hand_off"] : []
            menuListTimer.start()
        } else {
            return false
        }
        return true
    }
    // PGG_AUTO_ACT=menu-list. The list is HEAD's — the one commit sure to have a chip — and a graph row's menu the row
    // under it's, as `menu-hover` raises it. Each stage waits for the one before to land; the list goes, or stays out,
    // in the same turn as the ask, so the answer is read once the menu is up — for `own`, once the beat the hand's
    // leaving started has run out. `swap` raises the menu on the list and puts the hand on another row's chip: the
    // list is the menu's and is not handed over (`kept=`).
    PageStage {
        id: menuListTimer
        name: "menu_list"
        property string how: ""
        /// The nearest built row below HEAD's with a chip of its own, or null.
        function otherChipped(head) {
            for (let i = head + 1; i < graphPane.view.count; i++) {
                const row = graphPane.view.itemAtIndex(i)
                if (row === null)
                    return null
                if (row.chipItem && row.chipItem.records.length > 0)
                    return row
            }
            return null
        }
        /// HEAD's row, or -1 while the graph has none to say.
        function headRow() {
            return graphModel.loading || graphModel.rowTotal === 0 ? -1 : graphModel.headRow
        }
        onTriggered: {
            const op = menuListTimer
            const head = op.headRow()
            const stacked = head < 0 ? null : graphPane.view.itemAtIndex(head)
            const other = head < 0 ? "" : graphModel.oidAt(head + 1)
            if (op.at("start")) {
                if (!Awaited.all("menu_list_rows", { "head": stacked !== null, "other": other !== "" }))
                    return
                if (op.how === "behind") {
                    op.input("row_menu", () => {
                        page.openRowMenu(other)
                    })
                } else {
                    op.input("list_opened", () => {
                        graphPane.view.chipExpandRequested(stacked.oid_hex, stacked.index, stacked.chipItem.records,
                                                           stacked.chipItem)
                    })
                }
                return
            }
            if (op.how === "behind" && op.at("row_menu")) {
                if (!Awaited.all("menu_list_menu", { "menu": commitMenu.opened, "head": stacked !== null }))
                    return
                op.input("list_asked", () => {
                    graphPane.view.chipExpandRequested(stacked.oid_hex, stacked.index, stacked.chipItem.records,
                                                       stacked.chipItem)
                })
                return
            }
            if (op.at("list_opened")) {
                if (!Awaited.all("menu_list_open", { "list": refList.opened }))
                    return
                if (op.how === "under") {
                    op.input("row_menu", () => {
                        page.openRowMenu(other)
                    })
                    return
                }
                // Not built yet: no press has gone in.
                if (!op.input("list_menu", () => refList.menuRow(0)))
                    return
                if (op.how === "swap") {
                    const next = op.otherChipped(head)
                    if (next === null) {
                        op.complete({ "name": "menu_list_none", "when": () => ({}),
                                      "report": () => "menu_list how=swap other=none" })
                        return
                    }
                    // The hand onto the other row's chip, as that row reports it.
                    op.input("hand_moved", () => {
                        graphPane.view.chipExpandRequested(next.oid_hex, next.index, next.chipItem.records,
                                                           next.chipItem)
                    })
                    return
                }
                // The hand off the chip, as its row reports it (`GraphRowDelegate.settlePointed`).
                op.input("hand_off", () => {
                    graphPane.view.chipCollapseRequested(stacked.oid_hex, stacked.chipItem)
                })
                return
            }
            op.complete({
                "name": "menu_list_after",
                "when": () => ({ "menu": commitMenu.opened, "beat": op.how !== "own" || !rowHost.listSettling }),
                "report": () => {
                    const at = op.headRow()
                    const now = at < 0 ? null : graphPane.view.itemAtIndex(at)
                    return "menu_list how=" + op.how + " menu=" + commitMenu.opened
                           + " list=" + refList.opened + " raised=" + page.menuArea
                           + " kept=" + (refList.opened && now !== null && rowHost.refListOid === now.oid_hex)
                } })
        }
    }
    // The row under a standing menu, asked for its card where the row's own ask goes in (`rowHoverRequested`). Asked
    // once the menu is up, read a beat later: a card that was going to open opens synchronously.
    PageStage {
        id: menuHoverTimer
        name: "menu_hover"
        order: ["asked"]
        property string oidHex: ""
        onTriggered: {
            const op = menuHoverTimer
            if (op.at("start")) {
                if (!Awaited.all("menu_hover_ready", { "ready": commitMenu.opened }))
                    return
                const row = graphPane.view.itemAtIndex(graphModel.rowOf(op.oidHex))
                // Not laid out yet: asking now would wait for an answer to a question nobody put.
                if (!Awaited.all("menu_hover_row", { "row": !!row }))
                    return
                op.input("asked", () => {
                    graphPane.view.rowHoverRequested(row, true)
                })
                return
            }
            op.complete({ "name": "menu_hover_ready", "when": () => ({ "ready": commitMenu.opened }),
                          "report": () => "menu_hover menu=" + commitMenu.opened + " card=" + rowCard.opened
                                          + " list=" + refList.opened })
        }
    }
}

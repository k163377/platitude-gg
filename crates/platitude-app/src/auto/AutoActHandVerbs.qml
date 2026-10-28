pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// PGG_AUTO_ACT=middle-hand <surface>: the middle button's hand, pressed on one surface that scrolls and left drifting
/// for the shot (デザイン規約 §中クリックの自動スクロール). Each surface seats its own hand on its own terms (a list's
/// frame, a box's words, the pane over the log's text picker), so a picture of one says nothing about the next and
/// each is pressed where it stands.
///
/// The run waits for the surface having moved — a hand that started is not yet the gesture. The hand is left running,
/// so the anchor's ring is in the picture.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var graphModel: driver.graphModel
    readonly property var worktree: driver.worktree
    readonly property var detailsModel: driver.detailsModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var wipPane: driver.wipPane
    readonly property var carriedPane: driver.carriedPane
    readonly property var planPane: driver.planPane
    readonly property var navProbe: driver.navProbe

    /// How far from the anchor the pointer is put: out of the dead zone, and far enough that the surface moves within
    /// a second.
    readonly property real reach: Metrics.middleScrollDeadZone + 64
    /// How far the surface has to have gone to have gone at all — past a rounding, short of a row.
    readonly property real moved: 8
    /// The row of `edges` whose subject and body both run past their boxes. A commit, so both boxes are read-only and
    /// the answer is the same on every platform — a writable box hands a middle click to the paste where the platform
    /// has one (that side is `wip-description` / `wip-summary`).
    readonly property int longMessageRow: 7
    /// The `chosen` surface's range: thirty of `spread`'s commits overrun their list's share of the pane at any window.
    readonly property int chosenFrom: 1
    readonly property int chosenTo: 30
    /// Where `plan` opens from: that many commits under the tip of `spread`, a plan longer than its pane.
    readonly property int planBack: 40

    /// Runs `act` if it is this family's, and says whether it was.
    function run(act, arg) {
        if (act !== "middle-hand")
            return false
        handTimer.surface = arg
        // The requests that bring the surface into its state are stages of their own, before the hand's.
        handTimer.order = acts.asksOf(arg).concat(["pressed"])
        handTimer.start()
        return true
    }

    // ---- the requests --------------------------------------------------------------------------------------------
    /// The requests that bring a surface into its state, in the order they are made: each an input of `handTimer`'s
    /// (`request`), made again every tick until its effect shows (rules/app-ui.md §UI 自動化) — but the choice's two
    /// presses, each made once on a row laid out (`choose`). The log's reads are an operation of their own
    /// (`readsTimer`), started once the log keeps them.
    function asksOf(name) {
        if (name === "files" || name === "description" || name === "summary")
            return ["selected"]
        if (name === "block")
            return ["selected", "floored"]
        if (name === "chosen")
            return ["chose_first", "chose_last"]
        if (name === "wip-files")
            return ["wip"]
        if (name === "wip-block")
            return ["wip", "floored"]
        if (name === "wip-description" || name === "wip-summary")
            return ["wip", "typed"]
        if (name === "peek")
            return ["folded", "peeked"]
        if (name === "ref-list")
            return ["expanded"]
        if (name === "commands")
            return ["raised", "kept"]
        if (name === "carried")
            return ["stood"]
        if (name === "plan")
            return ["planned"]
        return []
    }
    /// The next of the surface's requests to make, or "" once they are all made.
    function nextAsk() {
        const asks = acts.asksOf(handTimer.surface)
        if (handTimer.at("start"))
            return asks.length === 0 ? "" : asks[0]
        for (let i = 0; i + 1 < asks.length; i++) {
            if (handTimer.at(asks[i]))
                return asks[i + 1]
        }
        return ""
    }
    /// What a request waits for before it can be made, or "" once it can.
    function askWait(ask) {
        if (ask === "selected" && acts.detailsOid(handTimer.surface) === "")
            return "the commit's row"
        if (ask === "floored" && handTimer.surface === "block" && !driver.cardSettled)
            return "the commit read"
        if (ask === "expanded" && !acts.headItem())
            return "HEAD's row"
        if (ask === "kept" && page.commandsPane === null)
            return "the log's panel"
        if (ask === "stood" && !acts.carriedItem())
            return "the worktree's row"
        if (ask === "planned" && acts.planFrom() === "")
            return "the row the plan opens from"
        return ""
    }
    /// What a request is made again for, until it shows.
    function askedFor(ask) {
        if (ask === "selected")
            return "the commit in the right pane"
        if (ask === "floored")
            return "a block taller than its pane"
        if (ask === "chose_first")
            return "row " + acts.chosenFrom + " laid out"
        if (ask === "chose_last")
            return "row " + acts.chosenTo + " laid out"
        if (ask === "wip")
            return "the worktree's face"
        if (ask === "typed")
            return "the words in the box"
        if (ask === "folded")
            return "the rail"
        if (ask === "peeked")
            return "the peek"
        if (ask === "expanded")
            return "the card"
        if (ask === "raised")
            return "the log"
        if (ask === "kept")
            return "the log keeping its reads"
        if (ask === "stood")
            return "the worktree's pane"
        return "the plan"
    }
    /// `handTimer`'s press for the request `ask`: made where its effect is not there yet, answering whether it is.
    function request(ask) {
        const name = handTimer.surface
        if (ask === "selected") {
            // Through the page's own door: a page on a dirty tree opens on the worktree (`RepoPage.trySelectDefault`).
            const oid = acts.detailsOid(name)
            if (page.selectedOid !== oid)
                page.activateRow(oid)
            return page.selectedOid === oid
        }
        if (ask === "floored") {
            acts.toFloor()
            const flick = name === "block" ? detailsPane.messageBlock : wipPane.commitBlock
            return flick.visible && flick.contentHeight > flick.height + 1
        }
        if (ask === "chose_first" || ask === "chose_last")
            return acts.choose(ask === "chose_first")
        if (ask === "wip") {
            if (!page.wipShown)
                page.showWip()
            return page.wipShown
        }
        if (ask === "typed")
            return acts.typeWords(name)
        if (ask === "folded") {
            if (!page.sidebarCollapsed)
                page.foldByHand(true)
            return page.sidebarCollapsed
        }
        if (ask === "peeked") {
            // A rest and then the walk into it is the one state that keeps a peek standing with no pointer
            // (`NavProbe.peekInto`).
            if (!navProbe.peekStanding) {
                navProbe.peekAt("tag")
                navProbe.peekInto("tag")
            }
            return navProbe.peekStanding
        }
        if (ask === "expanded")
            return acts.expand()
        if (ask === "raised") {
            if (!page.commandsOpen)
                page.raiseCommands()
            return page.commandsOpen
        }
        if (ask === "kept")
            return acts.keepReads()
        if (ask === "stood")
            return acts.standCarried()
        return acts.openPlan()
    }

    /// The commit the right pane is put on for a details surface.
    function detailsOid(name) {
        return name === "files" || name === "block" ? worktree.headOid : graphModel.oidAt(acts.longMessageRow)
    }
    /// The panes taken down to their own floor (規約 §窓の床): the blocks scroll only once the pane is too short for
    /// them. The window's floor holds the log at its floor too, so the window goes down to it and the log comes up to
    /// take that share — with the log down the panes stand taller than their floor.
    function toFloor() {
        const window = page.Window.window
        const floor = Math.ceil(window.floorHeight)
        if (window.height !== floor)
            window.height = floor
        if (!page.commandsOpen)
            page.commandsOpen = true
    }
    /// The choice of commits, made as `graph-choose-range` makes it: a plain press, then Shift on the last. False while
    /// the row is not laid out — the view is asked for it.
    function choose(first) {
        const want = first ? acts.chosenFrom : acts.chosenTo
        const item = graphPane.view.itemAtIndex(want)
        if (!item) {
            graphPane.view.positionViewAtIndex(want, ListView.Contain)
            return false
        }
        item.leftClick(first ? Qt.NoModifier : Qt.ShiftModifier)
        return true
    }
    /// A box of the worktree's given words that run past it, where it holds none.
    function typeWords(name) {
        const commitBlock = wipPane.commitBlock
        if (name === "wip-description") {
            if (commitBlock.bodyText === "")
                wipPane.setMessage("", acts.longBody())
            return commitBlock.bodyText !== ""
        }
        if (commitBlock.subjectText === "")
            wipPane.setMessage(acts.longSubject(), "")
        return commitBlock.subjectText !== ""
    }
    function longBody() {
        let lines = []
        for (let n = 1; n <= 40; n++)
            lines.push("Line " + n + " of a description that runs past the box it is written in.")
        return lines.join("\n")
    }
    function longSubject() {
        return "A summary pasted in whole, long past the one line a summary is meant to be, ".repeat(24)
    }
    /// HEAD's graph row, laid out — null while it is not.
    function headItem() {
        const row = graphModel.rowOf(worktree.headOid)
        return row >= 0 ? graphPane.view.itemAtIndex(row) : null
    }
    /// The card a chip column unfolds into, on HEAD's row, whose chips are `manytags`' two thousand.
    function expand() {
        const card = driver.refList
        if (!card.opened) {
            const stacked = acts.headItem()
            if (!stacked)
                return false
            graphPane.view.chipExpandRequested(stacked.oid_hex, stacked.index, stacked.chipItem.records,
                                               stacked.chipItem)
        }
        return card.opened
    }
    /// The log told to keep its background reads, and the reads that fill it past its panel set going.
    function keepReads() {
        const log = page.commandsPane.commandsModel
        if (!log.backgroundReads)
            log.setBackgroundReads(true)
        if (!log.backgroundReads)
            return false
        readsTimer.start()
        return true
    }
    /// The graph row of another worktree, laid out — null while it is not.
    function carriedItem() {
        const row = driver.rowOfWorktree("here")
        return row >= 0 ? graphPane.view.itemAtIndex(row) : null
    }
    /// Another worktree's files, opened by pressing its graph row (as `carried-read` does); preset `carried-many`
    /// overfills the pane.
    function standCarried() {
        if (page.carriedPath === "") {
            const item = acts.carriedItem()
            if (!item)
                return false
            item.leftClick(Qt.NoModifier)
        }
        return page.carriedPath !== ""
    }
    /// The commit the plan opens from, "" while the graph has no row that far down.
    function planFrom() {
        return graphModel.oidAt(graphModel.rowOf(worktree.headOid) + acts.planBack)
    }
    /// The interactive-rebase plan, opened the way its menu entry opens it (`rebase-plan`) from far enough down that
    /// its rows run past the pane.
    function openPlan() {
        if (!page.planShown) {
            const fromOid = acts.planFrom()
            if (fromOid === "")
                return false
            page.openRowMenu(fromOid)
            page.startRebasePlan(fromOid)
            driver.commitMenu.close()
        }
        return page.planShown
    }

    // ---- the surfaces --------------------------------------------------------------------------------------------
    /// The surface an argument names, once its requests are made and it holds more than it has room for: its hand,
    /// what it scrolls (`offset`) and which way it has room (`down`); until then `wait`, what it waits for.
    function seat(name) {
        if (name === "files" || name === "block" || name === "description" || name === "summary")
            return acts.detailsSeat(name)
        if (name === "chosen")
            return acts.chosenSeat()
        if (name.startsWith("wip-"))
            return acts.wipSeat(name)
        if (name === "tags")
            return acts.listSeat(navProbe.listOf("tag"))
        if (name === "peek")
            return acts.listSeat(navProbe.peek.list)
        if (name === "ref-list")
            return acts.listSeat(driver.refList.list)
        if (name === "commands")
            return acts.commandsSeat()
        if (name === "carried")
            return acts.listSeat(carriedPane.view)
        if (name === "plan")
            return page.planActive ? acts.listSeat(planPane.view) : acts.waiting("the plan's rows")
        return acts.waiting("a surface this verb knows")
    }
    function waiting(what) {
        return { wait: what }
    }
    /// How far a seat's surface has gone: a box's own place for its words, elsewhere the view's.
    function offset(seat) {
        if (seat.box === "description")
            return seat.item.descriptionAt
        if (seat.box === "summary")
            return seat.item.summaryAt
        return seat.item.contentY
    }
    function listSeat(view) {
        if (!view || !view.visible || view.count === 0)
            return acts.waiting("rows")
        if (view.contentHeight <= view.height + 1)
            return acts.waiting("more rows than room")
        return {
            hand: view.hand,
            item: view,
            box: "",
            down: view.contentY < view.clampY(Number.MAX_VALUE) - 1
        }
    }
    function flickSeat(hand, flick) {
        if (!flick.visible || flick.contentHeight <= flick.height + 1)
            return acts.waiting("a block taller than its pane")
        return {
            hand: hand,
            item: flick,
            box: "",
            down: flick.contentY < flick.contentHeight - flick.height - 1
        }
    }
    /// A box whose words run past it: its hand shows exactly then (`DescriptionBox`), and the words open at their first
    /// line (`pinnedTop`), so the room is below. Pressed on its words even when they claim the press — where the
    /// platform pastes, that claim is what the run is about.
    function boxSeat(hand, block, box) {
        if (!hand.visible)
            return acts.waiting("words past the box")
        return { hand: hand, item: block, box: box, down: true, words: true }
    }
    /// Where the run presses: a box's middle, or elsewhere the first of a few points down the middle and along the
    /// edge that nothing under the hand claims (`MiddleAutoScroll.claimedAt`) — the surface's own hand is what is read.
    function pressPoint(seat) {
        const hand = seat.hand
        const middle = Qt.point(hand.width / 2, hand.height / 2)
        if (seat.words)
            return middle
        for (const fx of [0.5, 0.97]) {
            for (const fy of [0.5, 0.25, 0.75, 0.1, 0.9]) {
                const at = Qt.point(hand.width * fx, hand.height * fy)
                if (!hand.claimedAt(at))
                    return at
            }
        }
        return middle
    }

    /// The right pane on one commit, read.
    function detailsSeat(name) {
        if (page.selectedOid !== acts.detailsOid(name))
            return acts.waiting("the commit in the right pane")
        if (!driver.cardSettled)
            return acts.waiting("the commit read")
        if (name === "files")
            return acts.listSeat(detailsPane.filesWalk.view)
        const block = detailsPane.messageBlock
        if (name === "description")
            return acts.boxSeat(block.descriptionHand, block, "description")
        if (name === "summary")
            return acts.boxSeat(block.summaryHand, block, "summary")
        return acts.flickSeat(block.hand, block)
    }
    function wipSeat(name) {
        if (!page.wipShown)
            return acts.waiting("the worktree's face")
        const commitBlock = wipPane.commitBlock
        if (name === "wip-files") {
            const standing = wipPane.bucketPanes
            for (let i = 0; i < standing.length; i++)
                if (standing[i].section === "unstaged")
                    return acts.listSeat(standing[i].list)
            return acts.waiting("the unstaged bucket")
        }
        if (name === "wip-block")
            return acts.flickSeat(commitBlock.hand, commitBlock)
        if (name === "wip-description")
            return acts.boxSeat(commitBlock.descriptionHand, commitBlock, "description")
        if (name === "wip-summary")
            return acts.boxSeat(commitBlock.summaryHand, commitBlock, "summary")
        return acts.waiting("a surface of the worktree's this verb knows")
    }
    /// The choice read and drawn.
    function chosenSeat() {
        if (page.chosenCount !== acts.chosenTo - acts.chosenFrom + 1 || !detailsModel.selectionLoaded
                || !detailsPane.chosenListDrawn)
            return acts.waiting("the choice read")
        return acts.listSeat(detailsPane.chosenView)
    }
    /// The log, filled past its panel (`readsTimer`).
    function commandsSeat() {
        const pane = page.commandsPane
        if (pane === null)
            return acts.waiting("the log's panel")
        const view = pane.view
        if (view.contentHeight <= view.height + 1)
            return acts.waiting("more rows than room")
        return {
            hand: pane.hand,
            item: view,
            box: "",
            // The log follows its newest row, so the room it has is above.
            down: view.contentY < view.clampY(Number.MAX_VALUE) - 1
        }
    }

    // ---- the press, the drift, the claim --------------------------------------------------------------------------
    PageStage {
        id: handTimer
        name: "middle_hand"
        order: ["pressed"]
        property string surface: ""
        /// Whether the surface has gone further than a rounding since the press.
        function went() {
            return Math.abs(acts.offset(handTimer.witnessed("seat")) - handTimer.witnessed("from")) > acts.moved
        }
        /// What the hand answered: `took=` / `claimed=` as the press found them, the rest as it stands.
        function line() {
            const hand = handTimer.witnessed("seat").hand
            const took = handTimer.witnessed("took")
            const claimed = handTimer.witnessed("claimed")
            const answered = claimed ? !took : took && handTimer.went() && hand.scrolling
            return "middle_hand surface=" + handTimer.surface
                   + " answered=" + answered
                   + " took=" + took
                   + " claimed=" + claimed
                   + " moved=" + handTimer.went()
                   + " scrolling=" + hand.scrolling
                   + " ticks=" + hand.ticks
        }
        onTriggered: {
            const op = handTimer
            if (!op.at("pressed")) {
                const ask = acts.nextAsk()
                if (ask !== "") {
                    const before = acts.askWait(ask)
                    // What the surface waits for, said when it changes (rules/app-ui.md §UI 自動化).
                    op.note("middle_hand surface=" + op.surface + " waiting="
                            + (before !== "" ? before : acts.askedFor(ask)))
                    if (before === "")
                        op.input(ask, () => acts.request(ask))
                    return
                }
                const seat = acts.seat(op.surface)
                if (seat.wait !== undefined) {
                    op.note("middle_hand surface=" + op.surface + " waiting=" + seat.wait)
                    return
                }
                // Pressed once the hand's frame has stood across a swapped frame — a box grows to its words a layout
                // after they land, and an earlier press anchors the ring off the box's middle.
                if (!op.still("middle_hand_seat", seat.hand.width + "x" + seat.hand.height))
                    return
                // Where the press went in, in the hand's frame; and whether something under the hand claimed the
                // point, read before the press: the hand steps aside then, and nothing else says why a press started
                // nothing.
                op.input("pressed", () => {
                    const at = op.witness("at", acts.pressPoint(seat))
                    op.witness("seat", seat)
                    op.witness("from", acts.offset(seat))
                    op.witness("claimed", seat.hand.claimedAt(at))
                    if (op.witness("took", seat.hand.press(at)))
                        seat.hand.drift(Qt.point(at.x, at.y + (seat.down ? acts.reach : -acts.reach)))
                }, () => "middle_hand surface=" + op.surface + " pressed took=" + op.witnessed("took")
                         + " claimed=" + op.witnessed("claimed")
                         + " at=" + Math.round(op.witnessed("at").x) + "," + Math.round(op.witnessed("at").y))
                return
            }
            // A hand that stepped aside has nothing to wait for; one that took the press waits for the surface to go.
            op.complete({
                "name": "middle_hand_went",
                "when": () => ({ "went": !op.witnessed("took")
                                         || (op.went() && op.witnessed("seat").hand.ticks >= 2) }),
                "report": () => op.line() })
        }
    }
    // The log's reads for `commands`, asked one at a time off its row count, so the asks never outrun the rows: each
    // ask a generation of its own, begun again once the last one's rows have landed, until the log holds more than it
    // has room for.
    PageStage {
        id: readsTimer
        name: "middle_hand_reads"
        onTriggered: {
            const op = readsTimer
            const view = page.commandsPane.view
            if (view.contentHeight > view.height + 1) {
                op.stop()
                return
            }
            if (op.at("asked")) {
                if (!Awaited.all("middle_hand_reads", { "landed": view.count !== op.witnessed("count") }))
                    return
                op.again()
            }
            op.input("asked", () => {
                op.witness("count", view.count)
                driver.repoTab.refreshQuick()
            })
        }
    }
}

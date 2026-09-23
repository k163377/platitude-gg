pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// PGG_AUTO_ACT=middle-hand <surface>: the middle button's hand, pressed on one surface that scrolls and left drifting
/// for the shot (デザイン規約 §中クリックの自動スクロール). Every surface that scrolls carries a hand of its own, seated
/// on its own terms — on a list's own frame, over a box's words, at the pane over the log's text picker — so a
/// picture of one says nothing about the next, and each is pressed where it stands.
///
/// The press goes in at the hand's own `press`, the one line its handler is, and the pointer is then put a little past
/// the dead zone towards wherever the surface has room to go. **What the run waits for is the surface having moved**:
/// a hand that started and a surface that went are two claims, and the second is the gesture. The hand is left
/// running, so the anchor's ring is in the picture.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on hangs off
/// that driver; the names it owns are read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timer below; it is a sizeless holder.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is the file that
    /// builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var graphModel: driver.graphModel
    readonly property var workTree: driver.workTree
    readonly property var detailsModel: driver.detailsModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var wipPane: driver.wipPane
    readonly property var carriedPane: driver.carriedPane
    readonly property var planPane: driver.planPane
    readonly property var navProbe: driver.navProbe

    /// Where the pointer is put, from the anchor: out of the dead zone on the one axis the surface goes along, and far
    /// enough that the drift is pixels a tick and the surface moves inside a second.
    readonly property real reach: Metrics.middleScrollDeadZone + 64
    /// How far the surface has to have gone to have gone at all — past a rounding, short of a row.
    readonly property real moved: 8
    /// The row of `edges` whose subject is two thousand characters long and whose body runs past its box (`text-bar`
    /// reads the same row). **A commit nobody can rewrite from here**, so both its boxes are read-only and the answer
    /// is one on every platform — a box that can be written in hands a middle click to the paste where the platform
    /// has one, and the working tree's two are where that side is read (`wip-description` / `wip-summary`).
    readonly property int longMessageRow: 7
    /// The commits the `chosen` surface holds: the plain press on the first and the held one on the last. Thirty of
    /// `spread`'s commits run past the share of the pane their list is given, whatever the window.
    readonly property int chosenFrom: 1
    readonly property int chosenTo: 30
    /// Where `plan` opens from: that many commits under the tip of `spread`, a plan longer than its pane.
    readonly property int planBack: 40

    /// Runs `act` if it is this family's, and says whether it was.
    function run(act, arg) {
        if (act !== "middle-hand")
            return false
        handTimer.begin(arg)
        return true
    }

    // ---- the surfaces ------------------------------------------------------------------------------------------
    /// The surface an argument names, once it stands with more in it than it has room for: its hand, how far it has
    /// gone (`at`), and which way it has room to go (`down`). Until then, what it is waiting for (`wait`), which the
    /// timer names as it changes. **Every request made on the way can be made again** — a commit put in the pane, a
    /// pane put on screen, the window taken to its floor — and it is made again on every tick until its effect shows
    /// (規約 §UI 自動化: 効果を待つ要求は撃ち直す). The two presses a choice is made of are made once each, off the
    /// timer's own count.
    function seat(name) {
        if (name === "files" || name === "block")
            return acts.detailsSeat(name, workTree.headOid)
        if (name === "description" || name === "summary")
            return acts.detailsSeat(name, graphModel.oidAt(acts.longMessageRow))
        if (name === "chosen")
            return acts.chosenSeat()
        if (name.startsWith("wip-"))
            return acts.wipSeat(name)
        if (name === "tags")
            return acts.listSeat(navProbe.listOf("tag"))
        if (name === "peek")
            return acts.peekSeat()
        if (name === "ref-list")
            return acts.refListSeat()
        if (name === "commands")
            return acts.commandsSeat()
        if (name === "carried")
            return acts.carriedSeat()
        if (name === "plan")
            return acts.planSeat()
        return acts.waiting("a surface this verb knows")
    }
    function waiting(what) {
        return { wait: what }
    }
    /// A list with more rows than room, sent by its own hand.
    function listSeat(view) {
        if (!view || !view.visible || view.count === 0)
            return acts.waiting("rows")
        if (view.contentHeight <= view.height + 1)
            return acts.waiting("more rows than room")
        return {
            hand: view.hand,
            at: () => view.contentY,
            down: view.contentY < view.clampY(Number.MAX_VALUE) - 1
        }
    }
    /// A block that scrolls as one piece: its content taller than its frame.
    function flickSeat(hand, flick) {
        if (!flick.visible || flick.contentHeight <= flick.height + 1)
            return acts.waiting("a block taller than its pane")
        return {
            hand: hand,
            at: () => flick.contentY,
            down: flick.contentY < flick.contentHeight - flick.height - 1
        }
    }
    /// A box whose words run past it. Its hand is on screen exactly then (`DescriptionBox`), so that is the wait; the
    /// words open at their first line (`pinnedTop`), so the room is below. **Pressed on its words** whatever claims
    /// them (`pressPoint`): where they are being written and the platform pastes, the claim is what the run is about.
    function boxSeat(hand, at) {
        if (!hand.visible)
            return acts.waiting("words past the box")
        return { hand: hand, at: at, down: true, words: true }
    }
    /// Where the run presses: the middle of the hand's frame — or, on a surface with boxes of words standing on it, the
    /// first point in a few down its middle and along its edge that nothing under the hand claims
    /// (`MiddleAutoScroll.claimedAt`: a box with a hand of its own, or words being written where the platform pastes).
    /// What those claims do is read on the boxes themselves; here the surface's own hand is.
    function pressPoint(seat) {
        const hand = seat.hand
        const middle = Qt.point(hand.width / 2, hand.height / 2)
        if (seat.words)
            return middle
        for (const fx of [0.5, 0.97]) {
            for (const fy of [0.5, 0.25, 0.75, 0.1, 0.9]) {
                const x = hand.width * fx
                const y = hand.height * fy
                if (!hand.claimedAt(x, y))
                    return Qt.point(x, y)
            }
        }
        return middle
    }
    /// The pane's own window, taken down to its floor: the blocks scroll only once the pane is too short to hold them
    /// (規約 §窓の床), and the floor is the shortest a reader can make it.
    function toFloor() {
        const window = page.Window.window
        const floor = Math.ceil(window.floorHeight)
        if (window.height !== floor)
            window.height = floor
    }

    /// The right pane on one commit. **A page opened on a dirty tree is showing the working tree** (`RepoPage.
    /// trySelectDefault`), so the commit is put in the pane through the page's own door (`pane-bar` goes in the same
    /// way) and read before anything is asked of it.
    function detailsSeat(name, oid) {
        if (oid === "")
            return acts.waiting("the commit's row")
        if (page.selectedOid !== oid) {
            page.activateRow(oid)
            return acts.waiting("the commit in the right pane")
        }
        if (!driver.cardSettled)
            return acts.waiting("the commit read")
        if (name === "files")
            return acts.listSeat(detailsPane.filesWalk.view)
        const block = detailsPane.messageBlock
        if (name === "description")
            return acts.boxSeat(block.descriptionHand, () => block.descriptionAt)
        if (name === "summary")
            return acts.boxSeat(block.summaryHand, () => block.summaryAt)
        acts.toFloor()
        return acts.flickSeat(block.hand, block)
    }
    /// The working tree's face: its buckets, its block, and the two boxes — the boxes typed into first, past their
    /// own room, the way their own hook types (`WipPane.setMessage`).
    function wipSeat(name) {
        if (!page.wipShown) {
            page.showWip()
            return acts.waiting("the working tree's face")
        }
        const commitBlock = wipPane.commitBlock
        if (name === "wip-files") {
            const standing = wipPane.bucketPanes
            for (let i = 0; i < standing.length; i++)
                if (standing[i].section === "unstaged")
                    return acts.listSeat(standing[i].list)
            return acts.waiting("the unstaged bucket")
        }
        if (name === "wip-block") {
            acts.toFloor()
            return acts.flickSeat(commitBlock.hand, commitBlock)
        }
        if (name === "wip-description") {
            if (commitBlock.bodyText === "")
                wipPane.setMessage("", acts.longBody())
            return acts.boxSeat(commitBlock.descriptionHand, () => commitBlock.descriptionAt)
        }
        if (name === "wip-summary") {
            if (commitBlock.subjectText === "")
                wipPane.setMessage(acts.longSubject(), "")
            return acts.boxSeat(commitBlock.summaryHand, () => commitBlock.summaryAt)
        }
        return acts.waiting("a surface of the working tree's this verb knows")
    }
    /// A body longer than its box and a subject longer than its cap.
    function longBody() {
        let lines = []
        for (let n = 1; n <= 40; n++)
            lines.push("Line " + n + " of a description that runs past the box it is written in.")
        return lines.join("\n")
    }
    function longSubject() {
        return "A summary pasted in whole, long past the one line a summary is meant to be, ".repeat(24)
    }

    /// The choice of commits: the plain press on one row, then the held press on the last, each through the row's own
    /// click (`graph-choose-range` makes the same two). A row the view has not laid out is sent into view and pressed
    /// on a later tick.
    function chosenSeat() {
        if (handTimer.pressedChoice < 2) {
            const want = handTimer.pressedChoice === 0 ? acts.chosenFrom : acts.chosenTo
            const item = graphPane.view.itemAtIndex(want)
            if (!item) {
                graphPane.view.positionViewAtIndex(want, ListView.Contain)
                return acts.waiting("row " + want + " laid out")
            }
            item.leftClick(handTimer.pressedChoice === 0 ? Qt.NoModifier : Qt.ShiftModifier)
            handTimer.pressedChoice++
            return acts.waiting("the choice")
        }
        if (page.chosenCount !== acts.chosenTo - acts.chosenFrom + 1 || !detailsModel.selectionLoaded
                || !detailsPane.chosenListDrawn)
            return acts.waiting("the choice read")
        return acts.listSeat(detailsPane.chosenView)
    }
    /// The TAGS section the folded rail opens beside itself, on `manytags`' two thousand. The rest on the cell and the
    /// hand walked down into what it opened are the one state that keeps a peek standing with no pointer to hold it
    /// (`NavProbe.peekInto`).
    function peekSeat() {
        if (!page.sidebarCollapsed) {
            page.foldByHand(true)
            return acts.waiting("the rail")
        }
        if (!navProbe.peekStanding) {
            navProbe.peekAt("tag")
            navProbe.peekInto("tag")
            return acts.waiting("the peek")
        }
        return acts.listSeat(navProbe.peek.list)
    }
    /// The card a chip column unfolds into, on HEAD's row, whose chips are `manytags`' two thousand.
    function refListSeat() {
        const card = driver.refList
        if (!card.opened) {
            const row = graphModel.rowOf(workTree.headOid)
            const stacked = row >= 0 ? graphPane.view.itemAtIndex(row) : null
            if (!stacked)
                return acts.waiting("HEAD's row")
            graphPane.view.chipExpandRequested(stacked.oid_hex, stacked.index, stacked.chipItem.records,
                                               stacked.chipItem)
            return acts.waiting("the card")
        }
        return acts.listSeat(card.list)
    }
    /// The log, raised and filled past its panel. The reads a window makes on its own are what a log fills with here,
    /// so the panel is told to keep them and the page is asked to read again whenever the last reading has landed —
    /// one at a time, off the log's own count, so the asks never outrun the rows.
    function commandsSeat() {
        if (!page.commandsOpen) {
            page.raiseCommands()
            return acts.waiting("the log")
        }
        const pane = page.commandsPane
        if (pane === null)
            return acts.waiting("the log's panel")
        if (!pane.commandsModel.backgroundReads)
            pane.commandsModel.setBackgroundReads(true)
        const view = pane.view
        if (view.contentHeight <= view.height + 1) {
            if (view.count !== handTimer.readAt) {
                handTimer.readAt = view.count
                driver.repoTab.refreshQuick()
            }
            return acts.waiting("more rows than room")
        }
        return {
            hand: pane.hand,
            at: () => view.contentY,
            // The log follows its newest row, so the room it has is above.
            down: view.contentY < view.clampY(Number.MAX_VALUE) - 1
        }
    }
    /// Another copy's files: its row in the graph pressed through the row itself (`carried-read` presses the same),
    /// and the pane it opens holding more files than it has room for (`carried-many`).
    function carriedSeat() {
        if (page.carriedPath === "") {
            const row = driver.rowOfCopy("here")
            const item = row >= 0 ? graphPane.view.itemAtIndex(row) : null
            if (!item)
                return acts.waiting("the copy's row")
            item.leftClick(Qt.NoModifier)
            return acts.waiting("the copy's pane")
        }
        return acts.listSeat(carriedPane.view)
    }
    /// The interactive-rebase plan, opened the way its menu entry opens it (`rebase-plan`) from far enough down that
    /// its rows run past the pane.
    function planSeat() {
        if (!page.planShown) {
            const fromOid = graphModel.oidAt(graphModel.rowOf(workTree.headOid) + acts.planBack)
            if (fromOid === "")
                return acts.waiting("the row the plan opens from")
            page.openRowMenu(fromOid)
            page.startRebasePlan(fromOid)
            driver.commitMenu.close()
            return acts.waiting("the plan")
        }
        if (!page.planActive)
            return acts.waiting("the plan's rows")
        return acts.listSeat(planPane.view)
    }

    // ---- the press, the drift, the claim --------------------------------------------------------------------------
    SampleTimer {
        id: handTimer
        property string surface: ""
        /// What the surface was last seen waiting for, said once each time it changes — a run that stops at the
        /// ceiling says where (規約 §UI 自動化: 段を持つドライバは段が変わるたびに名乗る).
        property string waitingFor: ""
        property var standing: null
        property real fromAt: 0
        property bool took: false
        /// Whether something under the hand claimed the point pressed, read before the press — the hand steps aside
        /// exactly then, and nothing else says why a press started nothing.
        property bool claimed: false
        /// How many of the choice's two presses have gone in, and the log's row count the last read was asked at.
        property int pressedChoice: 0
        property int readAt: -1
        /// The frame the hand stood in on the last tick. **Pressed once it has stood still for one**: a box grows to
        /// its words a layout after they land, and a press made before that anchors the ring where the box's middle
        /// was going to be.
        property string seatSize: ""
        function begin(name) {
            handTimer.surface = name
            handTimer.waitingFor = ""
            handTimer.standing = null
            handTimer.pressedChoice = 0
            handTimer.readAt = -1
            handTimer.seatSize = ""
            handTimer.start()
        }
        onTriggered: {
            if (handTimer.standing === null) {
                const seat = acts.seat(handTimer.surface)
                if (seat.wait !== undefined) {
                    if (seat.wait !== handTimer.waitingFor) {
                        handTimer.waitingFor = seat.wait
                        Harness.report("middle_hand surface=" + handTimer.surface + " waiting=" + seat.wait)
                    }
                    return
                }
                const size = seat.hand.width + "x" + seat.hand.height
                if (size !== handTimer.seatSize) {
                    handTimer.seatSize = size
                    return
                }
                handTimer.standing = seat
                handTimer.fromAt = seat.at()
                const at = acts.pressPoint(seat)
                handTimer.claimed = seat.hand.claimedAt(at.x, at.y)
                handTimer.took = seat.hand.press(at.x, at.y)
                if (handTimer.took)
                    seat.hand.drift(at.x, at.y + (seat.down ? acts.reach : -acts.reach))
                Harness.report("middle_hand surface=" + handTimer.surface + " pressed took=" + handTimer.took
                               + " claimed=" + handTimer.claimed
                               + " at=" + Math.round(at.x) + "," + Math.round(at.y))
                return
            }
            const hand = handTimer.standing.hand
            const went = Math.abs(handTimer.standing.at() - handTimer.fromAt) > acts.moved
            // A hand that stepped aside has nothing to wait for: the press went on to what claimed it, and whether it
            // started anything is already said. One that took it is waited out until the surface has gone.
            if (handTimer.took && (!went || hand.ticks < 2))
                return
            handTimer.stop()
            const answered = handTimer.claimed ? !handTimer.took : handTimer.took && went && hand.scrolling
            Harness.report("middle_hand surface=" + handTimer.surface
                           + " answered=" + answered
                           + " took=" + handTimer.took
                           + " claimed=" + handTimer.claimed
                           + " moved=" + went
                           + " scrolling=" + hand.scrolling
                           + " ticks=" + hand.ticks)
            driver.complete()
        }
    }
}

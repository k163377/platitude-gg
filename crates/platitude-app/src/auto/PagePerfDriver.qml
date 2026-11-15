import QtQuick
import platitude
import platitude.ui

/// Each model change must reach a frame before the next action.
/// The animation duration is a sampling window.
///
/// **The frame that answers a change is the first one whose scene graph synchronised after it** (`PerfFrameWait`).
/// A swapped frame alone says no such thing: on the threaded render loop `frameSwapped` is queued from the render
/// thread, and the first to arrive after a request is as a rule a frame that was already on its way.
///
/// **An operation starts on a page that owes nothing** (`owed`): timed from a page still arriving, it is timed
/// against that page's own reads, and one operation's leftovers are the next one's load. `_PGG_PERF_BACK_TO_BACK`
/// asks for exactly that load instead. What no term here sees is a colouring the product goes on with for a diff
/// already closed: a file the product colours is measured as a `coloured` case, which stays until the colours land.
///
/// **Two frames are timed for each input**: the one that first shows it taken (`askFeedback` — the row selected,
/// the pane up and waiting), which the operation budget is read against
/// (internal-docs/メモリと操作応答の改善計画.md §2), and the one with the answer in it (`note`), its completion.
Item {
    id: driver

    property Item page
    property RepoTab repoTab
    property GraphModel graphModel
    property WorktreeModel worktree
    property NavSectionModel branchesModel
    property DetailsModel detailsModel
    property DiffModel diffModel
    property GraphPane graphPane
    property DiffPane diffPane
    property int operation: 0
    property string wantedPath: ""
    property string wantedFingerprint: ""
    property int wantedGeneration: -1
    readonly property string caseName: PerfProbe.caseField(driver.operation, 0)
    readonly property bool wantsColour: PerfProbe.caseField(driver.operation, 3) === "coloured"
    property string stage: "opening"
    property string wantedOid: ""
    property real actionStart: 0
    /// What the idle stage goes on to once the page owes nothing (`resume`), and whether the first operation has
    /// been reached.
    property string resumeWith: ""
    property bool begun: false
    /// The feedback frame being waited for: the measurement it becomes, and the state it has to show.
    property string feedbackKind: ""
    property var feedbackShown: null
    // The stage's frame and the input's feedback frame, waited for side by side. Property values, not bare children
    // (rules-refs/app-ui.md, the `QtObject` parts the collector takes).
    readonly property PerfFrameWait stageFrame: PerfFrameWait { id: stageFrame }
    readonly property PerfFrameWait feedbackFrame: PerfFrameWait { id: feedbackFrame }
    property bool sawDetails: false
    property bool sawDiff: false
    property bool scrollValid: true
    property real scrollStart: 0

    readonly property bool graphVisible: graphPane.visible && !page.diffShown
                                        && graphPane.width > 0 && graphPane.height > 0
    readonly property bool ready: AppBackend.identityState === "ready" && repoTab.state === "open" && !graphModel.loading
                                  && graphModel.finishCount > 0 && branchesModel.refsLoaded
                                  && worktree.loaded

    // Every stage change goes through here, and says itself (`perf_stage`).
    // Stage names are hyphenated (`details-frame`) so a `perf_stage` line never contains the `perf_*_frame` events
    // `perf::interactions` finds by substring.
    function enter(next) {
        driver.stage = next
        Harness.report("perf_stage " + next + " operation=" + driver.operation)
    }

    function fail(reason) {
        driver.enter("failed")
        Harness.report("perf_failed reason=" + reason)
        if (PerfProbe.verifying)
            Qt.quit()
    }

    function waitFrame(next) {
        graphPane.view.forceLayout()
        if (page.diffShown)
            diffPane.view.forceLayout()
        driver.stageFrame.ask()
        driver.enter(next)
        // `update()`, not `requestUpdate()`: a bare request over a still offscreen scene swaps no frame
        // (rules-refs/app-ui.md「`frameSwapped` を待つなら頼むのは `window.update()`」).
        page.Window.window.update()
    }

    /// What the next operation still waits on, by name: every read the page owes (`PageSettled.missing`), and any
    /// git of the application's still running beside them (`Harness.gitInFlight`). Both answer for work already
    /// asked for — work a timer or a debounce has yet to ask for is not in either. A back-to-back run waits for the
    /// page to be ready before its first operation, and on nothing after.
    function owed() {
        if (PerfProbe.backToBack)
            return driver.begun || driver.ready ? [] : ["ready"]
        const terms = PageSettled.missing(page)
        if (!driver.ready)
            terms.push("ready")
        if (Harness.gitInFlight() > 0)
            terms.push("git")
        return terms
    }
    function idleThen(what) {
        driver.resumeWith = what
        driver.enter("idle")
        driver.tick(false)
    }
    function resume() {
        const what = driver.resumeWith
        driver.resumeWith = ""
        if (what === "choose") {
            // The first is the page's own: everything it opened with is in.
            if (!driver.begun && !PerfProbe.backToBack)
                Harness.report("perf_first_idle clock_ms=" + PerfProbe.clockMs())
            driver.begun = true
            driver.choose()
        } else if (what === "open") {
            driver.openFile()
        } else if (what === "scroll") {
            driver.beginScroll()
        } else if (what === "diff-scroll") {
            driver.beginDiffScroll()
        } else {
            driver.fail("resume-unknown:" + what)
        }
    }

    /// Asks for the frame that shows `shown` — the state the input just made, which has to stand at once: the
    /// operation's first answer is immediate by design (実装計画 §9), and a run in which it is not has no feedback
    /// frame to time.
    function askFeedback(kind, shown) {
        if (!shown()) {
            driver.fail("input-not-shown-at-once")
            return false
        }
        driver.feedbackKind = kind
        driver.feedbackShown = shown
        driver.feedbackFrame.ask()
        page.Window.window.update()
        return true
    }
    /// Ends the run where a completion's frame arrived with the input's feedback frame still untimed, and answers
    /// whether the run goes on. Both waits are asked of the same passes, feedback first, so this holds an invariant.
    function requireFeedback() {
        if (driver.feedbackKind === "")
            return true
        driver.fail("completion-frame-before-feedback")
        return false
    }

    /// Moves the stage on where its condition holds. `onBeat`: called from the sampler's beat, a turn of its own —
    /// an idle run puts its next input in only there. From a model's signal the page is still inside the drain that
    /// settled it, and an input put in then is timed with the rest of that drain. A back-to-back run asks for no
    /// such stillness, and goes on where the condition is first seen.
    function tick(onBeat) {
        const still = onBeat === true || PerfProbe.backToBack
        if (driver.stage === "opening" && (graphModel.rowTotal > 0 || driver.ready) && driver.graphVisible)
            driver.waitFrame("graph-frame")
        else if (driver.stage === "idle") {
            // Only on the turn an input could go in: `owed` borrows the hub for `gitInFlight`, and a model's signal
            // is a drain of its own that it would be read inside.
            if (still) {
                const owed = driver.owed()
                Awaited.note("perf_idle waiting=" + (owed.length === 0 ? "nothing" : owed.join(",")))
                if (owed.length === 0)
                    driver.resume()
            }
        } else if (driver.stage === "click-guard") {
            if (still)
                driver.choose()
        } else if (driver.stage === "details" && !detailsModel.loading && detailsModel.shaHex === driver.wantedOid)
            driver.waitFrame("details-frame")
        else if (driver.stage === "diff" && !diffModel.loading && diffModel.title === driver.wantedPath
                 && diffModel.fingerprint !== "" && page.diffShown)
            driver.waitFrame("diff-frame")
        else if (driver.stage === "colour" && driver.sameDiff() && diffModel.coloured) {
            driver.wantedGeneration = diffModel.rowsGen
            driver.waitFrame("colour-frame")
        }
    }

    function sameDiff() {
        return page.diffShown && page.diffPath === driver.wantedPath && detailsModel.shaHex === driver.wantedOid
                && !diffModel.loading && diffModel.fingerprint === driver.wantedFingerprint
    }

    /// One timed frame, from the input (`actionStart`) to now. `asked_ms` is where in that stretch the frame was
    /// asked for — the state it shows was there — and `passed_over` how many frames went by without it.
    function note(point, wait) {
        driver.measurement(point + " elapsed_ms=" + (PerfProbe.clockMs() - driver.actionStart)
                       + " asked_ms=" + (wait.askedAt - driver.actionStart) + " passed_over=" + wait.passedOver
                       + " case=" + driver.caseName + " operation=" + driver.operation
                       + " oid=" + driver.wantedOid + " rows_gen=" + diffModel.rowsGen
                       + " fingerprint=" + diffModel.fingerprint)
    }

    function choose() {
        if (PerfProbe.selection === "none") {
            if (page.selectedOid !== "" || detailsModel.shaHex !== "" || detailsModel.loading || page.diffShown) {
                driver.fail("none-selected-a-commit")
                return
            }
            // ListView gives its first row a current index though the page never activated a commit; clear it too.
            const hadCurrent = graphPane.view.currentIndex >= 0
            graphPane.setCurrentRow(-1)
            Harness.report("perf_selection mode=none oid=none")
            if (hadCurrent)
                driver.waitFrame("none-frame")
            else
                driver.afterInteraction()
            return
        }
        let row = -1
        const caseOid = PerfProbe.caseField(driver.operation, 1)
        const oid = caseOid !== "" ? caseOid
                    : PerfProbe.selection === "head" ? worktree.headOid : ""
        if (oid !== "")
            row = graphModel.rowOf(oid)
        else if (PerfProbe.selection === "first") {
            // Past the WIP row and any stash over the tip (`GraphModel.newestCommitRow`).
            row = graphModel.newestCommitRow()
        }
        if (row < 0) {
            driver.fail("selection-not-in-loaded-graph")
            return
        }
        driver.wantedOid = graphModel.oidAt(row)
        graphPane.view.positionViewAtIndex(row, ListView.Contain)
        graphPane.view.forceLayout()
        const item = graphPane.view.itemAtIndex(row)
        if (!item) {
            driver.fail("selection-delegate-missing")
            return
        }
        // A click the product would take for the second half of a double-click selects nothing: it is put in once
        // that window has closed (`tick`).
        if (driver.secondHalf(item)) {
            if (driver.stage !== "click-guard")
                driver.enter("click-guard")
            return
        }
        driver.enter("details")
        Harness.report("perf_selection mode=" + PerfProbe.selection + " oid=" + driver.wantedOid
                       + " case=" + driver.caseName + " operation=" + driver.operation)
        driver.actionStart = PerfProbe.clockMs()
        page.releasePressedAway(null)
        item.leftClick(Qt.NoModifier)
        // The row lit (`GraphRowDelegate.selected` follows the view's current row) and the page holding the commit:
        // the second is what the click made, since the first row can be the view's current one before any click.
        if (driver.askFeedback("perf_details_feedback", () => graphPane.view.currentIndex === row
                                                              && page.selectedOid === driver.wantedOid))
            driver.tick(false)
    }

    /// Whether a click on `item` now is one the product drops as the second half of a double-click: inside the
    /// window the last click opened, on the same target (`ReclickGesture.doubleGuard`). Rows with no name all
    /// answer to one key (`GraphList.noteClick`), so two of them clicked in a row are that target twice. The
    /// gesture's other reason to drop a click, a hush, is not waited out — it ends the run as feedback never shown.
    function secondHalf(item) {
        const view = graphPane.view
        return view.clickGuarded && (item.nameChip === null ? "" : item.nameChip.key) === view.clickedKey
    }

    function afterDetails() {
        if (!driver.requireFeedback())
            return
        driver.sawDetails = true
        driver.note("perf_details_frame", driver.stageFrame)
        if (PerfProbe.withDiff)
            driver.idleThen("open")
        else
            driver.afterInteraction()
    }

    function openFile() {
        let file = 0
        const caseFile = PerfProbe.caseField(driver.operation, 2)
        if (caseFile !== "") {
            file = -1
            for (let i = 0; i < detailsModel.fileTotal; i++) {
                if (detailsModel.filePathAt(i) === caseFile) {
                    file = i
                    break
                }
            }
        }
        if (file < 0 || detailsModel.fileTotal === 0) {
            driver.fail("diff-file-missing")
            return
        }
        driver.enter("diff")
        driver.wantedPath = detailsModel.filePathAt(file)
        const origPath = detailsModel.fileOrigPathAt(file)
        Harness.report("perf_file path=" + driver.wantedPath)
        driver.actionStart = PerfProbe.clockMs()
        page.openDiff("commit", driver.wantedPath, origPath)
        if (driver.askFeedback("perf_diff_feedback", () => page.diffShown && page.diffPath === driver.wantedPath))
            driver.tick(false)
    }

    function afterInteraction() {
        if (driver.operation + 1 < PerfProbe.operationCount()) {
            if (page.diffShown)
                page.closeDiff()
            driver.operation++
            driver.waitFrame("next-frame")
            return
        }
        if (Harness.autoScroll) {
            const returning = page.diffShown
            if (returning)
                page.closeDiff()
            graphPane.view.positionViewAtBeginning()
            if (returning)
                driver.waitFrame("scroll-frame")
            else
                driver.idleThen("scroll")
        } else {
            driver.finish()
        }
    }

    function afterDiff() {
        if (PerfProbe.diffScroll) {
            diffPane.view.forceLayout()
            if (!diffPane.visible || diffBench.to <= diffBench.from) {
                driver.fail("diff-scroll-hidden-or-no-overflow")
                return
            }
            const before = diffPane.view.contentY
            diffPane.view.positionViewAtBeginning()
            // A still offscreen scene swaps no further frame, and this diff's frame was already seen: wait for one
            // only if the viewport moved.
            if (diffPane.view.contentY === before)
                driver.idleThen("diff-scroll")
            else
                driver.waitFrame("diff-scroll-frame")
        } else {
            driver.afterInteraction()
        }
    }

    function beginDiffScroll() {
        driver.scrollStart = diffPane.view.contentY
        driver.scrollValid = true
        driver.enter("diff-scrolling")
        PerfProbe.scrollSurface("diff")
        PerfProbe.scrollContext(driver.caseName, driver.operation)
        PerfProbe.beginScroll()
        diffBench.start()
    }

    function finish() {
        driver.enter("finished")
        driver.measurement("perf_complete selection=" + PerfProbe.selection + " details=" + driver.sawDetails
                          + " diff=" + driver.sawDiff + " graph=" + driver.graphVisible
                          + " scrolled=" + Harness.autoScroll + " rows=" + graphModel.rowTotal)
        driver.measurement("perf_operations count=" + (driver.operation + 1))
        page.perfFinished()
    }

    /// Measurements retain their observation point; verification attributes them to this run without rereading them.
    function measurement(line) {
        Harness.report(PerfProbe.verifying ? "measurement run=" + Awaited.runId + " text=" + line : line)
    }

    function beginScroll() {
        graphPane.view.forceLayout()
        driver.reportViewport("start")
        if (!driver.graphVisible || scrollBench.to <= scrollBench.from) {
            driver.fail("scroll-hidden-or-no-overflow")
            return
        }
        driver.enter("scrolling")
        driver.scrollStart = graphPane.view.contentY
        PerfProbe.scrollSurface("graph")
        PerfProbe.beginScroll()
        scrollBench.start()
    }

    function reportViewport(at) {
        const view = graphPane.view
        Harness.report("perf_viewport clock_ms=" + PerfProbe.clockMs() + " at=" + at
                          + " y=" + view.contentY + " origin=" + view.originY
                          + " height=" + view.contentHeight + " viewport=" + view.height
                          + " row=" + view.indexAt(view.width / 2, view.contentY + view.height / 2))
    }

    function frame() {
        if (driver.stage === "diff-scrolling") {
            driver.scrollValid = driver.scrollValid && driver.sameDiff() && diffPane.visible
            PerfProbe.frame()
            return
        }
        if (driver.stage === "scrolling") {
            driver.scrollValid = driver.scrollValid && driver.graphVisible
            PerfProbe.frame()
            return
        }
        if (driver.feedbackFrame.swapped()) {
            if (!driver.feedbackShown()) {
                driver.fail("feedback-gone-before-its-frame")
                return
            }
            driver.note(driver.feedbackKind, driver.feedbackFrame)
            driver.feedbackKind = ""
        }
        if (!driver.stage.endsWith("-frame") || !driver.stageFrame.swapped())
            return
        if (driver.stage === "graph-frame") {
            if (!driver.graphVisible) {
                driver.fail("first-graph-hidden")
                return
            }
            Harness.report("perf_graph_frame clock_ms=" + PerfProbe.clockMs() + " visible=true passed_over="
                           + driver.stageFrame.passedOver)
            driver.idleThen("choose")
        } else if (driver.stage === "none-frame") {
            if (graphPane.view.currentIndex >= 0)
                driver.fail("none-retained-current-row")
            else
                driver.afterInteraction()
        } else if (driver.stage === "details-frame") {
            driver.afterDetails()
        } else if (driver.stage === "diff-frame") {
            if (!driver.requireFeedback())
                return
            driver.sawDiff = true
            driver.wantedFingerprint = diffModel.fingerprint
            driver.note("perf_diff_frame", driver.stageFrame)
            if (driver.wantsColour) {
                driver.enter("colour")
                driver.tick(false)
            } else {
                driver.afterDiff()
            }
        } else if (driver.stage === "colour-frame") {
            if (!driver.sameDiff() || !diffModel.coloured || diffModel.rowsGen !== driver.wantedGeneration) {
                driver.fail("colour-frame-changed-target")
                return
            }
            driver.note("perf_colour_frame", driver.stageFrame)
            driver.afterDiff()
        } else if (driver.stage === "next-frame") {
            driver.idleThen("choose")
        } else if (driver.stage === "diff-scroll-frame") {
            driver.idleThen("diff-scroll")
        } else if (driver.stage === "diff-end-frame") {
            const view = diffPane.view
            const row = view.indexAt(view.width / 2, view.contentY + view.height / 2)
            if (!driver.sameDiff() || row < 0 || !view.itemAtIndex(row)) {
                driver.fail("diff-scroll-ended-without-visible-row")
                return
            }
            driver.measurement("perf_diff_scroll_frame visible=true case=" + driver.caseName
                           + " operation=" + driver.operation + " row=" + row)
            driver.afterInteraction()
        } else if (driver.stage === "scroll-frame") {
            driver.idleThen("scroll")
        } else if (driver.stage === "scroll-end-frame") {
            const view = graphPane.view
            const row = view.indexAt(view.width / 2, view.contentY + view.height / 2)
            const item = row >= 0 ? view.itemAtIndex(row) : null
            if (!driver.graphVisible || !item || !item.visible) {
                driver.fail("scroll-ended-without-visible-row")
                return
            }
            Harness.report("perf_scroll_frame visible=true row=" + row)
            driver.finish()
        }
    }

    Connections {
        target: driver.graphModel
        function onStatsChanged() { driver.tick(false) }
    }
    Connections {
        target: driver.branchesModel
        function onChanged() { driver.tick(false) }
    }
    Connections {
        target: driver.worktree
        function onChanged() { driver.tick(false) }
    }
    Connections {
        target: driver.repoTab
        function onChanged() { driver.tick(false) }
    }
    Connections {
        target: driver.detailsModel
        function onChanged() { driver.tick(false) }
    }
    Connections {
        target: driver.diffModel
        function onChanged() { driver.tick(false) }
    }
    Connections {
        target: driver.page.Window.window
        function onAfterAnimating() {
            driver.stageFrame.passed()
            driver.feedbackFrame.passed()
        }
        function onAfterSynchronizing() {
            driver.stageFrame.synced()
            driver.feedbackFrame.synced()
        }
        function onFrameSwapped() { driver.frame() }
    }

    NumberAnimation {
        id: diffBench
        target: driver.diffPane.view
        property: "contentY"
        from: driver.diffPane.view.originY
        to: from + Math.max(0, driver.diffPane.view.contentHeight - driver.diffPane.view.height)
        // waits(measured): sample visible diff frames over this animation; movement and a final row are required
        duration: 2000
        onFinished: {
            const moved = Math.abs(driver.diffPane.view.contentY - driver.scrollStart)
            PerfProbe.endScroll(driver.diffPane.view.count, moved, driver.scrollValid)
            if (!driver.scrollValid || moved <= 0)
                driver.fail("diff-scroll-not-observed")
            else
                driver.waitFrame("diff-end-frame")
        }
    }

    NumberAnimation {
        id: scrollBench
        target: driver.graphPane.view
        property: "contentY"
        from: driver.graphPane.view.originY
        to: from + Math.max(0, Math.min(3000 * Theme.graphRowHeight,
                                      driver.graphPane.view.contentHeight - driver.graphPane.view.height))
        // The window the frame rate is read over; `onFinished` fails the run unless the view moved while on screen.
        // waits(measured): the length of the sample, judged by what the scroll did
        duration: 12000
        onFinished: {
            driver.reportViewport("end")
            const moved = Math.abs(driver.graphPane.view.contentY - driver.scrollStart)
            const visible = driver.scrollValid && driver.graphVisible
            PerfProbe.endScroll(driver.graphModel.rowTotal, moved, visible)
            if (!visible || moved <= 0)
                driver.fail("scroll-not-observed")
            else
                driver.waitFrame("scroll-end-frame")
        }
    }

    // Re-reads the predicates on the beat — `tick()` reads inputs no signal here is wired to (the pane's size and
    // visibility, the page's diff, what the page and its git still owe) — and re-asks for a frame each beat, since a
    // request taken while the window was not visible swaps nothing. Off over a scroll: the animation's end moves
    // that stage on.
    SampleTimer {
        running: driver.stage !== "finished" && driver.stage !== "failed" && !driver.stage.endsWith("scrolling")
        onTriggered: {
            driver.tick(true)
            if (driver.stageFrame.step !== "" || driver.feedbackFrame.step !== "")
                driver.page.Window.window.update()
        }
    }

    Component.onCompleted: {
        driver.enter(driver.stage)
        driver.tick(false)
    }
    onReadyChanged: driver.tick(false)
}

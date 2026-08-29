pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The runs the diff pane's dispatcher hands off to: the ones that read a place, write and read it again, or
/// send the code sideways. Each is a chain of its own, which is why none of them is a branch.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are read back once below, so the code under them reads as it
/// did when it was all one file.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing
// and is never given a size.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so what is written under them reads as it did.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktreeModel: driver.worktreeModel
    readonly property var diffPane: driver.diffPane
    readonly property var renderedBarrier: driver.barrierRendered

    // What the dispatcher starts, under names of its own: an alias cannot carry an id's own name.
    readonly property alias stepCodeSend: codeSendTimer
    readonly property alias stepDiffSweep: diffSweepTimer
    readonly property alias stepFollow: followTimer
    readonly property alias stepKeepPlace: keepPlaceTimer
    readonly property alias stepLineBack: lineBackTimer
    readonly property alias stepLineRun: lineRunTimer

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "colour-place") {
            page.showWip()
            page.toggleDiff("unstaged", arg, "")
            colourPlaceTimer.begin()
        } else {
            return false
        }
        return true
    }

    /// How far down a diff the reader is taken before the thing that could cost them their place happens — the rebuild
    /// a partial write asks for (`keep-place`), and the swap the colours arrive in (`colour-place`). One number for
    /// both, because both are judged on getting exactly it back, and a place nobody can name is not one either of them
    /// can be caught losing.
    readonly property real readY: 400
    function reportPlace(at, room) {
        AppBackend.report("diff_place at=" + Math.round(at)
                          + " want=" + Math.round(acts.readY)
                          + " room=" + Math.round(room))
    }
    // One line staged from the diff, then the same file moved from the file list — the diff has to follow both, and
    // following only the first is the observed failure this verb pins (2026-08-17 ユーザー報告: the line was gone from
    // the unstaged side and never came back when the file was unstaged).
    //
    // Three answers in one run, because they are one story: the line goes (the rows shrink), the line comes back (the
    // rows are as they were), and staging the rest empties the side being read — where the pane follows the file to the
    // side it went to rather than closing on the reader (`RepoPage.followEmptySide`). Each step waits for its own write
    // to land *and* for the pane to say so — the rows and the key are the output, the write is only the cause.
    SampleTimer {
        id: lineBackTimer
        property int step: 0
        property int rows0: 0
        property int rows1: 0
        property bool shrank: false
        property bool back: false
        function begin() {
            lineBackTimer.step = 0
            lineBackTimer.shrank = false
            lineBackTimer.back = false
            lineBackTimer.start()
        }
        /// Whether the write this step asked for has landed. The sequence is read immediately before asking, so its
        /// moving is the whole of the evidence — waiting to *see* `busyCount` rise as well wedges on a write that
        /// begins and ends inside one tick, which is what the container did while the host did not (2026-08-17 実測:
        /// `line-back` PASS on Windows, watchdog on Linux).
        function wroteAndSettled() {
            return repoTab.busyCount === 0
                    && repoTab.writeSeq > driver.writeSeqBefore
        }
        function expect() {
            driver.writeSeqBefore = repoTab.writeSeq
        }
        onTriggered: {
            const rows = diffPane.view.count
            if (lineBackTimer.step === 0) {
                const line = diffPane.firstChangedLine(0)
                if (line < 0)
                    return
                lineBackTimer.rows0 = rows
                lineBackTimer.expect()
                diffPane.stageLine(0, line)
                lineBackTimer.step = 1
                return
            }
            if (lineBackTimer.step === 1) {
                if (!lineBackTimer.wroteAndSettled() || rows >= lineBackTimer.rows0)
                    return
                lineBackTimer.rows1 = rows
                lineBackTimer.shrank = true
                lineBackTimer.expect()
                // The file list's own `−`, which is the half that was never reaching the pane.
                repoTab.unstagePath(page.diffPath)
                lineBackTimer.step = 2
                return
            }
            if (lineBackTimer.step === 2) {
                if (!lineBackTimer.wroteAndSettled() || rows !== lineBackTimer.rows0)
                    return
                lineBackTimer.back = true
                lineBackTimer.expect()
                repoTab.stagePath(page.diffPath)
                lineBackTimer.step = 3
                return
            }
            if (!lineBackTimer.wroteAndSettled() || page.diffKind !== "staged" || rows === 0)
                return
            lineBackTimer.stop()
            AppBackend.report("line_back back=" + lineBackTimer.back
                              + " shrank=" + lineBackTimer.shrank
                              + " followed=" + (page.diffKind + ":" + page.diffPath)
                              + " rows0=" + lineBackTimer.rows0
                              + " rows1=" + lineBackTimer.rows1)
            renderedBarrier.begin()
        }
    }
    // Line after line, the way a hand does it. The pane refuses a press while the rows it would be written against are
    // still coming (`RepoPage.diffSettling`), so this waits for exactly that and no clock — which is also the thing
    // that broke: held on a signal the file list only sends when its rows differ, the pane went quiet for good at the
    // second line of a file already on both sides, and no `+` anywhere would go in again (2026-08-17 ユーザー報告).
    SampleTimer {
        id: lineRunTimer
        readonly property int want: 3
        property int done: 0
        /// How long the list has gone without a row to name, which is not the same as having none (see below).
        property int waited: 0
        function begin() {
            lineRunTimer.done = 0
            lineRunTimer.waited = 0
            lineRunTimer.start()
        }
        function report() {
            AppBackend.report("line_run staged=" + lineRunTimer.done
                              + " want=" + lineRunTimer.want
                              + " rows=" + diffPane.view.count
                              + " waited=" + lineRunTimer.waited)
        }
        onTriggered: {
            // The pane is still catching up with the last press.
            if (page.diffSettling)
                return
            if (lineRunTimer.done === lineRunTimer.want) {
                lineRunTimer.stop()
                lineRunTimer.report()
                renderedBarrier.begin()
                return
            }
            // A row is named by walking the list's own items, and the list builds them a frame after the model hands
            // the rows over: read too early it names nothing, which is not the same as there being nothing
            // (`keep-place` learned it too). So an empty answer is waited on — but not for ever. **The 5s cut-off is
            // a deliberate exception to the no-fixed-time rule** (app-ui.md §UI 自動化の因果性): it never passes a run
            // as green — the report says `staged=n want=m` and the judge fails the shortfall — it only converts "the
            // fixture has fewer changed lines than the run asks for" from a silent watchdog into a diagnosable line.
            const line = diffPane.firstChangedLine(0)
            if (line < 0) {
                lineRunTimer.waited += lineRunTimer.interval
                if (lineRunTimer.waited < 5000)
                    return
                lineRunTimer.stop()
                lineRunTimer.report()
                renderedBarrier.begin()
                return
            }
            lineRunTimer.waited = 0
            lineRunTimer.done++
            diffPane.stageLine(0, line)
        }
    }
    // Where the reader lands when the file under the open diff is moved whole from the file list. Two answers, and
    // which one is right depends on what is left behind (デザイン規約 §diff の中のステージ): with other files still on that side the
    // pane takes the next of them, and with none left it stays on the same file and reads it from the side it went to.
    //
    // The argument names the file to open; the verb decides its own second step from what the tree holds afterwards,
    // and reports both.
    SampleTimer {
        id: followTimer
        property int step: 0
        property string was: ""
        property string landed: ""
        property bool alone: false
        function begin() {
            followTimer.step = 0
            followTimer.landed = ""
            followTimer.start()
        }
        onTriggered: {
            if (followTimer.step === 0) {
                if (diffPane.view.count === 0)
                    return
                followTimer.was = page.diffKind + ":" + page.diffPath
                // Whether this side has anything else on it, read before the write takes the file off it.
                followTimer.alone =
                    worktreeModel.besidePath(page.diffKind, page.diffPath) === ""
                // The sequence read here is the whole test below: seeing `busyCount` rise as well wedges on a write
                // that begins and ends inside one tick (see `lineBackTimer`).
                driver.writeSeqBefore = repoTab.writeSeq
                // The file list's own `+` / `−`, whole file at a time.
                if (page.diffKind === "staged")
                    repoTab.unstagePath(page.diffPath)
                else
                    repoTab.stagePath(page.diffPath)
                followTimer.step = 1
                return
            }
            // The landing is the output: the pane has to have moved off the key it was on and settled somewhere with
            // rows.
            const now = page.diffShown ? page.diffKind + ":" + page.diffPath : ""
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore
                    || now === followTimer.was || (page.diffShown && diffPane.view.count === 0))
                return
            followTimer.stop()
            followTimer.landed = now
            AppBackend.report("diff_follow shown=" + page.diffShown
                              + " alone=" + followTimer.alone
                              + " was=" + followTimer.was
                              + " landed=" + followTimer.landed)
            renderedBarrier.begin()
        }
    }
    // Sending the diff's code sideways, by the bar's own path and then by the hand that carries the rows with it. What
    // is read back is where the code and the rows ended up, never what was asked for: a bar bound to nothing still
    // takes a press, and a hand wired to nothing still starts.
    //
    // The wait is for the view (`keep-place` learned the same lesson): rows that have arrived are not rows the list has
    // laid out, and until it has, `codeMax` is measured against a width of nothing. A diff with nowhere sideways to go
    // says so and stops there rather than at the watchdog — a run over one photographs a pane that proves nothing
    // (app-ui.md §UI 自動化の因果性).
    SampleTimer {
        id: codeSendTimer
        property bool sent: false
        function begin() {
            codeSendTimer.sent = false
            codeSendTimer.start()
        }
        function laidOut() {
            return diffPane.view.count > 0 && diffPane.view.width > 0
                    && diffPane.view.contentHeight > 0
        }
        function report() {
            AppBackend.report("code_send bar=" + diffPane.codeBarShown
                              + " hand=" + diffPane.codeHandOn
                              + " at=" + Math.round(diffPane.codeAt)
                              + " max=" + Math.round(diffPane.codeMax)
                              + " down=" + Math.round(diffPane.view.contentY))
        }
        onTriggered: {
            if (!codeSendTimer.sent) {
                if (!codeSendTimer.laidOut())
                    return
                if (diffPane.codeMax <= 0) {
                    codeSendTimer.stop()
                    codeSendTimer.report()
                    renderedBarrier.begin()
                    return
                }
                // Half the way by the bar's own path, and the rest — with the rows — by the hand, started from the
                // middle of the view and drifted down and to the right. The anchor's ring is part of the picture, so
                // the hand is left running for the shot.
                diffPane.sendCode(diffPane.codeMax / 2)
                diffPane.startCodeHand(diffPane.view.width / 2,
                                       diffPane.view.height / 2)
                diffPane.driftCodeHand(diffPane.view.width / 2 + 120,
                                       diffPane.view.height / 2 + 120)
                codeSendTimer.sent = true
                return
            }
            // The hand ticks on its own clock, and both of its axes have to be seen moving: the rows have come down,
            // and the code has gone further than the bar's half left it.
            if (diffPane.view.contentY <= 0 || diffPane.codeAt <= diffPane.codeMax / 2)
                return
            codeSendTimer.stop()
            codeSendTimer.report()
            renderedBarrier.begin()
        }
    }
    // PG_AUTO_ACT=diff-sweep: the diff's text taken from the ground under the last row of a short file instead of from
    // the rows themselves — the one place inside the code column where a press used to reach nothing
    // (規約 §diff の中身をコピーする). The shape is `details-sweep`'s, and so are its two claims:
    //
    // **Nine starts, not one** (`reach=`). A reach that worked from a single place in the ground is exactly the fault
    // the right pane's values shipped with, and the middle is the one place that hides it (2026-08-28 ユーザー報告).
    // Each start is judged on its own, over a board cleared first — a run that read the selection once at the end
    // would report the last try and call the other eight green.
    //
    // **And a press on a hunk's heading is still the hunk's** (`ours=`). The two words there act on the hunk
    // (規約 §diff の中のステージ), and a widening that took their face is the one way this could cost anybody anything.
    SampleTimer {
        id: diffSweepTimer
        /// Where the ground began at the previous sample, for the settle below.
        property string lastGeom: ""
        onTriggered: {
            // The rows arrive a frame ahead of the view that lays them out, and a sweep aimed at ground that is about
            // to be a row lands on neither (`details-sweep`, the same wait).
            const hand = diffPane.textHand
            if (diffPane.view.count <= 0)
                return
            const geom = Math.round(hand.groundTop) + "," + diffPane.view.count
            if (geom !== diffSweepTimer.lastGeom) {
                diffSweepTimer.lastGeom = geom
                return
            }
            diffSweepTimer.stop()
            let reach = 0
            let tries = 0
            const across = [0.05, 0.5, 0.95]
            const down = [0.05, 0.5, 0.95]
            for (let i = 0; i < across.length; i++) {
                for (let j = 0; j < down.length; j++) {
                    tries++
                    diffPane.diffModel.clearSelect()
                    if (hand.sweepFromGround(across[i], down[j]) && diffPane.diffModel.selHasNew)
                        reach++
                }
            }
            // The wash of the last sweep is left standing, so the picture is of a diff a hand picked out. The press
            // below lands on a heading and is refused, which takes nothing down.
            const hunkRow = hand.firstHunkRow()
            const ours = hunkRow < 0 || hand.pressOnHunk(hunkRow)
            // `ground=` is the run's own honesty: a file that fills the frame has nowhere to sweep from, and a
            // `reach=0/9` off one is a fixture that stopped saying anything rather than a hand that stopped working.
            // `ours=` reads the way `details_sweep` reads it — false is the pass, and it says the heading's own two
            // words kept every press across their face.
            AppBackend.report("diff_sweep reach=" + reach + "/" + tries
                              + " ground=" + hand.hasGround
                              + " ours=" + ours
                              + " hunkRow=" + hunkRow
                              + " " + diffPane.pickTally())
            renderedBarrier.begin()
        }
    }
    // Reading part way down a long diff and then writing: the rebuild has to come back to the same place.
    //
    // The wait is for the view, not for the model (`diff-step` learned the same lesson): rows that have arrived are not
    // rows the list has laid out, and until it has there is no place to lose — the scroll goes nowhere and the restore
    // has nothing to undo. So what is waited for is the room the reading consumes, and a diff that is laid out and
    // still too short says so and stops there rather than at the watchdog: nothing that short can hold a place, and a
    // run over it photographs a pane that proves nothing (app-ui.md §UI 自動化の因果性).
    SampleTimer {
        id: keepPlaceTimer
        /// The line of the first hunk that gets staged, which is what rebuilds the diff under the reader.
        property int line: -1
        property bool wrote: false
        function begin(atLine) {
            keepPlaceTimer.line = atLine
            keepPlaceTimer.wrote = false
            keepPlaceTimer.start()
        }
        /// Rows the list has actually put down, as against rows it has been handed: `contentHeight` is still zero for
        /// the first of those and `maxY` cannot be read before it.
        function laidOut() {
            return diffPane.view.count > 0 && diffPane.view.height > 0
                    && diffPane.view.contentHeight > 0
        }
        onTriggered: {
            if (!keepPlaceTimer.wrote) {
                if (!keepPlaceTimer.laidOut())
                    return
                if (diffPane.view.maxY < acts.readY) {
                    keepPlaceTimer.stop()
                    acts.reportPlace(diffPane.view.contentY, diffPane.view.maxY)
                    renderedBarrier.begin()
                    return
                }
                diffPane.scrollTo(acts.readY)
                driver.writeSeqBefore = repoTab.writeSeq
                page.stageSelection(0, keepPlaceTimer.line)
                keepPlaceTimer.wrote = true
                return
            }
            // Both edges of the write, and then the one output the whole verb is about: the rebuilt list put back on
            // the place. A write that emptied this side never gets a row back and so never lands anywhere — which is a
            // fixture with no place in it, and the run waits rather than passing on the silence.
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore
                    || diffPane.placeLandedY < 0)
                return
            keepPlaceTimer.stop()
            acts.reportPlace(diffPane.placeLandedY, diffPane.view.maxY)
            renderedBarrier.begin()
        }
    }
    // A reader who scrolled before the colours landed: the whole list is swapped again when the colours turn up
    // (`DiffModel::lay_out_rows`), and that swap must not cost the place being read.
    Timer {
        id: colourPlaceTimer
        interval: 50
        repeat: true
        property int waited: 0
        property bool scrolled: false
        function begin() {
            colourPlaceTimer.waited = 0
            colourPlaceTimer.scrolled = false
            colourPlaceTimer.start()
        }
        onTriggered: {
            colourPlaceTimer.waited += colourPlaceTimer.interval
            if (!colourPlaceTimer.scrolled) {
                // Read down the file the moment the rows are there, which is well before the colours are.
                if (diffPane.firstChangedLine(0) < 0)
                    return
                diffPane.scrollTo(acts.readY)
                colourPlaceTimer.scrolled = true
                return
            }
            if (!diffPane.diffModel.coloured)
                return
            colourPlaceTimer.stop()
            AppBackend.report("colour_place coloured="
                              + diffPane.diffModel.coloured
                              + " at=" + Math.round(diffPane.view.contentY)
                              + " rows=" + diffPane.view.count
                              + " waited=" + colourPlaceTimer.waited)
            driver.complete()
        }
    }
}

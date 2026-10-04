pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The diff pane's verbs. Most wait first for a changed row of the first hunk (`stageRowTimer`) and branch from there;
/// the rest are chains of their own that the dispatch starts by name.
///
/// **One file on purpose**: every sampler but `colourPlaceTimer` is started by id from `stageRowTimer`'s tail
/// (rules-refs/structure.md「dispatcher が id で start する切片は独立ファイルにできない」), and `colour-place` shares
/// `readY` with `keep-place` and `reachNow` with `code-grow` / `-shrink` / `-swap`. A split needs each family to own
/// its entry first (the open and the first-hunk wait), as `AutoActSplitVerbs` does.
// An `Item` only because `QtObject` has no default property to hold the timers.
Item {
    id: acts

    /// `var`: typing it would be circular — `AutoActDriver` is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktreeModel: driver.worktreeModel
    readonly property var diffPane: driver.diffPane
    readonly property var graphPane: driver.graphPane
    readonly property var diffRowMenu: driver.diffRowMenu
    readonly property var renderedBarrier: driver.barrierRendered
    readonly property var writeBarrier: driver.barrierWrite

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (act === "stage-hunk" || act === "stage-line"
            || act === "discard-hunk" || act === "discard-hunk-go"
            || act === "diff-file" || act === "conflict-sides" || act === "line-tools"
            || act === "hunk-tools" || act === "keep-place" || act === "diff-tick"
            || act === "code-send" || act === "line-back"
            || act === "code-grow" || act === "code-shrink" || act === "code-swap"
            || act === "diff-select" || act === "diff-copy"
            || act === "diff-menu" || act === "diff-copy-removed" || act === "diff-sweep"
            || act === "diff-band-sweep" || act === "diff-bar" || act === "diff-blank"
            || act === "diff-follow" || act === "line-run" || act === "diff-escape") {
            // `<path>` or `<bucket>:<path>` (an untracked file has no unstaged diff; a conflict is read from
            // `conflicts`). Only the bucket names count as a prefix, so a path with a colon still opens.
            const cut = arg.indexOf(":")
            const head = cut > 0 ? arg.substring(0, cut) : ""
            const named = head === "staged" || head === "unstaged"
                          || head === "untracked" || head === "conflicts"
            page.showWip()
            // A rename's source comes off the model, as a row click hands it over; without it the diff reads as a new
            // file.
            const wtPath = named ? arg.substring(cut + 1) : arg
            // A missing path is the run's own mistake and must read as one: git takes an empty pathspec as the whole
            // tree, so the pane would fill with a plausible diff and the verb would report rows of no file.
            if (wtPath === "") {
                Harness.report("diff_arg act=" + act + " named=false")
                renderedBarrier.begin()
                // True even here — falsy reads as "not mine" to the dispatch chain.
                return true
            }
            page.toggleDiff(named ? head : "unstaged", wtPath,
                            worktreeModel.origOf(wtPath))
            stageRowTimer.begin()
        } else if (act === "preview" || act === "preview-unstaged" || act === "preview-staged"
                   || act === "preview-close") {
            // Completion is the pane settling (`stageRowTimer`). No path: said and stopped on the rendered barrier — no
            // read would answer a wait, and the missing must_say line fails the run.
            if (arg === "") {
                Harness.report("diff_arg act=" + act + " named=false")
                renderedBarrier.begin()
            } else {
                page.toggleDiff(act === "preview" ? "untracked"
                                : act === "preview-staged" ? "staged" : "unstaged", arg, "")
                stageRowTimer.begin()
            }
        } else if (act === "colour-place") {
            page.showWip()
            page.toggleDiff("unstaged", arg, "")
            colourPlaceTimer.begin()
        } else {
            return false
        }
        return true
    }

    /// The drag the four text verbs share (デザイン規約 §diff の中身をコピーする), entered through the functions the hand's
    /// own handlers call (`DiffTextSelect`) since hover and a real button cannot be injected. It runs from the head of
    /// the first line past the first removed one so the selection holds both sides — a one-sided drag passes on half
    /// the rule.
    function pickDiffText(act) {
        const removedRow = diffPane.firstRemovedRow()
        const lastRow = Math.min(diffPane.view.count - 1, removedRow + 1)
        diffPane.pickText(0, 1, 0, lastRow, driver.pastLineEnd)
        // `door=`: a headless drag delivers no press, so whether the press sheet that brings the keyboard is on top is
        // asked directly — without the keyboard, `Ctrl+C` cannot take the selection.
        Harness.report("diff_pick door=" + diffPane.doorOnTop() + " " + diffPane.pickTally()
                          + " removedRow=" + removedRow + " to=" + lastRow)
        if (act === "diff-select")
            return
        if (act === "diff-copy") {
            diffPane.copySelection()
            acts.reportCopy("diff_copy")
            return
        }
        // Right-click inside the drag, so the selection stands — the half of the rule these verbs test.
        diffPane.askCodeMenu(0, removedRow, 0)
        if (act === "diff-copy-removed") {
            diffRowMenu.menu.close()
            diffRowMenu.copyRemovedNow()
            acts.reportCopy("diff_copy_removed")
        }
    }
    /// What reached the clipboard, read off `ClipboardHelper.lastCopied` (the clipboard cannot be read back).
    /// `holdsRemoved=` is the claim: false for the plain copy, true for the menu's second row.
    function reportCopy(name) {
        const text = driver.clipboard.lastCopied
        const removed = diffPane.diffModel.removedText()
        Harness.report(name + " holdsRemoved=" + (removed !== "" && text.indexOf(removed) >= 0)
                          + " lines=" + (text === "" ? 0 : text.split("\n").length))
    }
    // Waits for the rows, never a fixed time (rules-refs/app-ui.md「diff の行を触る自動化フックは行の到着で計る」). No
    // ceiling: a row that never lands is ended by the watchdog. A slower beat of its own: each tick walks the rows.
    // waits(paced): the loop ends on the diff having arrived
    Timer {
        id: stageRowTimer
        interval: 50
        repeat: true
        property int waited: 0
        function begin() {
            stageRowTimer.waited = 0
            stageRowTimer.start()
        }
        // Whether what the verb is about to name is on screen — by default a changed line of the first hunk.
        function ready() {
            // These need no changed line, and demanding one would hold them to the watchdog: `diff-file` may open a
            // picture or binary with no rows, a typed-over conflict side is a removal, and the band names its path
            // regardless.
            if (["diff-file", "conflict-sides", "diff-tick", "diff-band-sweep",
                 "diff-escape"].indexOf(Harness.autoAct) >= 0)
                return diffPane.diffSettled()
            // The previews also wait for the asynchronous decode, or the shot is an empty frame under a caption.
            if (["preview", "preview-unstaged", "preview-staged", "preview-close"].indexOf(Harness.autoAct) >= 0)
                return diffPane.diffSettled() && diffPane.picturesSettled
            return diffPane.firstChangedLine(0) >= 0
        }
        onTriggered: {
            // waits(measured): printed in the report below and compared with nothing — the verb ends on `ready()`
            stageRowTimer.waited += stageRowTimer.interval
            const arrived = stageRowTimer.ready()
            if (!arrived)
                return
            stageRowTimer.stop()
            const act = Harness.autoAct
            // The first changed line — a hunk numbers its context lines too (`firstChangedLine`).
            const line = diffPane.firstChangedLine(0)
            // Said before acting, so a verb that then fails its write reports both.
            Harness.report("diff_row act=" + act + " ready=" + arrived
                              + " rows=" + diffPane.view.count
                              + " line=" + line
                              + " waited=" + stageRowTimer.waited)
            // The line endings get a report line: the picture cannot say which kind the pane decided on.
            if (act === "diff-file") {
                const d = diffPane.diffModel
                Harness.report("line_endings kind=" + d.endingKind
                                  + " scope=" + d.endingScope
                                  + " lines=" + d.endingLines
                                  + " text=" + Words.lineEndings(
                                      d.endingKind, d.endingFrom, d.endingTo,
                                  d.endingLines, d.endingScope, d.endingExt))
                driver.complete()
                return
            }
            // Sweeps the band's path (規約 §右のペインの字は掴める). `cut=` is what the picture cannot answer: whether the
            // field holds a value longer than the band.
            if (act === "diff-band-sweep") {
                Harness.report("diff_band_sweep "
                    + diffPane.headerHand.sweepAir(7, "cut=" + diffPane.headerCut))
                driver.complete()
                return
            }
            // Which rows name the two sides: the picture cannot say how many should have, and a resolved conflict is
            // where none did.
            if (act === "conflict-sides") {
                Harness.report("conflict_sides " + diffPane.sideTally())
                driver.complete()
                return
            }
            // The page's tick. The file is untouched, so core answers with nothing (`RepoSession::refresh_diff`): the
            // ask itself is the edge, read in the same beat. `loading=`: a tick must not reload the pane like a click.
            if (act === "diff-tick") {
                const asked = page.pollDiff()
                Harness.report("diff_tick asked=" + asked
                                  + " loading=" + diffPane.diffModel.loading
                                  + " rows=" + diffPane.view.count)
                driver.complete()
                return
            }
            // `kind=` and `pictures=` (sides decoded) are what the picture cannot tell: an unread pane and a `file:`
            // URL naming nothing both photograph as an empty frame.
            if (act === "preview" || act === "preview-unstaged" || act === "preview-staged"
                || act === "preview-close") {
                const was = diffPane.diffModel.previewKind
                Harness.report("preview_pane kind=" + was
                                  + " binary=" + diffPane.diffModel.isBinary
                                  + " pictures=" + diffPane.picturesShown)
                if (act !== "preview-close") {
                    driver.complete()
                    return
                }
                // Closed by the header `✕`'s own path. Whether the decoded pictures left the process is read from
                // outside; QML can only say the pane let go of the URLs (`url=empty`).
                page.closeDiffToGraph()
                Harness.report("preview_close was=" + was
                                  + " kind=" + diffPane.diffModel.previewKind
                                  + " shown=" + page.diffShown
                                  + " pictures=" + diffPane.picturesShown
                                  + " url=" + (diffPane.diffModel.previewNewUrl === ""
                                               && diffPane.diffModel.previewOldUrl === "" ? "empty" : "held"))
                renderedBarrier.begin()
                return
            }
            // Escape enters the page's own handler, the one line `Keys.onEscapePressed` calls
            // (`RepoPage.escapePressed`; verify-ui「注入はハンドラ本体そのものへ入れる」). `took=` and `shown=` are
            // separate failures the picture cannot tell apart.
            if (act === "diff-escape") {
                // The hand enters the pane first (`DiffPane.handArrived`, the wheel's path): closing the pane that
                // holds the keyboard is the case that breaks — where the keyboard lands decides whether a second
                // Escape reaches anything.
                diffPane.handArrived()
                escapeTimer.hand = diffPane.view.activeFocus
                escapeTimer.took = page.escapePressed()
                escapeTimer.start()
                return
            }
            // Hover cannot be injected, so the row the pointer would be on is named.
            if (act === "line-tools") {
                diffPane.showLineTools(0, line)
                renderedBarrier.begin()
                return
            }
            // A heading's own row is line -1 (`flatten_patches`). Its words are judged once the row is built
            // (`AutoActDriver.headingBarrier`).
            if (act === "hunk-tools") {
                diffPane.showLineTools(0, -1)
                driver.barrierHeading.start()
                return
            }
            if (act === "diff-select" || act === "diff-copy"
                || act === "diff-menu" || act === "diff-copy-removed") {
                acts.pickDiffText(act)
                renderedBarrier.begin()
                return
            }
            // These three read the laid-out view, a frame behind the rows — their timers wait for it.
            if (act === "diff-sweep") {
                diffSweepTimer.start()
                return
            }
            if (act === "diff-bar") {
                diffBarTimer.start()
                return
            }
            if (act === "diff-blank") {
                diffBlankTimer.start()
                return
            }
            if (act === "keep-place") {
                keepPlaceTimer.begin(line)
                return
            }
            if (act === "stage-hunk" || act === "stage-line") {
                // A line through its own `+` mark, a hunk through its heading's word.
                driver.pressWrite(act, () => {
                    if (act === "stage-line")
                        diffPane.stageLine(0, line)
                    else
                        page.stageSelection(0, -1)
                    return true
                })
                stagedTimer.start()
                return
            }
            if (act === "code-send") {
                codeSendTimer.begin()
                return
            }
            if (act === "code-grow") {
                codeGrowTimer.begin()
                return
            }
            if (act === "code-shrink") {
                codeShrinkTimer.begin(line)
                return
            }
            if (act === "code-swap") {
                codeSwapTimer.begin()
                return
            }
            if (act === "line-run") {
                lineRunTimer.begin()
                return
            }
            if (act === "diff-follow") {
                followTimer.begin()
                return
            }
            if (act === "line-back") {
                lineBackTimer.begin()
                return
            }
            // A hunk is the smallest discard. The write is armed on the hold's end (`holdToEnd`), which the button
            // takes only once it is no longer waiting on git (`holdWhenLive`).
            if (act === "discard-hunk-go") {
                driver.holdWhenLive(diffPane, () => stagedTimer.start())
            } else {
                renderedBarrier.begin()
            }
        }
    }
    // The three in-pane writes (line, hunk, discarded hunk). One that empties its side moves the pane to the next file
    // (`diff-follow`), so the run ends on where the pane lands, not on the write's answer — else the shot is an empty
    // body under the next file's name and the census misses `DiffRowDelegate`.
    SampleTimer {
        id: stagedTimer
        onTriggered: {
            // A closed pane is done. What it lands on need not have rows — a picture, a binary or an embedded
            // repository has none (`DiffPane.diffSettled`) — so waiting for rows would hold those to the watchdog.
            if (!driver.wroteAndSettled() || page.diffSettling
                    || (page.diffShown && !(diffPane.diffSettled() && diffPane.picturesSettled)))
                return
            stagedTimer.stop()
            // The write barrier reports the rest (the contract, the status owed, the row taken out of its bucket).
            writeBarrier.start()
        }
    }
    // diff-bar: the text hand over the rows leaves the bar's strip free (`AppListView.barRoom`) — the bar is drawn over
    // the rows, so a hand covering the frame takes every press on the trough. Claims: `clear=` (the hand's right edge
    // against the bar's left) and `reach=` (the hand still answers at its last pixel). The settle waits for the bar, so
    // a fixture with no bar never reports and ends at the watchdog.
    SampleTimer {
        id: diffBarTimer
        property string lastGeom: ""
        onTriggered: {
            // Same settle as `diffSweepTimer`, and also for the bar to come out.
            if (diffPane.view.count <= 0 || !diffPane.textHand.barOut)
                return
            const geom = Math.round(diffPane.view.contentHeight) + "," + diffPane.view.count
            if (geom !== diffBarTimer.lastGeom) {
                diffBarTimer.lastGeom = geom
                return
            }
            diffBarTimer.stop()
            Harness.report("diff_bar " + diffPane.textHand.barTally())
            renderedBarrier.begin()
        }
    }

    // diff-blank: the blank right of a row's last character selects nothing and stops the wash. Entered with pixels,
    // unlike the text verbs' byte offsets (`pickDiffText`), which pass whatever byte is under a point. `quiet=` (a drag
    // wholly in the blank selects nothing) needs `clamped=` (head to blank takes exactly the row, as
    // `DiffModel.selectRow` does), or a hand that selects nothing at all passes. `quiet=` covers a second pass after a
    // sideways send too (`DiffCodeScroll.offset`); `sent=` says the code moved and that pass still had rows. `rows=0`
    // is a fixture with nowhere to press.
    SampleTimer {
        id: diffBlankTimer
        property string lastGeom: ""
        onTriggered: {
            // Same settle as `diffSweepTimer`.
            if (diffPane.view.count <= 0)
                return
            const geom = Math.round(diffPane.view.contentHeight) + "," + diffPane.view.count
            if (geom !== diffBlankTimer.lastGeom) {
                diffBlankTimer.lastGeom = geom
                return
            }
            diffBlankTimer.stop()
            const before = acts.blankTally(true)
            const across = acts.spanTakes()
            // Half the shortest reachable line, so every row pressed still ends inside the pane for the second pass.
            diffPane.sendCode(Math.min(diffPane.codeMax, before.shortest / 2))
            const after = acts.blankTally(false)
            // The claim's drags end selecting nothing, so one is left standing for the camera.
            acts.standPastTheEnd()
            Harness.report("diff_blank quiet=" + (before.quiet && after.quiet)
                              + " clamped=" + before.clamped
                              + " backwards=" + before.backwards
                              + " across=" + across
                              + " sent=" + (diffPane.codeAt > 0 && after.rows > 0)
                              + " rows=" + before.rows + "/" + after.rows
                              + " at=" + Math.round(diffPane.codeAt))
            renderedBarrier.begin()
        }
    }
    /// Drags beside every row whose line ends inside the pane: once wholly in the blank, and with `fromHead` also from
    /// the line's head out and back. Answers the reachable row count, the shortest line, and the claims (false at no
    /// rows). Enters `takeAt` / `followAt` / `releaseText`, the `MouseArea` handlers' own calls
    /// (verify-ui §壊れない動詞の実装と反復).
    function blankTally(fromHead) {
        const hand = diffPane.textHand
        let rows = 0
        let shortest = 0
        let quiet = true
        let clamped = true
        let backwards = true
        for (let i = 0; i < diffPane.view.count; i++) {
            const item = diffPane.view.itemAtIndex(i)
            // Only rows the plain copy takes, so a drag can be held against the row's own copy.
            if (!item || (item.kind !== "ctx" && item.kind !== "add") || item.codeInk <= 0)
                continue
            // The line's end in the hand's coordinates (the code scrolls under the pane).
            const ends = item.codeInk - diffPane.codeAt
            const y = item.y - diffPane.view.contentY + item.height / 2
            if (ends < 1 || ends + 4 >= hand.width || y < 0 || y >= hand.height)
                continue
            rows++
            shortest = shortest === 0 ? item.codeInk : Math.min(shortest, item.codeInk)
            const past = Math.min(hand.width - 1, ends + 200)
            const whole = acts.copyOfRow(i)

            diffPane.diffModel.clearSelect()
            hand.takeAt(ends + 4, y, Qt.LeftButton)
            hand.followAt(past, y)
            hand.releaseText()
            quiet = quiet && !diffPane.diffModel.selHasNew && diffPane.diffModel.selRemoved === 0

            if (fromHead) {
                clamped = clamped && acts.dragTakes(0, y, past, y, whole)
                // And from the blank back onto the row — the blank is a start too (規約 §diff の中身をコピーする).
                backwards = backwards && acts.dragTakes(past, y, 0, y, whole)
            }
        }
        return { rows: rows, shortest: shortest, quiet: rows > 0 && quiet,
                 clamped: fromHead && rows > 0 && clamped,
                 backwards: fromHead && rows > 0 && backwards }
    }
    /// What the plain `Copy` puts on the pad for one row selected by number — no pixels involved, so it is the answer
    /// the drags are held against. Read off the pad (`ClipboardHelper.lastCopied`); a copy that put nothing out answers
    /// "" rather than the pad's stale text.
    function copyOfRow(row) {
        diffPane.diffModel.clearSelect()
        diffPane.diffModel.selectRow(0, row)
        return diffPane.copySelection() ? driver.clipboard.lastCopied : ""
    }
    /// One drag between two points of the hand; whether its copy is `want` (never for an empty `want`).
    function dragTakes(fromX, fromY, toX, toY, want) {
        const hand = diffPane.textHand
        diffPane.diffModel.clearSelect()
        hand.takeAt(fromX, fromY, Qt.LeftButton)
        hand.followAt(toX, toY)
        hand.releaseText()
        return diffPane.copySelection() && driver.clipboard.lastCopied === want && want !== ""
    }
    /// A drag from the head of the first reachable row into the blank beside the last, held against the rows between
    /// copied one by one — the multi-row join (removed lines left out) a single-row drag cannot test. Answers
    /// `rows=<n>` when fewer than two rows are reachable.
    function spanTakes() {
        const hand = diffPane.textHand
        const reach = []
        for (let i = 0; i < diffPane.view.count; i++) {
            const item = diffPane.view.itemAtIndex(i)
            if (!item || (item.kind !== "ctx" && item.kind !== "add") || item.codeInk <= 0)
                continue
            const ends = item.codeInk - diffPane.codeAt
            const y = item.y - diffPane.view.contentY + item.height / 2
            if (ends >= 1 && ends + 4 < hand.width && y >= 0 && y < hand.height)
                reach.push({ row: i, ends: ends, y: y })
        }
        if (reach.length < 2)
            return "rows=" + reach.length
        const first = reach[0]
        const last = reach[reach.length - 1]
        const lines = []
        for (let i = first.row; i <= last.row; i++) {
            const item = diffPane.view.itemAtIndex(i)
            if (item && (item.kind === "ctx" || item.kind === "add"))
                lines.push(acts.copyOfRow(i))
        }
        const want = lines.join("\n")
        const took = acts.dragTakes(0, first.y, Math.min(hand.width - 1, last.ends + 200), last.y, want)
        return "" + (took && lines.length > 1)
    }
    /// The shot: a drag past the end of two rows, left standing — the band has to stop at the ink.
    function standPastTheEnd() {
        const hand = diffPane.textHand
        let stood = 0
        diffPane.diffModel.clearSelect()
        for (let i = 0; i < diffPane.view.count && stood < 2; i++) {
            const item = diffPane.view.itemAtIndex(i)
            if (!item || (item.kind !== "ctx" && item.kind !== "add") || item.codeInk <= 0)
                continue
            const ends = item.codeInk - diffPane.codeAt
            const y = item.y - diffPane.view.contentY + item.height / 2
            if (ends < 1 || ends + 4 >= hand.width || y < 0 || y >= hand.height)
                continue
            if (stood === 0)
                hand.takeAt(0, y, Qt.LeftButton)
            hand.followAt(Math.min(hand.width - 1, ends + 200), y)
            stood++
        }
        hand.releaseText()
    }
    /// Whether the list has laid its rows out, not merely received them: until then `contentHeight` is zero, so `maxY`
    /// and `codeMax` read as nothing.
    function viewLaidOut() {
        return diffPane.view.count > 0 && diffPane.view.width > 0
                && diffPane.view.height > 0 && diffPane.view.contentHeight > 0
    }
    /// How far down the reader is taken before the rebuild (`keep-place`) or the colour swap (`colour-place`); both are
    /// judged on getting exactly this back.
    readonly property real readY: 400
    function reportPlace(at, room) {
        Harness.report("diff_place at=" + Math.round(at)
                          + " want=" + Math.round(acts.readY)
                          + " room=" + Math.round(room))
    }
    // line-back: one line staged from the diff, then the whole file moved from the file list — the diff must follow
    // both. The rows shrink, come back on unstage, and staging the file empties the side, so the pane follows it
    // (`RepoPage.followEmptySide`). Each step waits for its write and then for the pane's rows, which are the output.
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
        /// The write landed and the pane stopped rereading (`RepoPage.diffSettling`) — the answer triggers a reread
        /// whose rows replace these. Never also wait to see `busyCount` rise: a write can start and finish in one tick.
        function wroteAndSettled() {
            return driver.wroteAndSettled() && !page.diffSettling
        }
        onTriggered: {
            const rows = diffPane.view.count
            if (lineBackTimer.step === 0) {
                const line = diffPane.firstChangedLine(0)
                if (line < 0)
                    return
                lineBackTimer.rows0 = rows
                driver.pressWrite("stage-line", () => {
                    diffPane.stageLine(0, line)
                    return true
                })
                lineBackTimer.step = 1
                return
            }
            if (lineBackTimer.step === 1) {
                if (!lineBackTimer.wroteAndSettled() || rows >= lineBackTimer.rows0)
                    return
                lineBackTimer.rows1 = rows
                lineBackTimer.shrank = true
                // The file list's own `−` path.
                driver.pressWrite("unstage-path", () => {
                    repoTab.unstagePath(page.diffPath)
                    return true
                })
                lineBackTimer.step = 2
                return
            }
            if (lineBackTimer.step === 2) {
                if (!lineBackTimer.wroteAndSettled() || rows !== lineBackTimer.rows0)
                    return
                lineBackTimer.back = true
                driver.pressWrite("stage-path", () => {
                    repoTab.stagePath(page.diffPath)
                    return true
                })
                lineBackTimer.step = 3
                return
            }
            if (!lineBackTimer.wroteAndSettled() || page.diffKind !== "staged" || rows === 0)
                return
            lineBackTimer.stop()
            Harness.report("line_back back=" + lineBackTimer.back
                              + " shrank=" + lineBackTimer.shrank
                              + " followed=" + (page.diffKind + ":" + page.diffPath)
                              + " rows0=" + lineBackTimer.rows0
                              + " rows1=" + lineBackTimer.rows1)
            renderedBarrier.begin()
        }
    }
    // line-run: lines staged one after another. The pane refuses a press while its rows are still coming
    // (`RepoPage.diffSettling`), so each waits on exactly that. Settling tied to a file-list signal sent only when its
    // rows differ never ends at the second line of a file already on both sides — the case this pins.
    SampleTimer {
        id: lineRunTimer
        readonly property int want: 3
        property int done: 0
        /// How long (ms) the list has gone without a row to name.
        property int waited: 0
        function begin() {
            lineRunTimer.done = 0
            lineRunTimer.waited = 0
            lineRunTimer.start()
        }
        function report() {
            Harness.report("line_run staged=" + lineRunTimer.done
                              + " want=" + lineRunTimer.want
                              + " rows=" + diffPane.view.count
                              + " waited=" + lineRunTimer.waited)
        }
        onTriggered: {
            if (page.diffSettling)
                return
            if (lineRunTimer.done === lineRunTimer.want) {
                lineRunTimer.stop()
                lineRunTimer.report()
                renderedBarrier.begin()
                return
            }
            // The list builds its items a frame after the rows arrive, so an early empty answer is not "none" and is
            // waited on.
            const line = diffPane.firstChangedLine(0)
            if (line < 0) {
                // waits(ceiling): the shortfall is still reported and still fails; this only names it before the watchdog
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
    // diff-escape, read once the pane has left the screen: the swap is the layout's, and in the key's own turn the page
    // still holds the keyboard through the outgoing pane. `kept=`: the page is a focus scope, so it keeps the keyboard
    // and the next Escape lands (`tests/qml/tst_escape.qml`).
    SampleTimer {
        id: escapeTimer
        property bool hand: false
        property bool took: false
        onTriggered: {
            if (page.diffShown || diffPane.view.visible)
                return
            escapeTimer.stop()
            Harness.report("diff_escape took=" + escapeTimer.took
                              + " shown=" + page.diffShown
                              + " key=" + (page.diffKey === "" ? "empty" : "held")
                              + " folded=" + page.sidebarCollapsed
                              + " hand=" + escapeTimer.hand
                              + " kept=" + page.activeFocus
                              // One item holds active focus, so this also says nothing else kept it: the arrows
                              // now reach `GraphPane.stepRow`.
                              + " graph=" + graphPane.view.activeFocus)
            renderedBarrier.begin()
        }
    }
    // diff-follow: where the pane lands when its file is moved whole from the file list — the next file on that side,
    // or the same file on its new side when none is left (デザイン規約 §diff の中のステージ).
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
                // Read before the write takes the file off this side.
                followTimer.alone =
                    worktreeModel.besidePath(page.diffKind, page.diffPath) === ""
                // The file list's own `+` / `−` path.
                driver.pressWrite("follow:" + page.diffKind, () => {
                    if (page.diffKind === "staged")
                        repoTab.unstagePath(page.diffPath)
                    else
                        repoTab.stagePath(page.diffPath)
                    return true
                })
                followTimer.step = 1
                return
            }
            // The landing is the output: off its old key, with rows (or closed), and done rereading.
            const now = page.diffShown ? page.diffKind + ":" + page.diffPath : ""
            if (!driver.wroteAndSettled()
                    || now === followTimer.was || (page.diffShown && diffPane.view.count === 0)
                    || page.diffSettling)
                return
            followTimer.stop()
            followTimer.landed = now
            Harness.report("diff_follow shown=" + page.diffShown
                              + " alone=" + followTimer.alone
                              + " was=" + followTimer.was
                              + " landed=" + followTimer.landed)
            renderedBarrier.begin()
        }
    }
    // code-send: half by the bar's own path, the rest by the hand that carries the rows too. Reads where the code and
    // rows ended up — a bar bound to nothing still takes a press, a hand wired to nothing still starts. Waits for
    // `viewLaidOut`; nowhere sideways to go is said and stopped (rules-refs/app-ui.md「欠けた前提は言って止まる」).
    SampleTimer {
        id: codeSendTimer
        property bool sent: false
        function begin() {
            codeSendTimer.sent = false
            codeSendTimer.start()
        }
        function report() {
            Harness.report("code_send bar=" + diffPane.codeBarShown
                              + " hand=" + diffPane.codeHandOn
                              + " at=" + Math.round(diffPane.codeAt)
                              + " max=" + Math.round(diffPane.codeMax)
                              + " measured=" + Math.round(diffPane.codeMeasured)
                              + " drawn=" + Math.round(diffPane.codeDrawn)
                              + " down=" + Math.round(diffPane.view.contentY))
        }
        onTriggered: {
            if (!codeSendTimer.sent) {
                if (!acts.viewLaidOut())
                    return
                if (diffPane.codeMax <= 0) {
                    codeSendTimer.stop()
                    codeSendTimer.report()
                    renderedBarrier.begin()
                    return
                }
                // The anchor's ring is part of the picture, so the hand is left running for the shot.
                diffPane.sendCode(diffPane.codeMax / 2)
                diffPane.startCodeHand(diffPane.view.width / 2,
                                       diffPane.view.height / 2)
                diffPane.driftCodeHand(diffPane.view.width / 2 + 120,
                                       diffPane.view.height / 2 + 120)
                codeSendTimer.sent = true
                return
            }
            // Both of the hand's axes must be seen moving: rows down, code past the bar's half.
            if (diffPane.view.contentY <= 0 || diffPane.codeAt <= diffPane.codeMax / 2)
                return
            codeSendTimer.stop()
            codeSendTimer.report()
            renderedBarrier.begin()
        }
    }
    // The reach as one string (measured, drawn, widest row), compared look to look until it stops moving: delegates are
    // built a layout after the jump, and a `Text` answers at the family's advance until its fallback font resolves
    // (`DiffTextMetrics`). The row is in it so a width filed under the wrong row shows as movement.
    function reachNow() {
        return Math.round(diffPane.codeMeasured) + "," + Math.round(diffPane.codeDrawn)
                + "," + diffPane.codeWidestRow
    }
    // code-grow: a row the pick missed, arriving from below. The pane measures the model's longest lines by column
    // count before any row exists (`DiffTextMetrics.codeW`); the laid-out rows settle the real reach (`DiffReach`).
    // `--preset widelate`'s `late.txt` makes the two disagree: combining pairs fill the pick, and the line drawn widest
    // is plain ASCII at the foot. Claims the picture cannot carry:
    //
    //  - `grew=` the drawn width rose when that row was laid out; `past=` beyond what the pick measured.
    //  - `kept=` the same width under the same row after going back to the head and down again (delegate reuse).
    //  - `ends=` sent to the far end with that row on screen, the frame's right edge is the line's end
    //    (`DiffPane.codeInkRight` against the pane's own gap). The only claim that catches a reach running past every
    //    line — `codeMax` derives from the same width, so it agrees with itself everywhere else.
    //
    // The send is last, so the shot stands at the diff's end, the only frame where room past the text would show.
    SampleTimer {
        id: codeGrowTimer
        /// 0 reading the head, 1 sent to the foot, 2 back at the head, 3 at the foot again, 4 sent to the far end.
        property int step: 0
        property real measured: 0
        property real drawn0: 0
        property real drawn: 0
        property real back: 0
        property real again: 0
        property int row: -1
        property int rowAgain: -1
        /// Text end and frame edge at the far end.
        property real ink: 0
        property real room: 0
        /// The reach at the previous look of this step (`acts.reachNow`).
        property string lastReach: ""
        function begin() {
            codeGrowTimer.step = 0
            codeGrowTimer.lastReach = ""
            codeGrowTimer.start()
        }
        /// Whether the row jumped to is built and the widths have stopped moving.
        function arrived(atRow) {
            if (!diffPane.view.itemAtIndex(atRow))
                return false
            const now = acts.reachNow()
            if (now === codeGrowTimer.lastReach)
                return true
            codeGrowTimer.lastReach = now
            return false
        }
        function goTo(y) {
            codeGrowTimer.lastReach = ""
            diffPane.scrollTo(y)
        }
        /// The gap past the text at the far end. The pane holds exactly `Theme.spaceSm` there, so more is a reach past
        /// every line and less one that stopped short.
        readonly property real tail: codeGrowTimer.room - codeGrowTimer.ink
        function report() {
            const grew = codeGrowTimer.drawn > codeGrowTimer.drawn0
            const past = codeGrowTimer.drawn > codeGrowTimer.measured
            const kept = codeGrowTimer.row >= 0
                    && codeGrowTimer.back === codeGrowTimer.drawn
                    && codeGrowTimer.again === codeGrowTimer.drawn
                    && codeGrowTimer.rowAgain === codeGrowTimer.row
            // A pixel of slack: the send's gutter and the row's are separate items that round independently.
            const ends = codeGrowTimer.ink > 0
                    && Math.abs(codeGrowTimer.tail - Theme.spaceSm) <= 1
            Harness.report("code_grow grew=" + grew + " past=" + past + " kept=" + kept + " ends=" + ends
                              + " measured=" + Math.round(codeGrowTimer.measured)
                              + " drawn0=" + Math.round(codeGrowTimer.drawn0)
                              + " drawn=" + Math.round(codeGrowTimer.drawn)
                              + " back=" + Math.round(codeGrowTimer.back)
                              + " again=" + Math.round(codeGrowTimer.again)
                              + " row=" + codeGrowTimer.row
                              + " rowAgain=" + codeGrowTimer.rowAgain
                              + " at=" + Math.round(diffPane.codeAt)
                              + " max=" + Math.round(diffPane.codeMax)
                              + " ink=" + Math.round(codeGrowTimer.ink)
                              + " room=" + Math.round(codeGrowTimer.room)
                              + " tail=" + Math.round(codeGrowTimer.tail)
                              + " want=" + Math.round(Theme.spaceSm)
                              + " down=" + Math.round(diffPane.view.maxY))
        }
        function finish() {
            codeGrowTimer.stop()
            codeGrowTimer.report()
            renderedBarrier.begin()
        }
        onTriggered: {
            const foot = diffPane.view.count - 1
            if (codeGrowTimer.step === 0) {
                if (!acts.viewLaidOut())
                    return
                // A diff that fits its frame has no row to arrive from below: said and stopped
                // (rules-refs/app-ui.md「欠けた前提は言って止まる」).
                if (diffPane.view.maxY <= 0) {
                    codeGrowTimer.finish()
                    return
                }
                codeGrowTimer.measured = diffPane.codeMeasured
                codeGrowTimer.drawn0 = diffPane.codeDrawn
                codeGrowTimer.step = 1
                codeGrowTimer.goTo(diffPane.view.maxY)
                return
            }
            if (codeGrowTimer.step === 1) {
                if (!codeGrowTimer.arrived(foot))
                    return
                codeGrowTimer.drawn = diffPane.codeDrawn
                codeGrowTimer.row = diffPane.codeWidestRow
                codeGrowTimer.step = 2
                codeGrowTimer.goTo(0)
                return
            }
            if (codeGrowTimer.step === 2) {
                if (!codeGrowTimer.arrived(0))
                    return
                codeGrowTimer.back = diffPane.codeDrawn
                codeGrowTimer.step = 3
                codeGrowTimer.goTo(diffPane.view.maxY)
                return
            }
            if (codeGrowTimer.step === 3) {
                if (!codeGrowTimer.arrived(foot))
                    return
                codeGrowTimer.again = diffPane.codeDrawn
                codeGrowTimer.rowAgain = diffPane.codeWidestRow
                // All the way right with the widest row on screen — otherwise a short line in view answers `ends=`.
                codeGrowTimer.step = 4
                codeGrowTimer.lastReach = ""
                diffPane.sendCode(diffPane.codeMax)
                return
            }
            // The send is immediate, but the rows are redrawn a layout later: wait for the ink to stand still.
            const ink = diffPane.codeInkRight()
            const now = Math.round(diffPane.codeAt) + "," + Math.round(ink)
            if (now !== codeGrowTimer.lastReach) {
                codeGrowTimer.lastReach = now
                return
            }
            codeGrowTimer.ink = ink
            codeGrowTimer.room = diffPane.view.width
            codeGrowTimer.finish()
        }
    }
    // code-shrink: the same file reread with its widest line staged out (`--preset widelate`'s `top.txt`, whose first
    // changed row is that line). At risk is the place along the row: a reach answering zero between the two readings,
    // or latching its largest value, loses it. `kept=` the place, `shrank=` less room, `narrower=` both halves of the
    // reach came down.
    SampleTimer {
        id: codeShrinkTimer
        /// The first hunk's line that gets staged — the file's widest.
        property int line: -1
        property bool wrote: false
        property real at0: 0
        property real max0: 0
        property real measured0: 0
        property real drawn0: 0
        property int rows0: 0
        property string lastReach: ""
        function begin(atLine) {
            codeShrinkTimer.line = atLine
            codeShrinkTimer.wrote = false
            codeShrinkTimer.lastReach = ""
            codeShrinkTimer.start()
        }
        function report() {
            const kept = codeShrinkTimer.at0 > 0
                    && Math.round(diffPane.codeAt) === Math.round(codeShrinkTimer.at0)
            const shrank = diffPane.codeMax < codeShrinkTimer.max0
            const narrower = diffPane.codeMeasured < codeShrinkTimer.measured0
                    && diffPane.codeDrawn < codeShrinkTimer.drawn0
            Harness.report("code_shrink kept=" + kept + " shrank=" + shrank + " narrower=" + narrower
                              + " at0=" + Math.round(codeShrinkTimer.at0)
                              + " at=" + Math.round(diffPane.codeAt)
                              + " max0=" + Math.round(codeShrinkTimer.max0)
                              + " max=" + Math.round(diffPane.codeMax)
                              + " measured0=" + Math.round(codeShrinkTimer.measured0)
                              + " measured=" + Math.round(diffPane.codeMeasured)
                              + " drawn0=" + Math.round(codeShrinkTimer.drawn0)
                              + " drawn=" + Math.round(diffPane.codeDrawn)
                              + " rows0=" + codeShrinkTimer.rows0
                              + " rows=" + diffPane.view.count)
        }
        function finish() {
            codeShrinkTimer.stop()
            codeShrinkTimer.report()
            renderedBarrier.begin()
        }
        onTriggered: {
            if (!codeShrinkTimer.wrote) {
                if (!acts.viewLaidOut())
                    return
                // Nowhere sideways to go is no place to keep.
                if (diffPane.codeMax <= 0) {
                    codeShrinkTimer.finish()
                    return
                }
                // An eighth along: short of where the shorter reading ends, so `kept=` means kept, not clamped.
                diffPane.sendCode(diffPane.codeMax / 8)
                codeShrinkTimer.at0 = diffPane.codeAt
                codeShrinkTimer.max0 = diffPane.codeMax
                codeShrinkTimer.measured0 = diffPane.codeMeasured
                codeShrinkTimer.drawn0 = diffPane.codeDrawn
                codeShrinkTimer.rows0 = diffPane.view.count
                driver.pressWrite("stage-line", () => {
                    diffPane.stageLine(0, codeShrinkTimer.line)
                    return true
                })
                codeShrinkTimer.wrote = true
                return
            }
            // The write, the reread, then the reread rows' widths standing still.
            if (!driver.wroteAndSettled()
                    || page.diffSettling || diffPane.view.count === 0)
                return
            const now = acts.reachNow()
            if (now !== codeShrinkTimer.lastReach) {
                codeShrinkTimer.lastReach = now
                return
            }
            codeShrinkTimer.finish()
        }
    }
    // code-swap: switching file drops both place and width. The second file is the row beside the first in the list
    // (`NavSectionModel.beside_path`). `room=` makes `dropped=` mean something — a file with nowhere sideways to go
    // sits at its left edge regardless (`--preset widelate`'s `plain.txt` is narrower yet still wider than the pane).
    SampleTimer {
        id: codeSwapTimer
        property int step: 0
        property string was: ""
        property string landed: ""
        property real at0: 0
        property real max0: 0
        property real drawn0: 0
        property string lastReach: ""
        function begin() {
            codeSwapTimer.step = 0
            codeSwapTimer.landed = ""
            codeSwapTimer.lastReach = ""
            codeSwapTimer.start()
        }
        function report() {
            const dropped = codeSwapTimer.at0 > 0 && Math.round(diffPane.codeAt) === 0
            const room = diffPane.codeMax > 0
            const narrower = diffPane.codeDrawn > 0 && diffPane.codeDrawn < codeSwapTimer.drawn0
            Harness.report("code_swap dropped=" + dropped + " room=" + room + " narrower=" + narrower
                              + " at0=" + Math.round(codeSwapTimer.at0)
                              + " at=" + Math.round(diffPane.codeAt)
                              + " max0=" + Math.round(codeSwapTimer.max0)
                              + " max=" + Math.round(diffPane.codeMax)
                              + " drawn0=" + Math.round(codeSwapTimer.drawn0)
                              + " measured=" + Math.round(diffPane.codeMeasured)
                              + " drawn=" + Math.round(diffPane.codeDrawn)
                              + " was=" + codeSwapTimer.was
                              + " landed=" + codeSwapTimer.landed)
        }
        function finish() {
            codeSwapTimer.stop()
            codeSwapTimer.report()
            renderedBarrier.begin()
        }
        onTriggered: {
            if (codeSwapTimer.step === 0) {
                if (!acts.viewLaidOut())
                    return
                if (diffPane.codeMax <= 0) {
                    codeSwapTimer.finish()
                    return
                }
                diffPane.sendCode(diffPane.codeMax / 2)
                codeSwapTimer.at0 = diffPane.codeAt
                codeSwapTimer.max0 = diffPane.codeMax
                codeSwapTimer.drawn0 = diffPane.codeDrawn
                codeSwapTimer.was = page.diffKind + ":" + page.diffPath
                // `<bucket>:<path>` of the row beside this one under the same heading; none ends the run here.
                const beside = worktreeModel.besidePath(page.diffKind, page.diffPath)
                if (beside === "") {
                    codeSwapTimer.finish()
                    return
                }
                const cut = beside.indexOf(":")
                const path = beside.substring(cut + 1)
                codeSwapTimer.landed = beside
                page.toggleDiff(beside.substring(0, cut), path, worktreeModel.origOf(path))
                codeSwapTimer.step = 1
                return
            }
            // On the other file, done reading and laid out, before its width counts.
            if (page.diffKind + ":" + page.diffPath !== codeSwapTimer.landed
                    || page.diffSettling || !acts.viewLaidOut())
                return
            const now = acts.reachNow()
            if (now !== codeSwapTimer.lastReach) {
                codeSwapTimer.lastReach = now
                return
            }
            codeSwapTimer.finish()
        }
    }
    // diff-sweep: the text taken from the ground under a short file's last row (規約 §diff の中身をコピーする), shaped
    // like `details-sweep`. `reach=` sweeps from nine starts, each judged over a cleared selection — one start (the
    // middle hides it) or one read at the end would pass a partial reach. `ours=`: a press on a hunk heading's words
    // stays the hunk's (規約 §diff の中のステージ).
    SampleTimer {
        id: diffSweepTimer
        property string lastGeom: ""
        onTriggered: {
            // Settle: the view lays the rows out a frame after they arrive, and ground about to become a row is
            // neither.
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
            // The last sweep's wash stays for the shot; the heading press below is refused and takes nothing down.
            const hunkRow = hand.firstHunkRow()
            const ours = hunkRow < 0 || hand.pressOnHunk(hunkRow)
            // `ground=false` is a fixture with nowhere to sweep from. `ours=false` is the pass, as in `details_sweep`.
            Harness.report("diff_sweep reach=" + reach + "/" + tries
                              + " ground=" + hand.hasGround
                              + " ours=" + ours
                              + " hunkRow=" + hunkRow
                              + " " + diffPane.pickTally())
            renderedBarrier.begin()
        }
    }
    // keep-place: read part way down a long diff, then write — the rebuild must come back to the same place. Waits for
    // `viewLaidOut` (before it the scroll goes nowhere); a laid-out diff with less than `readY` of room is said and
    // stopped (rules-refs/app-ui.md「欠けた前提は言って止まる」).
    SampleTimer {
        id: keepPlaceTimer
        /// The first hunk's line whose staging rebuilds the diff under the reader.
        property int line: -1
        property bool wrote: false
        function begin(atLine) {
            keepPlaceTimer.line = atLine
            keepPlaceTimer.wrote = false
            keepPlaceTimer.start()
        }
        onTriggered: {
            if (!keepPlaceTimer.wrote) {
                if (!acts.viewLaidOut())
                    return
                if (diffPane.view.maxY < acts.readY) {
                    keepPlaceTimer.stop()
                    acts.reportPlace(diffPane.view.contentY, diffPane.view.maxY)
                    renderedBarrier.begin()
                    return
                }
                diffPane.scrollTo(acts.readY)
                driver.pressWrite("stage-selection", () => {
                    page.stageSelection(0, keepPlaceTimer.line)
                    return true
                })
                keepPlaceTimer.wrote = true
                return
            }
            // Then the rebuilt list put back on the place. A write that emptied this side never lands, and the run
            // waits out the watchdog.
            if (!driver.wroteAndSettled()
                    || diffPane.placeLandedY < 0)
                return
            keepPlaceTimer.stop()
            acts.reportPlace(diffPane.placeLandedY, diffPane.view.maxY)
            renderedBarrier.begin()
        }
    }
    // colour-place: a reader scrolled before the colours land. They rewrite every row (`DiffModel::repaint_rows`),
    // which must cost neither the place nor the width. `held=`: rows are markup either way (`markup::styled`), so the
    // colour tags are neither drawn nor measured and `rows_gen` does not move — a row measured in another format reads
    // its `<font …>` tags as letters, and a reading counted as new drops every filed width.
    // waits(paced): the run ends on the colours having landed, and the beat only decides how close the last look is
    Timer {
        id: colourPlaceTimer
        interval: 50
        repeat: true
        property int waited: 0
        property bool scrolled: false
        property real measuredWas: 0
        property real drawnWas: 0
        /// The reach at the previous look since the colours landed, so rows mid-rewrite are not read.
        property string lastReach: ""
        function begin() {
            colourPlaceTimer.waited = 0
            colourPlaceTimer.scrolled = false
            colourPlaceTimer.lastReach = ""
            colourPlaceTimer.start()
        }
        /// Keeps the reach as the reading before. Also taken at the scroll: a small diff can be coloured within one
        /// look, and a row that answered `firstChangedLine` is already built and measured.
        function look() {
            colourPlaceTimer.measuredWas = diffPane.codeMeasured
            colourPlaceTimer.drawnWas = diffPane.codeDrawn
        }
        onTriggered: {
            // waits(measured): printed in the report and compared with nothing — the run ends on `coloured`
            colourPlaceTimer.waited += colourPlaceTimer.interval
            if (!colourPlaceTimer.scrolled) {
                // The open again while the pane is not reading this file: asked once and lost, nothing else brings
                // the rows.
                if (!page.diffShown || page.diffKey !== "unstaged:" + Harness.autoActArg)
                    page.openDiff("unstaged", Harness.autoActArg, "")
                // Scroll as soon as the rows are there, before the colours.
                if (!Awaited.all("colour_place", { "rows": diffPane.firstChangedLine(0) >= 0 }))
                    return
                diffPane.scrollTo(acts.readY)
                colourPlaceTimer.scrolled = true
                colourPlaceTimer.look()
                return
            }
            if (!diffPane.diffModel.coloured) {
                Awaited.at("colour_place", "coloured")
                colourPlaceTimer.look()
                return
            }
            const now = acts.reachNow()
            if (now !== colourPlaceTimer.lastReach) {
                Awaited.at("colour_place", "still")
                colourPlaceTimer.lastReach = now
                return
            }
            colourPlaceTimer.stop()
            const held = colourPlaceTimer.drawnWas > 0
                    && Math.round(diffPane.codeMeasured) === Math.round(colourPlaceTimer.measuredWas)
                    && Math.round(diffPane.codeDrawn) === Math.round(colourPlaceTimer.drawnWas)
            Harness.report("colour_place coloured="
                              + diffPane.diffModel.coloured
                              + " at=" + Math.round(diffPane.view.contentY)
                              + " held=" + held
                              + " rows=" + diffPane.view.count
                              + " waited=" + colourPlaceTimer.waited
                              + " measured0=" + Math.round(colourPlaceTimer.measuredWas)
                              + " measured=" + Math.round(diffPane.codeMeasured)
                              + " drawn0=" + Math.round(colourPlaceTimer.drawnWas)
                              + " drawn=" + Math.round(diffPane.codeDrawn))
            driver.complete()
        }
    }
}

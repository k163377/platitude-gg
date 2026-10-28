pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The diff pane's verbs. Most wait first for a changed row of the first hunk (`stageRowTimer`) and branch from there;
/// the rest are chains of their own that the dispatch starts by name.
///
/// **One file on purpose**: every operation but `colourPlaceTimer` is started by id from `stageRowTimer`'s tail
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
    readonly property var unstagedModel: driver.unstagedModel
    readonly property var diffPane: driver.diffPane
    readonly property var graphPane: driver.graphPane
    readonly property var diffRowMenu: driver.diffRowMenu
    readonly property var writeBarrier: driver.barrierWrite
    /// The verbs that open a file's preview: each waits for the decode and reports what the pane shows.
    readonly property var previews: ["preview", "preview-unstaged", "preview-staged", "preview-close"]
    /// The verbs whose own operation takes over at the rows. `diff-sweep`, `-bar` and `-blank` read the laid-out
    /// view, a frame behind the rows — their operations wait for it.
    readonly property var samplerActs: ["diff-sweep", "diff-bar", "diff-blank", "keep-place", "code-send", "code-grow",
                                        "code-shrink", "code-swap", "line-run", "diff-follow", "line-back"]

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (["stage-hunk", "stage-line", "discard-hunk", "discard-hunk-go", "diff-file", "conflict-sides", "line-tools",
             "hunk-tools", "keep-place", "diff-tick", "code-send", "line-back", "code-grow", "code-shrink", "code-swap",
             "diff-select", "diff-copy", "diff-menu", "diff-copy-removed", "diff-sweep", "diff-band-sweep", "diff-bar",
             "diff-blank", "diff-follow", "line-run", "diff-escape"].indexOf(act) >= 0) {
            // `<path>` or `<bucket>:<path>` (an untracked file has no unstaged diff; a conflict is read from
            // `conflicts`). Only the bucket names count as a prefix, so a path with a colon still opens.
            const cut = arg.indexOf(":")
            const head = cut > 0 ? arg.substring(0, cut) : ""
            const named = head === "staged" || head === "unstaged"
                          || head === "untracked" || head === "conflicts"
            page.showWip()
            // A rename's source comes off the model, as a row click hands it over; without it the diff reads as a new
            // file.
            const filePath = named ? arg.substring(cut + 1) : arg
            // A missing path is the run's own mistake and must read as one: git takes an empty pathspec as the whole
            // tree, so the pane would fill with a plausible diff and the verb would report rows of no file.
            if (filePath === "") {
                driver.act.complete(() => "diff_arg act=" + act + " named=false")
                // True even here — falsy reads as "not mine" to the dispatch chain.
                return true
            }
            page.toggleDiff(named ? head : "unstaged", filePath,
                            unstagedModel.origOf(filePath))
            acts.beginRows(act)
        } else if (acts.previews.indexOf(act) >= 0) {
            // Completion is the pane settling (`stageRowTimer`). No path: said and completed in the dispatch — no
            // read would answer a wait, and the missing must_say line fails the run.
            if (arg === "") {
                driver.act.complete(() => "diff_arg act=" + act + " named=false")
            } else {
                page.toggleDiff(act === "preview" ? "untracked"
                                : act === "preview-staged" ? "staged" : "unstaged", arg, "")
                acts.beginRows(act)
            }
        } else if (act === "colour-place") {
            page.showWip()
            page.toggleDiff("unstaged", arg, "")
            colourPlaceTimer.since = PerfProbe.clockMs()
            colourPlaceTimer.start()
        } else {
            return false
        }
        return true
    }

    /// Starts the wait for the rows, with the stages `act` goes through there.
    function beginRows(act) {
        stageRowTimer.order = acts.rowStages(act)
        stageRowTimer.since = PerfProbe.clockMs()
        stageRowTimer.start()
    }
    /// The stages `stageRowTimer` goes through for `act` before it completes or hands the run on: `rows`, said first,
    /// where another operation takes over; then the one input the verb puts in at the rows, if it has one.
    function rowStages(act) {
        if (act === "diff-escape")
            return ["rows", "escaped"]
        if (act === "hunk-tools")
            return ["rows", "tools"]
        if (act === "stage-hunk" || act === "stage-line")
            return ["rows", "pressed"]
        if (act === "discard-hunk-go" || acts.samplerActs.indexOf(act) >= 0)
            return ["rows"]
        const once = { "diff-band-sweep": "swept", "diff-tick": "ticked", "preview-close": "closed",
                       "line-tools": "tools", "diff-select": "picked", "diff-copy": "picked", "diff-menu": "picked",
                       "diff-copy-removed": "picked" }
        return once[act] === undefined ? [] : [once[act]]
    }
    /// Hands the run to the operation of `act` (one of `samplerActs`), with the first changed line the rows landed
    /// on for the two that write it.
    function startSampler(act, line) {
        if (act === "keep-place")
            keepPlaceTimer.changedLine = line
        if (act === "code-shrink")
            codeShrinkTimer.changedLine = line
        const samplers = {
            "diff-sweep": diffSweepTimer,
            "diff-bar": diffBarTimer,
            "diff-blank": diffBlankTimer,
            "keep-place": keepPlaceTimer,
            "code-send": codeSendTimer,
            "code-grow": codeGrowTimer,
            "code-shrink": codeShrinkTimer,
            "code-swap": codeSwapTimer,
            "line-run": lineRunTimer,
            "diff-follow": followTimer,
            "line-back": lineBackTimer
        }
        samplers[act].start()
    }

    /// The drag the four text verbs share (デザイン規約 §diff の中身をコピーする), entered through the functions the hand's
    /// own handlers call (`DiffTextSelect`) since hover and a real button cannot be injected. It runs from the head of
    /// the first line past the first removed one so the selection holds both sides — a one-sided drag passes on half
    /// the rule. Answers what the drag picked, read as it lands.
    function pickDiffText(act) {
        const removedRow = diffPane.firstRemovedRow()
        const lastRow = Math.min(diffPane.view.count - 1, removedRow + 1)
        diffPane.pickText(0, 1, 0, lastRow, driver.pastLineEnd)
        // `door=`: a headless drag delivers no press, so whether the press sheet that brings the keyboard is on top is
        // asked directly — without the keyboard, `Ctrl+C` cannot take the selection.
        const picked = "diff_pick door=" + diffPane.doorOnTop() + " " + diffPane.pickTally()
                       + " removedRow=" + removedRow + " to=" + lastRow
        if (act === "diff-select")
            return picked
        if (act === "diff-copy") {
            diffPane.copySelection()
            return picked
        }
        // Right-click inside the drag, so the selection stands — the half of the rule these verbs test.
        diffPane.askCodeMenu(0, removedRow, 0)
        if (act === "diff-copy-removed") {
            diffRowMenu.menu.close()
            diffRowMenu.copyRemovedNow()
        }
        return picked
    }
    /// What reached the clipboard, read off `ClipboardHelper.lastCopied` (the clipboard cannot be read back).
    /// `holdsRemoved=` is the claim: false for the plain copy, true for the menu's second row.
    function copyLine(name) {
        const text = driver.clipboard.lastCopied
        const removed = diffPane.diffModel.removedText()
        return name + " holdsRemoved=" + (removed !== "" && text.indexOf(removed) >= 0)
               + " lines=" + (text === "" ? 0 : text.split("\n").length)
    }
    /// Which line endings the pane decided on: the picture cannot say which kind.
    function endingsLine() {
        const d = diffPane.diffModel
        return "line_endings kind=" + d.endingKind + " scope=" + d.endingScope
               + " lines=" + d.endingLines + " text=" + Words.lineEndings(d.endingKind, d.endingFrom,
                   d.endingTo, d.endingLines, d.endingScope, d.endingExt)
    }
    /// Whether a hunk or a line puts a seat out, and the band's word: a line's mark comes up under a hand alone, so a
    /// picture without one cannot say whether the lines offer one. `why=` and `mode=` are the lines over the rows as
    /// they stand, `order=` which of them stand, top to bottom (`DiffFileNotices`).
    function piecesLine() {
        const notes = diffPane.notices
        return "diff_pieces offered=" + diffPane.piecesOffered + " whole=" + diffPane.stageOffered
               + " why=" + notes.wholeShown + " mode=" + notes.modeShown
               + " order=" + notes.fileLinesOrder
    }
    /// `kind=` and `pictures=` (sides decoded) are what the picture cannot tell: an unread pane and a `file:` URL
    /// naming nothing both photograph as an empty frame.
    function previewLine() {
        return "preview_pane kind=" + diffPane.diffModel.previewKind
               + " binary=" + diffPane.diffModel.isBinary
               + " pictures=" + diffPane.picturesShown
    }
    // Waits for the rows, never a fixed time (rules-refs/app-ui.md「diff の行を触る自動化フックは行の到着で計る」). No
    // ceiling: a row that never lands is ended by the watchdog.
    PageStage {
        id: stageRowTimer
        name: "diff_rows"
        /// When the dispatch asked for the file (`PerfProbe.clockMs`), for the report's `waited=` — a diagnosis,
        /// compared with nothing.
        property real since: 0
        /// The first changed line as the rows landed, for the stage that acts on it.
        property int changedLine: -1
        // Whether what the verb is about to name is on screen — by default a changed line of the first hunk.
        function ready() {
            // These need no changed line, and demanding one would hold them to the watchdog: `diff-file` may open a
            // picture or binary with no rows, a typed-over conflict side is a removal, and the band names its path
            // regardless.
            if (["diff-file", "conflict-sides", "diff-tick", "diff-band-sweep",
                 "diff-escape"].indexOf(Harness.autoAct) >= 0)
                return diffPane.diffSettled()
            // The previews also wait for the asynchronous decode, or the shot is an empty frame under a caption.
            if (acts.previews.indexOf(Harness.autoAct) >= 0)
                return diffPane.diffSettled() && diffPane.picturesSettled
            return diffPane.firstChangedLine(0) >= 0
        }
        /// The verbs that end at the rows: each reads what it came for — after its one input, if it has one — and
        /// completes. `rowLine` is the rows as they landed, said first.
        function endAtRows(act, line, rowLine) {
            const op = stageRowTimer
            if (act === "diff-file") {
                op.complete(() => [rowLine, acts.endingsLine(), acts.piecesLine()])
                return
            }
            // Sweeps the band's path (規約 §右のペインの字は掴める). `cut=` is what the picture cannot answer: whether the
            // field holds a value longer than the band.
            if (act === "diff-band-sweep") {
                let swept = ""
                op.input("swept", () => {
                    swept = diffPane.headerHand.sweepAir(7, "cut=" + diffPane.headerCut)
                })
                op.complete(() => [rowLine, "diff_band_sweep " + swept])
                return
            }
            // Which rows name the two sides: the picture cannot say how many should have, and a resolved conflict is
            // where none did.
            if (act === "conflict-sides") {
                op.complete(() => [rowLine, "conflict_sides " + diffPane.sideTally()])
                return
            }
            // The page's tick. The file is untouched, so core answers with nothing (`RepoSession::refresh_diff`): the
            // ask itself is the edge, read as it goes in. `loading=`: a tick must not reload the pane like a click.
            if (act === "diff-tick") {
                let ticked = ""
                op.input("ticked", () => {
                    const asked = page.pollDiff()
                    ticked = "diff_tick asked=" + asked
                             + " loading=" + diffPane.diffModel.loading
                             + " rows=" + diffPane.view.count
                })
                op.complete(() => [rowLine, ticked])
                return
            }
            if (acts.previews.indexOf(act) >= 0) {
                if (act !== "preview-close") {
                    op.complete(() => [rowLine, acts.previewLine()])
                    return
                }
                // Closed by the header `✕`'s own path, once the pane has been read: after the close nothing of it is
                // left to read. Whether the decoded pictures left the process is read from outside; QML can only say
                // the pane let go of the URLs (`url=empty`).
                const stood = acts.previewLine()
                const was = diffPane.diffModel.previewKind
                op.input("closed", () => {
                    page.closeDiffToGraph()
                })
                op.complete(() => [rowLine, stood,
                                   "preview_close was=" + was
                                   + " kind=" + diffPane.diffModel.previewKind
                                   + " shown=" + page.diffShown
                                   + " pictures=" + diffPane.picturesShown
                                   + " url=" + (diffPane.diffModel.previewNewUrl === ""
                                                && diffPane.diffModel.previewOldUrl === "" ? "empty" : "held")])
                return
            }
            // Hover cannot be injected, so the row the pointer would be on is named.
            if (act === "line-tools") {
                op.input("tools", () => {
                    diffPane.showLineTools(0, line)
                })
                op.complete(() => rowLine)
                return
            }
            if (act === "diff-select" || act === "diff-copy" || act === "diff-menu" || act === "diff-copy-removed") {
                let picked = ""
                op.input("picked", () => {
                    picked = acts.pickDiffText(act)
                })
                op.complete(() => [rowLine, picked].concat(
                                act === "diff-copy" ? [acts.copyLine("diff_copy")]
                                : act === "diff-copy-removed" ? [acts.copyLine("diff_copy_removed")] : []))
                return
            }
            // `discard-hunk`: the rows it would take are the whole of it.
            op.complete(() => rowLine)
        }
        /// The verbs that go on from the rows: each puts its one input in, once, and hands the run to the operation
        /// that waits for what it set going.
        function goOn(act, line) {
            const op = stageRowTimer
            // Escape enters the page's own handler, the one line `Keys.onEscapePressed` calls
            // (`RepoPage.escapePressed`; verify-ui「注入はハンドラ本体そのものへ入れる」). `took=` and `shown=` are
            // separate failures the picture cannot tell apart.
            if (act === "diff-escape") {
                // The hand enters the pane first (`DiffPane.handArrived`, the wheel's path): closing the pane that
                // holds the keyboard is the case that breaks — where the keyboard lands decides whether a second
                // Escape reaches anything.
                op.input("escaped", () => {
                    diffPane.handArrived()
                    escapeTimer.hand = diffPane.view.activeFocus
                    escapeTimer.took = page.escapePressed()
                })
                op.stop()
                escapeTimer.start()
                return
            }
            // A heading's own row is line -1 (`flatten_patches`). Its words are judged once the row is built
            // (`AutoActDriver.headingBarrier`).
            if (act === "hunk-tools") {
                op.input("tools", () => {
                    diffPane.showLineTools(0, -1)
                })
                op.stop()
                driver.barrierHeading.start()
                return
            }
            if (acts.samplerActs.indexOf(act) >= 0) {
                op.stop()
                acts.startSampler(act, line)
                return
            }
            if (act === "stage-hunk" || act === "stage-line") {
                // A line through its own `+` mark, a hunk through its heading's word.
                const pressed = op.input("pressed", () => driver.pressWrite(act, () => {
                    if (act === "stage-line")
                        diffPane.stageLine(0, line)
                    else
                        page.stageSelection(0, -1)
                    return true
                }))
                if (!pressed)
                    return
                op.stop()
                stagedTimer.start()
                return
            }
            // `discard-hunk-go`. A hunk is the smallest discard. The write is armed on the hold's end (`holdToEnd`),
            // which the button takes only once it is no longer waiting on git (`holdWhenLive`).
            op.stop()
            driver.holdWhenLive(diffPane, () => stagedTimer.start())
        }
        onTriggered: {
            const op = stageRowTimer
            const act = Harness.autoAct
            if (op.at("start")) {
                const arrived = op.ready()
                // The page too: the rows the read brought are built a polish after it lands (`rows:diff`).
                if (!Awaited.all("diff_rows_ready", { "ready": arrived, "page": PageSettled.settled(page) }))
                    return
                // The first changed line — a hunk numbers its context lines too (`firstChangedLine`).
                const line = diffPane.firstChangedLine(0)
                // Read as the rows land, and said before acting, so a verb that then fails its write reports both.
                const rowLine = "diff_row act=" + act + " ready=" + arrived
                                + " rows=" + diffPane.view.count
                                + " line=" + line
                                + " waited=" + Math.round(PerfProbe.clockMs() - op.since)
                if (op.order[0] !== "rows") {
                    op.endAtRows(act, line, rowLine)
                    return
                }
                op.changedLine = line
                op.advance("rows", () => rowLine)
            }
            op.goOn(act, op.changedLine)
        }
    }
    // The three in-pane writes (line, hunk, discarded hunk). One that empties its side moves the pane to the next file
    // (`diff-follow`), so the run ends on where the pane lands, not on the write's answer — else the shot is an empty
    // body under the next file's name and the census misses `DiffRowDelegate`.
    PageStage {
        id: stagedTimer
        name: "diff_staged"
        order: ["landed"]
        onTriggered: {
            // A closed pane is done. What it lands on need not have rows — a picture, a binary or an embedded
            // repository has none (`DiffPane.diffSettled`) — so waiting for rows would hold those to the watchdog.
            if (!Awaited.all("diff_staged", {
                "written": acts.wroteAndSettled(),
                "landed": !(page.diffShown && !(diffPane.diffSettled() && diffPane.picturesSettled)) }))
                return
            stagedTimer.advance("landed")
            stagedTimer.stop()
            // The write barrier reports the rest (the contract, the status owed, the row taken out of its bucket).
            writeBarrier.start()
        }
    }
    // diff-bar: the text hand over the rows leaves the bar's strip free (`AppListView.barRoom`) — the bar is drawn over
    // the rows, so a hand covering the frame takes every press on the trough. Claims: `clear=` (the hand's right edge
    // against the bar's left) and `reach=` (the hand still answers at its last pixel). The settle waits for the bar, so
    // a fixture with no bar never reports and ends at the watchdog.
    PageStage {
        id: diffBarTimer
        name: "diff_bar"
        onTriggered: {
            // Settle: the rows' height read until it stands across a frame (the view lays them out a frame after they
            // arrive), and the bar out.
            if (!Awaited.all("diff_bar_ready", { "ready": !(diffPane.view.count <= 0 || !diffPane.textHand.barOut) }))
                return
            const geom = Math.round(diffPane.view.contentHeight) + "," + diffPane.view.count
            if (!diffBarTimer.still("diff_bar_geom", geom))
                return
            diffBarTimer.complete(() => "diff_bar " + diffPane.textHand.barTally())
        }
    }

    // diff-blank: the blank right of a row's last character selects nothing and stops the wash. Entered with pixels,
    // unlike the text verbs' byte offsets (`pickDiffText`), which pass whatever byte is under a point. `quiet=` (a drag
    // wholly in the blank selects nothing) needs `clamped=` (head to blank takes exactly the row, as
    // `DiffModel.selectRow` does), or a hand that selects nothing at all passes. `quiet=` covers a second pass after a
    // sideways send too (`DiffCodeScroll.offset`); `sent=` says the code moved and that pass still had rows. `rows=0`
    // is a fixture with nowhere to press.
    PageStage {
        id: diffBlankTimer
        name: "diff_blank"
        order: ["dragged"]
        /// What the drags found, read as each let go.
        property var found: null
        onTriggered: {
            const op = diffBlankTimer
            // Settle: the rows' height read until it stands across a frame, as `diffBarTimer` does.
            if (!Awaited.all("diff_blank_ready", { "ready": !(diffPane.view.count <= 0) }))
                return
            const geom = Math.round(diffPane.view.contentHeight) + "," + diffPane.view.count
            if (!op.still("diff_blank_geom", geom))
                return
            op.input("dragged", () => {
                const before = acts.blankTally(true)
                const across = acts.spanTakes()
                // Half the shortest reachable line, so every row pressed still ends inside the pane for the second
                // pass.
                diffPane.sendCode(Math.min(diffPane.codeMax, before.shortest / 2))
                const after = acts.blankTally(false)
                // The claim's drags end selecting nothing, so one is left standing for the camera.
                acts.standPastTheEnd()
                op.found = { "before": before, "across": across, "after": after }
            })
            op.complete(() => "diff_blank quiet=" + (op.found.before.quiet && op.found.after.quiet)
                              + " clamped=" + op.found.before.clamped
                              + " backwards=" + op.found.before.backwards
                              + " across=" + op.found.across
                              + " sent=" + (diffPane.codeAt > 0 && op.found.after.rows > 0)
                              + " rows=" + op.found.before.rows + "/" + op.found.after.rows
                              + " at=" + Math.round(diffPane.codeAt))
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
            const at = acts.reachOf(i)
            if (!at)
                continue
            rows++
            shortest = shortest === 0 ? at.ink : Math.min(shortest, at.ink)
            const whole = acts.copyOfRow(i)

            diffPane.diffModel.clearSelect()
            hand.takeAt(Qt.point(at.ends + 4, at.y), Qt.LeftButton)
            hand.followAt(at.blank)
            hand.releaseText()
            quiet = quiet && !diffPane.diffModel.selHasNew && diffPane.diffModel.selRemoved === 0

            if (fromHead) {
                clamped = clamped && acts.dragTakes(Qt.point(0, at.y), at.blank, whole)
                // And from the blank back onto the row — the blank is a start too (規約 §diff の中身をコピーする).
                backwards = backwards && acts.dragTakes(at.blank, Qt.point(0, at.y), whole)
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
    function dragTakes(from, to, want) {
        const hand = diffPane.textHand
        diffPane.diffModel.clearSelect()
        hand.takeAt(from, Qt.LeftButton)
        hand.followAt(to)
        hand.releaseText()
        return diffPane.copySelection() && driver.clipboard.lastCopied === want && want !== ""
    }
    /// A drag from the head of the first reachable row into the blank beside the last, held against the rows between
    /// copied one by one — the multi-row join (removed lines left out) a single-row drag cannot test. Answers
    /// `rows=<n>` when fewer than two rows are reachable.
    function spanTakes() {
        const reach = []
        for (let i = 0; i < diffPane.view.count; i++) {
            const at = acts.reachOf(i)
            if (at)
                reach.push(at)
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
        const took = acts.dragTakes(Qt.point(0, first.y), last.blank, want)
        return "" + (took && lines.length > 1)
    }
    /// The shot: a drag past the end of two rows, left standing — the band has to stop at the ink.
    function standPastTheEnd() {
        const hand = diffPane.textHand
        let stood = 0
        diffPane.diffModel.clearSelect()
        for (let i = 0; i < diffPane.view.count && stood < 2; i++) {
            const at = acts.reachOf(i)
            if (!at)
                continue
            if (stood === 0)
                hand.takeAt(Qt.point(0, at.y), Qt.LeftButton)
            hand.followAt(at.blank)
            stood++
        }
        hand.releaseText()
    }
    /// Row `i` as the drags reach it, or null: a row the plain copy takes (so a drag can be held against the row's own
    /// copy) whose line ends inside the hand. `ends` is that end in the hand's coordinates (the code scrolls under the
    /// pane), `y` the row's middle, `blank` a point past the end.
    function reachOf(i) {
        const hand = diffPane.textHand
        const item = diffPane.view.itemAtIndex(i)
        if (!item || (item.kind !== "ctx" && item.kind !== "add") || item.codeInk <= 0)
            return null
        const ends = item.codeInk - diffPane.codeAt
        const y = item.y - diffPane.view.contentY + item.height / 2
        if (ends < 1 || ends + 4 >= hand.width || y < 0 || y >= hand.height)
            return null
        return { row: i, ink: item.codeInk, ends: ends, y: y, blank: Qt.point(Math.min(hand.width - 1, ends + 200), y) }
    }
    /// Whether the list has laid its rows out, not merely received them: until then `contentHeight` is zero, so `maxY`
    /// and `codeMax` read as nothing.
    function viewLaidOut() {
        return diffPane.view.count > 0 && diffPane.view.width > 0
                && diffPane.view.height > 0 && diffPane.view.contentHeight > 0
    }
    /// The write landed and the pane stopped rereading (`RepoPage.diffSettling`) — the answer triggers a reread whose
    /// rows replace these. Never also wait to see `busyCount` rise: a write can start and finish in one tick.
    function wroteAndSettled() {
        return driver.wroteAndSettled() && !page.diffSettling
    }
    /// How far down the reader is taken before the rebuild (`keep-place`) or the colour swap (`colour-place`); both are
    /// judged on getting exactly this back.
    readonly property real readY: 400
    /// Where the view stands now, against `readY` and the room it has.
    function placeLine() {
        return "diff_place at=" + Math.round(diffPane.view.contentY)
               + " want=" + Math.round(acts.readY)
               + " room=" + Math.round(diffPane.view.maxY)
    }
    // line-back: one line staged from the diff, then the whole file moved from the file list — the diff must follow
    // both. The rows shrink, come back on unstage, and staging the file empties the side, so the pane follows it
    // (`RepoPage.followEmptySide`). Each stage waits for its write and then for the pane's rows, which are the output.
    PageStage {
        id: lineBackTimer
        name: "line_back"
        order: ["line_staged", "path_unstaged", "path_staged"]
        property int rows0: 0
        property int rows1: 0
        /// Whether the rows had shrunk, and come back, as the next press went in.
        property bool shrank: false
        property bool back: false
        onTriggered: {
            const op = lineBackTimer
            const rows = diffPane.view.count
            if (op.at("start")) {
                const line = diffPane.firstChangedLine(0)
                if (!Awaited.all("line_back_line", { "line": line >= 0 }))
                    return
                op.input("line_staged", () => {
                    op.rows0 = rows
                    return driver.pressWrite("stage-line", () => {
                        diffPane.stageLine(0, line)
                        return true
                    })
                })
                return
            }
            if (op.at("line_staged")) {
                if (!Awaited.all("line_back_shrunk", { "written": acts.wroteAndSettled(),
                                                       "shrunk": rows < op.rows0 }))
                    return
                // The file list's own `−` path.
                op.input("path_unstaged", () => {
                    op.rows1 = rows
                    op.shrank = rows < op.rows0
                    return driver.pressWrite("unstage-path", () => {
                        repoTab.unstagePath(page.diffPath)
                        return true
                    })
                })
                return
            }
            if (op.at("path_unstaged")) {
                if (!Awaited.all("line_back_back", { "written": acts.wroteAndSettled(),
                                                     "back": rows === op.rows0 }))
                    return
                op.input("path_staged", () => {
                    op.back = rows === op.rows0
                    return driver.pressWrite("stage-path", () => {
                        repoTab.stagePath(page.diffPath)
                        return true
                    })
                })
                return
            }
            if (!Awaited.all("line_back_followed", { "written": acts.wroteAndSettled(),
                                                     "staged": page.diffKind === "staged",
                                                     "rows": rows !== 0 }))
                return
            op.complete(() => "line_back back=" + op.back
                              + " shrank=" + op.shrank
                              + " followed=" + page.diffKey
                              + " rows0=" + op.rows0
                              + " rows1=" + op.rows1)
        }
    }
    // line-run: lines staged one after another. The pane refuses a press while its rows are still coming
    // (`RepoPage.diffSettling`), so each waits on exactly that. Settling tied to a file-list signal sent only when its
    // rows differ never ends at the second line of a file already on both sides — the case this pins. A line that
    // never comes is named by the ceiling (`line_run_ready waiting=line`).
    PageStage {
        id: lineRunTimer
        name: "line_run"
        order: ["first_line", "second_line", "third_line"]
        readonly property int want: 3
        /// How many lines have gone in: the stages this generation has reached.
        function staged() {
            for (let n = lineRunTimer.order.length; n > 0; n--) {
                if (lineRunTimer.at(lineRunTimer.order[n - 1]))
                    return n
            }
            return 0
        }
        onTriggered: {
            const op = lineRunTimer
            const done = op.at(op.order[op.order.length - 1])
            // The list builds its items a frame after the rows arrive, so an early empty answer is not "none" and is
            // waited on.
            const line = diffPane.firstChangedLine(0)
            if (!Awaited.all("line_run_ready", { "settled": !page.diffSettling, "line": done || line >= 0 }))
                return
            if (done) {
                const staged = op.staged()
                op.complete(() => "line_run staged=" + staged
                                  + " want=" + op.want
                                  + " rows=" + diffPane.view.count)
                return
            }
            op.input(op.order[op.staged()], () => {
                diffPane.stageLine(0, line)
            })
        }
    }
    // diff-escape, read once the pane has left the screen: the swap is the layout's, and in the key's own turn the page
    // still holds the keyboard through the outgoing pane. `kept=`: the page is a focus scope, so it keeps the keyboard
    // and the next Escape lands (`tests/qml/tst_escape.qml`).
    PageStage {
        id: escapeTimer
        name: "diff_escape"
        /// Whether the rows held the keyboard once the hand was in, and whether the page took the key — read as the
        /// key went in (`stageRowTimer`).
        property bool hand: false
        property bool took: false
        onTriggered: {
            if (!Awaited.all("diff_escape_ready", { "ready": !(page.diffShown || diffPane.view.visible) }))
                return
            escapeTimer.complete(() => "diff_escape took=" + escapeTimer.took
                                        + " shown=" + page.diffShown
                                        + " key=" + (page.diffKey === "" ? "empty" : "held")
                                        + " folded=" + page.sidebarCollapsed
                                        + " hand=" + escapeTimer.hand
                                        + " kept=" + page.activeFocus
                                        // One item holds active focus, so this also says nothing else kept it: the
                                        // arrows now reach `GraphPane.stepRow`.
                                        + " graph=" + graphPane.view.activeFocus)
        }
    }
    // diff-follow: where the pane lands when its file is moved whole from the file list — the next file on that side,
    // or the same file on its new side when none is left (デザイン規約 §diff の中のステージ).
    PageStage {
        id: followTimer
        name: "diff_follow"
        order: ["moved_file"]
        property string was: ""
        property bool alone: false
        onTriggered: {
            const op = followTimer
            if (op.at("start")) {
                if (!Awaited.all("diff_follow_rows", { "rows": diffPane.view.count > 0 }))
                    return
                op.input("moved_file", () => {
                    op.was = page.diffKey
                    // Read before the write takes the file off this side.
                    op.alone = unstagedModel.besidePath(page.diffKind, page.diffPath) === ""
                    // The file list's own `+` / `−` path.
                    return driver.pressWrite("follow:" + page.diffKind, () => {
                        if (page.diffKind === "staged")
                            repoTab.unstagePath(page.diffPath)
                        else
                            repoTab.stagePath(page.diffPath)
                        return true
                    })
                })
                return
            }
            // The landing is the output: off its old key, with rows (or closed), and done rereading.
            const now = page.diffKey
            if (!Awaited.all("diff_follow_landed", {
                "written": acts.wroteAndSettled(),
                "moved": now !== op.was,
                "rows": !(page.diffShown && diffPane.view.count === 0),
                "page": PageSettled.settled(page) }))
                return
            op.complete(() => "diff_follow shown=" + page.diffShown
                              + " alone=" + op.alone
                              + " was=" + op.was
                              + " landed=" + page.diffKey)
        }
    }
    // code-send: half by the bar's own path, the rest by the hand that carries the rows too. Reads where the code and
    // rows ended up — a bar bound to nothing still takes a press, a hand wired to nothing still starts. Waits for
    // `viewLaidOut`; nowhere sideways to go is said and completed from `start`, which is why no `order` is declared
    // (rules-refs/app-ui.md「欠けた前提は言って止まる」).
    PageStage {
        id: codeSendTimer
        name: "code_send"
        function line() {
            return "code_send bar=" + diffPane.codeBarShown
                   + " hand=" + diffPane.codeHandOn
                   + " at=" + Math.round(diffPane.codeAt)
                   + " max=" + Math.round(diffPane.codeMax)
                   + " measured=" + Math.round(diffPane.codeMeasured)
                   + " drawn=" + Math.round(diffPane.codeDrawn)
                   + " down=" + Math.round(diffPane.view.contentY)
        }
        onTriggered: {
            const op = codeSendTimer
            if (op.at("start")) {
                if (!Awaited.all("code_send_laid", { "laid": acts.viewLaidOut() }))
                    return
                if (diffPane.codeMax <= 0) {
                    op.complete(() => op.line())
                    return
                }
                // The anchor's ring is part of the picture, so the hand is left running for the shot.
                op.input("sent", () => {
                    diffPane.sendCode(diffPane.codeMax / 2)
                    const mid = Qt.point(diffPane.view.width / 2, diffPane.view.height / 2)
                    diffPane.startCodeHand(mid)
                    diffPane.driftCodeHand(Qt.point(mid.x + 120, mid.y + 120))
                })
                return
            }
            // Both of the hand's axes must be seen moving: rows down, code past the bar's half.
            if (!Awaited.all("code_send_moved", {
                "moved": !(diffPane.view.contentY <= 0 || diffPane.codeAt <= diffPane.codeMax / 2) }))
                return
            op.complete(() => op.line())
        }
    }
    // The reach as one string (measured, drawn, widest row), held until it stands across a frame: delegates are built a
    // layout after the jump, and a `Text` answers at the family's advance until its fallback font resolves
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
    // The send is last, so the shot stands at the diff's end, the only frame where room past the text would show. A
    // diff that fits its frame completes from `start`, which is why no `order` is declared.
    PageStage {
        id: codeGrowTimer
        name: "code_grow"
        property real measured: 0
        property real drawn0: 0
        property real drawn: 0
        property real back: 0
        property real again: 0
        property int row: -1
        property int rowAgain: -1
        /// Text end and frame edge at the far end, read once the ink stood there.
        property real ink: 0
        property real room: 0
        /// Whether the row jumped to is built and the widths have stood across a frame since the jump.
        function arrived(atRow) {
            return !!diffPane.view.itemAtIndex(atRow) && codeGrowTimer.still("reach", acts.reachNow())
        }
        /// The gap past the text at the far end. The pane holds exactly `Theme.spaceSm` there, so more is a reach past
        /// every line and less one that stopped short.
        readonly property real tail: codeGrowTimer.room - codeGrowTimer.ink
        function line() {
            const grew = codeGrowTimer.drawn > codeGrowTimer.drawn0
            const past = codeGrowTimer.drawn > codeGrowTimer.measured
            const kept = codeGrowTimer.row >= 0
                    && codeGrowTimer.back === codeGrowTimer.drawn
                    && codeGrowTimer.again === codeGrowTimer.drawn
                    && codeGrowTimer.rowAgain === codeGrowTimer.row
            // A pixel of slack: the send's gutter and the row's are separate items that round independently.
            const ends = codeGrowTimer.ink > 0
                    && Math.abs(codeGrowTimer.tail - Theme.spaceSm) <= 1
            return "code_grow grew=" + grew + " past=" + past + " kept=" + kept + " ends=" + ends
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
                   + " down=" + Math.round(diffPane.view.maxY)
        }
        onTriggered: {
            const op = codeGrowTimer
            const foot = diffPane.view.count - 1
            if (op.at("start")) {
                if (!Awaited.all("code_grow_laid", { "laid": acts.viewLaidOut() }))
                    return
                // A diff that fits its frame has no row to arrive from below: said and completed
                // (rules-refs/app-ui.md「欠けた前提は言って止まる」).
                if (diffPane.view.maxY <= 0) {
                    op.complete(() => op.line())
                    return
                }
                op.input("foot_sent", () => {
                    op.measured = diffPane.codeMeasured
                    op.drawn0 = diffPane.codeDrawn
                    diffPane.scrollTo(diffPane.view.maxY)
                })
                return
            }
            if (op.at("foot_sent")) {
                if (!Awaited.all("code_grow_foot", { "arrived": op.arrived(foot) }))
                    return
                op.input("top_sent", () => {
                    op.drawn = diffPane.codeDrawn
                    op.row = diffPane.codeWidestRow
                    diffPane.scrollTo(0)
                })
                return
            }
            if (op.at("top_sent")) {
                if (!Awaited.all("code_grow_top", { "arrived": op.arrived(0) }))
                    return
                op.input("foot_again_sent", () => {
                    op.back = diffPane.codeDrawn
                    diffPane.scrollTo(diffPane.view.maxY)
                })
                return
            }
            if (op.at("foot_again_sent")) {
                if (!Awaited.all("code_grow_foot_again", { "arrived": op.arrived(foot) }))
                    return
                // All the way right with the widest row on screen — otherwise a short line in view answers `ends=`.
                op.input("far_sent", () => {
                    op.again = diffPane.codeDrawn
                    op.rowAgain = diffPane.codeWidestRow
                    diffPane.sendCode(diffPane.codeMax)
                })
                return
            }
            // The send is immediate, but the rows are redrawn a layout later: wait for the ink to stand still.
            const ink = diffPane.codeInkRight()
            const now = Math.round(diffPane.codeAt) + "," + Math.round(ink)
            if (!Awaited.all("code_grow_reach", { "still": op.still("code_grow_reach", now) }))
                return
            op.ink = ink
            op.room = diffPane.view.width
            op.complete(() => op.line())
        }
    }
    // code-shrink: the same file reread with its widest line staged out (`--preset widelate`'s `top.txt`, whose first
    // changed row is that line). At risk is the place along the row: a reach answering zero between the two readings,
    // or latching its largest value, loses it. `kept=` the place, `shrank=` less room, `narrower=` both halves of the
    // reach came down. Nowhere sideways to go completes from `start`, which is why no `order` is declared.
    PageStage {
        id: codeShrinkTimer
        name: "code_shrink"
        /// The first hunk's line that gets staged — the file's widest.
        property int changedLine: -1
        property real at0: 0
        property real max0: 0
        property real measured0: 0
        property real drawn0: 0
        property int rows0: 0
        function line() {
            const kept = codeShrinkTimer.at0 > 0
                    && Math.round(diffPane.codeAt) === Math.round(codeShrinkTimer.at0)
            const shrank = diffPane.codeMax < codeShrinkTimer.max0
            const narrower = diffPane.codeMeasured < codeShrinkTimer.measured0
                    && diffPane.codeDrawn < codeShrinkTimer.drawn0
            return "code_shrink kept=" + kept + " shrank=" + shrank + " narrower=" + narrower
                   + " at0=" + Math.round(codeShrinkTimer.at0)
                   + " at=" + Math.round(diffPane.codeAt)
                   + " max0=" + Math.round(codeShrinkTimer.max0)
                   + " max=" + Math.round(diffPane.codeMax)
                   + " measured0=" + Math.round(codeShrinkTimer.measured0)
                   + " measured=" + Math.round(diffPane.codeMeasured)
                   + " drawn0=" + Math.round(codeShrinkTimer.drawn0)
                   + " drawn=" + Math.round(diffPane.codeDrawn)
                   + " rows0=" + codeShrinkTimer.rows0
                   + " rows=" + diffPane.view.count
        }
        onTriggered: {
            const op = codeShrinkTimer
            if (op.at("start")) {
                if (!Awaited.all("code_shrink_laid", { "laid": acts.viewLaidOut() }))
                    return
                // Nowhere sideways to go is no place to keep.
                if (diffPane.codeMax <= 0) {
                    op.complete(() => op.line())
                    return
                }
                op.input("wrote", () => {
                    // An eighth along: short of where the shorter reading ends, so `kept=` means kept, not clamped.
                    diffPane.sendCode(diffPane.codeMax / 8)
                    op.at0 = diffPane.codeAt
                    op.max0 = diffPane.codeMax
                    op.measured0 = diffPane.codeMeasured
                    op.drawn0 = diffPane.codeDrawn
                    op.rows0 = diffPane.view.count
                    return driver.pressWrite("stage-line", () => {
                        diffPane.stageLine(0, op.changedLine)
                        return true
                    })
                })
                return
            }
            // The write, the reread, then the reread rows' widths standing still.
            const written = acts.wroteAndSettled()
            const rows = diffPane.view.count !== 0
            if (!Awaited.all("code_shrink_reach", {
                "written": written,
                "rows": rows,
                "still": written && rows && op.still("code_shrink_reach", acts.reachNow()) }))
                return
            op.complete(() => op.line())
        }
    }
    // code-swap: switching file drops both place and width. The second file is the row beside the first in the list
    // (`NavSectionModel.beside_path`). `room=` makes `dropped=` mean something — a file with nowhere sideways to go
    // sits at its left edge regardless (`--preset widelate`'s `plain.txt` is narrower yet still wider than the pane).
    // Nowhere sideways to go, or no file beside, completes before the swap, which is why no `order` is declared.
    PageStage {
        id: codeSwapTimer
        name: "code_swap"
        property string was: ""
        /// The file swapped to, as it was asked for.
        property string landed: ""
        property real at0: 0
        property real max0: 0
        property real drawn0: 0
        function line() {
            const dropped = codeSwapTimer.at0 > 0 && Math.round(diffPane.codeAt) === 0
            const room = diffPane.codeMax > 0
            const narrower = diffPane.codeDrawn > 0 && diffPane.codeDrawn < codeSwapTimer.drawn0
            return "code_swap dropped=" + dropped + " room=" + room + " narrower=" + narrower
                   + " at0=" + Math.round(codeSwapTimer.at0)
                   + " at=" + Math.round(diffPane.codeAt)
                   + " max0=" + Math.round(codeSwapTimer.max0)
                   + " max=" + Math.round(diffPane.codeMax)
                   + " drawn0=" + Math.round(codeSwapTimer.drawn0)
                   + " measured=" + Math.round(diffPane.codeMeasured)
                   + " drawn=" + Math.round(diffPane.codeDrawn)
                   + " was=" + codeSwapTimer.was
                   + " landed=" + codeSwapTimer.landed
        }
        onTriggered: {
            const op = codeSwapTimer
            if (op.at("start")) {
                if (!Awaited.all("code_swap_laid", { "laid": acts.viewLaidOut() }))
                    return
                if (diffPane.codeMax <= 0) {
                    op.complete(() => op.line())
                    return
                }
                op.input("sent", () => {
                    diffPane.sendCode(diffPane.codeMax / 2)
                    op.at0 = diffPane.codeAt
                    op.max0 = diffPane.codeMax
                    op.drawn0 = diffPane.codeDrawn
                    op.was = page.diffKey
                })
                // `<bucket>:<path>` of the row beside this one under the same heading; none ends the run here.
                const beside = unstagedModel.besidePath(page.diffKind, page.diffPath)
                if (beside === "") {
                    op.complete(() => op.line())
                    return
                }
                op.input("swapped", () => {
                    const cut = beside.indexOf(":")
                    const path = beside.substring(cut + 1)
                    op.landed = beside
                    page.toggleDiff(beside.substring(0, cut), path, unstagedModel.origOf(path))
                })
                return
            }
            // On the other file, done reading and laid out, before its width counts.
            const there = page.diffKey === op.landed
            const read = !page.diffSettling
            const laid = acts.viewLaidOut()
            if (!Awaited.all("code_swap_reach", {
                "landed": there,
                "read": read,
                "laid": laid,
                "still": there && read && laid && op.still("code_swap_reach", acts.reachNow()) }))
                return
            op.complete(() => op.line())
        }
    }
    // diff-sweep: the text taken from the ground under a short file's last row (規約 §diff の中身をコピーする), shaped
    // like `details-sweep`. `reach=` sweeps from nine starts, each judged over a cleared selection — one start (the
    // middle hides it) or one read at the end would pass a partial reach. `ours=`: a press on a hunk heading's words
    // stays the hunk's (規約 §diff の中のステージ).
    PageStage {
        id: diffSweepTimer
        name: "diff_sweep"
        order: ["swept"]
        /// What the sweeps and the heading's press found, read as each landed.
        property var found: null
        onTriggered: {
            const op = diffSweepTimer
            // Settle: the view lays the rows out a frame after they arrive, and ground about to become a row is
            // neither.
            const hand = diffPane.textHand
            if (!Awaited.all("diff_sweep_ready", { "ready": !(diffPane.view.count <= 0) }))
                return
            const geom = Math.round(hand.groundTop) + "," + diffPane.view.count
            if (!op.still("diff_sweep_geom", geom))
                return
            op.input("swept", () => {
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
                op.found = { "reach": reach, "tries": tries, "hunkRow": hunkRow, "ours": ours }
            })
            // `ground=false` is a fixture with nowhere to sweep from. `ours=false` is the pass, as in `details_sweep`.
            op.complete(() => "diff_sweep reach=" + op.found.reach + "/" + op.found.tries
                              + " ground=" + diffPane.textHand.hasGround
                              + " ours=" + op.found.ours
                              + " hunkRow=" + op.found.hunkRow
                              + " " + diffPane.pickTally())
        }
    }
    // keep-place: read part way down a long diff, then write — the file read again must leave the view on the same
    // place: the line staged above it goes, and the rows on screen stay where they stand (`DiffModel.layOutKept`, Qt
    // keeping the rows on screen as rows above them go). Waits for `viewLaidOut` (before it the scroll goes nowhere); a
    // laid-out diff with less than `readY` of room is said and completed from `start`, which is why no `order` is
    // declared (rules-refs/app-ui.md「欠けた前提は言って止まる」).
    PageStage {
        id: keepPlaceTimer
        name: "keep_place"
        /// The first hunk's line whose staging reads the diff again under the reader.
        property int changedLine: -1
        /// The file's reads (`DiffModel.rowsArrived`), counted by the connection below, and the count as the write
        /// went in.
        property int reads: 0
        property int readsAtWrite: 0
        onTriggered: {
            const op = keepPlaceTimer
            if (op.at("start")) {
                if (!Awaited.all("keep_place_laid", { "laid": acts.viewLaidOut() }))
                    return
                if (diffPane.view.maxY < acts.readY) {
                    op.complete(() => acts.placeLine())
                    return
                }
                op.input("wrote", () => {
                    diffPane.scrollTo(acts.readY)
                    op.readsAtWrite = op.reads
                    return driver.pressWrite("stage-selection", () => {
                        page.stageSelection(0, op.changedLine)
                        return true
                    })
                })
                return
            }
            // Then the file read again and weighed by the pane. A write that emptied this side never lands, and the
            // run waits out the watchdog.
            if (!Awaited.all("keep_place_read", { "written": driver.wroteAndSettled(),
                                                  "read": op.reads > op.readsAtWrite,
                                                  "kept": !diffPane.rowKeep.busy }))
                return
            op.complete(() => acts.placeLine())
        }
    }
    Connections {
        target: keepPlaceTimer.running ? diffPane.diffModel : null
        function onRowsArrived() {
            keepPlaceTimer.reads++
        }
    }
    // colour-place: a reader scrolled before the colours land. They rewrite every row (`DiffModel::repaint_rows`),
    // which must cost neither the place nor the width. `held=`: rows are markup either way (`markup::styled`), so the
    // colour tags are neither drawn nor measured and `rows_gen` does not move — a row measured in another format reads
    // its `<font …>` tags as letters, and a reading counted as new drops every filed width.
    PageStage {
        id: colourPlaceTimer
        name: "colour_place"
        order: ["scrolled", "coloured"]
        /// When the dispatch asked for the file (`PerfProbe.clockMs`), for the report's `waited=` — a diagnosis,
        /// compared with nothing.
        property real since: 0
        property real measuredWas: 0
        property real drawnWas: 0
        /// Keeps the reach as the reading before. Also taken at the scroll: a small diff can be coloured within one
        /// look, and a row that answered `firstChangedLine` is already built and measured.
        function look() {
            colourPlaceTimer.measuredWas = diffPane.codeMeasured
            colourPlaceTimer.drawnWas = diffPane.codeDrawn
        }
        onTriggered: {
            const op = colourPlaceTimer
            if (op.at("start")) {
                // The open again while the pane is not reading this file: asked once and lost, nothing else brings
                // the rows.
                if (!page.diffShown || page.diffKey !== "unstaged:" + Harness.autoActArg)
                    page.openDiff("unstaged", Harness.autoActArg, "")
                // Scroll as soon as the rows are there, before the colours.
                if (!Awaited.all("colour_place", { "rows": diffPane.firstChangedLine(0) >= 0 }))
                    return
                op.input("scrolled", () => {
                    diffPane.scrollTo(acts.readY)
                    op.look()
                })
                return
            }
            if (op.at("scrolled")) {
                if (!Awaited.all("colour_place_coloured", { "coloured": diffPane.diffModel.coloured })) {
                    op.look()
                    return
                }
                op.advance("coloured")
            }
            // The rows are rewritten as the colours land: read once the reach has stood across a frame since.
            if (!Awaited.all("colour_place_still", { "still": op.still("colour_place_reach", acts.reachNow()) }))
                return
            const waited = Math.round(PerfProbe.clockMs() - op.since)
            op.complete(() => {
                const held = op.drawnWas > 0
                        && Math.round(diffPane.codeMeasured) === Math.round(op.measuredWas)
                        && Math.round(diffPane.codeDrawn) === Math.round(op.drawnWas)
                return "colour_place coloured=" + diffPane.diffModel.coloured
                       + " at=" + Math.round(diffPane.view.contentY)
                       + " held=" + held
                       + " rows=" + diffPane.view.count
                       + " waited=" + waited
                       + " measured0=" + Math.round(op.measuredWas)
                       + " measured=" + Math.round(diffPane.codeMeasured)
                       + " drawn0=" + Math.round(op.drawnWas)
                       + " drawn=" + Math.round(diffPane.codeDrawn)
            })
        }
    }
}

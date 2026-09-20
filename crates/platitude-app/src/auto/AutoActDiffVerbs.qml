pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The diff pane's verbs. Most of them wait on the same thing first — a row of the first hunk being on screen —
/// so the wait is made once, below, and the verb it belongs to is chosen from there. The rest are chains of
/// their own: the ones that read a place, write and read it again, or send the code
/// sideways. The dispatch starts those by name.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktreeModel: driver.worktreeModel
    readonly property var diffPane: driver.diffPane
    readonly property var graphPane: driver.graphPane
    readonly property var diffRowMenu: driver.diffRowMenu
    readonly property var renderedBarrier: driver.barrierRendered
    readonly property var writeBarrier: driver.barrierWrite

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — each verb is named by one (`AutoActDriver`).
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
            // All enter through one file's diff and act on its first hunk. The bucket rides in front of the path
            // (`<bucket>:<path>`) when it is not the usual unstaged one: an untracked file has no unstaged diff at all,
            // a conflicted one is read from `conflicts`. Only the bucket names count as one, so a path carrying a colon
            // still opens.
            const cut = arg.indexOf(":")
            const head = cut > 0 ? arg.substring(0, cut) : ""
            const named = head === "staged" || head === "unstaged"
                          || head === "untracked" || head === "conflicts"
            page.showWip()
            // The source of a rename comes off the model: a row hands it over when it
            // is clicked, and a run that opened the destination alone would photograph a file git thinks appeared out
            // of nowhere.
            const wtPath = named ? arg.substring(cut + 1) : arg
            // A diff is of a file, and every way to open one on screen carries its name — so an argument that carries
            // none is the run's own mistake, and it has to read as one. **git will not call it an error**: an empty
            // pathspec matches the whole tree, so the pane fills with a diff that reads exactly like the file's, and a
            // fixture with one changed file in it renders down to the same rows. It parts company at the first write:
            // the tree moves, the list is asked about a path it never held, and the pane closes — correctly — on a
            // reader who was never on a file. What that leaves is a verb reporting rows it never owned, hours after
            // the argument it wanted was left off (measured, `line-run` staged one line of nothing and read
            // `rows=0`, `line-back` waited out the watchdog).
            if (wtPath === "") {
                Harness.report("diff_arg act=" + act + " named=false")
                renderedBarrier.begin()
                // A known verb answers true even on this early way out —
                // falsy would read as "not mine" to the dispatch chain.
                return true
            }
            page.toggleDiff(named ? head : "unstaged", wtPath,
                            worktreeModel.origOf(wtPath))
            stageRowTimer.begin()
        } else if (act === "preview" || act === "preview-unstaged" || act === "preview-staged"
                   || act === "preview-close") {
            // The toggle only asks; the read is a git subprocess away, so completion is the pane settling
            // (`stageRowTimer`). No path is a run with nothing to open: said and stopped on the rendered
            // surface (no read would answer a wait) — the wanted line is what fails it.
            // "preview-close" opens the unstaged side the same way and closes the pane once the pictures are there.
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

    /// The drag the four text verbs share, and what each of them does with it
    /// (デザイン規約 §diff の中身をコピーする). Hover and a real button cannot be injected, so this enters the same three
    /// functions the hand's own handlers call (`DiffTextSelect`).
    ///
    /// **The stretch is chosen to hold both sides**: from the head of the first line down past the first removed one,
    /// so the wash has something to be on and the menu's second word has something to take. A drag that reached only
    /// one side would prove half the rule and read as a pass.
    function pickDiffText(act) {
        const removedRow = diffPane.firstRemovedRow()
        const lastRow = Math.min(diffPane.view.count - 1, removedRow + 1)
        diffPane.pickText(0, 1, 0, lastRow, driver.pastLineEnd)
        // `door=` is the half a headless drag cannot make for itself: a run enters the hand's own functions and never
        // delivers a press, so what brings the keyboard — the sheet the pane watches presses from — is asked where it
        // stands. A selection nothing holds the keyboard for is not one `Ctrl+C` can take away.
        Harness.report("diff_pick door=" + diffPane.doorOnTop() + " " + diffPane.pickTally()
                          + " removedRow=" + removedRow + " to=" + lastRow)
        if (act === "diff-select")
            return
        if (act === "diff-copy") {
            diffPane.copySelection()
            acts.reportCopy("diff_copy")
            return
        }
        // The right-click lands inside what was just dragged, so the selection stands as it is — which is the half of
        // the rule this verb is about (the other half is a click outside it, and that one takes its own row).
        diffPane.askCodeMenu(0, removedRow, 0)
        if (act === "diff-copy-removed") {
            diffRowMenu.menu.close()
            diffRowMenu.copyRemovedNow()
            acts.reportCopy("diff_copy_removed")
        }
    }
    /// What reached the clipboard, read back off the pad the copy goes through (`ClipboardHelper.lastCopied`) — the
    /// clipboard itself will not say. `holdsRemoved=` is the whole claim of both verbs, from opposite sides: the plain
    /// copy left the old line out, and the menu's second row is exactly it.
    function reportCopy(name) {
        const text = driver.clipboard.lastCopied
        const removed = diffPane.diffModel.removedText()
        Harness.report(name + " holdsRemoved=" + (removed !== "" && text.indexOf(removed) >= 0)
                          + " lines=" + (text === "" ? 0 : text.split("\n").length))
    }
    // The diff has to arrive before a row of it can be staged. Asked for: a fixed wait
    // photographs an empty pane the same as a late one (measured, a verb fired against this repository named no
    // row and passed). The asking has no ceiling: a row that never lands leaves the run without a report line at all,
    // and the watchdog is what ends it.
    //
    // A beat of its own, because what it asks costs a diff read per tick.
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
        // Whether what the verb is about to name is on screen. They all act on the first hunk, so a changed line in it
        // is the one answer they share — "diff-file" alone reads the model, and the pictures and
        // binary files it also opens have no rows to find.
        function ready() {
            // "diff-file" alone reads the model, and "conflict-sides" reads every row there is — the
            // one it is about (a side's own line, once it has been typed over) is a removal, which is not a changed
            // line of the first hunk. The previews take the settled form too: what they open can be all picture or
            // binary notice and no rows, and no row of it is theirs to name.
            // "diff-band-sweep" is about the band above the rows, so the settled diff is
            // the whole of what it waits for — a file with no changed line in its first hunk still has a path in the
            // band, and demanding one would leave that run waiting out its watchdog.
            if (["diff-file", "conflict-sides", "diff-tick", "diff-band-sweep",
                 "diff-escape"].indexOf(Harness.autoAct) >= 0)
                return diffPane.diffSettled()
            // The previews wait for the pictures as well as the read: the decode is asynchronous, and a pane
            // photographed between the two shows an empty frame under a caption.
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
            // The line the line-level verbs mean, the first changed: a hunk numbers its lines through the context
            // it carries, which is not part of the change (see `firstChangedLine`).
            const line = diffPane.firstChangedLine(0)
            // Said before the acting, so a verb that goes on to fail its write says both. `waited=` is a count of
            // ticks.
            Harness.report("diff_row act=" + act + " ready=" + arrived
                              + " rows=" + diffPane.view.count
                              + " line=" + line
                              + " waited=" + stageRowTimer.waited)
            // The diff is the shot; the line endings get a report line of their own (a picture cannot say which of the
            // four kinds the pane decided on).
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
            // The band's own air, once the pane has settled on a file to name in it. The path there is the same one
            // the list's row hands over from its hover, and this band is where the eye already is while the diff is
            // being read (規約 §右のペインの字は掴める). `cut=` is the half the picture cannot answer on a wide pane:
            // whether the field is holding a value longer than the band, which is where the head-side cut is decided.
            if (act === "diff-band-sweep") {
                Harness.report("diff_band_sweep "
                    + diffPane.headerHand.sweepAir(7, "cut=" + diffPane.headerCut))
                driver.complete()
                return
            }
            // Which rows the two sides are named on. The bands themselves are in the picture, but "how many rows
            // should have carried one" is not — and a resolved conflict is exactly where none of them did.
            if (act === "conflict-sides") {
                Harness.report("conflict_sides " + diffPane.sideTally())
                driver.complete()
                return
            }
            // The page's tick, fired here. What it asks about is a file this run has not
            // touched, so core answers it with nothing (`RepoSession::refresh_diff`) and there is no arrival to
            // observe — the ask is the edge, and it is read in the same beat it is made. `loading=` is the other half
            // of the claim: a tick leaves the pane standing where a click would reload it.
            if (act === "diff-tick") {
                const asked = page.pollDiff()
                Harness.report("diff_tick asked=" + asked
                                  + " loading=" + diffPane.diffModel.loading
                                  + " rows=" + diffPane.view.count)
                driver.complete()
                return
            }
            // A preview's settled form — rows, a picture, or a binary notice — is the shot; `kind=` is said because
            // the picture cannot say it (a pane the read never reached photographs as the same black under the same
            // DIFF header).
            // `pictures=` is how many of the sides decoded — a `file:` URL that names nothing photographs as the
            // same caption over the same empty frame as one that was never handed over.
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
                // The pane closed through the same door the header's `✕` goes through. What the run is for is
                // outside it — whether the decoded pictures went with the URLs — and the outside reads the process;
                // the line says the pane let go of them (`url=empty`), which is the whole of what QML can say.
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
            // Escape over a settled diff. The key goes to the page's own handler, which is the last thing an
            // Escape nobody else claimed reaches (`RepoPage.escapePressed`, `tests/qml/tst_escape.qml`) — and the
            // one line `Keys.onEscapePressed` is made of, so a run entering here enters where the key does
            // (verify-ui「注入はハンドラ本体そのものへ入れる」). A keystroke cannot be injected.
            //
            // **`took=` and `shown=` are both said**: a handler that answered `false` and a pane that stayed open are
            // different failures, and the picture — the graph, back where it was — cannot tell either from a diff
            // that was never opened.
            if (act === "diff-escape") {
                // **The hand is sent into the pane first**, by the door the wheel uses (`DiffPane.handArrived`):
                // Escape from a file list is the easy half, and the half that breaks is this one — the pane it
                // closes is the pane holding the keyboard, and where that keyboard lands is what decides whether a
                // second Escape reaches anything at all.
                diffPane.handArrived()
                escapeTimer.hand = diffPane.view.activeFocus
                escapeTimer.took = page.escapePressed()
                escapeTimer.start()
                return
            }
            // The squares a line only puts out under the pointer, named (hover cannot be injected
            // on Windows).
            if (act === "line-tools") {
                diffPane.showLineTools(0, line)
                renderedBarrier.begin()
                return
            }
            // The heading's two words carry their colours only under the pointer, and hover cannot be injected, so the
            // row is named. A heading's own row is line -1 (`flatten_patches`).
            if (act === "hunk-tools") {
                diffPane.showLineTools(0, -1)
                renderedBarrier.begin()
                return
            }
            // The text picked out of the rows, and what the two ways of copying it hand over
            // (デザイン規約 §diff の中身をコピーする). All four drag over the same stretch: from the head of the first line to
            // past the end of the first removed one, so the selection is guaranteed to hold both sides — the new one
            // to wash and the old one for the menu's second word.
            if (act === "diff-select" || act === "diff-copy"
                || act === "diff-menu" || act === "diff-copy-removed") {
                acts.pickDiffText(act)
                renderedBarrier.begin()
                return
            }
            // The same text, taken from the ground under the last row — the one
            // place inside the code column where a press used to reach nothing (規約 §diff の中身をコピーする). The
            // rows have arrived by here; the view still has to lay them out, which is what the timer waits for.
            if (act === "diff-sweep") {
                diffSweepTimer.start()
                return
            }
            // The strip the list's own bar stands on, which the hand over the rows gives back (`AppListView.barRoom`).
            // The rows have arrived by here; the bar comes out once the view has laid them out, which is what the
            // timer waits for.
            if (act === "diff-bar") {
                diffBarTimer.start()
                return
            }
            // The blank right of a row's last character, which is the row's end and no place inside it. The rows
            // have arrived by here; where each of them was drawn to is known once the view has laid them out, which
            // is what the timer waits for.
            if (act === "diff-blank") {
                diffBlankTimer.start()
                return
            }
            // Reading part way down a long diff and then writing: the rebuild has to come back to the same place.
            if (act === "keep-place") {
                keepPlaceTimer.begin(line)
                return
            }
            if (act === "stage-hunk" || act === "stage-line") {
                // One line goes through its own mark — the press writes, there and then — and a hunk through its
                // heading's word.
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
            // Sending the code sideways, and the hand that sends it and the rows at once. Both read what moved rather
            // than what was asked for: a bar bound to nothing still takes a press.
            if (act === "code-send") {
                codeSendTimer.begin()
                return
            }
            // The three about where that width comes from and what it belongs to: a row nothing picked arriving from
            // below, the same file read again with its widest line taken out of it, and another file entirely.
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
            // No line-level discard exists — a hunk is the smallest piece that can be thrown away. The hold's end is
            // the press the write barrier is armed on (`holdToEnd`), said by the pane on the hunk's own signal.
            if (act === "discard-hunk-go") {
                driver.holdToEnd(diffPane)
                stagedTimer.start()
            } else {
                renderedBarrier.begin()
            }
        }
    }
    // The three presses that write from inside the pane: a line, a hunk, and a hunk thrown away. Each empties the
    // side it was made on, and an emptied side is the one thing the pane moves off by itself (`diff-follow` is that
    // move as a verb). **What the move brings is what the run shows**, so the write's answer is not the end:
    // stopped there, the picture is of a pane wearing the name of the file it followed to over an empty body, and the
    // census walk of that moment leaves `DiffRowDelegate` off the run's line — what it moved to lands after both.
    SampleTimer {
        id: stagedTimer
        onTriggered: {
            // The write's own edges, the pane's word that it has stopped reading (`RepoPage.diffSettling`), and — if
            // it is showing anything at all — something to look at. A pane that closed behind the press shows nothing
            // and is done. **What the pane lands on need not be rows**: a picture, a binary file and a repository of
            // its own have none by nature and never will (`DiffPane.diffSettled`), so asking for rows holds those to
            // the watchdog — an emptied bucket can leave any of them next in the list.
            if (!driver.wroteAndSettled() || page.diffSettling
                    || (page.diffShown && !(diffPane.diffSettled() && diffPane.picturesSettled)))
                return
            stagedTimer.stop()
            // Handed to the write's own barrier, which answers for the rest of it — the broken contract said by name,
            // the status a verb owes, the row a write takes out of its bucket. It is already owed nothing but a tick.
            writeBarrier.start()
        }
    }
    // PGG_AUTO_ACT=diff-bar: the bar down the side of the diff can still be grabbed — the strip it stands on is not
    // taken by the hand laid over the rows (`AppListView.barRoom`). The bar is drawn *over* the rows, so anything
    // covering the frame covers the bar with it, and the hand that picks the text out took every press on the
    // trough. Two claims:
    //
    // **`clear=`** — where this hand ends against where the bar begins, read off the two items
    // themselves. **`reach=`** — the hand still answers at its own last pixel, so the strip was given back
    // to the bar. And `out=` is the run's own honesty: a diff that fits its frame has
    // no bar to be kept clear of.
    SampleTimer {
        id: diffBarTimer
        /// What the view looked like at the previous sample, for the settle below.
        property string lastGeom: ""
        onTriggered: {
            // The rows arrive a frame ahead of the view that lays them out, and a bar that is not out yet is a bar
            // this cannot say anything about (`diff-sweep`, the same wait).
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

    // The blank right of a row's last character, where a drag has to select nothing and a wash has to stop. The hand
    // is entered through its own two functions with **pixels**, which is the half the four text verbs cannot reach —
    // those hand over byte offsets and would pass with any answer at all to "which byte is under this point"
    // (`pickDiffText`).
    //
    // Three claims, and each answers something the other two cannot:
    //
    //  - `quiet=` — a drag that starts and ends right of a row's ink selects nothing. The failure was here.
    //  - `clamped=` — and a drag that starts on the row and ends out there takes the line and exactly the line. Said
    //    against the same row selected by number (`DiffModel.selectRow`), so the two ways in have to agree; `quiet=`
    //    alone would go green on a hand that had stopped selecting altogether.
    //  - `sent=` — both again after the code has been sent sideways, where an offset counted from the wrong place
    //    lands every press somewhere else (`DiffCodeScroll.offset`).
    //
    // `rows=` is the run's own honesty: a fixture whose lines all run past the pane leaves nowhere to press, and
    // three trues off none of them say nothing (`diff_sweep ground=`, the same rule).
    SampleTimer {
        id: diffBlankTimer
        /// What the view looked like at the previous sample, for the settle below.
        property string lastGeom: ""
        onTriggered: {
            // The rows arrive a frame ahead of the view that lays them out, and a row that has not been laid out
            // has not been drawn to any width (`diff-sweep`, the same wait).
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
            // Half of the shortest line that was reachable, so every row this just pressed beside still ends inside
            // the pane afterwards — sending as far as the longest line goes would carry all of them off the left and
            // leave the second pass with nothing to say.
            diffPane.sendCode(Math.min(diffPane.codeMax, before.shortest / 2))
            const after = acts.blankTally(false)
            // The drags above end by selecting nothing, which is the claim and photographs as an empty pane. So one
            // more is left standing for the camera: out past a row's last character, where the band has to stop.
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
    /// Every row whose line ends inside the pane, dragged in beside it: once entirely in the blank right of it, and —
    /// while the head of the line is still on screen (`fromHead`) — once from that head out into the blank. Answers
    /// how many rows were reachable, the shortest line among them, and whether the drags said what they have to.
    /// **A pass that found no row says `rows=0`.**
    ///
    /// It enters `takeAt` / `followAt` / `releaseText`, which is what the `MouseArea`'s own handlers call: a hand
    /// that was never wired up reports nothing (verify-ui §壊れない動詞の実装).
    function blankTally(fromHead) {
        const hand = diffPane.textHand
        let rows = 0
        let shortest = 0
        let quiet = true
        let clamped = true
        let backwards = true
        for (let i = 0; i < diffPane.view.count; i++) {
            const item = diffPane.view.itemAtIndex(i)
            // The lines the plain copy takes, so that what is dragged out can be held against what the row holds.
            if (!item || (item.kind !== "ctx" && item.kind !== "add") || item.codeInk <= 0)
                continue
            // Where this row's own line ends, in the hand's coordinates: the code travels under the pane.
            const ends = item.codeInk - diffPane.codeAt
            const y = item.y - diffPane.view.contentY + item.height / 2
            if (ends < 1 || ends + 4 >= hand.width || y < 0 || y >= hand.height)
                continue
            rows++
            shortest = shortest === 0 ? item.codeInk : Math.min(shortest, item.codeInk)
            const past = Math.min(hand.width - 1, ends + 200)
            // What the row holds, taken the other way in — by row number, which reads nothing off the pixels.
            const whole = acts.copyOfRow(i)

            diffPane.diffModel.clearSelect()
            hand.takeAt(ends + 4, y, Qt.LeftButton)
            hand.followAt(past, y)
            hand.releaseText()
            quiet = quiet && !diffPane.diffModel.selHasNew && diffPane.diffModel.selRemoved === 0

            if (fromHead) {
                clamped = clamped && acts.dragTakes(0, y, past, y, whole)
                // And back the other way: a drag that begins out in the blank and ends on the row takes the same
                // line. The blank is a place to start from as much as a place to stop at — every place inside the
                // code column nobody else takes is one (規約 §diff の中身をコピーする).
                backwards = backwards && acts.dragTakes(past, y, 0, y, whole)
            }
        }
        return { rows: rows, shortest: shortest, quiet: rows > 0 && quiet,
                 clamped: fromHead && rows > 0 && clamped,
                 backwards: fromHead && rows > 0 && backwards }
    }
    /// What the plain `Copy` puts on the pad for one row, taken by row number — a way in that reads nothing off the
    /// pixels, so it can stand as the answer the drags below are held against.
    ///
    /// **Read off the pad** (`ClipboardHelper.lastCopied`): the claim is about what a reader
    /// ends up holding, and the copy is where a selection that is right on screen could still go wrong. `took` is
    /// what keeps a run honest — a copy that put nothing out leaves the pad saying whatever it said last.
    function copyOfRow(row) {
        diffPane.diffModel.clearSelect()
        diffPane.diffModel.selectRow(0, row)
        return diffPane.copySelection() ? driver.clipboard.lastCopied : ""
    }
    /// One drag from one point of the hand to another, and whether what it puts on the pad is `want`. The two ends
    /// carry their own y so that a drag down the rows goes in the same way one across a row does.
    function dragTakes(fromX, fromY, toX, toY, want) {
        const hand = diffPane.textHand
        diffPane.diffModel.clearSelect()
        hand.takeAt(fromX, fromY, Qt.LeftButton)
        hand.followAt(toX, toY)
        hand.releaseText()
        return diffPane.copySelection() && driver.clipboard.lastCopied === want && want !== ""
    }
    /// A drag that runs from the head of one row out into the blank beside a later one, held against those rows
    /// taken one at a time by number. **The rows between the two ends are the half a single-row drag cannot ask
    /// about**: the copy joins them with newlines and leaves the removed lines out, and an end that landed on the
    /// wrong place would come back a line short at either edge.
    ///
    /// Answers "" where the fixture has fewer than two reachable rows, so a run that proved nothing says so.
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
        // Every row the copy takes between the two ends, each read on its own — the same rows in the same order,
        // joined the way the copy joins them.
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
    /// The picture this verb is of: a drag that ran out past a row's last character, left where it ended. **The band
    /// has to stop at the ink** — reaching on into the blank is the failure, and it is the only part of this a camera
    /// can see. Two rows, so the eye has a second one to read the edge against.
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
    /// Rows the list has actually put down, as against rows it has been handed. Every verb that reads a place or a
    /// width off the view waits for this first: `contentHeight` is zero until the list has laid the rows out, so
    /// `maxY` reads as nothing to lose and `codeMax` as a width measured against a frame of nothing.
    function viewLaidOut() {
        return diffPane.view.count > 0 && diffPane.view.width > 0
                && diffPane.view.height > 0 && diffPane.view.contentHeight > 0
    }
    /// How far down a diff the reader is taken before the thing that could cost them their place happens — the rebuild
    /// a partial write asks for (`keep-place`), and the swap the colours arrive in (`colour-place`). One number for
    /// both, because both are judged on getting exactly it back, and a place nobody can name is not one either of them
    /// can be caught losing.
    readonly property real readY: 400
    function reportPlace(at, room) {
        Harness.report("diff_place at=" + Math.round(at)
                          + " want=" + Math.round(acts.readY)
                          + " room=" + Math.round(room))
    }
    // One line staged from the diff, then the same file moved from the file list — the diff has to follow both, and
    // following only the first is the observed failure this verb pins (observed: the line was gone from
    // the unstaged side and never came back when the file was unstaged).
    //
    // Three answers in one run, because they are one story: the line goes (the rows shrink), the line comes back (the
    // rows are as they were), and staging the rest empties the side being read — where the pane follows the file to the
    // side it went to (`RepoPage.followEmptySide`). Each step waits for its own write
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
        /// Whether the write this step asked for has landed, and the pane has stopped catching up with it. The
        /// sequence is read immediately before asking, so its moving is the whole of the evidence — waiting to *see*
        /// `busyCount` rise as well wedges on a write that begins and ends inside one tick, which is what the
        /// container did while the host did not (measured, `line-back` PASS on Windows, watchdog on Linux). The
        /// pane's own word (`RepoPage.diffSettling`) is the rest: the answer asks the file to be read again, and the
        /// rows that read brings replace the ones on screen — a step that went on from the answer alone reads rows
        /// about to be swapped, and the run ends with that swap still on its way.
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
                // The file list's own `−`, which is the half that was never reaching the pane.
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
    // Line after line, the way a hand does it. The pane refuses a press while the rows it would be written against are
    // still coming (`RepoPage.diffSettling`), so this waits for exactly that — which is also the thing
    // that broke: held on a signal the file list only sends when its rows differ, the pane went quiet for good at the
    // second line of a file already on both sides, and no `+` anywhere would go in again.
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
            Harness.report("line_run staged=" + lineRunTimer.done
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
            // (`keep-place` learned it too). So an empty answer is waited on — but not for ever. **The 5s cut-off
            // names a shortfall**: the report says `staged=n want=m` and the judge
            // fails it — it turns "the fixture has fewer changed lines than the run asks for"
            // from a silent watchdog into a diagnosable line.
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
    // Where the reader lands when the file under the open diff is moved whole from the file list. Two answers, and
    // which one is right depends on what is left behind (デザイン規約 §diff の中のステージ): with other files still on that side the
    // pane takes the next of them, and with none left it stays on the same file and reads it from the side it went to.
    //
    // The argument names the file to open; the verb decides its own second step from what the tree holds afterwards,
    // and reports both.
    // Escape over a diff the hand was inside, read **after the pane has actually gone off the screen**: the key is
    // answered in one turn, but the swap behind it is the layout's, and a page asked in the same turn is still
    // holding the keyboard through the pane that is on its way out. `kept=` is the claim — the page is a focus scope,
    // so the keyboard the closing pane let go of stays inside it and the next Escape has somewhere to land
    // (`RepoPage`, `tests/qml/tst_escape.qml`).
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
                              // **Active focus is held by one item at a time**, so this is also the file list
                              // saying it let go: the hand was in the diff, the diff is gone, and the arrows now
                              // enter where `GraphPane.stepRow` does.
                              + " graph=" + graphPane.view.activeFocus)
            renderedBarrier.begin()
        }
    }
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
                // The file list's own `+` / `−`, whole file at a time.
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
            // The landing is the output: the pane has to have moved off the key it was on and settled somewhere with
            // rows — and stopped reading (`RepoPage.diffSettling`), so the rows it lands with are the rows the run
            // ends on.
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
    // Sending the diff's code sideways, by the bar's own path and then by the hand that carries the rows with it. What
    // is read back is where the code and the rows ended up: a bar bound to nothing still
    // takes a press, and a hand wired to nothing still starts.
    //
    // The wait is for the view (`keep-place` learned the same lesson): rows that have arrived are not rows the list has
    // laid out, and until it has, `codeMax` is measured against a width of nothing. A diff with nowhere sideways to go
    // says so and stops there — a run over one photographs a pane that proves nothing
    // (app-ui.md §UI 自動化の因果性).
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
    // The reach as it stands, as one string — what the pane measured off the pick, what the rows came to, and which
    // row is holding that. Compared with the look before to know the widths have stopped moving: a `ListView` builds
    // its delegates on the layout after the jump that brought them into the frame, and a `Text` answers at the
    // family's own advance until whatever fallback carries its glyphs is resolved, so the first number out of either
    // is not the last (`DiffTextMetrics`). The row is in the string because a width that stayed while the row under it
    // changed is a width filed under the wrong row.
    function reachNow() {
        return Math.round(diffPane.codeMeasured) + "," + Math.round(diffPane.codeDrawn)
                + "," + diffPane.codeWidestRow
    }
    // PGG_AUTO_ACT=code-grow: a row nothing picked, arriving from below. The model hands over the longest few lines by
    // column count and the pane measures those before any row exists (`DiffTextMetrics.codeW`); what settles how far
    // there is to go is the rows themselves as they are laid out (`DiffReach`). This is the case where the two
    // disagree — `--preset widelate`'s `late.txt` fills the pick with lines of combining pairs, which the walk counts
    // a column for and the font draws nothing for, and the line really drawn furthest is plain ASCII at the foot of
    // the file. Three claims, none of which the picture can carry (a diff sent to its end and one with nowhere to go
    // frame alike):
    //
    //  - **`grew=`** the width the rows came to went up when that row was laid out, so a line no record named is
    //    reachable to its end.
    //  - **`past=`** it went past what the pick measured, which is the whole reason the rows are read at all.
    //  - **`kept=`** and it is the same width under the same row after the reader has been back to the head and down
    //    again — the trip a delegate is reused on, and where a width filed under the wrong row would show.
    //  - **`ends=`** and, sent to the far end with that row on screen, what stands at the frame's right edge is the
    //    end of the line. This is the one claim the other three cannot make between them: `codeMax` is worked out
    //    from the same width the rows filed, so a reach that ran past the end of every line agrees with itself
    //    everywhere except here, where it shows as room left over past the last character — which is exactly how the
    //    fault this whole mechanism replaced was seen (several screens of nothing). Read off where the widest row's
    //    Label was placed (`DiffPane.codeInkRight`), against the one gap the pane holds there itself.
    //
    // The send is the last thing done, so the picture is of the diff standing at its own end — the only frame in
    // which room past the text would be visible at all.
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
        /// Where the text ends and where the frame does, once the diff has been sent to its far end.
        property real ink: 0
        property real room: 0
        /// The reach at the previous look of this step (`acts.reachNow`).
        property string lastReach: ""
        function begin() {
            codeGrowTimer.step = 0
            codeGrowTimer.lastReach = ""
            codeGrowTimer.start()
        }
        /// Whether the row the view was sent to has been built and the widths have stopped moving. Both halves: the
        /// row is the output the jump was made for, and the widths are what is being read off it.
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
        /// The gap between the end of the text and the right edge of the frame, once the diff is at its far end. The
        /// pane holds exactly one there and nothing else does (`DiffPane`, `Theme.spaceSm`), so anything more is a
        /// reach that ran past the end of every line and anything less is one that stopped short of it.
        readonly property real tail: codeGrowTimer.room - codeGrowTimer.ink
        function report() {
            const grew = codeGrowTimer.drawn > codeGrowTimer.drawn0
            const past = codeGrowTimer.drawn > codeGrowTimer.measured
            const kept = codeGrowTimer.row >= 0
                    && codeGrowTimer.back === codeGrowTimer.drawn
                    && codeGrowTimer.again === codeGrowTimer.drawn
                    && codeGrowTimer.rowAgain === codeGrowTimer.row
            // A pixel of slack: the gutter the send is measured against and the gutter the row is built from are two
            // items laid out from the same numbers, and neither rounds for the other.
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
                // A diff that fits its frame has no row below the fold to arrive from, so there is nothing here to
                // see: said and stopped — the fixture stopped asking the
                // question (app-ui.md §UI 自動化の因果性).
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
                // All the way right, from a foot the widest row is standing on: the question is where *that* row's
                // text ends against the frame, and a send made with it off screen would be answered by whichever
                // short line happened to be in front of the reader.
                codeGrowTimer.step = 4
                codeGrowTimer.lastReach = ""
                diffPane.sendCode(diffPane.codeMax)
                return
            }
            // The send lands in the turn it is made — the clamp is arithmetic — but where the rows
            // are drawn after it is a layout away, so what is waited for is the ink standing still.
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
    // PGG_AUTO_ACT=code-shrink: the same file read again with its widest line taken out of it — which is what a partial
    // write leaves behind (`--preset widelate`'s `top.txt`, whose first hunk's first changed row is that line). The
    // reading place along the row is the thing at risk: the width it is clamped against drops under it, and a reach
    // that answered zero in the gap between the two readings — or one that never let go of the largest number it had
    // seen — would take the reader either back to the left edge or nowhere at all.
    //
    // **`kept=`** the place along the line is where it was, **`shrank=`** there is less to go than there was, and
    // **`narrower=`** both halves of the reach came down and not just the one. The picture cannot carry any of the
    // three: a diff standing part way along reads the same either way.
    SampleTimer {
        id: codeShrinkTimer
        /// The line of the first hunk that gets staged — the file's widest, so the reading left behind is a
        /// genuinely shorter one.
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
                // Nowhere sideways to go is no place to keep (`code-send`, the same honesty).
                if (diffPane.codeMax <= 0) {
                    codeShrinkTimer.finish()
                    return
                }
                // An eighth of the way along: far enough to be somewhere, and short of where the shorter reading of
                // this file ends, so what is being claimed is that the place was kept.
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
            // The write's own edges, the pane's word that it has stopped reading (`RepoPage.diffSettling`), and then
            // the widths of the rows that reading brought — which are the output the whole verb is about.
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
    // PGG_AUTO_ACT=code-swap: another file, and the place and the width both go with the last one. The argument names
    // the file to read first; the second is the row beside it in the list (`NavSectionModel.beside_path`), the way a
    // reader picks the next file, and the verb reports both.
    //
    // **`room=`** is what makes `dropped=` mean anything: a file with nowhere sideways to go would be at its left edge
    // however the reach behaved, so the file read second has to have somewhere to be that it is not
    // (`--preset widelate`'s `plain.txt`, narrower than the other two and still wider than the pane).
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
                // The row beside this one under the same heading, as `<bucket>:<path>`. A fixture with nothing beside
                // it says so here.
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
            // The other file's own rows: the pane has to be on it, to have
            // stopped reading, and to have laid something down before the width it comes to is anybody's answer.
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
    // PGG_AUTO_ACT=diff-sweep: the diff's text taken from the ground under the last row of a short file — the one
    // place inside the code column where a press used to reach nothing
    // (規約 §diff の中身をコピーする). The shape is `details-sweep`'s, and so are its two claims:
    //
    // **Nine starts** (`reach=`). A reach that worked from a single place in the ground is exactly the fault
    // the right pane's values shipped with, and the middle is the one place that hides it.
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
            // `reach=0/9` off one is a fixture that stopped saying anything.
            // `ours=` reads the way `details_sweep` reads it — false is the pass, and it says the heading's own two
            // words kept every press across their face.
            Harness.report("diff_sweep reach=" + reach + "/" + tries
                              + " ground=" + hand.hasGround
                              + " ours=" + ours
                              + " hunkRow=" + hunkRow
                              + " " + diffPane.pickTally())
            renderedBarrier.begin()
        }
    }
    // Reading part way down a long diff and then writing: the rebuild has to come back to the same place.
    //
    // The wait is for the view (`diff-step` learned the same lesson): rows that have arrived are not
    // rows the list has laid out, and until it has there is no place to lose — the scroll goes nowhere and the restore
    // has nothing to undo. So what is waited for is the room the reading consumes, and a diff that is laid out and
    // still too short says so and stops there: nothing that short can hold a place, and a
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
            // Both edges of the write, and then the one output the whole verb is about: the rebuilt list put back on
            // the place. A write that emptied this side never gets a row back and so never lands anywhere — which is a
            // fixture with no place in it, and the run waits.
            if (!driver.wroteAndSettled()
                    || diffPane.placeLandedY < 0)
                return
            keepPlaceTimer.stop()
            acts.reportPlace(diffPane.placeLandedY, diffPane.view.maxY)
            renderedBarrier.begin()
        }
    }
    // A reader who scrolled before the colours landed: the whole list is rewritten when the colours turn up
    // (`DiffModel::repaint_rows`), and that rewrite must cost neither the place being read nor the width it is read
    // against.
    //
    // **`held=`** is the width half. Every row is markup either way (`markup::styled`), so the tags the colours bring
    // are markup a `StyledText` row does not draw and the rulers do not measure — the reach is the same number before
    // and after, the rows are not of a new reading (`rows_gen` does not move), and nothing about the pick changes.
    // Each of those is a way this went wrong: a row measured in one format and drawn in another read its own
    // `<font …>` tags as letters, and a reading counted as new would have thrown away every width the rows had filed.
    // The reading before is taken on the last look at which the colours had not landed, which is the last moment the
    // question is about — the swap is a second away from the rows and this looks every 50ms.
    // waits(paced): the run ends on the colours having landed, and the beat only decides how close the last look is
    Timer {
        id: colourPlaceTimer
        interval: 50
        repeat: true
        property int waited: 0
        property bool scrolled: false
        property real measuredWas: 0
        property real drawnWas: 0
        /// The reach at the previous look since the colours landed (`acts.reachNow`), so what is read is the rewritten
        /// rows and not the ones being rewritten.
        property string lastReach: ""
        function begin() {
            colourPlaceTimer.waited = 0
            colourPlaceTimer.scrolled = false
            colourPlaceTimer.lastReach = ""
            colourPlaceTimer.start()
        }
        /// The reach as it stands, kept as the reading before. Taken at the scroll as well as at every look after it:
        /// a small diff can be coloured inside one look, and then the width at the scroll is the only one this run
        /// ever had that was measured before the tags. The rows are there by then — a row that answered
        /// `firstChangedLine` has been built, and a built row has said what it was laid out at.
        function look() {
            colourPlaceTimer.measuredWas = diffPane.codeMeasured
            colourPlaceTimer.drawnWas = diffPane.codeDrawn
        }
        onTriggered: {
            // waits(measured): printed in the report and compared with nothing — the run ends on `coloured`
            colourPlaceTimer.waited += colourPlaceTimer.interval
            if (!colourPlaceTimer.scrolled) {
                // Read down the file the moment the rows are there, which is well before the colours are.
                if (diffPane.firstChangedLine(0) < 0)
                    return
                diffPane.scrollTo(acts.readY)
                colourPlaceTimer.scrolled = true
                colourPlaceTimer.look()
                return
            }
            if (!diffPane.diffModel.coloured) {
                colourPlaceTimer.look()
                return
            }
            const now = acts.reachNow()
            if (now !== colourPlaceTimer.lastReach) {
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

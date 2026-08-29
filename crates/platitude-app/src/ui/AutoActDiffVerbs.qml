pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The diff pane's verbs. All of them wait on the same thing first — a row of the first hunk being on screen —
/// so the wait is made once, below, and the verb it belongs to is chosen from there.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing
// and is never given a size.
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
    readonly property var diffRowMenu: driver.diffRowMenu
    readonly property var renderedBarrier: driver.barrierRendered
    readonly property var writeBarrier: driver.barrierWrite

    // The runs this dispatcher hands off to, under the names it starts them by.
    AutoActDiffStepVerbs {
        id: steps
        driver: acts.driver
    }
    readonly property var codeSendTimer: steps.stepCodeSend
    readonly property var diffSweepTimer: steps.stepDiffSweep
    readonly property var followTimer: steps.stepFollow
    readonly property var keepPlaceTimer: steps.stepKeepPlace
    readonly property var lineBackTimer: steps.stepLineBack
    readonly property var lineRunTimer: steps.stepLineRun

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "stage-hunk" || act === "stage-line"
            || act === "discard-hunk" || act === "discard-hunk-go"
            || act === "diff-file" || act === "conflict-sides" || act === "line-tools"
            || act === "hunk-tools" || act === "keep-place" || act === "diff-tick"
            || act === "code-send" || act === "line-back"
            || act === "diff-select" || act === "diff-copy"
            || act === "diff-menu" || act === "diff-copy-removed" || act === "diff-sweep"
            || act === "diff-band-sweep" || act === "diff-bar"
            || act === "diff-follow" || act === "line-run") {
            // All enter through one file's diff and act on its first hunk. The bucket rides in front of the path
            // (`<bucket>:<path>`) when it is not the usual unstaged one: an untracked file has no unstaged diff at all,
            // a conflicted one is read from `conflicts`. Only the bucket names count as one, so a path carrying a colon
            // still opens.
            const cut = arg.indexOf(":")
            const head = cut > 0 ? arg.substring(0, cut) : ""
            const named = head === "staged" || head === "unstaged"
                          || head === "untracked" || head === "conflicts"
            page.showWip()
            // The source of a rename comes off the model rather than out of the argument: a row hands it over when it
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
                AppBackend.report("diff_arg act=" + act + " named=false")
                renderedBarrier.begin()
                // A known verb answers true even on this early way out —
                // falsy would read as "not mine" to the dispatch chain.
                return true
            }
            page.toggleDiff(named ? head : "unstaged", wtPath,
                            worktreeModel.origOf(wtPath))
            stageRowTimer.begin()
        } else if (act === "preview" || act === "preview-unstaged" || act === "preview-staged") {
            // The toggle only asks; the read is a git subprocess away, so completion is the pane settling
            // (`stageRowTimer`), not the ask. No path is a run with nothing to open: said and stopped on the rendered
            // surface, rather than holding a wait no read will answer — the wanted line is what fails it.
            if (arg === "") {
                AppBackend.report("diff_arg act=" + act + " named=false")
                renderedBarrier.begin()
            } else {
                page.toggleDiff(act === "preview" ? "untracked"
                                : act === "preview-unstaged" ? "unstaged" : "staged", arg, "")
                stageRowTimer.begin()
            }
        } else {
            return steps.run(act, arg)
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
        diffPane.pickText(1, 0, lastRow, driver.pastLineEnd)
        AppBackend.report("diff_pick " + diffPane.pickTally()
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
        diffPane.askCodeMenu(removedRow, 0)
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
        AppBackend.report(name + " holdsRemoved=" + (removed !== "" && text.indexOf(removed) >= 0)
                          + " lines=" + (text === "" ? 0 : text.split("\n").length))
    }
    // The diff has to arrive before a row of it can be staged. Asked for rather than waited out: a fixed wait
    // photographs an empty pane the same as a late one (measured, a verb fired against this repository named no
    // row and passed). The asking has no ceiling: a row that never lands leaves the run without a report line at all,
    // and the watchdog is what ends it.
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
        // is the one answer they share — "diff-file" alone reads the model instead of a row, and the pictures and
        // binary files it also opens have no rows to find.
        function ready() {
            // "diff-file" alone reads the model instead of a row, and "conflict-sides" reads every row there is — the
            // one it is about (a side's own line, once it has been typed over) is a removal, which is not a changed
            // line of the first hunk. The previews take the settled form too: what they open can be all picture or
            // binary notice and no rows, and no row of it is theirs to name.
            // "diff-band-sweep" is about the band above the rows and not about a row at all, so the settled diff is
            // the whole of what it waits for — a file with no changed line in its first hunk still has a path in the
            // band, and demanding one would leave that run waiting out its watchdog.
            if (["diff-file", "conflict-sides", "diff-tick", "diff-band-sweep",
                 "preview", "preview-unstaged", "preview-staged"].indexOf(AppBackend.autoAct) >= 0)
                return diffPane.diffSettled()
            return diffPane.firstChangedLine(0) >= 0
        }
        onTriggered: {
            stageRowTimer.waited += stageRowTimer.interval
            const arrived = stageRowTimer.ready()
            if (!arrived)
                return
            stageRowTimer.stop()
            const act = AppBackend.autoAct
            // Which line the line-level verbs mean. Not 0: a hunk numbers its lines through the context it carries, and
            // the context is not part of the change (see `firstChangedLine`).
            const line = diffPane.firstChangedLine(0)
            // Said before the acting, so a verb that goes on to fail its write says both. `waited=` is ticks, not a
            // clock.
            AppBackend.report("diff_row act=" + act + " ready=" + arrived
                              + " rows=" + diffPane.view.count
                              + " line=" + line
                              + " waited=" + stageRowTimer.waited)
            // The diff is the shot; the line endings get a report line of their own (a picture cannot say which of the
            // four kinds the pane decided on).
            if (act === "diff-file") {
                const d = diffPane.diffModel
                AppBackend.report("line_endings kind=" + d.endingKind
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
                AppBackend.report("diff_band_sweep "
                    + diffPane.headerHand.sweepAir(7, "cut=" + diffPane.headerCut))
                driver.complete()
                return
            }
            // Which rows the two sides are named on. The bands themselves are in the picture, but "how many rows
            // should have carried one" is not — and a resolved conflict is exactly where none of them did.
            if (act === "conflict-sides") {
                AppBackend.report("conflict_sides " + diffPane.sideTally())
                driver.complete()
                return
            }
            // The page's tick, fired here rather than waited for. What it asks about is a file this run has not
            // touched, so core answers it with nothing (`RepoSession::refresh_diff`) and there is no arrival to
            // observe — the ask is the edge, and it is read in the same beat it is made. `loading=` is the other half
            // of the claim: a tick must not put the pane back into the state a click does.
            if (act === "diff-tick") {
                const asked = page.pollDiff()
                AppBackend.report("diff_tick asked=" + asked
                                  + " loading=" + diffPane.diffModel.loading
                                  + " rows=" + diffPane.view.count)
                driver.complete()
                return
            }
            // A preview's settled form — rows, a picture, or a binary notice — is the shot; `kind=` is said because
            // the picture cannot say it (a pane the read never reached photographs as the same black under the same
            // DIFF header).
            if (act === "preview" || act === "preview-unstaged" || act === "preview-staged") {
                AppBackend.report("preview_pane kind=" + diffPane.diffModel.previewKind
                                  + " binary=" + diffPane.diffModel.isBinary)
                driver.complete()
                return
            }
            // The squares a line only puts out under the pointer, named rather than hovered (hover cannot be injected
            // on Windows).
            if (act === "line-tools") {
                diffPane.showLineTools(0, line)
                renderedBarrier.begin()
                return
            }
            // The heading's two words carry their colours only under the pointer, and hover cannot be injected, so the
            // row is named instead. A heading's own row is line -1 (`flatten_patches`).
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
            // The same text, taken from the ground under the last row instead of from the rows themselves — the one
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
            // Reading part way down a long diff and then writing: the rebuild has to come back to the same place.
            if (act === "keep-place") {
                keepPlaceTimer.begin(line)
                return
            }
            if (act === "stage-hunk" || act === "stage-line") {
                driver.writeSeqBefore = repoTab.writeSeq
                // One line goes through its own mark — the press writes, there and then — and a hunk through its
                // heading's word.
                if (act === "stage-line")
                    diffPane.stageLine(0, line)
                else
                    page.stageSelection(0, -1)
                writeBarrier.start()
                return
            }
            // Sending the code sideways, and the hand that sends it and the rows at once. Both read what moved rather
            // than what was asked for: a bar bound to nothing still takes a press.
            if (act === "code-send") {
                codeSendTimer.begin()
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
            // No line-level discard exists — a hunk is the smallest piece that can be thrown away.
            if (act === "discard-hunk-go") {
                driver.writeSeqBefore = repoTab.writeSeq
                diffPane.completeHold()
                writeBarrier.start()
            } else {
                renderedBarrier.begin()
            }
        }
    }
    // PG_AUTO_ACT=diff-bar: the bar down the side of the diff can still be grabbed — the strip it stands on is not
    // taken by the hand laid over the rows (`AppListView.barRoom`). The bar is drawn *over* the rows, so anything
    // covering the frame covers the bar with it, and the hand that picks the text out took every press on the trough
    //. Two claims:
    //
    // **`clear=`** — where this hand ends against where the bar begins, read off the two items rather than off the
    // rule that places them. **`reach=`** — the hand still answers at its own last pixel, so the strip was given back
    // to the bar and not eaten out of the code. And `out=` is the run's own honesty: a diff that fits its frame has
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
            AppBackend.report("diff_bar " + diffPane.textHand.barTally())
            renderedBarrier.begin()
        }
    }
}

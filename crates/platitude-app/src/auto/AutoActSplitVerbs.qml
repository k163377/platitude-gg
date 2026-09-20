pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The verbs of the diff read side by side (デザイン規約 §diff を 2 列で読む). All three open one file's diff the way
/// `diff-file` does, wait for its rows, flip the view through the band's own toggle, and wait for the rows laid
/// out the other way round — then each does its one thing: `diff-split` photographs and tallies the rows,
/// `split-tools` names a line on each side and reads whether that side's mark came out, `split-copy` drags down
/// each column and reads what each copy holds.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var worktreeModel: driver.worktreeModel
    readonly property var diffPane: driver.diffPane

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act !== "diff-split" && act !== "split-tools" && act !== "split-copy")
            return false
        // The same door `diff-file` opens: the bucket rides in front of the path where it is not the usual
        // unstaged one, and the source of a rename comes off the model (`AutoActDiffVerbs.run`).
        const cut = arg.indexOf(":")
        const head = cut > 0 ? arg.substring(0, cut) : ""
        const named = head === "staged" || head === "unstaged" || head === "untracked" || head === "conflicts"
        const wtPath = named ? arg.substring(cut + 1) : arg
        if (wtPath === "") {
            Harness.report("diff_arg act=" + act + " named=false")
            driver.barrierRendered.begin()
            return true
        }
        page.showWip()
        page.toggleDiff(named ? head : "unstaged", wtPath, worktreeModel.origOf(wtPath))
        rowsTimer.begin()
        return true
    }

    /// The rows as one column first — the reading the file opens in — so the flip is the flip a reader makes, and
    /// not a model laid out split from the start.
    SampleTimer {
        id: rowsTimer
        function begin() {
            rowsTimer.start()
        }
        onTriggered: {
            // The first hunk has a changed line on screen: the same readiness the line-level verbs wait for.
            if (diffPane.firstChangedLine(0) < 0)
                return
            rowsTimer.stop()
            splitTimer.begin()
        }
    }

    /// The flip, through the band's toggle — the very handler a press runs (`DiffViewToggle.onClicked` is one line,
    /// and this is it) — and the wait for what it asked for: the rows of a new reading, laid out, with the first
    /// change read across.
    SampleTimer {
        id: splitTimer
        property int genBefore: 0
        function begin() {
            splitTimer.genBefore = diffPane.diffModel.rowsGen
            diffPane.viewToggle.chosen(true)
            splitTimer.start()
        }
        onTriggered: {
            const m = diffPane.diffModel
            if (!m.split || m.rowsGen === splitTimer.genBefore || diffPane.view.count === 0)
                return
            // Laid out, not only arrived: a row's two sides exist only once the view has built the delegate.
            const paired = diffPane.firstPairedRow()
            if (paired < 0)
                return
            splitTimer.stop()
            acts.act(paired)
        }
    }

    /// What each verb does once the rows stand side by side. `paired` is the first row read across: a removed line
    /// on the left, the added one that replaced it on the right.
    function act(paired) {
        const act = Harness.autoAct
        const m = diffPane.diffModel
        if (act === "diff-split") {
            // The choice is the machine's: reported to the store the way the window reports it on its timer, and
            // read back off the store — the half of the wiring a picture of two columns cannot answer.
            page.reportLayout()
            Harness.report("diff_split split=" + m.split + " saved=" + AppBackend.startDiffSplit()
                              + " rows=" + diffPane.view.count + " paired=" + paired + " " + m.splitTally())
            driver.complete()
            return
        }
        const row = diffPane.view.itemAtIndex(paired)
        if (act === "split-tools") {
            // The pointer named on the left line, then on the right one, and each time whether the mark that
            // came out is that side's alone — the row's answer (`markShown`), read off the marks themselves.
            diffPane.showLineTools(row.hunk, row.line)
            const left = row.markShown(0)
            const leftAlone = !row.markShown(1)
            diffPane.showLineTools(row.hunk, row.pair_line)
            const right = row.markShown(1)
            const rightAlone = !row.markShown(0)
            Harness.report("split_tools left=" + left + " right=" + right
                              + " crossed=" + !(leftAlone && rightAlone)
                              + " row=" + paired + " line=" + row.line + " pairLine=" + row.pair_line)
            driver.complete()
            return
        }
        // `split-copy`: the same two rows down each column — the paired row and the one under it — and what the
        // plain `Copy` hands over from each. The old column is the file as it was, so its copy already holds the
        // removed line and the menu's second word has nothing left to offer; the new column's copy is the file as
        // it is, and the removed line across from its first row is what that word offers. The two copies agree
        // with each other where they should: the old column's first line is the line the new column calls removed.
        const last = Math.min(diffPane.view.count - 1, paired + 1)
        diffPane.pickText(0, paired, 0, last, driver.pastLineEnd)
        diffPane.copySelection()
        const leftCopy = driver.clipboard.lastCopied
        const leftRemoved = m.selRemoved
        diffPane.pickText(1, paired, 0, last, driver.pastLineEnd)
        diffPane.copySelection()
        const rightCopy = driver.clipboard.lastCopied
        const rightRemoved = m.selRemoved
        const removed = m.removedText()
        Harness.report("split_copy left=" + acts.lines(leftCopy) + " leftRemoved=" + leftRemoved
                          + " right=" + acts.lines(rightCopy) + " rightRemoved=" + rightRemoved
                          + " agree=" + (removed !== "" && leftCopy.indexOf(removed) === 0
                                         && rightCopy.indexOf(removed) < 0)
                          + " " + diffPane.pickTally())
        driver.complete()
    }
    function lines(text) {
        return text === "" ? 0 : text.split("\n").length
    }
}

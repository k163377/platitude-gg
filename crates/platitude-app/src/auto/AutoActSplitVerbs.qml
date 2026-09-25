pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The verbs of the diff read side by side (デザイン規約 §diff を 2 列で読む). All three open one file's diff as
/// `diff-file` does and flip it through the band's own toggle before doing their one thing (`act`).
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: naming its type would be a cycle — the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var worktreeModel: driver.worktreeModel
    readonly property var diffPane: driver.diffPane

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act !== "diff-split" && act !== "split-tools" && act !== "split-copy")
            return false
        // `[<bucket>:]<path>`, opened as `diff-file` opens it (`AutoActDiffVerbs.run`).
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

    /// The rows in one column first, so the flip is the one a reader makes, not a model split from the start.
    SampleTimer {
        id: rowsTimer
        function begin() {
            rowsTimer.start()
        }
        onTriggered: {
            if (diffPane.firstChangedLine(0) < 0)
                return
            rowsTimer.stop()
            splitTimer.begin()
        }
    }

    /// The flip through `chosen(true)` — all a press on `DiffViewToggle` runs — and the wait for the new rows laid
    /// out with the first change read across.
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

    /// `paired` is the first row read across: a removed line on the left, the added one that replaced it on the right.
    function act(paired) {
        const act = Harness.autoAct
        const m = diffPane.diffModel
        if (act === "diff-split") {
            // Reported to the store as the window's timer does, and read back — a picture of two columns can't.
            page.reportLayout()
            Harness.report("diff_split split=" + m.split + " saved=" + AppBackend.startDiffSplit()
                              + " rows=" + diffPane.view.count + " paired=" + paired + " " + m.splitTally())
            driver.complete()
            return
        }
        const row = diffPane.view.itemAtIndex(paired)
        if (act === "split-tools") {
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
        // `split-copy`: the old column's copy already holds the removed line, so the menu's removed-lines row has
        // nothing to offer (`selRemoved`); the new column's lacks it, and that row offers it.
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

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
    readonly property var unstagedModel: driver.unstagedModel
    readonly property var diffPane: driver.diffPane

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act !== "diff-split" && act !== "split-tools" && act !== "split-copy")
            return false
        // `[<bucket>:]<path>`, opened as `diff-file` opens it (`AutoActDiffVerbs.run`).
        const cut = arg.indexOf(":")
        const head = cut > 0 ? arg.substring(0, cut) : ""
        const named = head === "staged" || head === "unstaged" || head === "untracked" || head === "conflicts"
        const filePath = named ? arg.substring(cut + 1) : arg
        if (filePath === "") {
            driver.act.complete(() => "diff_arg act=" + act + " named=false")
            return true
        }
        page.showWip()
        page.toggleDiff(named ? head : "unstaged", filePath, unstagedModel.origOf(filePath))
        // The flip, then the verb's one thing (`act`).
        splitTimer.order = act === "diff-split" ? ["flipped", "reported"]
                         : act === "split-tools" ? ["flipped", "left_tools", "right_tools"]
                         : ["flipped", "left_copied", "right_copied"]
        splitTimer.start()
        return true
    }

    /// The rows in one column first, so the flip is the one a reader makes, not a model split from the start. Then the
    /// flip through `chosen(true)` — all a press on `DiffViewToggle` runs — and the wait for the new rows laid out with
    /// the first change read across.
    PageStage {
        id: splitTimer
        name: "diff_split"
        /// The rows' generation before the flip, read at it: the split rows are a later one.
        property int genBefore: 0
        /// `split-tools`: what each side's marks showed as each column's tools went up, and the lines they went up on.
        property var left: false
        property var leftAlone: false
        property var right: false
        property var rightAlone: false
        property var line: 0
        property var pairLine: 0
        /// `split-copy`: what each column's copy came away with, and whether the menu's removed-lines row had
        /// something to offer then.
        property string leftCopy: ""
        property var leftRemoved: false
        property string rightCopy: ""
        property var rightRemoved: false
        onTriggered: {
            const op = splitTimer
            if (op.at("start")) {
                if (!Awaited.all("diff_split_rows", { "rows": diffPane.firstChangedLine(0) >= 0 }))
                    return
                op.input("flipped", () => {
                    op.genBefore = diffPane.diffModel.rowsGen
                    diffPane.viewToggle.chosen(true)
                })
                return
            }
            const m = diffPane.diffModel
            const arrived = m.split && m.rowsGen !== op.genBefore && diffPane.view.count !== 0
            // Laid out, not only arrived: a row's two sides exist only once the view has built the delegate.
            const paired = arrived ? diffPane.firstPairedRow() : -1
            if (!Awaited.all("diff_split_paired", { "split": arrived, "paired": paired >= 0 }))
                return
            acts.act(op, paired)
        }
    }

    /// `paired` is the first row read across: a removed line on the left, the added one that replaced it on the right.
    /// Each press goes in once, in `op`'s beat, and the run completes from there.
    function act(op, paired) {
        const act = Harness.autoAct
        if (act === "diff-split") {
            // Reported to the store as the window's timer does, and read back — a picture of two columns can't.
            op.input("reported", () => {
                page.reportLayout()
            })
            op.complete(() => {
                const m = diffPane.diffModel
                return "diff_split split=" + m.split + " saved=" + AppBackend.startDiffSplit()
                       + " rows=" + diffPane.view.count + " paired=" + paired + " " + m.splitTally()
            })
            return
        }
        const row = diffPane.view.itemAtIndex(paired)
        if (act === "split-tools") {
            // Each side read as its own tools go up: the second column's take the first's marks down.
            op.input("left_tools", () => {
                diffPane.showLineTools(row.hunk, row.line)
                op.line = row.line
                op.left = row.markShown(0)
                op.leftAlone = !row.markShown(1)
            })
            op.input("right_tools", () => {
                diffPane.showLineTools(row.hunk, row.pair_line)
                op.pairLine = row.pair_line
                op.right = row.markShown(1)
                op.rightAlone = !row.markShown(0)
            })
            op.complete(() => "split_tools left=" + op.left + " right=" + op.right
                              + " crossed=" + !(op.leftAlone && op.rightAlone)
                              + " row=" + paired + " line=" + op.line + " pairLine=" + op.pairLine)
            return
        }
        // `split-copy`: the old column's copy already holds the removed line, so the menu's removed-lines row has
        // nothing to offer (`selRemoved`); the new column's lacks it, and that row offers it.
        const last = Math.min(diffPane.view.count - 1, paired + 1)
        op.input("left_copied", () => {
            diffPane.pickText(0, paired, 0, last, driver.pastLineEnd)
            diffPane.copySelection()
            op.leftCopy = driver.clipboard.lastCopied
            op.leftRemoved = diffPane.diffModel.selRemoved
        })
        op.input("right_copied", () => {
            diffPane.pickText(1, paired, 0, last, driver.pastLineEnd)
            diffPane.copySelection()
            op.rightCopy = driver.clipboard.lastCopied
            op.rightRemoved = diffPane.diffModel.selRemoved
        })
        op.complete(() => {
            const removed = diffPane.diffModel.removedText()
            return "split_copy left=" + acts.lines(op.leftCopy) + " leftRemoved=" + op.leftRemoved
                   + " right=" + acts.lines(op.rightCopy) + " rightRemoved=" + op.rightRemoved
                   + " agree=" + (removed !== "" && op.leftCopy.indexOf(removed) === 0
                                  && op.rightCopy.indexOf(removed) < 0)
                   + " " + diffPane.pickTally()
        })
    }
    function lines(text) {
        return text === "" ? 0 : text.split("\n").length
    }
}

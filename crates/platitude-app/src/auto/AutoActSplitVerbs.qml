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
            argRefused.start()
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

    /// A verb given no file says so and ends there: the line its row asks for is never said, and the run fails on it.
    /// Its completion is its own — the verbs of this family defer theirs (`AutoActCompletion.defersCompletion`).
    PageStage {
        id: argRefused
        name: "split_arg"
        onTriggered: argRefused.complete({ "when": () => ({}),
                                           "report": () => "diff_arg act=" + Harness.autoAct + " named=false" })
    }

    /// The rows in one column first, so the flip is the one a reader makes, not a model split from the start. Then the
    /// flip through `chosen(true)` — all a press on `DiffViewToggle` runs — and the wait for the new rows laid out with
    /// the first change read across.
    ///
    /// Witnessed on the way (`witness`): `genBefore`, the rows' generation as the flip went in — the split rows are a
    /// later one; `paired`, the first row read across as the rows stood laid out. `split-tools`: what each side's marks
    /// showed as each column's tools went up, and the lines they went up on. `split-copy`: what each column's copy came
    /// away with, and whether the menu's removed-lines row had something to offer then.
    PageStage {
        id: splitTimer
        name: "diff_split"
        onTriggered: {
            const op = splitTimer
            if (op.at("start")) {
                if (!Awaited.all("diff_split_rows", { "rows": diffPane.firstChangedLine(0) >= 0 }))
                    return
                op.input("flipped", () => {
                    op.witness("genBefore", diffPane.diffModel.rowsGen)
                    diffPane.viewToggle.chosen(true)
                })
                return
            }
            if (!op.at("flipped"))
                return
            const m = diffPane.diffModel
            const arrived = m.split && m.rowsGen !== op.witnessed("genBefore") && diffPane.view.count !== 0
            // Laid out, not only arrived: a row's two sides exist only once the view has built the delegate.
            const paired = arrived ? diffPane.firstPairedRow() : -1
            if (!Awaited.all("diff_split_paired", { "split": arrived, "paired": paired >= 0 }))
                return
            acts.act(op, op.witness("paired", paired))
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
            op.complete({
                "when": () => ({}),
                "report": () => "diff_split split=" + diffPane.diffModel.split
                                + " saved=" + AppBackend.startDiffSplit()
                                + " rows=" + diffPane.view.count + " paired=" + op.witnessed("paired")
                                + " " + diffPane.diffModel.splitTally() })
            return
        }
        const row = diffPane.view.itemAtIndex(paired)
        if (act === "split-tools") {
            // Each side read as its own tools go up: the second column's take the first's marks down.
            op.input("left_tools", () => {
                diffPane.showLineTools(row.hunk, row.line)
                op.witness("line", row.line)
                op.witness("left", row.markShown(0))
                op.witness("leftAlone", !row.markShown(1))
            })
            op.input("right_tools", () => {
                diffPane.showLineTools(row.hunk, row.pair_line)
                op.witness("pairLine", row.pair_line)
                op.witness("right", row.markShown(1))
                op.witness("rightAlone", !row.markShown(0))
            })
            op.complete({
                "when": () => ({}),
                "report": () => "split_tools left=" + op.witnessed("left") + " right=" + op.witnessed("right")
                                + " crossed=" + !(op.witnessed("leftAlone") && op.witnessed("rightAlone"))
                                + " row=" + op.witnessed("paired") + " line=" + op.witnessed("line")
                                + " pairLine=" + op.witnessed("pairLine") })
            return
        }
        // `split-copy`: the old column's copy already holds the removed line, so the menu's removed-lines row has
        // nothing to offer (`selRemoved`); the new column's lacks it, and that row offers it.
        const last = Math.min(diffPane.view.count - 1, paired + 1)
        op.input("left_copied", () => {
            diffPane.pickText(0, paired, 0, last, driver.pastLineEnd)
            diffPane.copySelection()
            op.witness("leftCopy", driver.clipboard.lastCopied)
            op.witness("leftRemoved", diffPane.diffModel.selRemoved)
        })
        op.input("right_copied", () => {
            diffPane.pickText(1, paired, 0, last, driver.pastLineEnd)
            diffPane.copySelection()
            op.witness("rightCopy", driver.clipboard.lastCopied)
            op.witness("rightRemoved", diffPane.diffModel.selRemoved)
        })
        op.complete({ "when": () => ({}), "report": () => acts.copyLine(op) })
    }
    /// What `split-copy`'s two copies came away with (`op`'s witnesses), against the removed text as it stands.
    function copyLine(op) {
        const removed = diffPane.diffModel.removedText()
        const leftCopy = op.witnessed("leftCopy")
        const rightCopy = op.witnessed("rightCopy")
        return "split_copy left=" + acts.lines(leftCopy) + " leftRemoved=" + op.witnessed("leftRemoved")
               + " right=" + acts.lines(rightCopy) + " rightRemoved=" + op.witnessed("rightRemoved")
               + " agree=" + (removed !== "" && leftCopy.indexOf(removed) === 0 && rightCopy.indexOf(removed) < 0)
               + " " + diffPane.pickTally()
    }
    function lines(text) {
        return text === "" ? 0 : text.split("\n").length
    }
}

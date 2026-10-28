pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The find bar over the diff (デザイン規約 §diff の中を探す): what a query marks and where the bar stands, how it steps
/// round, what two typed lines and the three options do, how it reads two columns, how it is seeded from the hand's
/// selection, and how it goes. Every verb opens one file's diff as `diff-file` does, then does its one thing.
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
        if (["diff-find", "diff-find-step", "diff-find-lines", "diff-find-options", "diff-find-split",
             "diff-find-drop", "diff-find-keep", "diff-find-close", "diff-find-seed", "diff-find-long"]
                .indexOf(act) < 0)
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
        rowsTimer.act = act
        settled.act = act
        // The step is pressed once the first hit is drawn, and the run ends once the hit it steps to is.
        settled.order = act === "diff-find-step" ? ["stepped"] : []
        rowsTimer.start()
        return true
    }

    /// Ctrl+F's door (`RepoPage.startFind`), then the line typed: assigning the text runs the same search a keystroke
    /// runs. `raise` selects what stood in the box, so the assignment replaces it.
    function openWith(query) {
        page.startFind()
        diffPane.findBar.query = query
    }

    /// The rows wearing the standing hit among those built: a two-line hit wears two.
    function rowsStoodOn() {
        const view = diffPane.view
        let rows = 0
        for (let i = 0; i < view.count; i++) {
            const row = view.itemAtIndex(i)
            if (row && (row.foundNow.length > 0 || row.pairFoundNow.length > 0))
                rows++
        }
        return rows
    }
    /// Whether any built row wears a hit on its own line (`left`) and on its right side (`right`).
    function columnsWorn() {
        const view = diffPane.view
        let left = false
        let right = false
        for (let i = 0; i < view.count; i++) {
            const row = view.itemAtIndex(i)
            if (!row)
                continue
            left = left || row.found.length > 0
            right = right || row.pairFound.length > 0
        }
        return { "left": left, "right": right }
    }
    /// Whether the row the bar stands on is built and wears the standing hit — what a picture is taken of.
    function standingDrawn() {
        const m = diffPane.diffModel
        if (m.findAt === 0)
            return false
        return acts.rowsStoodOn() > 0
    }

    /// The rows first: the find bar stands over them. Then the verb's one thing, which either ends the run here or
    /// hands it to the operation that waits for what it set going.
    PageStage {
        id: rowsTimer
        name: "diff_find_rows"
        order: ["performed"]
        property string act: ""
        /// What the hand picked before the box opened on it (`diff-find-seed`).
        property string picked: ""
        /// The box as it opened on the pick, read as it opens.
        function seedLine() {
            const bar = diffPane.findBar
            return "diff_find_seed open=" + bar.open + " query=" + bar.query
                   + " seeded=" + (rowsTimer.picked !== "" && bar.query === rowsTimer.picked)
                   + " matches=" + bar.matches + " focused=" + bar.boxFocused
        }
        onTriggered: {
            const op = rowsTimer
            if (!Awaited.all("diff_find_rows", { "rows": diffPane.firstChangedLine(0) >= 0 }))
                return
            op.input("performed", () => {
                acts.perform(op.act)
            }, op.act === "diff-find-seed" ? () => op.seedLine() : undefined)
            const m = diffPane.diffModel
            if (op.act === "diff-find-drop" || op.act === "diff-find-keep") {
                op.complete(() => "diff_find_drop act=" + op.act + " open=" + diffPane.findBar.open
                                  + " searching=" + m.findSearching + " matches=" + m.findCount)
                return
            }
            if (op.act === "diff-find-close") {
                op.complete(() => "diff_find_close open=" + diffPane.findBar.open + " searching=" + m.findSearching
                                  + " matches=" + m.findCount + " rowsFocus=" + diffPane.view.activeFocus)
                return
            }
            op.stop()
            if (op.act === "diff-find-options")
                tipTimer.start()
            else
                settled.start()
        }
    }

    /// The verb's one thing, put in at the rows. What a step between two inputs reads is kept for the line that
    /// says it.
    function perform(act) {
        const bar = diffPane.findBar
        const m = diffPane.diffModel
        if (act === "diff-find-seed") {
            // The hand picks the word on the first added line (`008: reworded …`, bytes 5 to 13), then Ctrl+F: the
            // box opens on what was picked.
            const row = diffPane.firstRemovedRow() + 1
            diffPane.pickText(0, row, 5, row, 13)
            rowsTimer.picked = m.selectionText()
            page.startFind()
            return
        }
        if (act === "diff-find-split")
            m.setSplit(true)
        if (act === "diff-find-options") {
            acts.openWith("Reworded")
            bar.toggleCase()
            const byCase = bar.matches
            bar.toggleCase()
            bar.query = "note"
            bar.toggleWord()
            const byWord = bar.matches
            bar.toggleWord()
            const plain = bar.matches
            bar.query = "re\\w+ded"
            bar.toggleRegex()
            const byRegex = bar.matches
            // A pattern that cannot be read: refused — the frame wears `warning` and the tooltip the reason. The run
            // waits for the tip (`scene.png` shows where it stands, `app.png` cannot).
            bar.query = "("
            tipTimer.line = "diff_find_options case=" + byCase + " word=" + byWord
                            + " regex=" + byRegex + " bad=" + (m.findError !== "") + " badMatches=" + bar.matches
                            + " error=" + m.findError + " refused=" + bar.refused
            tipTimer.plain = plain
            return
        }
        if (act === "diff-find-long") {
            // A line far wider than the box, read as a pattern so it still finds the 20: the box is sent sideways
            // to keep the caret — at the line's end, where typing leaves it — in view.
            acts.openWith("reworded|this pattern runs on far past the end of the box, so the box is sent sideways"
                          + " to keep the caret in view while the lines are typed")
            bar.toggleRegex()
            bar.caretToEnd()
            return
        }
        acts.openWith(act === "diff-find-lines" ? "down the file\n0" : "reworded")
        if (act === "diff-find-drop" || act === "diff-find-keep") {
            // A press landing elsewhere, entered where `FocusRelease.pressedAway` enters, with no place of its own
            // (= on none of ours): an empty box goes with it, a typed one stays (規約 §コミットを探す).
            if (act === "diff-find-drop")
                bar.query = ""
            page.releasePressedAway(null)
            return
        }
        if (act === "diff-find-close") {
            // Escape's door (`DiffFindBar.dismiss`): the marks come off, the rows take the keyboard back.
            bar.dismiss()
        }
    }

    /// The refusal's tip up (`DiffFindBar.tipShown`): then a press elsewhere, which a refused line must weather as a
    /// typed one does (`kept=`), and the picture with the strip and its tip standing.
    PageStage {
        id: tipTimer
        name: "diff_find_tip"
        order: ["pressed_away"]
        /// What the options said as they were tried, read between the presses (`perform`).
        property string line: ""
        property int plain: 0
        onTriggered: {
            if (!Awaited.all("diff_find_tip_ready", { "ready": diffPane.findBar.tipShown }))
                return
            tipTimer.input("pressed_away", () => {
                page.releasePressedAway(null)
            })
            tipTimer.complete(() => tipTimer.line + " tip=true kept=" + diffPane.findBar.open
                                     + " plain=" + tipTimer.plain)
        }
    }

    /// Reported once the row the bar stands on is built and wears the hit — the rows are laid a frame behind the
    /// model, and a split re-lays them all.
    PageStage {
        id: settled
        name: "diff_find_settled"
        property string act: ""
        /// Where the view stood before the step.
        property real before: 0
        onTriggered: {
            const op = settled
            if (!Awaited.all("diff_find_settled_ready", { "ready": acts.standingDrawn() }))
                return
            const bar = diffPane.findBar
            const m = diffPane.diffModel
            const view = diffPane.view
            if (op.act === "diff-find-step" && op.at("start")) {
                // Back from the first: round to the last, which stands off screen, so the view moves. Said as it
                // lands; the run ends once the hit it stepped to is drawn.
                op.input("stepped", () => {
                    op.before = view.contentY
                    diffPane.findPrevious()
                }, () => "diff_find_step at=" + m.findAt + " of=" + m.findCount
                         + " moved=" + (view.contentY !== op.before) + " wrapped=" + (m.findAt === m.findCount))
                return
            }
            if (op.act === "diff-find-split") {
                op.complete(() => {
                    const worn = acts.columnsWorn()
                    return "diff_find_split split=" + m.split + " matches=" + m.findCount + " at=" + m.findAt
                           + " right=" + worn.right + " left=" + worn.left
                })
                return
            }
            // Their lines were said on the way here.
            if (op.act === "diff-find-seed" || op.act === "diff-find-step") {
                op.complete(() => [])
                return
            }
            if (op.act === "diff-find-long") {
                op.complete(() => "diff_find_long open=" + bar.open + " matches=" + m.findCount
                                  + " overflows=" + bar.boxOverflows + " scrolled=" + (bar.boxScrolled > 0)
                                  + " focused=" + bar.boxFocused + " sent=" + Math.round(bar.boxScrolled))
                return
            }
            // `diff-find` / `diff-find-lines`: the plain answer, and how many rows the standing hit covers.
            op.complete(() => (op.act === "diff-find-lines" ? "diff_find_lines" : "diff_find")
                              + " open=" + bar.open + " matches=" + m.findCount + " at=" + m.findAt
                              + " lines=" + bar.lines + " spans=" + acts.rowsStoodOn()
                              + " focused=" + bar.boxFocused + " refused=" + bar.refused
                              + " query=" + bar.query.replace(/\n/g, "⏎"))
        }
    }
}

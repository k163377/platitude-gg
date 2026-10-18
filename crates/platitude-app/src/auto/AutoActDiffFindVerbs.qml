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
    readonly property var renderedBarrier: driver.barrierRendered

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
            Harness.report("diff_arg act=" + act + " named=false")
            renderedBarrier.begin()
            return true
        }
        page.showWip()
        page.toggleDiff(named ? head : "unstaged", filePath, unstagedModel.origOf(filePath))
        rowsTimer.act = act
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

    /// The rows first: the find bar stands over them.
    SampleTimer {
        id: rowsTimer
        property string act: ""
        onTriggered: {
            if (diffPane.firstChangedLine(0) < 0)
                return
            rowsTimer.stop()
            acts.perform(rowsTimer.act)
        }
    }

    function perform(act) {
        const bar = diffPane.findBar
        const m = diffPane.diffModel
        const view = diffPane.view
        if (act === "diff-find-seed") {
            // The hand picks the word on the first added line (`008: reworded …`, bytes 5 to 13), then Ctrl+F: the
            // box opens on what was picked.
            const row = diffPane.firstRemovedRow() + 1
            diffPane.pickText(0, row, 5, row, 13)
            const picked = m.selectionText()
            page.startFind()
            Harness.report("diff_find_seed open=" + bar.open + " query=" + bar.query
                              + " seeded=" + (picked !== "" && bar.query === picked)
                              + " matches=" + bar.matches + " focused=" + bar.boxFocused)
            settled.act = act
            settled.start()
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
            tipTimer.start()
            return
        }
        if (act === "diff-find-long") {
            // A line far wider than the box, read as a pattern so it still finds the 20: the box is sent sideways
            // to keep the caret — at the line's end, where typing leaves it — in view.
            acts.openWith("reworded|this pattern runs on far past the end of the box, so the box is sent sideways"
                          + " to keep the caret in view while the lines are typed")
            bar.toggleRegex()
            bar.caretToEnd()
            settled.act = act
            settled.start()
            return
        }
        acts.openWith(act === "diff-find-lines" ? "down the file\n0" : "reworded")
        if (act === "diff-find-drop" || act === "diff-find-keep") {
            // A press landing elsewhere, entered where `FocusRelease.pressedAway` enters, with no place of its own
            // (= on none of ours): an empty box goes with it, a typed one stays (規約 §コミットを探す).
            if (act === "diff-find-drop")
                bar.query = ""
            page.releasePressedAway(null)
            Harness.report("diff_find_drop act=" + act + " open=" + bar.open + " searching=" + m.findSearching
                              + " matches=" + m.findCount)
            renderedBarrier.begin()
            return
        }
        if (act === "diff-find-close") {
            // Escape's door (`DiffFindBar.dismiss`): the marks come off, the rows take the keyboard back.
            bar.dismiss()
            Harness.report("diff_find_close open=" + bar.open + " searching=" + m.findSearching
                              + " matches=" + m.findCount + " rowsFocus=" + view.activeFocus)
            renderedBarrier.begin()
            return
        }
        settled.act = act
        settled.start()
    }

    /// The refusal's tip up (`DiffFindBar.tipShown`): then a press elsewhere, which a refused line must weather as a
    /// typed one does (`kept=`), and the picture with the strip and its tip standing.
    SampleTimer {
        id: tipTimer
        property string line: ""
        property int plain: 0
        onTriggered: {
            if (!diffPane.findBar.tipShown)
                return
            tipTimer.stop()
            page.releasePressedAway(null)
            Harness.report(tipTimer.line + " tip=true kept=" + diffPane.findBar.open + " plain=" + tipTimer.plain)
            renderedBarrier.begin()
        }
    }

    /// Reported once the row the bar stands on is built and wears the hit — the rows are laid a frame behind the
    /// model, and a split re-lays them all.
    SampleTimer {
        id: settled
        property string act: ""
        onTriggered: {
            if (!acts.standingDrawn())
                return
            settled.stop()
            const bar = diffPane.findBar
            const m = diffPane.diffModel
            const view = diffPane.view
            if (settled.act === "diff-find-step") {
                // Back from the first: round to the last, which stands off screen, so the view moves.
                const before = view.contentY
                diffPane.findPrevious()
                Harness.report("diff_find_step at=" + m.findAt + " of=" + m.findCount
                                  + " moved=" + (view.contentY !== before) + " wrapped=" + (m.findAt === m.findCount))
                settled.act = "diff-find-step-moved"
                settled.start()
                return
            }
            if (settled.act === "diff-find-split") {
                const worn = acts.columnsWorn()
                Harness.report("diff_find_split split=" + m.split + " matches=" + m.findCount + " at=" + m.findAt
                                  + " right=" + worn.right + " left=" + worn.left)
                renderedBarrier.begin()
                return
            }
            if (settled.act === "diff-find-seed" || settled.act === "diff-find-step-moved") {
                renderedBarrier.begin()
                return
            }
            if (settled.act === "diff-find-long") {
                Harness.report("diff_find_long open=" + bar.open + " matches=" + m.findCount
                                  + " overflows=" + bar.boxOverflows + " scrolled=" + (bar.boxScrolled > 0)
                                  + " focused=" + bar.boxFocused + " sent=" + Math.round(bar.boxScrolled))
                renderedBarrier.begin()
                return
            }
            // `diff-find` / `diff-find-lines`: the plain answer, and how many rows the standing hit covers.
            const lines = bar.lines
            Harness.report((settled.act === "diff-find-lines" ? "diff_find_lines" : "diff_find")
                              + " open=" + bar.open + " matches=" + m.findCount + " at=" + m.findAt
                              + " lines=" + lines + " spans=" + acts.rowsStoodOn()
                              + " focused=" + bar.boxFocused + " refused=" + bar.refused
                              + " query=" + bar.query.replace(/\n/g, "⏎"))
            renderedBarrier.begin()
        }
    }
}

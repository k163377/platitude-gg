pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The diff pane's colours landing under a reader (`colour-place`).
// An `Item` only because `QtObject` has no default property to hold the timers.
Item {
    id: acts

    /// `var`: typing it would be circular — `AutoActDriver` is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var diffPane: driver.diffPane

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (act !== "colour-place")
            return false
        page.showWip()
        page.toggleDiff("unstaged", arg, "")
        colourPlaceTimer.since = PerfProbe.clockMs()
        colourPlaceTimer.start()
        return true
    }

    // colour-place: a reader scrolled before the colours land. They rewrite every row (`DiffModel::repaint_rows`),
    // which must cost neither the place nor the width. `held=`: rows are markup either way (`markup::styled`), so the
    // colour tags are neither drawn nor measured and `rows_gen` does not move — a row measured in another format reads
    // its `<font …>` tags as letters, and a reading counted as new drops every filed width.
    //
    // The reach before the colours is witnessed: as the scroll left it (`scrolled`) — a small diff can be coloured
    // within one look, and a row that answered `firstChangedLine` is already built and measured — and once the rows
    // the scroll brought in stood across a polish pass (`looked`), null where the colours came first.
    PageStage {
        id: colourPlaceTimer
        name: "colour_place"
        order: ["opened", "scrolled", "looked", "coloured"]
        /// When the dispatch asked for the file (`PerfProbe.clockMs`), for the report's `waited=` — a diagnosis,
        /// compared with nothing.
        property real since: 0
        /// Whether the pane is reading this run's file.
        function reading() {
            return page.diffShown && page.diffKey === "unstaged:" + Harness.autoActArg
        }
        /// Both halves of the reach as they stand.
        function reach() {
            return { "measured": diffPane.codeMeasured, "drawn": diffPane.codeDrawn }
        }
        /// The reach as the colours left it, against the reading before them.
        function line() {
            const op = colourPlaceTimer
            const looked = op.witnessed("looked")
            const was = looked !== null ? looked : op.witnessed("scrolled")
            const held = was.drawn > 0
                    && Math.round(diffPane.codeMeasured) === Math.round(was.measured)
                    && Math.round(diffPane.codeDrawn) === Math.round(was.drawn)
            return "colour_place coloured=" + diffPane.diffModel.coloured
                   + " at=" + Math.round(diffPane.view.contentY)
                   + " held=" + held
                   + " rows=" + diffPane.view.count
                   + " waited=" + Math.round(PerfProbe.clockMs() - op.since)
                   + " measured0=" + Math.round(was.measured)
                   + " measured=" + Math.round(diffPane.codeMeasured)
                   + " drawn0=" + Math.round(was.drawn)
                   + " drawn=" + Math.round(diffPane.codeDrawn)
        }
        onTriggered: {
            const op = colourPlaceTimer
            if (op.at("start")) {
                // The open asked again while the pane is not reading this file: one asked once can be lost, and
                // nothing else brings the rows.
                const opened = op.input("opened", () => {
                    if (!op.reading())
                        page.openDiff("unstaged", Harness.autoActArg, "")
                    return op.reading()
                })
                if (!opened)
                    return
            }
            if (op.at("opened")) {
                // Scroll as soon as the rows are there, before the colours.
                if (!Awaited.all("colour_place", { "rows": diffPane.firstChangedLine(0) >= 0 }))
                    return
                op.input("scrolled", () => {
                    diffPane.scrollTo(driver.readY)
                    op.witness("scrolled", op.reach())
                })
                return
            }
            if (op.at("scrolled")) {
                const coloured = diffPane.diffModel.coloured
                if (!Awaited.all("colour_place_looked", {
                        "looked": coloured || op.still("colour_place_before", driver.diffReach()) }))
                    return
                op.witness("looked", coloured ? null : op.reach())
                op.advance("looked")
            }
            if (op.at("looked")) {
                if (!Awaited.all("colour_place_coloured", { "coloured": diffPane.diffModel.coloured }))
                    return
                op.advance("coloured")
            }
            // The rows are rewritten as the colours land: read once the reach has stood across a polish pass since.
            if (!Awaited.all("colour_place_still", { "still": op.still("colour_place_reach", driver.diffReach()) }))
                return
            op.complete({ "when": () => ({ "coloured": diffPane.diffModel.coloured }), "report": () => op.line() })
        }
    }
}

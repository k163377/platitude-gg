pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The commit-details pane: picking text out of it, the grip that grows its description, how it fits the room it
/// is given, and the corner that steps aside for it.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are read back once below, so the code under them reads as it
/// did when it was all one file.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing
// and is never given a size.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so what is written under them reads as it did.
    readonly property var page: driver.page
    readonly property var graphModel: driver.graphModel
    readonly property var detailsPane: driver.detailsPane
    readonly property var wipPane: driver.wipPane
    readonly property var gitCorner: driver.gitCorner
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "details-select" || act === "details-select-away") {
            // `<value>` or `<value>:<row>` — the graph row defaults to the top one, since what this is about is the
            // pane rather than which commit is in it. `-away` takes two values instead, `<first>+<then>`, and is
            // about the first one letting go.
            const away = act === "details-select-away"
            const plus = arg.indexOf("+")
            detailsSelectTimer.firstWhich = away && plus > 0 ? arg.substring(0, plus) : ""
            const rest = away && plus > 0 ? arg.substring(plus + 1) : arg
            const cut = rest.indexOf(":")
            detailsSelectTimer.which = cut > 0 ? rest.substring(0, cut) : rest
            page.activateRow(graphModel.oidAt(cut > 0 ? Number(rest.substring(cut + 1)) : 0))
            detailsSelectTimer.start()
        } else if (act === "details-sweep") {
            const sweepCut = arg.indexOf(":")
            detailsSweepTimer.which = sweepCut > 0 ? arg.substring(0, sweepCut) : arg
            page.activateRow(graphModel.oidAt(sweepCut > 0 ? Number(arg.substring(sweepCut + 1)) : 0))
            detailsSweepTimer.start()
        } else if (act === "details-grow" || act === "details-grow-squeeze") {
            // The corner grip pulled past what the pane can spare; `-squeeze` then takes the pane's room back with the
            // log.
            page.activateRow(graphModel.oidAt(Number(arg)))
            descGrowTimer.pane = detailsPane
            descGrowTimer.paneName = "details"
            descGrowTimer.squeeze = act === "details-grow-squeeze"
            descGrowTimer.start()
        } else if (act === "wip-grow" || act === "wip-grow-squeeze") {
            // The argument is the description itself: the box starts empty, so a run that types nothing has nothing to
            // open.
            page.showWip()
            wipPane.setMessage("feat: write the summary", arg)
            descGrowTimer.pane = wipPane
            descGrowTimer.paneName = "wip"
            descGrowTimer.squeeze = act === "wip-grow-squeeze"
            descGrowTimer.start()
        } else if (act === "details-fit") {
            // Overflow shows as glyphs cut at the window's edge, which headless cannot see, so the pane reports the
            // number. `--preset edges` holds the wall.
            //
            // **A run that named no row keeps the one the page opened on** — the newest commit of the current
            // branch. Not `Number("")`, which is 0: row 0 is the working tree's wherever the tree is dirty, and
            // both presets this verb is pointed at are (`basic`, `edges`). Landing there puts the WIP pane in
            // front and empties the selection, so the card below can never settle and the run says nothing at all
            // until the watchdog — 120s of `cardSettled=false` with no report of any kind, on both OSes
            // (2026-08-29. The verb had only ever been run with its row, so this had been red since it was
            // written).
            if (arg !== "")
                page.activateRow(graphModel.oidAt(Number(arg)))
            detailsFitTimer.start()
        } else if (act === "corner") {
            // Both sides of the corner's one rule: preset `basic` leaves the corner bare, `long` runs rows into it.
            // Read as a pair — one half alone frames like a label always on, or always off.
            if (arg === "" || arg === "wip")
                page.showWip()
            else
                page.activateRow(graphModel.oidAt(Number(arg)))
            cornerTimer.start()
        } else if (act === "divider-refuse") {
            // A drag carried past one of a divider's bounds, named by the argument. The log's bar is not on screen
            // while the log is shut — open it and let it lay out before measuring against a bar that has no geometry
            // yet.
            if (arg === "log-min" && !page.commandsOpen) {
                page.commandsOpen = true
                splitRefuseTimer.start()
            } else if (arg === "desc-max" || arg === "desc-min") {
                // Row 1, not row 0: row 0 of every preset is the uncommitted row, and landing on it puts the working
                // tree in the right-hand pane — the box this pulls on would be off screen.
                page.activateRow(graphModel.oidAt(1))
                descGrowTimer.pane = detailsPane
                descGrowTimer.paneName = "details"
                descGrowTimer.squeeze = false
                descGrowTimer.refuse = arg
                descGrowTimer.start()
            } else {
                page.reportDividerRefusal(arg)
                renderedBarrier.begin()
            }
        } else {
            return false
        }
        return true
    }
    /// The pane's values are fields the reader drags over (規約 §右のペインの字は掴める). A drag cannot be injected, so
    /// this picks the field out the way `Ctrl+A` does and reads back what it holds — the same pair `tip-copy` uses,
    /// and for the same reason: **a picture cannot answer this**. A `Text` put back in place of the field would draw
    /// the identical row, and the selection's wash is a few pixels of colour that a scaled-down look loses.
    ///
    /// The selection is left standing (`persistentSelection`), so the shot is of a value picked out.
    SampleTimer {
        id: detailsSelectTimer
        /// `author` / `date` / `mate` / `hash` / `parent`.
        property string which: ""
        property bool asked: false
        /// The value picked out first, for the run that is about the one after it letting go (`-away`).
        property string firstWhich: ""
        onTriggered: {
            if (!driver.cardSettled)
                return
            if (!detailsSelectTimer.asked) {
                if (detailsSelectTimer.firstWhich !== "")
                    detailsPane.valueRow.selectValue(detailsSelectTimer.firstWhich)
                detailsPane.valueRow.selectValue(detailsSelectTimer.which)
                detailsSelectTimer.asked = true
                return
            }
            detailsSelectTimer.stop()
            const got = detailsPane.valueRow.selectedValue(detailsSelectTimer.which)
            const want = detailsPane.valueRow.shownValue(detailsSelectTimer.which)
            if (detailsSelectTimer.firstWhich !== "") {
                // The fault this pane shipped with: every field kept what it was holding, so a reader who swept a
                // second value found the first still lit (2026-08-28 ユーザー報告 — three at once). **Both halves are
                // said**: a run that simply cleared everything would answer `dropped=true` on its own.
                AppBackend.report("details_select_away dropped="
                                  + (detailsPane.valueRow.selectedValue(detailsSelectTimer.firstWhich) === "")
                                  + " held=" + (want !== "" && got === want)
                                  + " first=" + detailsSelectTimer.firstWhich
                                  + " then=" + detailsSelectTimer.which)
                driver.complete()
                return
            }
            // `match=` is the whole claim: what came out of the field is what the model says the pane is showing. The
            // text rides along for the eye.
            AppBackend.report("details_select match=" + (want !== "" && got === want)
                              + " which=" + detailsSelectTimer.which + " text=" + got)
            driver.complete()
        }
    }

    /// The range selection, taken from the gaps around the values rather than from the values themselves
    /// (規約 §右のペインの字は掴める). **Four claims in one line, and each answers something the others cannot**: the
    /// sweep reaches the value from every corner of the gap (`reach`), the keyboard went with it so `Ctrl+C` will
    /// land (`caret`), the words answer their own press wherever nothing stands over them (`grabs`), and a press on
    /// the value's own box is still somebody else's (`ours` — false on the plate's two, which is what says their
    /// controls kept every press across their whole face).
    SampleTimer {
        id: detailsSweepTimer
        /// `author` / `date` / `mate` / `hash` / `parent`.
        property string which: ""
        /// The row's geometry at the previous sample, for the settle below.
        property string lastGeom: ""
        onTriggered: {
            // **Both sides of the comparison have to exist first.** The details arrive a frame ahead of the row that
            // draws them, so `cardSettled` alone is not enough: the model's value can still be empty, and a field
            // with no width yet is passed over by the sweep exactly as an absent one is. Either way the run reports
            // an empty selection as a failure of the wiring (3 of 6 runs before this wait, 1 of 10 with only half of
            // it — 2026-08-28).
            const which = detailsSweepTimer.which
            if (!driver.cardSettled || !detailsPane.valueRow.valueReady(which) || detailsPane.valueRow.shownValue(which) === "")
                return
            // **And the row has to have stopped moving.** The pane lays out more than once on its way to a commit,
            // and a sweep run against a half-laid-out row starts from a gap of a dozen pixels and lands on the line
            // above the one it aimed at — every one of its nine tries, in about a fifth of runs (2026-08-28). Two
            // samples with the same geometry is the settle; no number is written down, so it holds at any pane width.
            const geom = detailsPane.valueRow.valueGeom(which)
            if (geom !== detailsSweepTimer.lastGeom) {
                detailsSweepTimer.lastGeom = geom
                return
            }
            detailsSweepTimer.stop()
            const want = detailsPane.valueRow.shownValue(which)
            // **Every corner of the gap, not just its middle.** A reach that only worked level with the words is the
            // fault this shipped with, and the middle is the one place that hides it (2026-08-28 ユーザー報告).
            let reach = 0
            let tries = 0
            let took = true
            let got = ""
            let on = ""
            let caret = false
            let miss = ""
            const across = [0.15, 0.5, 0.9]
            const down = [0.05, 0.5, 0.95]
            for (let i = 0; i < across.length; i++) {
                for (let j = 0; j < down.length; j++) {
                    tries++
                    if (!detailsPane.valueRow.sweepAt(which, across[i], down[j])) {
                        took = false
                        continue
                    }
                    got = detailsPane.valueRow.selectedValue(which)
                    // Read before the next sweep, and before the press below — both clear the board on their way in.
                    on = detailsPane.valueRow.sweptField()
                    caret = detailsPane.valueRow.sweptCaret()
                    if (want !== "" && got === want)
                        reach++
                    else if (miss === "")
                        // The first start that came away with the wrong thing, and what the sweep saw while it did.
                        // Without it the line reports only the last try, which is the one that worked.
                        miss = "x" + across[i] + ",y" + down[j] + "," + detailsPane.valueRow.sweptTrace()
                }
            }
            const ours = detailsPane.valueRow.pressOnControl(which)
            const grabs = detailsPane.valueRow.valueGrabs(which)
            // `caret=` is the half a selection does not say: `Ctrl+C` goes to the field holding the keyboard, so a
            // value picked out without it is not one the reader can take away. `took=` and `on=` are the diagnosis
            // when `grabbed` comes back false — the gesture was refused at its start, or it landed on another line.
            AppBackend.report("details_sweep reach=" + reach + "/" + tries
                              + " caret=" + caret
                              + " grabs=" + grabs
                              + " ours=" + ours
                              + " which=" + which + " took=" + took
                              + " on=" + on + " miss=[" + miss + "] text=" + got)
            driver.complete()
        }
    }

    // The details have to arrive, and the column has to be laid out with them, before there is anything to measure.
    SampleTimer {
        id: detailsFitTimer
        // A pane width the splitter left on a fraction can put a fraction in the answer; what this verb is about is
        // tens of pixels.
        /// The row's geometry at the previous sample, for the settle below.
        property string lastGeom: ""
        onTriggered: {
            if (!driver.cardSettled || detailsPane.width <= 0 || detailsPane.height <= 0)
                return
            // **And the row has to have stopped moving.** The pane lays out more than once on its way to a commit,
            // and mid-way the row and the block it stands in are both some other width — where `fills` compares one
            // against the other and answers true about a frame nobody sees (`row=173/173`, 2026-08-28).
            const geom = detailsPane.valueRow.width + "x" + detailsPane.valueRow.parent.width
            if (geom !== detailsFitTimer.lastGeom) {
                detailsFitTimer.lastGeom = geom
                return
            }
            detailsFitTimer.stop()
            AppBackend.report(
            "details_fit fills=" + detailsPane.valueRow.fillsBlock + " fits=" + (detailsPane.contentOverflow < 1)
            + " row=" + Math.round(detailsPane.valueRow.width) + "/" + Math.round(detailsPane.valueRow.parent.width) + " over=" + Math.round(detailsPane.contentOverflow)
            + " pane=" + Math.round(detailsPane.width)
            // The other axis rides along unjudged, the way `edge=` does in `window_fill`: how far the column runs past
            // the pane's own bottom is what says whether this pane needs a scroll of its own, and the answer depends on
            // the window, not on this verb.
            + " overH=" + Math.round(detailsPane.contentOverHeight)
            + " paneH=" + Math.round(detailsPane.height))
            driver.complete()
        }
    }
    // The rows have to arrive, and the list be laid out with them, before what they leave bare is worth measuring.
    SampleTimer {
        id: cornerTimer
        // `shown=` is the label's own visibility, not the room that decided it: reporting what was asked for would go
        // green with the binding cut.
        onTriggered: {
            if (gitCorner.parent === null || gitCorner.width <= 0)
                return
            cornerTimer.stop()
            AppBackend.report(
            "git_corner pane=" + (page.wipShown ? "wip" : "details")
            + " shown=" + gitCorner.visible
            + " room=" + Math.round(gitCorner.roomLeft)
            + " needs=" + Math.round(gitCorner.roomNeeded))
            driver.complete()
        }
    }
    // Same wait as details-fit, for the same reason: the message has to be in the box, and the box laid out with it,
    // before there is a ceiling to pull on.
    SampleTimer {
        id: descGrowTimer
        // Pulled past everything, so where it stops is the bound itself rather than a number this verb chose.
        readonly property int pull: 1000
        /// Which pane's box to pull. The two carry the same box and hooks under the same names, so this verb is written
        /// once.
        property var pane: detailsPane
        property string paneName: "details"
        /// Whether to take the pane's room away again afterwards, by raising the command log under it — the one way a
        /// headless run can make the pane shorter than the box it is already holding.
        property bool squeeze: false
        property int frameBefore: 0
        /// Which end this run is carrying the grip past, or empty for the ordinary pull. Same wait and same box — the
        /// difference is that the grip is in hand, so the box answers instead of just stopping (規約 §掴める境界は答える).
        property string refuse: ""
        onTriggered: {
            if (descGrowTimer.pane.width <= 0 || descGrowTimer.pane.height <= 0 || descGrowTimer.pane.descCap <= 0)
                return
            descGrowTimer.stop()
            descGrowTimer.frameBefore = page.Window.window.frameCounter
            if (descGrowTimer.refuse !== "") {
                descGrowTimer.pane.pullDescriptionPast(
                    descGrowTimer.refuse === "desc-max")
                descGrowSettle.start()
                return
            }
            descGrowTimer.pane.growDescription(descGrowTimer.pull)
            if (descGrowTimer.squeeze)
                page.toggleCommands()
            descGrowSettle.start()
        }
    }
    // The layout runs after that handler, so what the pull left behind is read a beat later: asked in the same breath,
    // the list still reports the height it had before it gave any of it up.
    //
    // `grip=` says the corner was offered at all, `keeps=` that what the box borrowed room from is still on screen —
    // the author card in the details pane, the commit button in the editor — which the picture cannot answer, because
    // the overflow draws over the window's own footer.
    SampleTimer {
        id: descGrowSettle
        onTriggered: {
            if (page.Window.window.frameCounter <= descGrowTimer.frameBefore)
                return
            if (descGrowTimer.refuse !== "" && !page.refusalShown)
                return
            descGrowSettle.stop()
            if (descGrowTimer.refuse !== "") {
                AppBackend.report("divider_refuse refuses=" + page.refusalShown
                                  + " line=" + descGrowTimer.pane.descGrips
                                  + " case=" + descGrowTimer.refuse
                                  + " box=" + Math.round(descGrowTimer.pane.descHeight)
                                  + " wants=" + Math.round(descGrowTimer.pane.descWants))
                driver.complete()
                return
            }
            AppBackend.report(
            "description_grow keeps=" + descGrowTimer.pane.descKeeps
            + " pane=" + descGrowTimer.paneName
            + " grip=" + descGrowTimer.pane.descGrips
            + " box=" + Math.round(descGrowTimer.pane.descHeight)
            + " wants=" + Math.round(descGrowTimer.pane.descWants)
            + " cap=" + Math.round(descGrowTimer.pane.descCap)
            + " rows=" + descGrowTimer.pane.descListRows)
            driver.complete()
        }
    }
    // The log has to be on screen and laid out before the bar above it has a place to be measured from.
    SampleTimer {
        id: splitRefuseTimer
        onTriggered: {
            if (!page.commandsOpen || !page.commandsShown)
                return
            splitRefuseTimer.stop()
            page.reportDividerRefusal(AppBackend.autoActArg)
            renderedBarrier.begin()
        }
    }
}

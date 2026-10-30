pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Verbs for the stand-in for the tab in front (`TabPin`) and the strip's travel to the row it stands for — by
/// pressing the stand-in, or by opening a repository into a strip standing elsewhere (デザイン規約 §タブの所作).
/// Apart from `WindowTabActs`, which is what a hand does to a tab (carry, close, measure). Built by
/// `WindowAutoActDriver`.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar
    required property TabProbe tabProbe

    /// Stages the strip for `op`: the window on its floor so the strip overflows (`floor`), tab `front` in front
    /// (`front`), and the run sent to whichever end leaves it off screen (`run`) — the first three stages of every
    /// verb below. Answers true once the stand-in stands; each earlier beat puts at most one of those inputs in.
    ///
    /// The floor, because whether N tabs overflow depends on the installed fonts (`tab-widths` differs per OS), as
    /// for `tab-edge`. Nothing here closes a tab or takes the page down, so every precondition is read on every beat
    /// (app-ui.md §UI 自動化). `verb` is the diagnostic's prefix, so the crowding line names which verb stalled.
    function staged(op, front, verb) {
        // The stand-in is drawn off the front tab's item, and a row the model just gained has none until the next
        // layout.
        if (acts.pageRepeater.count < 2 || acts.tabProbe.tabItemCount() !== acts.pageRepeater.count)
            return false
        // And the page under the strip settled, so the photograph holds a settled repository (the switch below
        // builds a new page).
        const page = acts.window.curPage
        if (page === null || page.pageTab.state !== "open"
                || !page.pageWorktree.loaded || page.pageGraph.finishCount === 0)
            return false
        if (op.at("start")) {
            op.input("floor", () => {
                acts.window.width = Math.ceil(acts.window.floorWidth)
            })
            return false
        }
        // A strip that fits has no edge for the stand-in to ride; which width crowds it cannot be assumed, so
        // wait on the strip's own word.
        if (!acts.topBar.bandTabScrolls) {
            // Still uncrowded once the floor has laid out (the band's numbers standing across a polish pass): nothing left
            // to wait for, so say the band's numbers to choose the tab count against.
            const band = verb + " crowded=false tabs=" + acts.pageRepeater.count
                         + " content=" + Math.round(acts.topBar.bandTabContent)
                         + " view=" + Math.round(acts.topBar.bandTabsWidth)
                         + " windowW=" + Math.round(acts.window.width)
                         + " floorW=" + Math.ceil(acts.window.floorWidth)
            if (op.still("band", band))
                op.note(band)
            return false
        }
        if (op.at("floor")) {
            op.input("front", () => acts.tabsModel.setCurrentIndex(front))
            return false
        }
        if (acts.tabsModel.currentIndex !== front)
            return false
        if (op.at("front")) {
            // The strip's answer is the press's: a strip with nowhere to travel was never sent, and is asked again.
            op.input("run", () => acts.topBar.sendTabRunAway())
            return false
        }
        return acts.topBar.tabPinShown
    }

    /// The tab `op` asked for arrived, read now: one tab more than it `had`, its item laid out, the strip still with
    /// the tab whole and the stand-in gone, and the page opened.
    function arrived(op) {
        const page = acts.window.curPage
        return { "tab": acts.pageRepeater.count === op.witnessed("had") + 1,
                 "item": acts.tabProbe.tabItemCount() === acts.pageRepeater.count,
                 "still": !acts.topBar.tabRunTravelling,
                 "whole": acts.topBar.frontTabWhole,
                 "pinGone": !acts.topBar.tabPinShown,
                 "open": page !== null && page.pageTab.state === "open",
                 "worktree": page !== null && page.pageWorktree.loaded,
                 "graph": page !== null && page.pageGraph.finishCount !== 0,
                 "page": PageSettled.windowSettled(window) }
    }

    // PGG_AUTO_ACT=tab-pin: the stand-in at the edge the tab in front went out of. The argument is that tab's index —
    // the first (default) rides the left edge, the last the right. `onScreen=false` is said because a front tab
    // genuinely at the edge frames the same.
    WindowStage {
        id: tabPinTimer
        name: "tab_pin"
        order: ["floor", "front", "run"]
        running: Harness.autoAct === "tab-pin"
        readonly property int front: Number(Harness.autoActArg || 0)
        onTriggered: {
            // The staging first, on its own: its diagnostic is the beat's one line while it waits.
            if (!acts.staged(tabPinTimer, tabPinTimer.front, "tab_pin"))
                return
            // The line is judged as one run of words (`verify/verbs.rs`), so the order is the table's, with `left=` —
            // fixed only by the default argument — at its end. `kept=` / `crushed=`: a name cut away to the mark
            // photographs as a short name would — on the stand-in (`TabPin.nameKept`) and on the rows behind it,
            // which no picture here holds.
            tabPinTimer.complete({
                "name": "tab_pin_ready",
                "when": () => ({ "page": PageSettled.windowSettled(window) }),
                "report": () => "tab_pin kept=" + acts.topBar.tabPinNameKept
                                + " crushed=" + acts.tabProbe.tabNamesCrushed()
                                + " stood=" + acts.topBar.tabPinShown
                                + " onScreen=" + acts.topBar.frontTabWhole
                                + " left=" + acts.topBar.tabPinRidesLeft
                                + " at=" + tabPinTimer.front
                                + " run=" + acts.topBar.runOffset()
                                + " tabs=" + acts.pageRepeater.count
                                + " active=" + acts.tabsModel.currentIndex })
        }
    }

    // PGG_AUTO_ACT=tab-pin-go: the stand-in pressed; the strip travels until its row is whole and the stand-in steps
    // aside. `travelled=` (`from=` → `run=`) is said because a strip that was never sent away frames the same.
    WindowStage {
        id: tabPinGoTimer
        name: "tab_pin_go"
        order: ["floor", "front", "run", "pressed"]
        running: Harness.autoAct === "tab-pin-go"
        readonly property int front: Number(Harness.autoActArg || 0)
        onTriggered: {
            // The staging gates the press only: read again afterwards, it would wait for the stand-in this verb takes
            // away (app-ui.md §UI 自動化).
            if (!tabPinGoTimer.at("pressed")) {
                if (!acts.staged(tabPinGoTimer, tabPinGoTimer.front, "tab_pin_go"))
                    return
                // The press is its own answer: a stand-in not there takes none, and is pressed again. Where the run
                // stood before it is kept once it took.
                tabPinGoTimer.input("pressed", () => {
                    const from = acts.topBar.runOffset()
                    if (!acts.topBar.pressTabPin())
                        return false
                    tabPinGoTimer.witness("from", from)
                })
                return
            }
            // The travel takes a moment (`TabStrip.showFrontTab`); a mid-flight shot is neither answer, so wait for
            // its end.
            // `crushed=` as in `tab-pin`, for the tab in front this picture holds at the crowded strip's width.
            tabPinGoTimer.complete({
                "name": "tab_pin_go",
                "when": () => ({ "still": !acts.topBar.tabRunTravelling,
                                 "whole": acts.topBar.frontTabWhole,
                                 "pinGone": !acts.topBar.tabPinShown,
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => "tab_pin_go crushed=" + acts.tabProbe.tabNamesCrushed()
                                + " gone=" + !acts.topBar.tabPinShown
                                + " onScreen=" + acts.topBar.frontTabWhole
                                + " travelled=" + (acts.topBar.runOffset() !== tabPinGoTimer.witnessed("from"))
                                + " at=" + tabPinGoTimer.front
                                + " from=" + tabPinGoTimer.witnessed("from")
                                + " run=" + acts.topBar.runOffset()
                                + " tabs=" + acts.pageRepeater.count })
        }
    }

    // PGG_AUTO_ACT=tab-open-go: a repository opened into a crowded strip standing away from where the new tab lands;
    // the band travels to it and the stand-in steps aside (デザイン規約 §タブの所作). Staged as above with the first
    // tab in front, so the run is at the far end the new tab arrives at. The argument is a repository the strip has
    // not got — everything handed to the run at startup is already in it. `travelled=` is said because a strip
    // already sitting on that end frames the same.
    WindowStage {
        id: tabOpenGoTimer
        name: "tab_open_go"
        order: ["floor", "front", "run", "opened"]
        running: Harness.autoAct === "tab-open-go"
        onTriggered: {
            if (!tabOpenGoTimer.at("opened")) {
                // The staging gates the opening only, as in `tab-pin-go`. Where the run stood and how many tabs there
                // were as the repository was asked for, kept.
                if (!acts.staged(tabOpenGoTimer, 0, "tab_open_go") || Harness.autoActArg === "")
                    return
                tabOpenGoTimer.input("opened", () => {
                    tabOpenGoTimer.witness("from", acts.topBar.runOffset())
                    tabOpenGoTimer.witness("had", acts.pageRepeater.count)
                    acts.tabsModel.openRepositoryPath(Harness.autoActArg)
                })
                return
            }
            // The new tab's item comes with the next layout, and until then the strip answers for the tabs it
            // already had. And its page opened, so the photograph is the page the tab was opened for.
            if (!tabOpenGoTimer.complete({
                    "name": "tab_open_go",
                    "when": () => acts.arrived(tabOpenGoTimer),
                    "report": () => "tab_open_go arrived=" + acts.topBar.frontTabWhole
                                    + " gone=" + !acts.topBar.tabPinShown
                                    + " travelled=" + (acts.topBar.runOffset() !== tabOpenGoTimer.witnessed("from"))
                                    + " from=" + tabOpenGoTimer.witnessed("from")
                                    + " run=" + acts.topBar.runOffset()
                                    + " tabs=" + acts.pageRepeater.count
                                    + " active=" + acts.tabsModel.currentIndex }))
                // The strip's account of the ask when it changes, so a run stuck here says whether the ask was
                // answered and where a travel would go.
                tabOpenGoTimer.note("tab_open_go strip " + acts.topBar.frontAskAccount())
        }
    }

    // PGG_AUTO_ACT=tab-open-moved-on: the same arrival, then a press anywhere (`Main.pressLandedAnywhere`, the
    // handler's own body), then the run narrowing under the new tab. The press ends the ask, so the strip stays put
    // and the stand-in comes up for the cut tab (デザイン規約 §タブの所作「帯自身の所作は頼みの外」). Narrowed by
    // widening the window off its floor and giving the width back, so the run ends as the staging's own.
    WindowStage {
        id: tabMovedOnTimer
        name: "tab_open_moved_on"
        order: ["floor", "front", "run", "opened", "pressed", "widened", "narrowed"]
        running: Harness.autoAct === "tab-open-moved-on"
        onTriggered: {
            const op = tabMovedOnTimer
            if (op.at("start") || op.at("floor") || op.at("front") || op.at("run")) {
                if (!acts.staged(op, 0, "tab_open_moved_on") || Harness.autoActArg === "")
                    return
                op.input("opened", () => {
                    op.witness("had", acts.pageRepeater.count)
                    acts.tabsModel.openRepositoryPath(Harness.autoActArg)
                })
                return
            }
            if (op.at("opened")) {
                if (!Awaited.all("tab_open_moved_on", acts.arrived(op))) {
                    // The ask's account meanwhile, as in `tab-open-go`.
                    op.note("tab_open_moved_on strip " + acts.topBar.frontAskAccount())
                    return
                }
                // The press anywhere, then the window off its floor, in the one beat — the run's width with the
                // window on its floor kept as it widens.
                op.input("pressed", () => acts.window.pressLandedAnywhere())
                op.input("widened", () => {
                    op.witness("floorRun", acts.topBar.bandTabRun)
                    acts.window.width = Math.ceil(acts.window.floorWidth) + 120
                })
                return
            }
            if (op.at("widened")) {
                if (acts.topBar.bandTabRun <= op.witnessed("floorRun") || acts.topBar.tabRunTravelling)
                    return
                op.input("narrowed", () => {
                    op.witness("from", acts.topBar.runOffset())
                    acts.window.width = Math.ceil(acts.window.floorWidth)
                })
                return
            }
            // Back at the staging's run: settled either way — standing with the stand-in up, or travelled until the
            // tab is whole — so the report can judge which.
            op.complete({
                "name": "tab_open_moved_on_narrowed",
                "when": () => ({ "narrow": acts.topBar.bandTabRun <= op.witnessed("floorRun"),
                                 "still": !acts.topBar.tabRunTravelling,
                                 "standing": acts.topBar.tabPinShown || acts.topBar.frontTabWhole }),
                "report": () => {
                    const account = acts.topBar.frontAskAccount()
                    return "tab_open_moved_on travelled=" + (acts.topBar.runOffset() !== op.witnessed("from"))
                           + " stood=" + acts.topBar.tabPinShown
                           + " ask=" + (account.indexOf("ask=true") === 0)
                           + " from=" + op.witnessed("from")
                           + " run=" + acts.topBar.runOffset()
                           + " strip " + account
                } })
        }
    }
}

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

    /// Staging progress, shared by the timers below: only one verb runs in a process.
    property bool floorSet: false
    property bool frontSet: false
    property bool runSent: false
    /// The crowding wait: a tick spent on the floor, and its diagnostic said.
    property bool settling: false
    property bool told: false

    /// Stages the strip: the window on its floor so the strip overflows, tab `front` in front, and the run sent to
    /// whichever end leaves it off screen. Answers true once the stand-in stands; each earlier tick advances one step.
    ///
    /// The floor, because whether N tabs overflow depends on the installed fonts (`tab-widths` differs per OS), as
    /// for `tab-edge`. Nothing here closes a tab or takes the page down, so every precondition is read on every tick
    /// (app-ui.md §UI 自動化). `verb` is the report prefix, so the crowding diagnostic names which verb stalled.
    function staged(front, verb) {
        // The stand-in is drawn off the front tab's item, and a row the model just gained has none until the next
        // layout.
        if (acts.pageRepeater.count < 2 || acts.tabProbe.tabItemCount() !== acts.pageRepeater.count) {
            Awaited.at(verb, "rows")
            return false
        }
        // And the page under the strip settled, so the photograph holds a settled repository (the switch below
        // builds a new page).
        const page = acts.window.curPage
        if (page === null || page.pageTab.state !== "open"
                || !page.pageWt.loaded || page.pageGraph.finishCount === 0) {
            Awaited.at(verb, "page")
            return false
        }
        if (!acts.floorSet) {
            acts.window.width = Math.ceil(acts.window.floorWidth)
            acts.floorSet = true
            Awaited.at(verb, "floor")
            return false
        }
        // A strip that fits has no edge for the stand-in to ride; which width crowds it cannot be assumed, so
        // wait on the strip's own word.
        if (!acts.topBar.bandTabScrolls) {
            // Still uncrowded a tick after the floor: nothing left to wait for, so say the band's numbers once — a
            // tick late, so the resize has laid out — to choose the tab count against.
            if (acts.settling && !acts.told) {
                acts.told = true
                Harness.report(verb + " crowded=false tabs=" + acts.pageRepeater.count
                                  + " content=" + Math.round(acts.topBar.bandTabContent)
                                  + " view=" + Math.round(acts.topBar.bandTabsWidth)
                                  + " windowW=" + Math.round(acts.window.width)
                                  + " floorW=" + Math.ceil(acts.window.floorWidth))
            }
            acts.settling = true
            Awaited.at(verb, "crowded")
            return false
        }
        if (!acts.frontSet) {
            acts.frontSet = true
            acts.tabsModel.setCurrentIndex(front)
            Awaited.at(verb, "front")
            return false
        }
        if (acts.tabsModel.currentIndex !== front) {
            Awaited.at(verb, "front-arrives")
            return false
        }
        if (!acts.runSent) {
            // Latched on the strip's answer: a strip with nowhere to travel was never sent, and counting it as sent
            // leaves the wait to the watchdog.
            acts.runSent = acts.topBar.sendTabRunAway()
            Awaited.at(verb, "run")
            return false
        }
        if (!acts.topBar.tabPinShown) {
            Awaited.at(verb, "pin")
            return false
        }
        return true
    }

    // PGG_AUTO_ACT=tab-pin: the stand-in at the edge the tab in front went out of. The argument is that tab's index —
    // the first (default) rides the left edge, the last the right. `onScreen=false` is said because a front tab
    // genuinely at the edge frames the same.
    SampleTimer {
        id: tabPinTimer
        running: Harness.autoAct === "tab-pin"
        readonly property int front: Number(Harness.autoActArg || 0)
        onTriggered: {
            if (!acts.staged(tabPinTimer.front, "tab_pin"))
                return
            stop()
            // The line is judged as one run of words (`verify/verbs.rs`), so the order is the table's, with `left=` —
            // fixed only by the default argument — at its end. `kept=` / `crushed=`: a name cut away to the mark
            // photographs as a short name would — on the stand-in (`TabPin.nameKept`) and on the rows behind it,
            // which no picture here holds.
            Harness.report(
                "tab_pin kept=" + acts.topBar.tabPinNameKept
                + " crushed=" + acts.tabProbe.tabNamesCrushed()
                + " stood=" + acts.topBar.tabPinShown
                + " onScreen=" + acts.topBar.frontTabWhole
                + " left=" + acts.topBar.tabPinRidesLeft
                + " at=" + tabPinTimer.front
                + " run=" + acts.topBar.runOffset()
                + " tabs=" + acts.pageRepeater.count
                + " active=" + acts.tabsModel.currentIndex)
            acts.window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=tab-pin-go: the stand-in pressed; the strip travels until its row is whole and the stand-in steps
    // aside. `travelled=` (`from=` → `run=`) is said because a strip that was never sent away frames the same.
    SampleTimer {
        id: tabPinGoTimer
        running: Harness.autoAct === "tab-pin-go"
        readonly property int front: Number(Harness.autoActArg || 0)
        property bool pressed: false
        property int from: 0
        onTriggered: {
            // The staging gates the press only: read again afterwards, it would wait for the stand-in this verb takes
            // away (app-ui.md §UI 自動化).
            if (!tabPinGoTimer.pressed) {
                if (!acts.staged(tabPinGoTimer.front, "tab_pin_go"))
                    return
                tabPinGoTimer.from = acts.topBar.runOffset()
                tabPinGoTimer.pressed = acts.topBar.pressTabPin()
                return
            }
            // The travel takes a moment (`TabStrip.showFrontTab`); a mid-flight shot is neither answer, so wait for
            // its end.
            if (!Awaited.all("tab_pin_go", {
                    "still": !acts.topBar.tabRunTravelling,
                    "whole": acts.topBar.frontTabWhole,
                    "pinGone": !acts.topBar.tabPinShown
                }))
                return
            stop()
            // `crushed=` as in `tab-pin`, for the tab in front this picture holds at the crowded strip's width.
            Harness.report(
                "tab_pin_go crushed=" + acts.tabProbe.tabNamesCrushed()
                + " gone=" + !acts.topBar.tabPinShown
                + " onScreen=" + acts.topBar.frontTabWhole
                + " travelled=" + (acts.topBar.runOffset() !== tabPinGoTimer.from)
                + " at=" + tabPinGoTimer.front
                + " from=" + tabPinGoTimer.from
                + " run=" + acts.topBar.runOffset()
                + " tabs=" + acts.pageRepeater.count)
            acts.window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=tab-open-go: a repository opened into a crowded strip standing away from where the new tab lands;
    // the band travels to it and the stand-in steps aside (デザイン規約 §タブの所作). Staged as above with the first
    // tab in front, so the run is at the far end the new tab arrives at. The argument is a repository the strip has
    // not got — everything handed to the run at startup is already in it. `travelled=` is said because a strip
    // already sitting on that end frames the same.
    SampleTimer {
        id: tabOpenGoTimer
        running: Harness.autoAct === "tab-open-go"
        property bool opened: false
        property int from: 0
        property int had: 0
        property string account: ""
        onTriggered: {
            if (!tabOpenGoTimer.opened) {
                // The staging gates the opening only, as in `tab-pin-go`.
                if (!acts.staged(0, "tab_open_go") || Harness.autoActArg === "")
                    return
                tabOpenGoTimer.from = acts.topBar.runOffset()
                tabOpenGoTimer.had = acts.pageRepeater.count
                tabOpenGoTimer.opened = true
                acts.tabsModel.openRepositoryPath(Harness.autoActArg)
                return
            }
            // The new tab's item comes with the next layout, and until then the strip answers for the tabs it
            // already had. And its page opened, so the photograph is the page the tab was opened for.
            const page = acts.window.curPage
            if (!Awaited.all("tab_open_go", {
                    "tab": acts.pageRepeater.count === tabOpenGoTimer.had + 1,
                    "item": acts.tabProbe.tabItemCount() === acts.pageRepeater.count,
                    "still": !acts.topBar.tabRunTravelling,
                    "whole": acts.topBar.frontTabWhole,
                    "pinGone": !acts.topBar.tabPinShown,
                    "open": page !== null && page.pageTab.state === "open",
                    "worktree": page !== null && page.pageWt.loaded,
                    "graph": page !== null && page.pageGraph.finishCount !== 0
                })) {
                // The strip's account of the ask when it changes, so a run stuck here says whether the ask was
                // answered and where a travel would go.
                const account = acts.topBar.frontAskAccount()
                if (account !== tabOpenGoTimer.account) {
                    tabOpenGoTimer.account = account
                    Harness.report("tab_open_go strip " + account)
                }
                return
            }
            stop()
            Harness.report(
                "tab_open_go arrived=" + acts.topBar.frontTabWhole
                + " gone=" + !acts.topBar.tabPinShown
                + " travelled=" + (acts.topBar.runOffset() !== tabOpenGoTimer.from)
                + " from=" + tabOpenGoTimer.from
                + " run=" + acts.topBar.runOffset()
                + " tabs=" + acts.pageRepeater.count
                + " active=" + acts.tabsModel.currentIndex)
            acts.window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=tab-open-moved-on: the same arrival, then a press anywhere (`Main.pressLandedAnywhere`, the
    // handler's own body), then the run narrowing under the new tab. The press ends the ask, so the strip stays put
    // and the stand-in comes up for the cut tab (デザイン規約 §タブの所作「帯自身の所作は頼みの外」). Narrowed by
    // widening the window off its floor and giving the width back, so the run ends as the staging's own.
    SampleTimer {
        id: tabMovedOnTimer
        running: Harness.autoAct === "tab-open-moved-on"
        property string step: "stage"
        property int had: 0
        property real floorRun: 0
        property int from: 0
        property string account: ""
        onTriggered: {
            if (tabMovedOnTimer.step === "stage") {
                if (!acts.staged(0, "tab_open_moved_on") || Harness.autoActArg === "")
                    return
                tabMovedOnTimer.had = acts.pageRepeater.count
                tabMovedOnTimer.step = "arrive"
                acts.tabsModel.openRepositoryPath(Harness.autoActArg)
                return
            }
            if (tabMovedOnTimer.step === "arrive") {
                const page = acts.window.curPage
                if (!Awaited.all("tab_open_moved_on", {
                        "tab": acts.pageRepeater.count === tabMovedOnTimer.had + 1,
                        "item": acts.tabProbe.tabItemCount() === acts.pageRepeater.count,
                        "still": !acts.topBar.tabRunTravelling,
                        "whole": acts.topBar.frontTabWhole,
                        "pinGone": !acts.topBar.tabPinShown,
                        "open": page !== null && page.pageTab.state === "open",
                        "worktree": page !== null && page.pageWt.loaded,
                        "graph": page !== null && page.pageGraph.finishCount !== 0
                    })) {
                    // The ask's account meanwhile, as in `tab-open-go`.
                    const account = acts.topBar.frontAskAccount()
                    if (account !== tabMovedOnTimer.account) {
                        tabMovedOnTimer.account = account
                        Harness.report("tab_open_moved_on strip " + account)
                    }
                    return
                }
                acts.window.pressLandedAnywhere()
                tabMovedOnTimer.floorRun = acts.topBar.bandTabRun
                acts.window.width = Math.ceil(acts.window.floorWidth) + 120
                tabMovedOnTimer.step = "wide"
                Awaited.at("tab_open_moved_on", "wide")
                return
            }
            if (tabMovedOnTimer.step === "wide") {
                if (acts.topBar.bandTabRun <= tabMovedOnTimer.floorRun || acts.topBar.tabRunTravelling)
                    return
                tabMovedOnTimer.from = acts.topBar.runOffset()
                acts.window.width = Math.ceil(acts.window.floorWidth)
                tabMovedOnTimer.step = "narrow"
                Awaited.at("tab_open_moved_on", "narrow")
                return
            }
            // Back at the staging's run: settled either way — standing with the stand-in up, or travelled until the
            // tab is whole — so the report can judge which.
            if (acts.topBar.bandTabRun > tabMovedOnTimer.floorRun || acts.topBar.tabRunTravelling
                    || !(acts.topBar.tabPinShown || acts.topBar.frontTabWhole))
                return
            stop()
            const account = acts.topBar.frontAskAccount()
            Harness.report(
                "tab_open_moved_on travelled=" + (acts.topBar.runOffset() !== tabMovedOnTimer.from)
                + " stood=" + acts.topBar.tabPinShown
                + " ask=" + (account.indexOf("ask=true") === 0)
                + " from=" + tabMovedOnTimer.from
                + " run=" + acts.topBar.runOffset()
                + " strip " + account)
            acts.window.finishAutoAct()
        }
    }
}

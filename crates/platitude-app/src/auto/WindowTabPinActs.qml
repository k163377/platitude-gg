pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The stand-in for the tab in front, and the strip travelling to the row it stands for: the two halves of one rule,
/// read as a pair (`TabPin`).
///
/// Its own file rather than more of `WindowTabActs`: that one is about what a hand does to a tab — carrying it,
/// closing it, measuring what the strip made of it — and this is about the one thing the strip draws when a tab is
/// nowhere to be seen. Built by `WindowAutoActDriver`, which hands down the parts of the window these reach into.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is never
// given a size.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar
    /// The harness's own window onto the strip's laid-out tabs (`auto/TabProbe.qml`).
    required property TabProbe tabProbe

    /// How far the staging below has got. Only one verb runs in a process, so the two timers share it.
    property bool floorSet: false
    property bool frontSet: false
    property bool runSent: false

    /// The strip, put where both verbs start from: the window down on its floor so the strip has to overflow, the tab
    /// the run named brought to the front, and the run sent to whichever end leaves that tab off screen. Answers true
    /// once the stand-in is standing; every earlier tick advances the staging by one step and answers false.
    ///
    /// The window goes down on its floor because whether a given number of tabs overflows at all is a question about
    /// the installed fonts and the band's own furniture — `tab-widths` answers it differently on each OS — and this
    /// needs a strip that overflows on every machine (`tab-edge` is staged the same way and for the same reason).
    ///
    /// Every precondition here survives what the staging does to it, so all of them are read on every tick: nothing
    /// below closes a tab, empties the strip or takes the page down, and the one thing that does move — which tab is
    /// in front — is waited for by name rather than assumed (規約 §UI 自動化の因果性).
    function staged(front) {
        // Every row standing in the strip, not merely open: the stand-in is drawn off the item the view built for the
        // tab in front, and a row the model has only just gained has none until the next layout.
        if (acts.pageRepeater.count < 2 || acts.tabProbe.tabItemCount() !== acts.pageRepeater.count)
            return false
        // And the page under the strip settled, so what is photographed underneath is a repository rather than one
        // still opening — the switch below builds a new page, and an unfinished one frames the same either way.
        const page = acts.window.curPage
        if (page === null || page.pageTab.state !== "open"
                || !page.pageWt.loaded || page.pageGraph.finishCount === 0)
            return false
        if (!acts.floorSet) {
            acts.window.width = Math.ceil(acts.window.floorWidth)
            acts.floorSet = true
            return false
        }
        // A strip that fits has no edge for a stand-in to ride. The resize is what makes one, and the strip itself
        // says when that has taken — no width of its own is waited on, because which width crowds a strip is the
        // thing this cannot assume.
        if (!acts.topBar.bandTabScrolls)
            return false
        if (!acts.frontSet) {
            acts.frontSet = true
            acts.tabsModel.setCurrentIndex(front)
            return false
        }
        if (acts.tabsModel.currentIndex !== front)
            return false
        if (!acts.runSent) {
            // Latched on the strip's answer, the way every other hook here is: a strip with nowhere to travel was
            // never sent anywhere, and reporting it as though it had been leaves the wait to the watchdog.
            acts.runSent = acts.topBar.sendTabRunAway()
            return false
        }
        return acts.topBar.tabPinShown
    }

    // PG_AUTO_ACT=tab-pin: the stand-in, standing at the edge the tab in front went out of. The argument names which
    // tab that is — with the first one (the default) the strip is sent to its far end and the stand-in rides the left
    // edge; with the last, the other way about.
    //
    // The picture holds the stand-in but not the rule behind it: a strip whose front tab is genuinely at the edge
    // frames exactly like one standing in for a tab that is nowhere on it, so `onScreen=false` has to be said out loud.
    SampleTimer {
        id: tabPinTimer
        running: AppBackend.autoAct === "tab-pin"
        readonly property int front: Number(AppBackend.autoActArg || 0)
        onTriggered: {
            if (!acts.staged(tabPinTimer.front))
                return
            stop()
            // The two halves of the rule first and next to each other: the line is judged as a run of words, and a
            // reading put between them is a reading the table would have to spell out to get past
            // (`verify/verbs.rs`). Which edge it took comes after, because only the default argument fixes it.
            AppBackend.report(
                "tab_pin stood=" + acts.topBar.tabPinShown
                + " onScreen=" + acts.topBar.frontTabWhole
                + " left=" + acts.topBar.tabPinRidesLeft
                + " at=" + tabPinTimer.front
                + " run=" + acts.topBar.runOffset()
                + " tabs=" + acts.pageRepeater.count
                + " active=" + acts.tabsModel.currentIndex)
            acts.window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=tab-pin-go: and the press that takes it away again. The strip travels until the row it stood for is
    // whole on screen, and the stand-in steps aside at the end of that travel.
    //
    // Nothing here is a picture: a settled strip with its front tab in view is the same photograph whether it
    // travelled there or was never sent away at all. `travelled=` is the strip having moved under the press —
    // `from=` and `run=` are the two ends of it — and `gone=` is the stand-in's own answer to having arrived.
    SampleTimer {
        id: tabPinGoTimer
        running: AppBackend.autoAct === "tab-pin-go"
        readonly property int front: Number(AppBackend.autoActArg || 0)
        property bool pressed: false
        property int from: 0
        onTriggered: {
            // The staging is a precondition of the press and of nothing else: read again afterwards it ends in "the
            // stand-in is up", which is the very thing this verb takes away — and a verb waiting on its own answer
            // being undone waits for the watchdog (規約 §UI 自動化の因果性; measured before this branch was written).
            if (!tabPinGoTimer.pressed) {
                if (!acts.staged(tabPinGoTimer.front))
                    return
                tabPinGoTimer.from = acts.topBar.runOffset()
                tabPinGoTimer.pressed = acts.topBar.pressTabPin()
                return
            }
            // The travel is quick rather than instant (`TabStrip.showFrontTab`), so what is waited for is its end:
            // a shot taken mid-flight holds a strip halfway to somewhere, which is neither answer.
            if (acts.topBar.tabRunTravelling || !acts.topBar.frontTabWhole || acts.topBar.tabPinShown)
                return
            stop()
            AppBackend.report(
                "tab_pin_go gone=" + !acts.topBar.tabPinShown
                + " onScreen=" + acts.topBar.frontTabWhole
                + " travelled=" + (acts.topBar.runOffset() !== tabPinGoTimer.from)
                + " at=" + tabPinGoTimer.front
                + " from=" + tabPinGoTimer.from
                + " run=" + acts.topBar.runOffset()
                + " tabs=" + acts.pageRepeater.count)
            acts.window.finishAutoAct()
        }
    }
}

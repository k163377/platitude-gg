pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The stand-in for the tab in front, and the strip travelling to the row it stands for: the two halves of one rule,
/// read as a pair (`TabPin`). And the third way that travel is asked for — a repository opened into a strip standing
/// somewhere else (デザイン規約 §タブの所作), which is the same arrival by another road.
///
/// Its own file: `WindowTabActs` is about what a hand does to a tab — carrying it,
/// closing it, measuring what the strip made of it — and this is about the one thing the strip draws when a tab is
/// nowhere to be seen. Built by `WindowAutoActDriver`, which hands down the parts of the window these reach into.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar
    /// The harness's own window onto the strip's laid-out tabs (`auto/TabProbe.qml`).
    required property TabProbe tabProbe

    /// How far the staging below has got. Only one verb runs in a process, so the timers below share it.
    property bool floorSet: false
    property bool frontSet: false
    property bool runSent: false
    /// The crowding wait's own two: one tick spent with the floor in place, and the diagnostic for it said.
    property bool settling: false
    property bool told: false

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
    /// in front — is waited for by name (規約 §UI 自動化の因果性).
    ///
    /// `verb` is the prefix the caller reports under, wanted only for the diagnostic the crowding wait writes: the
    /// three verbs staged here go quiet in the same place, and the line has to name which one did.
    function staged(front, verb) {
        // Every row standing in the strip: the stand-in is drawn off the item the view built for the
        // tab in front, and a row the model has only just gained has none until the next layout.
        if (acts.pageRepeater.count < 2 || acts.tabProbe.tabItemCount() !== acts.pageRepeater.count) {
            Awaited.at(verb, "rows")
            return false
        }
        // And the page under the strip settled, so what is photographed underneath is a settled repository
        // — the switch below builds a new page, and an unfinished one frames the same either way.
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
        // A strip that fits has no edge for a stand-in to ride. The resize is what makes one, and what is
        // waited on is the strip's own word, because which width crowds a strip is the thing this cannot
        // assume.
        if (!acts.topBar.bandTabScrolls) {
            // A strip still uncrowded a tick after the floor took is one this staging cannot carry: the wait above has
            // nothing left to wait for, and the watchdog that ends such a run names the verb. Said once,
            // and a tick late so the resize has laid out — the numbers the count has to be chosen against
            // are the band's own, and they differ per OS (`tab-widths`).
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
            // Latched on the strip's answer, the way every other hook here is: a strip with nowhere to travel was
            // never sent anywhere, and reporting it as though it had been leaves the wait to the watchdog.
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

    // PGG_AUTO_ACT=tab-pin: the stand-in, standing at the edge the tab in front went out of. The argument names which
    // tab that is — with the first one (the default) the strip is sent to its far end and the stand-in rides the left
    // edge; with the last, the other way about.
    //
    // The picture holds the stand-in alone: a strip whose front tab is genuinely at the edge
    // frames exactly like one standing in for a tab that is nowhere on it, so `onScreen=false` has to be said out loud.
    SampleTimer {
        id: tabPinTimer
        running: Harness.autoAct === "tab-pin"
        readonly property int front: Number(Harness.autoActArg || 0)
        onTriggered: {
            if (!acts.staged(tabPinTimer.front, "tab_pin"))
                return
            stop()
            // The two halves of the rule first and next to each other: the line is judged as a run of words, and a
            // reading put between them is a reading the table would have to spell out to get past
            // (`verify/verbs.rs`). Which edge it took comes after, because only the default argument fixes it.
            //
            // `kept=` stands in front of the pair: the stand-in is dressed as the tab it stands for and drawn at
            // that tab's width, so the one thing left for it to get wrong is what it says in that width — and a
            // name cut away to the mark photographs as a short name would (`TabPin.nameKept`).
            // `crushed=` is the same question asked of the rows behind it, which no picture of this verb holds at all.
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

    // PGG_AUTO_ACT=tab-pin-go: and the press that takes it away again. The strip travels until the row it stood for is
    // whole on screen, and the stand-in steps aside at the end of that travel.
    //
    // The report carries this: a settled strip with its front tab in view is the same photograph whether it
    // travelled there or was never sent away at all. `travelled=` is the strip having moved under the press —
    // `from=` and `run=` are the two ends of it — and `gone=` is the stand-in's own answer to having arrived.
    SampleTimer {
        id: tabPinGoTimer
        running: Harness.autoAct === "tab-pin-go"
        readonly property int front: Number(Harness.autoActArg || 0)
        property bool pressed: false
        property int from: 0
        onTriggered: {
            // The staging is a precondition of the press and of nothing else: read again afterwards it ends in "the
            // stand-in is up", which is the very thing this verb takes away — and a verb waiting on its own answer
            // being undone waits for the watchdog (規約 §UI 自動化の因果性).
            if (!tabPinGoTimer.pressed) {
                if (!acts.staged(tabPinGoTimer.front, "tab_pin_go"))
                    return
                tabPinGoTimer.from = acts.topBar.runOffset()
                tabPinGoTimer.pressed = acts.topBar.pressTabPin()
                return
            }
            // The travel takes a moment (`TabStrip.showFrontTab`), so what is waited for is its end:
            // a shot taken mid-flight holds a strip halfway to somewhere, which is neither answer. Named, as the
            // staging's steps are, so a run that stands here until its ceiling says which of them it was.
            if (!Awaited.all("tab_pin_go", {
                    "still": !acts.topBar.tabRunTravelling,
                    "whole": acts.topBar.frontTabWhole,
                    "pinGone": !acts.topBar.tabPinShown
                }))
                return
            stop()
            // `crushed=` first, as `tab-pin` has it: this is the verb whose picture holds the tab in front at the
            // width the crowded strip left it, and a name drawn away to the mark is the one thing in that picture
            // that reads as a short name (`TabProbe.tabNamesCrushed`).
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

    // PGG_AUTO_ACT=tab-open-go: a repository opened into a strip that is already crowded and standing away from where
    // the new tab lands. The band travels to it, and the stand-in it had up steps aside (デザイン規約 §タブの所作).
    //
    // Staged exactly as the two above are, with the first tab in front and the run sent to its far end — which is the
    // end a tab opened now arrives at. The argument is the repository to open: one the strip has not got, built
    // beside the twelve it came up with, because everything handed to the run at startup is already in the strip.
    //
    // The report answers this one too: a strip standing on its newest tab frames the same whether it travelled
    // there or was sitting on that end all along. `travelled=` is the band having moved under the ask, and
    // `arrived=` the tab it moved for being whole in the run at the end of it.
    SampleTimer {
        id: tabOpenGoTimer
        running: Harness.autoAct === "tab-open-go"
        property bool opened: false
        property int from: 0
        property int had: 0
        onTriggered: {
            if (!tabOpenGoTimer.opened) {
                // The staging is the precondition of the opening and of nothing else (`tab-pin-go` above): read again
                // afterwards it ends in "the stand-in is up", which this verb is here to take away.
                if (!acts.staged(0, "tab_open_go") || Harness.autoActArg === "")
                    return
                tabOpenGoTimer.from = acts.topBar.runOffset()
                tabOpenGoTimer.had = acts.pageRepeater.count
                tabOpenGoTimer.opened = true
                acts.tabsModel.openRepositoryPath(Harness.autoActArg)
                return
            }
            // The tab has to be standing in the strip before the travel it causes can be waited on: the item for a row
            // the model has only just gained arrives with the next layout, and until then the strip is answering for
            // the twelve it already had. And the repository under it opened, so what is photographed is the page the
            // tab was opened for. Each term named, so a run that stands here until its ceiling says which one it was.
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
                }))
                return
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
}

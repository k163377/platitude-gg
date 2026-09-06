pragma ComponentBehavior: Bound

import QtQuick
// For the attached `ToolTip` alone (`tab-name`; rules-refs/app-ui.md carries what an unimported attached type answers).
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

/// The tab strip's half of the window's PG_AUTO_ACT harness: opening a path that is already open, closing a
/// tab from the middle button, and every way a tab is carried along the strip or measured on it.
///
/// Built by `WindowAutoActDriver`, which is what `Main` builds when a verb was given; what these verbs act
/// on is handed down below, one property per part of the window they reach into.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is
// never given a size.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar
    /// The harness's own window onto the strip's laid-out tabs (`auto/TabProbe.qml`).
    required property TabProbe tabProbe
    required property Item mainUi
    required property Item gate

    // What remains after a middle-click is the output under test. Wait for the tab-model count edge rather than
    // allowing a fixed delay to stand in for it. The two tabs this verb needs are a precondition of the press and
    // nothing else: read again after it, they turn the verb's own answer — one tab fewer — into a wait nothing can end.
    SampleTimer {
        id: middleCloseTimer
        running: Harness.autoAct === "middle-close"
        property bool requested: false
        property int beforeCount: -1
        property string closedPath: ""
        onTriggered: {
            if (!middleCloseTimer.requested) {
                if (pageRepeater.count < 2)
                    return
                middleCloseTimer.beforeCount = pageRepeater.count
                const at = Number(Harness.autoActArg)
                middleCloseTimer.closedPath = tabProbe.tabPathAt(at)
                // Latched on the strip's answer rather than on the asking: the press has to land on an item, and the
                // row the model has just gained gets one with the layout.
                middleCloseTimer.requested = topBar.middleClickTab(at)
                return
            }
            // The strip is read back below, so it has to have caught up with the model before there is anything true to
            // say about which repository went and which is still standing.
            if (pageRepeater.count >= middleCloseTimer.beforeCount || tabProbe.tabItemCount() !== pageRepeater.count)
                return
            // And the tab that stayed has to be showing its repository: "the neighbour is still there" is the half of
            // this verb the picture carries, and a page still opening photographs the same whether it survived the
            // close or was never opened at all. The page driver's own baseline (AutoActDriver), asked of whichever page
            // the close left in front.
            const kept = window.curPage
            if (kept === null || kept.pageTab.state !== "open"
                    || !kept.pageWt.loaded || kept.pageGraph.finishCount === 0)
                return
            stop()
            // The verdict leads, and it is about the tab the press landed on: a count that merely fell would pass with
            // the wrong tab closed, and the titles cannot tell them apart.
            Harness.report("middle_close gone="
                              + !tabProbe.hasTabPath(middleCloseTimer.closedPath)
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // What the strip is holding after a tab was carried across it. The order is the output, and it is read off the
    // items rather than the model: the drag settles itself against where the tabs actually sit, so the walk is what
    // says the two agree. The tabs it needs are a precondition of the carry and are read in that branch alone —
    // afterwards, "the order is not the one it started as" is this verb's own answer.
    SampleTimer {
        id: tabDragTimer
        running: Harness.autoAct === "tab-drag"
        property bool requested: false
        property string before: ""
        property int from: 0
        property int to: 0
        onTriggered: {
            if (!tabDragTimer.requested) {
                // Every row standing in the strip, not merely open: the carry measures against the tabs' own places,
                // and a row the model has only just gained has none until the next layout.
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                // And the page under the strip settled, so that what is photographed underneath is a repository rather
                // than one still opening. A precondition of the carry and read nowhere else: the carry moves to the tab
                // it takes up, and a page that then has to open would turn this into a wait for something the verb
                // itself caused.
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                    return
                const asked = (Harness.autoActArg || "3:1").split(":")
                tabDragTimer.from = Number(asked[0])
                tabDragTimer.to = Number(asked[1])
                tabDragTimer.before = tabProbe.tabPaths()
                // Latched on the strip's answer, the way the middle click is: a carry that found no tab to take up
                // never happened, and reporting it as one would leave the wait to the watchdog.
                tabDragTimer.requested = topBar.dragTabTo(tabDragTimer.from, tabDragTimer.to)
                return
            }
            const paths = tabProbe.tabPaths()
            if (paths === tabDragTimer.before || tabProbe.tabItemCount() !== pageRepeater.count)
                return
            stop()
            // The verdict leads, and it is about the tab that was carried: an order that merely changed would pass
            // with any two tabs swapped, and every demo working tree is called the same thing in the picture.
            const was = tabDragTimer.before.split(",")
            Harness.report("tab_drag moved="
                              + (paths.split(",")[tabDragTimer.to] === was[tabDragTimer.from])
                              + " from=" + tabDragTimer.from
                              + " to=" + tabDragTimer.to
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + paths)
            window.finishAutoAct()
        }
    }

    // And the half of the carry that ends in no order at all: a tab drawn away from its own row while the hand is
    // still on it. The settled strip photographs the same whether it was ever drawn under the hand or only ever
    // jumped between rows, so the offset the transform is carrying says itself.
    SampleTimer {
        id: tabHoldTimer
        running: Harness.autoAct === "tab-hold"
        property bool requested: false
        onTriggered: {
            if (!tabHoldTimer.requested) {
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                    return
                tabHoldTimer.requested = topBar.holdTabAt(Number(Harness.autoActArg || 0))
                return
            }
            // The tab has to be drawn off its row before there is a picture worth taking; nothing is waited for
            // afterwards, because a hand that has not let go is the whole state.
            const shift = topBar.heldTabShift()
            if (shift === 0)
                return
            stop()
            Harness.report("tab_hold lifted=true at=" + (Harness.autoActArg || 0)
                              + " shift=" + shift
                              + " active=" + tabsModel.currentIndex
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // The strip travelling under a tab held past the end of it — the half of the carry that reaches a place which was
    // not on screen when the hand took hold. The window goes down on its floor first: whether a given number of tabs
    // overflows at all is a question about the installed fonts and the band's own furniture (`tab-widths` answers it
    // differently on each OS), and this verb needs a strip that overflows on every machine.
    SampleTimer {
        id: tabEdgeTimer
        running: Harness.autoAct === "tab-edge"
        property bool sized: false
        property bool requested: false
        property int from: 0
        onTriggered: {
            if (!tabEdgeTimer.requested) {
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                    return
                if (!tabEdgeTimer.sized) {
                    window.width = Math.ceil(window.floorWidth)
                    tabEdgeTimer.sized = true
                    return
                }
                // A strip that fits has no end to travel to. The resize is what makes one, and the strip itself says
                // when that has taken — no width of its own is waited on, because which width crowds a strip is the
                // thing this cannot assume.
                if (!topBar.bandTabScrolls)
                    return
                tabEdgeTimer.from = Number(Harness.autoActArg || 0)
                tabEdgeTimer.requested = topBar.carryTabPastEnd(tabEdgeTimer.from)
                return
            }
            // Travelled to its far end, carrying the tab to the end of the order: the strip stops on its own bound,
            // and the tab passes every neighbour that slides under it on the way there.
            if (!topBar.runAtEnd() || topBar.heldTabIndex() !== pageRepeater.count - 1)
                return
            stop()
            topBar.dropCarriedTab()
            Harness.report("tab_edge landed=true from=" + tabEdgeTimer.from
                              + " run=" + topBar.runOffset()
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // Reopening an existing path must select its tab without adding one.
    SampleTimer {
        id: openAgainTimer
        running: Harness.autoAct === "open-again"
        property bool requested: false
        property string asked: ""
        property int beforeCount: -1
        /// Which tab the ask is meant to bring to the front, where the run left that to the strip. -1 when the run
        /// named a path of its own: which tab holds a folder is the question there, and the whole point is that the
        /// spelling does not say.
        property int wantIndex: -1
        onTriggered: {
            if (!openAgainTimer.requested) {
                if (pageRepeater.count === 0)
                    return
                // The spelling to ask with, when the run named none, is the strip's own — and the strip is a view: the
                // item for a row the model has just gained arrives with the next layout. Asking with the "" it answers
                // until then opens nothing, and nothing opened is what this verb's completion looks like — it went
                // green having asked for nothing at all (measured).
                const path = Harness.autoActArg !== "" ? Harness.autoActArg : tabProbe.tabPathAt(0)
                if (path === "")
                    return
                openAgainTimer.asked = path
                openAgainTimer.wantIndex = Harness.autoActArg !== "" ? -1 : 0
                openAgainTimer.beforeCount = pageRepeater.count
                openAgainTimer.requested = true
                tabsModel.openRepositoryPath(path)
                return
            }
            // Reopening the first tab from a strip standing on another one has somewhere to arrive: without that, "went
            // to the tab it already had" and "did nothing whatever" are the same report.
            if (pageRepeater.count !== openAgainTimer.beforeCount
                    || tabsModel.currentIndex < 0
                    || (openAgainTimer.wantIndex >= 0
                        && tabsModel.currentIndex !== openAgainTimer.wantIndex)
                    // The strip is read back below (`middle-close` above).
                    || tabProbe.tabItemCount() !== pageRepeater.count)
                return
            // And the tab it arrived at has to be showing its repository: which tab came to the front is what the
            // picture carries here, and a page still opening looks the same whichever one it is (`middle-close` above,
            // same baseline).
            const front = window.curPage
            if (front === null || front.pageTab.state !== "open"
                    || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                return
            stop()
            Harness.report("open_again tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " asked=" + openAgainTimer.asked
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // What a tab switch carries and what it drops — the two halves of one answer, so one verb reports both.
    //
    // The strip is walked in three landings: leave a mark on the tab in front, go to the other one and read what
    // arrived there, come back and read what was kept. Each landing waits for the page it is about to read to be
    // whole, because a page still opening answers every question here the same way an emptied one does.
    //
    // **The tab is what is waited on, not the row.** Rows renumber; `currentTabId` is the tab that is actually in
    // front (`TabsModel::current_tab_id`), and comparing against it is what makes "the switch has happened" a fact
    // rather than a guess. And the page read at each landing is a *different object* every time — the page in front is
    // built for the tab in front and taken down with it — so nothing here may be held across a landing but the words
    // themselves.
    SampleTimer {
        id: tabCarryTimer
        running: Harness.autoAct === "tab-carry"
        /// 0 = mark the first tab, 1 = read the second, 2 = read the first again.
        property int step: 0
        /// The tab the mark was left on, so the walk knows which landing it is at without counting rows.
        property int firstTab: -1
        /// Words no repository can produce, so finding them again cannot be anything but this page having kept them.
        readonly property string typed: "chore: words that outlived a tab switch"
        /// Read at the second landing and reported at the third: the layout followed the reader, the words did not.
        property bool folded: false
        property bool log: false
        property bool otherEmpty: false
        function whole(page) {
            return page !== null && page.pageTab.state === "open"
                   && page.pageWt.loaded && page.pageGraph.finishCount > 0
        }
        onTriggered: {
            const page = window.curPage
            if (!tabCarryTimer.whole(page))
                return
            if (tabCarryTimer.step === 0) {
                // Two tabs are what this verb is about, and they are a precondition of the first landing alone: read
                // again afterwards they would be read against a strip this verb has already moved.
                if (pageRepeater.count < 2)
                    return
                // A layout nobody starts in, so "it followed" cannot be read off a page that was already like this.
                page.foldByHand(true)
                page.commandsOpen = true
                page.pageWip.setMessage(tabCarryTimer.typed, "")
                tabCarryTimer.firstTab = tabsModel.currentTabId
                tabCarryTimer.step = 1
                tabsModel.setCurrentIndex(tabsModel.currentIndex === 0 ? 1 : 0)
                return
            }
            if (tabCarryTimer.step === 1) {
                if (tabsModel.currentTabId === tabCarryTimer.firstTab)
                    return
                tabCarryTimer.folded = page.sidebarCollapsed
                tabCarryTimer.log = page.commandsOpen
                tabCarryTimer.otherEmpty = page.pageWip.subjectText === ""
                                           && page.pageWip.bodyText === ""
                tabCarryTimer.step = 2
                tabsModel.setCurrentIndex(tabsModel.currentIndex === 0 ? 1 : 0)
                return
            }
            if (tabsModel.currentTabId !== tabCarryTimer.firstTab)
                return
            stop()
            // `sessions=` is the release itself, and the only thing here a picture cannot say: two tabs in the strip,
            // one repository in memory. `finishCount` being above zero on a page built after the switch is the other
            // side of the same coin — the graph read itself again from nothing.
            Harness.report("tab_carry tabs=" + pageRepeater.count
                              + " sessions=" + Harness.openSessionCount()
                              + " folded=" + tabCarryTimer.folded
                              + " log=" + tabCarryTimer.log
                              + " empty=" + tabCarryTimer.otherEmpty
                              + " back=" + (page.pageWip.subjectText === tabCarryTimer.typed)
                              // …and the pane holding them is the one on screen: words put back behind the commit
                              // details are only half of them being kept.
                              + " wip=" + page.wipShown)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=tab-widths: numbers for the band's reason — a strip that narrowed the wrong tabs comes out looking
    // like one that got it right. `widths=` is the answer, and `room=` says which of the two things the strip gives up
    // it is living on (the mark's room first, the names after).
    //
    // `crushed=` is what the widths cannot say: a tab is as wide as its name, so a name drawn away to the mark alone
    // leaves the strip's own numbers looking exactly as they should (`TabProbe.tabNamesCrushed`). Said first, where
    // the whole of what is judged here stands together (`verify/verbs.rs`).
    SampleTimer {
        id: tabWidthActTimer
        running: Harness.autoAct === "tab-widths"
        onTriggered: {
            // Every row standing in the strip, not merely open: both readings below walk the items the view built,
            // and a row the model has only just gained has none until the next layout — `widths=` would carry a 0
            // for it, and `crushed=` would answer for a strip it had not seen (規約 §UI 自動化の因果性).
            if (topBar.bandTabCount <= 0 || topBar.bandTabRun <= 0
                    || tabProbe.tabItemCount() !== topBar.bandTabCount)
                return
            stop()
            Harness.report(
                "tab_widths crushed=" + tabProbe.tabNamesCrushed()
                + " tabs=" + topBar.bandTabCount
                + " run=" + Math.round(topBar.bandTabRun)
                + " cap=" + Math.round(topBar.tabTitleCap)
                + " room=" + Math.round(topBar.tabMarkRoom)
                + " floor=" + topBar.tabTitleMinW
                + " ease=" + Math.round(topBar.tabTitleEaseW)
                + " max=" + topBar.tabTitleMaxW
                + " content=" + Math.round(topBar.bandTabContent)
                + " view=" + Math.round(topBar.bandTabsWidth)
                + " scrolls=" + topBar.bandTabScrolls
                + " widths=" + tabProbe.tabWidths())
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=tab-mark: the argument is which tab the hand is on — one that is not in front, or the run says
    // nothing the picture of any other verb does not already say.
    SampleTimer {
        id: tabMarkActTimer
        running: Harness.autoAct === "tab-mark"
        property bool requested: false
        property string beforeMarks: ""
        onTriggered: {
            const pointed = Number(Harness.autoActArg || 1)
            if (topBar.bandTabCount < pointed)
                return
            if (!tabMarkActTimer.requested) {
                tabMarkActTimer.requested = true
                tabMarkActTimer.beforeMarks = tabProbe.tabMarks()
                tabProbe.pointAtTab(pointed)
                return
            }
            if (tabProbe.tabMarks() === tabMarkActTimer.beforeMarks)
                return
            stop()
            Harness.report(
                "tab_marks tabs=" + topBar.bandTabCount
                + " current=" + tabsModel.currentIndex
                + " pointed=" + pointed
                + " marks=" + tabProbe.tabMarks())
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=tab-name: the names a strip of namesakes settled on, and the whole path the hand asks for on top of
    // them. The argument is which tab to point at. Both halves in one run because they are one question — what this tab
    // stands on — asked of the strip and then of the hover (デザイン規約 §タブの所作 / §hover のツールチップ).
    SampleTimer {
        id: tabNameActTimer
        running: Harness.autoAct === "tab-name"
        property bool requested: false
        readonly property int pointed: Number(Harness.autoActArg || 0)
        onTriggered: {
            // Every row has to have an item before the names are read: the strip's answer for a row the layout has not
            // reached yet is the empty string, which reads exactly like a name that came out blank.
            if (topBar.bandTabCount === 0 || tabProbe.tabItemCount() < topBar.bandTabCount)
                return
            if (!tabNameActTimer.requested) {
                tabNameActTimer.requested = true
                tabProbe.pointAtTab(tabNameActTimer.pointed)
                return
            }
            const tip = mainUi.ToolTip.toolTip
            // The tip is on a delay, so this waits it out rather than reading the moment after the hand landed.
            if (!tip.visible)
                return
            stop()
            // `unique=` rather than the names themselves as the judged claim: what a name comes out as is spelled with
            // the platform's own separator, and no two tabs reading alike is the rule either way (`verify/verbs.rs`).
            const titles = tabProbe.tabTitles()
            const names = titles.split(",")
            Harness.report(
                "tab_names tabs=" + topBar.bandTabCount
                + " pointed=" + tabNameActTimer.pointed
                + " unique=" + names.every((name, at) => names.indexOf(name) === at)
                + " tip=" + tip.visible
                + " titles=" + titles
                + " said=" + tip.text)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=solo: the harness holds the real lock on the config directory before starting this process, so the
    // picture is of the mechanism and not of a flag that imitates it.
    SampleTimer {
        id: soloActTimer
        running: Harness.autoAct === "solo"
        onTriggered: {
            if (!AppBackend.alreadyRunning || !gate.visible)
                return
            stop()
            Harness.report(
                "solo blocked=" + AppBackend.alreadyRunning
                + " held=" + (AppBackend.heldElsewhere !== "")
                + " gate=" + gate.visible
                + " main=" + mainUi.visible)
            window.finishAutoAct()
        }
    }
}

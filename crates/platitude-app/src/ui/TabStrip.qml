pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The strip. Placed by hand rather than by a Row so the leftover is measurable — it is both what the tabs may grow into
// and where the window is taken hold of. No TabBar: full geometry control is what puts the selected tab's underline
// exactly on the band's bottom edge with no styling leftovers beneath it.
Item {
    id: tabStrip

    required property var tabsModel
    /// Whether the band this strip sits in is the window's title bar. When it is, the strip keeps a run of band clear
    /// to take hold of (`tabs.grabRun`).
    property bool captionMerged: false
    /// The RepoPage of the active tab (null while no tab is open). The app menu's one live entry is the only thing here
    /// that asks anything of it.
    property var curPage: null
    /// The tab in front, as this strip's own list built it — pushed by that tab (`TabItemDelegate.frontChanged`)
    /// rather than looked up, because the item for a row the model has only just gained arrives with the next layout.
    /// Null while no tab is open. The stand-in at the edge is drawn off it, and the travel to it is measured off it.
    property Item frontTab: null

    /// Automation: the run the tabs were handed, and what they made of it. A picture cannot say which tabs gave way and
    /// which were left alone — every strip that fits looks like every other one — so the widths themselves are the
    /// answer (`PG_AUTO_ACT=tab-widths`).
    readonly property int tabCount: tabs.count
    readonly property real runAvail: tabs.runAvail
    readonly property real contentWidth: tabs.contentWidth
    /// What the strip itself came out to. A broken term in its width expression turns it NaN, and NaN draws as "no tabs
    /// at all" with nothing on stderr — this is the number that catches it.
    readonly property real tabsWidth: tabs.width
    /// Band the tabs may not grow into, and the run itself for `Main` to measure in scene coordinates.
    readonly property real grabRun: tabs.grabRun
    readonly property Item grabRunItem: grabArea
    /// Whether the ☰'s card is standing. The grab run gives up being the window's caption while it is, so that a press
    /// on the band's empty run reaches the scene and takes the card down (`WindowChrome.captionYielded`).
    readonly property bool appMenuOpen: appMenu.opened

    /// Automation: the stand-in for the tab in front (`PG_AUTO_ACT=tab-pin` / `tab-pin-go`) — whether it is standing,
    /// which edge it took, whether the row it stands for is whole on screen, and whether the strip is still travelling
    /// towards it. A picture of a scrolled strip reads the same whichever of the four is true. Read off the stand-in
    /// itself, so a binding that came apart answers with what is drawn rather than with what was asked of it.
    readonly property bool tabPinShown: tabPin.visible
    readonly property bool tabPinRidesLeft: tabPin.rideLeft
    readonly property bool frontTabWhole: tabPin.frontWhole
    readonly property bool runTravelling: tabRun.travelling

    /// The longest and shortest a name is drawn at (`TabMetrics`), aliased for the band, which reads the strip.
    readonly property int tabTitleMaxW: tabMetrics.titleMaxW
    readonly property int tabTitleMinW: tabMetrics.titleMinW
    /// The length a name stops being eased at, pushed rather than bound (`TabMetrics.titleEaseW`).
    property real tabTitleEaseW: 0
    /// What every tab's name is capped at right now — the strip's answer to how much room it was given
    /// (`settleTitleCap`).
    property real tabTitleCap: tabStrip.tabTitleMaxW
    /// What the strip would take with nothing cut (`settleTitleCap`), and the least it is ever laid out at. Two tabs,
    /// not one (規約 §ウィンドウの縁).
    property real tabsWantWidth: 0
    readonly property int tabStripFloorW:
        menuButton.width + plusButton.width + tabs.grabRun + 2 * (tabMetrics.tabFixedW + tabStrip.tabTitleMinW)

    signal openRepositoryRequested()
    signal cloneRepositoryRequested()
    signal settingsRequested()
    /// The grab-run moved or changed size in the strip's own layout. `Main` folds in the shifts this strip cannot see
    /// from here (the maximised inset, the window resizing) and reports the strip on.
    signal captionStripMoved()

    /// Automation: the ☰, pressed (`PG_AUTO_ACT=app-menu`). Put in at the button rather than at the card it opens, so
    /// what answers is the band's real wiring and not a second way in written for the run (`TopBar.stashNow`) — the
    /// toggle this verb is about lives on the button's own handler.
    function clickAppMenu() {
        menuButton.clicked()
    }

    /// Automation: the ☰'s `Clone repository…` row (`PG_AUTO_ACT=clone-*`). The row's own `triggered` — the signal a
    /// press on it emits — so the handler that runs is the row's, and everything it reaches from there is the wiring a
    /// hand goes through.
    function clickCloneRow() {
        cloneRow.triggered()
    }

    /// The left button moves to the tab — and leaves a hand on it that may go on to carry it (`takeTab`) — the middle
    /// one closes it (デザイン規約 §タブの所作). The real press and the smoke hook both come through here.
    function pressTab(index, id, button) {
        if (button === Qt.MiddleButton) {
            tabStrip.tabsModel.closeTab(id)
            return
        }
        // A press outranks a travel in flight, whatever it goes on to be: the carry that may follow sends the strip
        // itself (`TabCarry.driftRun`), and two hands on `contentX` is one of them drawing over the other.
        tabRun.halt()
        tabStrip.tabsModel.setCurrentIndex(index)
    }

    /// Automation: the tab at `index`, carried to `to` and set down (`PG_AUTO_ACT=tab-drag`). The carrying is a
    /// pointer's, which no headless run has; everything after it is the same road a hand takes. Carried to exactly
    /// where it comes to rest: its trailing edge on the far edge of the tab it is going to, or its leading edge on
    /// the near one — anywhere in that place passes every tab in between and none beyond it.
    function dragTabTo(from, to) {
        const tab = tabs.itemAtIndex(from)
        const dest = tabs.itemAtIndex(to)
        if (!tab || !dest || from === to)
            return false
        // A hand presses before it carries, and the press is what moves to the tab. A hook that let itself skip that
        // would be proving a gesture nobody can make.
        tabStrip.pressTab(from, tab.tab_id, Qt.LeftButton)
        tabCarry.takeTab(from, tab.width / 2)
        tabCarry.carryTo(from, to > from ? dest.x + dest.width - tab.width : dest.x)
        tabCarry.dropTab()
        return true
    }

    /// Automation: the tab at `index`, taken up and carried half its own width without being set down
    /// (`PG_AUTO_ACT=tab-hold`) — the one thing a settled strip cannot show.
    function holdTabAt(index) {
        const tab = tabs.itemAtIndex(index)
        if (!tab)
            return false
        tabStrip.pressTab(index, tab.tab_id, Qt.LeftButton)
        tabCarry.takeTab(index, tab.width / 2)
        // Towards the middle of the strip: the ends are where a carry stops, and the last tab carried further right
        // is drawn exactly where it already was.
        const toward = index + 1 < tabs.count ? tab.width / 2 : -tab.width / 2
        tabCarry.carryTo(index, tab.x + toward)
        return true
    }

    /// Automation: how far the tab in hand is actually drawn from its own row (`tab-hold`). Read off that tab, so a
    /// transform that came apart answers 0 — which is also what nothing being held answers.
    function heldTabShift() {
        const at = tabCarry.heldIndex()
        const tab = at < 0 ? null : tabs.itemAtIndex(at)
        return tab ? Math.round(tab.shiftShown) : 0
    }

    /// Automation: the tab at `index`, carried against the far end of the run and left there (`PG_AUTO_ACT=tab-edge`).
    /// Held past the run — the distance is the speed, so the hook names it in tab widths rather than in pixels.
    function carryTabPastEnd(index) {
        const tab = tabs.itemAtIndex(index)
        if (!tab)
            return false
        tabStrip.pressTab(index, tab.tab_id, Qt.LeftButton)
        tabCarry.takeTab(index, tab.width / 2)
        tabCarry.carryTab(index, tabs.mapToItem(null, tabs.width + tab.width, 0).x)
        return true
    }

    /// Automation: where the tab in hand sits, and letting go of it — both asked of the hand that has it (`tab-edge`).
    function heldIndex() {
        return tabCarry.heldIndex()
    }
    function dropTab() {
        tabCarry.dropTab()
    }

    /// The strip, travelled until the tab in front is whole on screen — what a press on the stand-in at the edge means
    /// (デザイン規約 §タブの所作; the travel itself is `TabRun.showTab`).
    function showFrontTab() {
        return tabRun.showTab(tabStrip.frontTab)
    }

    /// Automation: where the strip stands in its run and whether it has reached the far end (`tab-edge`), and the run
    /// sent away from the tab in front so the stand-in has to appear (`tab-pin`). All three are the run's own
    /// (`TabRun`); the band reaches them through the strip like everything else.
    function runAtEnd() { return tabRun.atEnd() }
    function runOffset() { return tabRun.offset() }
    function sendRunAway() { return tabRun.sendAway(tabStrip.frontTab) }

    /// Automation: the stand-in, pressed (`tab-pin-go`). Put in at its own signal — the press is a pointer's, which no
    /// headless run has, and everything past it is the road a hand takes.
    function pressTabPin() {
        if (!tabPin.visible)
            return false
        tabPin.activated()
        return true
    }

    /// Automation: the middle click, landed on the tab at `index` (`PG_AUTO_ACT=middle-close`). Answers whether there
    /// was an item under it to press: the strip is a view, and the item for a row the model has only just gained
    /// arrives with the next layout — a caller reading the asking as a press would wait for a tab nobody touched.
    function middleClickTab(index) {
        const tab = tabs.itemAtIndex(index)
        if (!tab)
            return false
        tabStrip.pressTab(index, tab.tab_id, Qt.MiddleButton)
        return true
    }

    /// Automation: what the strip's own items came out as, asked of `TabProbe` and handed on under the names the band
    /// already calls them by (`TopBar`).
    function tabItemCount() { return tabProbe.tabItemCount() }
    function hasTabPath(path) { return tabProbe.hasTabPath(path) }
    function tabPaths() { return tabProbe.tabPaths() }
    function tabTitles() { return tabProbe.tabTitles() }
    function tabPathAt(index) { return tabProbe.tabPathAt(index) }
    function tabWidths() { return tabProbe.tabWidths() }
    function pointAtTab(index) { tabProbe.pointAtTab(index) }
    function tabMarks() { return tabProbe.tabMarks() }

    /// Hands the run out among the tab names, the longest giving way last (デザイン規約 §ウィンドウの縁); below `tabTitleMinW`
    /// the strip scrolls instead. Settled by hand rather than bound: the widths are read off a list of items, and a
    /// binding cannot see one of those arrive. Whole pixels throughout — a strip sized off fractional widths comes out
    /// a pixel over the run it was told to fit in, which is a strip that scrolls when nothing is out of room.
    function settleTitleCap() {
        let nat = []
        for (let i = 0; i < titleMeasure.count; i++) {
            const label = titleMeasure.itemAt(i)
            if (label)
                nat.push(Math.min(Math.ceil(label.implicitWidth), tabStrip.tabTitleMaxW))
        }
        // What the row is asked for, so the band's leftover is shared with the state group in proportion — asking for
        // the floor instead has the tabs down to three characters beside two whole badges. Measured off the same
        // hidden labels the cap is, so it does not move with the run it is about to be handed. The air a short name is
        // eased with counts here as a cost like the mark: it is spent whatever the cap comes to.
        tabStrip.tabTitleEaseW = tabMetrics.titleEaseW()
        const eased = nat.reduce(
            (sum, w) => sum + tabMetrics.titleEase(w, tabStrip.tabTitleMaxW, tabStrip.tabTitleEaseW), 0)
        tabStrip.tabsWantWidth = menuButton.width + plusButton.width + tabs.grabRun + eased
            + nat.reduce((sum, w) => sum + w, 0) + nat.length * tabMetrics.tabFixedW
        if (nat.length === 0) {
            tabStrip.tabTitleCap = tabStrip.tabTitleMaxW
            return
        }
        nat.sort((a, b) => a - b)
        let left = Math.floor(tabs.runAvail) - nat.length * tabMetrics.tabFixedW - eased
        let cap = tabStrip.tabTitleMaxW
        for (let i = 0; i < nat.length; i++) {
            const share = Math.floor(left / (nat.length - i))
            if (nat[i] > share) {
                cap = share
                break
            }
            left -= nat[i]
        }
        tabStrip.tabTitleCap = Math.max(tabStrip.tabTitleMinW, Math.min(cap, tabStrip.tabTitleMaxW))
    }

    implicitWidth: tabStrip.tabsWantWidth

    /// Not declared inside `tabs`: a Flickable adopts its children into contentItem, where they travel with the scroll.
    TabMetrics {
        id: tabMetrics
    }

    /// The names at their natural width, off screen: the strip's own labels are the ones being capped, so they cannot
    /// also be what the cap is measured from. These carry the font the strip draws in, the current tab's heavier
    /// weight included, so what comes back is the width the strip will ask for.
    Repeater {
        id: titleMeasure
        model: tabStrip.tabsModel
        onCountChanged: tabStrip.settleTitleCap()
        delegate: Label {
            required property int index
            required property string title

            visible: false
            text: title
            // The tracking a short name is set in comes into what it measures, or the strip and the tab would be
            // reading the same name at two different widths (`TabMetrics.titleTracking`).
            font.letterSpacing: tabMetrics.titleTracking(title.length)
            font.weight: tabStrip.tabsModel.currentIndex === index ? Font.DemiBold : Font.Normal
            onImplicitWidthChanged: tabStrip.settleTitleCap()
            Component.onCompleted: tabStrip.settleTitleCap()
        }
    }

    // App menu; most entries are placeholders until their phases land. Sized as the head of the folded sidebar's
    // column — `railWidth` wide, the rail cells' wash, and this band's own full height (`NavRail.cellHeight`). The
    // mark inside is a step under theirs: this one stands alone in a band, theirs are the cells (デザイン規約 §寸法).
    ToolButton {
        id: menuButton
        width: Theme.railWidth
        height: tabStrip.height
        padding: 0
        hoverEnabled: true
        Accessible.name: qsTr("Application menu")
        // Written out instead of borrowing HoverToolButton: an open menu keeps the wash, which no hover of its own can
        // say — what is on screen has to say which mark put it there (`NavRail`).
        background: Rectangle {
            color: menuButton.hovered || appMenu.opened ? Theme.bgHover : "transparent"
        }
        contentItem: Item {
            NavIcon {
                anchors.centerIn: parent
                width: Theme.iconLg
                height: Theme.iconLg
                kind: "menu"
                tint: Theme.textPrimary
            }
        }
        // The mark is the way in and the way out: a press on it while the card stands takes the card down, the way a
        // press anywhere else in the window does. Written as a toggle rather than as `open()` — `open()` on a card
        // already up does nothing, which reads as a mark that can never be pressed a second time.
        onClicked: appMenu.opened ? appMenu.close() : appMenu.open()
        AppMenu {
            id: appMenu
            // Outside the ☰ itself, not outside the card. The default policy calls the mark's own press "outside" and
            // closes on it — and the click that follows the same press opens the card again, so the second press never
            // shuts anything (`AppCombo.popup` was written from the same reading). Every press elsewhere in the window
            // still takes the card down, the band's own empty run included (`WindowChrome.captionYielded`).
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
            // A full-height cell ends where the band does, so the card would otherwise open on top of the divider.
            y: menuButton.height + Theme.splitterWidth
            AppMenuItem {
                text: Words.openRepository
                onTriggered: tabStrip.openRepositoryRequested()
            }
            AppMenuItem {
                id: cloneRow
                text: qsTr("Clone repository…")
                onTriggered: tabStrip.cloneRepositoryRequested()
            }
            AppMenuSeparator {}
            // A local re-read (no network), for where the on-tick refresh cannot reach.
            AppMenuItem {
                text: qsTr("Reload")
                enabled: tabStrip.curPage !== null
                onTriggered: tabStrip.curPage.pageTab.refreshAll()
            }
            AppMenuSeparator {}
            // One row, because there is one screen. A second entry naming a category of it would be a menu telling
            // the reader about the inside of the thing it opens (2026-08-29 ユーザー報告 — the two rows read as a
            // duplicate).
            AppMenuItem {
                text: qsTr("Settings…")
                onTriggered: tabStrip.settingsRequested()
            }
            AppMenuItem {
                text: qsTr("About Platitude GG")
                enabled: false
            }
            AppMenuSeparator {}
            AppMenuItem {
                text: qsTr("Exit")
                onTriggered: Qt.quit()
            }
        }
    }
    // As wide as the tabs it holds, up to what the strip has left; past that it scrolls. The tabs narrow before it
    // comes to that (`settleTitleCap`), so the scrolling starts where they can give no more.
    ListView {
        id: tabs
        x: menuButton.width
        height: tabStrip.height
        /// Band the tabs may not grow into: the empty run past the last tab is the only place left to take hold of the
        /// window, so opening one more tab may not squeeze it to nothing. One end cell plus the band's own margin —
        /// the width Chrome keeps between its own `+` and its window buttons (52px at 100%), in this theme's tokens.
        readonly property real grabRun: tabStrip.captionMerged ? Theme.railWidth + Theme.spaceMd : 0
        /// The run the tabs share out between them; the width below and the cap both read this one expression.
        //
        // `tabs.grabRun` stays qualified: an unqualified name here reads whatever id happens to share it — ids outrank
        // the enclosing object's own properties — and an Item minus a number is NaN, which takes the strip's width
        // with it (no tabs drawn, nothing said why).
        readonly property real runAvail:
            Math.max(0, tabStrip.width - menuButton.width - plusButton.width - tabs.grabRun)
        onRunAvailChanged: tabStrip.settleTitleCap()
        width: Math.max(0, Math.min(contentWidth, tabs.runAvail))
        orientation: ListView.Horizontal
        // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
        boundsBehavior: Flickable.StopAtBounds
        clip: true
        // Wheel only. Left interactive, the view watches every press for a drag and steals the grab at the platform's
        // threshold (4px on Windows), so a click with a little sideways motion cancels the tab's own MouseArea.
        interactive: false
        // Every delegate stays alive however far the strip is scrolled: the automation walks the items (`tabPaths` /
        // `middleClickTab`), and a released delegate answers with null. Tabs are counted in ones, so this is free.
        cacheBuffer: 65536
        // A plain wheel only ever reports "vertical", so either axis moves the strip sideways.
        WheelHandler {
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            onWheel: event => {
                const step = event.angleDelta.x !== 0 ? event.angleDelta.x : event.angleDelta.y
                // A hand on the wheel outranks a travel in flight: the strip goes where it is being sent.
                tabRun.halt()
                tabs.contentX = tabRun.clamp(tabs.contentX - step)
            }
        }
        model: tabStrip.tabsModel
        delegate: TabItemDelegate {
            id: tabItem
            tabsModel: tabStrip.tabsModel
            metrics: tabMetrics
            titleCap: tabStrip.tabTitleCap
            titleEaseW: tabStrip.tabTitleEaseW
            stripHeight: tabs.height
            held: tabCarry.heldId === tabItem.tab_id
            heldX: tabCarry.heldX
            onTabPressed: button => tabStrip.pressTab(tabItem.index, tabItem.tab_id, button)
            // `index` is read at the moment the hand reports, not at the one it took hold: the row this tab sits in is
            // what the drag has been changing all along.
            onTabTaken: grabX => tabCarry.takeTab(tabItem.index, grabX)
            onTabDragged: sceneX => tabCarry.carryTab(tabItem.index, sceneX)
            onTabDropped: tabCarry.dropTab()
            // Order-free: leaving one tab and arriving at another are two answers to the same question, and the item
            // that lost the front only takes the seat away if nobody has claimed it since.
            onFrontChanged: front => {
                if (front)
                    tabStrip.frontTab = tabItem
                else if (tabStrip.frontTab === tabItem)
                    tabStrip.frontTab = null
            }
        }
    }
    // The hand between the taking up and the setting down. Declared after the list it reads: what a carry measures
    // against is where the tabs sit, and the order it changes is the model's.
    TabCarry {
        id: tabCarry
        view: tabs
        tabsModel: tabStrip.tabsModel
    }
    // The headless run's window onto that same list, declared after it for the same reason.
    TabProbe {
        id: tabProbe
        view: tabs
    }
    // The tab in front, standing at the edge its own row went out of. Beside the list rather than inside it: a
    // Flickable's declared children are taken by its content item and travel with the scroll, and this is the one
    // thing in the strip that may not. Declared after the hand so it can ask whether one is on a tab.
    TabPin {
        id: tabPin
        tabsModel: tabStrip.tabsModel
        metrics: tabMetrics
        frontTab: tabStrip.frontTab
        titleCap: tabStrip.tabTitleCap
        titleEaseW: tabStrip.tabTitleEaseW
        runX: tabs.x
        runWidth: tabs.width
        runOffset: tabs.contentX
        carrying: tabCarry.heldId >= 0
        stripHeight: tabStrip.height
        onActivated: tabStrip.showFrontTab()
    }
    // Where the strip stands in its run, and the travel that sends it somewhere. Declared after the list it moves,
    // for the reason the hand is.
    TabRun {
        id: tabRun
        view: tabs
    }
    // Opening one more. Drawn rather than typed: a typed `+` resolves to whatever shape and line weight the platform
    // has, where every other mark in the window holds `Metrics.iconStroke` (デザイン規約 §寸法「印はフォントの字に任せない」).
    //
    // The mark is a step below its seat. The band's own marks are its two ends — the ☰ and the window buttons, each a
    // `railWidth` cell its full height (§ウィンドウの縁) — and this is not one of those: it follows the last tab, so what
    // it is level with is the `✕` standing in the tabs beside it, which is the ink the typed `+` carried anyway.
    //
    // The seat that ink sits in runs the band top to bottom, so a hand coming down the strip lands on the mark anywhere
    // in the band's depth — a bare `iconLg` box has to be aimed at. The one part of that depth the scene never sees is
    // the resize edge Windows keeps at the top of an unmaximised window, which is the band's own affair
    // (`winframe::hit_test`). Only the hit area reaches that far: the wash keeps a square `iconXl` box of its own,
    // since paint carried to the band's edges would read as one of the two end cells the band does own
    // (§当たり判定「広げるのは判定だけ」).
    //
    // Both the ink and the wash are written as padding and inset rather than as sizes of their own: a `Control`
    // stretches its `contentItem` over whatever the padding leaves and places its `background` inside the insets, so a
    // size written on either is gone on the next layout (手本 `TreeViewToggle`).
    HoverToolButton {
        id: plusButton
        x: tabs.x + tabs.width
        height: tabStrip.height
        padding: (Theme.iconXl - Theme.iconSm) / 2
        topPadding: Math.round((plusButton.height - Theme.iconSm) / 2)
        bottomPadding: plusButton.topPadding
        topInset: Math.round((plusButton.height - Theme.iconXl) / 2)
        bottomInset: plusButton.topInset
        // The seat's width, held to the box exactly: three of this strip's width expressions read `plusButton` — as
        // does `grabArea.x` — so a seat that drifts moves the run the tabs share out and the grab run with it.
        implicitWidth: Theme.iconXl
        implicitHeight: Theme.iconXl
        // A ToolButton names itself by its text, and this one has none. The words the menu's own row uses, since it
        // raises the same request — the mark is idiomatic, so it gets a name and no tip, like the ☰ and the three at
        // the far end (デザイン規約 §hover のツールチップ).
        Accessible.name: Words.openRepository
        contentItem: NavIcon {
            kind: "plus"
            tint: Theme.textPrimary
        }
        onClicked: tabStrip.openRepositoryRequested()
    }
    // The run of empty band past the last tab. The hit test answers HTCAPTION for this rectangle
    // (`winframe::hit_test`), so a press here never reaches the scene and every gesture is the platform's own. No
    // handlers: the scene's only job is saying where the run is.
    Item {
        id: grabArea
        x: plusButton.x + plusButton.width
        width: Math.max(0, tabStrip.width - x)
        height: tabStrip.height
        onXChanged: tabStrip.captionStripMoved()
        onWidthChanged: tabStrip.captionStripMoved()
        Component.onCompleted: tabStrip.captionStripMoved()
    }
}

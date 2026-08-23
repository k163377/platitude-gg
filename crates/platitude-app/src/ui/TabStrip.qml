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

    /// The longest a tab's name is ever drawn (デザイン規約 レイアウト初期値).
    readonly property int tabTitleMaxW: 180
    /// The shortest, in characters rather than pixels (同表): the same count costs a different number of pixels in each
    /// platform's UI font and at every scaling, so the length comes out of the font.
    readonly property int tabTitleMinChars: 3
    readonly property int tabTitleMinW:
        Math.ceil(tabTitleFont.advanceWidth("…") + tabStrip.tabTitleMinChars * tabTitleFont.averageCharacterWidth)
    /// What every tab's name is capped at right now — the strip's answer to how much room it was given
    /// (`settleTitleCap`).
    property real tabTitleCap: tabStrip.tabTitleMaxW
    /// What the strip would take with nothing cut (`settleTitleCap`), and the least it is ever laid out at. Two tabs,
    /// not one (2026-08-11 ユーザー指示、規約 §ウィンドウの縁).
    property real tabsWantWidth: 0
    readonly property int tabStripFloorW:
        menuButton.width + plusButton.width + tabs.grabRun + 2 * (tabs.tabFixedW + tabStrip.tabTitleMinW)

    signal openRepositoryRequested()
    signal identityEditRequested()
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

    /// The left button moves to the tab — and leaves a hand on it that may go on to carry it (`takeTab`) — the middle
    /// one closes it (デザイン規約 §タブの所作). The real press and the smoke hook both come through here.
    function pressTab(index, id, button) {
        if (button === Qt.MiddleButton)
            tabStrip.tabsModel.closeTab(id)
        else
            tabStrip.tabsModel.setCurrentIndex(index)
    }

    /// Automation: the tab at `index`, carried to `to` and set down (`PG_AUTO_ACT=tab-drag`). The carrying is a
    /// pointer's, which no headless run has; everything after it — the settling, the order, the tab that comes out in
    /// front — is the same road a hand takes.
    ///
    /// Carried to exactly where it comes to rest: its trailing edge on the far edge of the tab it is going to, or its
    /// leading edge on the near one. Anywhere in that place passes every tab in between and none beyond it.
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
    /// (`PG_AUTO_ACT=tab-hold`). The one thing a settled strip cannot show: a tab drawn away from its own row, with
    /// the hand still on it.
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
    /// Held one and a half tab widths past the run — the distance is the speed, so the hook names it in the strip's own
    /// terms rather than in pixels.
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

    /// Automation: whether the strip has travelled to its far end, and how far it has travelled (`tab-edge`). A strip
    /// with nothing to scroll answers false — there is no end to reach.
    function runAtEnd() {
        return tabs.contentWidth > tabs.width && tabs.contentX >= tabs.contentWidth - tabs.width - 1
    }
    function runOffset() {
        return Math.round(tabs.contentX)
    }

    /// Automation: the middle click, landed on the tab at `index` (`PG_AUTO_ACT=middle-close`). Answers whether there
    /// was an item under it to press: the strip is a view, and the item for a row the model has only just gained
    /// arrives with the next layout. A caller that read the asking as a press would go on waiting for a tab nobody
    /// touched.
    function middleClickTab(index) {
        const tab = tabs.itemAtIndex(index)
        if (!tab)
            return false
        tabStrip.pressTab(index, tab.tab_id, Qt.MiddleButton)
        return true
    }

    /// Automation: how many of the strip's rows have an item standing for them. `tabPaths` and `hasTabPath` walk those
    /// items, so this is what says whether their answer is the whole strip or only the part of it the layout has caught
    /// up with.
    function tabItemCount() {
        let n = 0
        for (let i = 0; i < tabs.count; i++)
            if (tabs.itemAtIndex(i))
                n++
        return n
    }

    /// Automation: whether any tab in the strip was opened on `path`. The closed tab's title cannot say it went — every
    /// demo working tree is called the same thing — and the path is the only thing that can.
    function hasTabPath(path) {
        for (let i = 0; i < tabs.count; i++) {
            const tab = tabs.itemAtIndex(i)
            if (tab && tab.repo_path === path)
                return true
        }
        return false
    }

    /// Automation: the paths rather than the titles — every demo repository is called the same thing, and a strip of
    /// one name proves nothing.
    function tabPaths() {
        let paths = []
        for (let i = 0; i < tabs.count; i++) {
            const tab = tabs.itemAtIndex(i)
            if (tab)
                paths.push(tab.repo_path)
        }
        return paths.join(",")
    }

    /// Automation: the path a tab was opened with, spelled the way the strip has it (`PG_AUTO_ACT=open-again`).
    function tabPathAt(index) {
        const tab = tabs.itemAtIndex(index)
        return tab ? tab.repo_path : ""
    }

    /// Automation: every tab's width, in the order they sit in (`PG_AUTO_ACT=tab-widths`).
    function tabWidths() {
        let widths = []
        for (let i = 0; i < tabs.count; i++) {
            const tab = tabs.itemAtIndex(i)
            widths.push(tab ? Math.round(tab.width) : 0)
        }
        return widths.join(",")
    }

    /// Automation: the pointer, set down on the tab at `index` — the half no headless run can reach any other way
    /// (`PG_AUTO_ACT=tab-mark`).
    function pointAtTab(index) {
        const tab = tabs.itemAtIndex(index)
        if (tab)
            tab.pointed = true
    }

    /// Automation: which tabs have their mark out, in strip order.
    function tabMarks() {
        let marks = []
        for (let i = 0; i < tabs.count; i++) {
            const tab = tabs.itemAtIndex(i)
            marks.push(tab ? tab.markShown : 0)
        }
        return marks.join(",")
    }

    /// Hands the run out among the tab names, the longest giving way last (デザイン規約 §ウィンドウの縁); below `tabTitleMinW` the
    /// strip scrolls instead. Settled by hand rather than bound: the widths are read off a list of items, and a binding
    /// cannot see one of those arrive. Whole pixels throughout: a strip sized off fractional widths comes out a pixel
    /// over the run it was told to fit in, which is a strip that scrolls when nothing is out of room (実測 content=897
    /// against run=896 without the rounding).
    function settleTitleCap() {
        let want = []
        for (let i = 0; i < titleMeasure.count; i++) {
            const label = titleMeasure.itemAt(i)
            if (label)
                want.push(Math.min(Math.ceil(label.implicitWidth), tabStrip.tabTitleMaxW))
        }
        // What the row is asked for, so the band's leftover is shared with the state group in proportion — asking for
        // the floor instead had the tabs down to three characters beside two whole badges (reported 2026-08-11).
        // Measured off the same hidden labels the cap is, so it does not move with the run it is about to be handed.
        tabStrip.tabsWantWidth = menuButton.width + plusButton.width + tabs.grabRun
            + want.reduce((sum, w) => sum + w, 0) + want.length * tabs.tabFixedW
        if (want.length === 0) {
            tabStrip.tabTitleCap = tabStrip.tabTitleMaxW
            return
        }
        want.sort((a, b) => a - b)
        let left = Math.floor(tabs.runAvail) - want.length * tabs.tabFixedW
        let cap = tabStrip.tabTitleMaxW
        for (let i = 0; i < want.length; i++) {
            const share = Math.floor(left / (want.length - i))
            if (want[i] > share) {
                cap = share
                break
            }
            left -= want[i]
        }
        tabStrip.tabTitleCap = Math.max(tabStrip.tabTitleMinW, Math.min(cap, tabStrip.tabTitleMaxW))
    }

    implicitWidth: tabStrip.tabsWantWidth

    FontMetrics {
        id: tabTitleFont
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontMd
    }

    /// The names at their natural width, off screen. The strip's own labels are the ones being capped, so they cannot
    /// also be what the cap is measured from. These carry the font the strip draws in — the heavier weight the current
    /// tab is set in included, which is wider — so what comes back is the width the strip will ask for.
    Repeater {
        id: titleMeasure
        model: tabStrip.tabsModel
        onCountChanged: tabStrip.settleTitleCap()
        delegate: Label {
            required property int index
            required property string title

            visible: false
            text: title
            font.weight: tabStrip.tabsModel.currentIndex === index ? Font.DemiBold : Font.Normal
            onImplicitWidthChanged: tabStrip.settleTitleCap()
            Component.onCompleted: tabStrip.settleTitleCap()
        }
    }

    // App menu; most entries are placeholders until their phases land. Sized as the head of the folded sidebar's column
    // — `railWidth` wide, the rail cells' wash, and this band's own full height, which is the run those cells answer
    // over as well (`NavRail.cellHeight`). The mark inside is a step under theirs: this one stands alone in a band,
    // theirs are the cells themselves (デザイン規約 §寸法).
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
        // press anywhere else in the window does. Written as a toggle rather than as `open()` because `open()` on a
        // card that is already up does nothing at all — and under the default close policy the press had already taken
        // it down, so the pair read as a mark that could never be pressed a second time (2026-08-23 ユーザー報告).
        onClicked: appMenu.opened ? appMenu.close() : appMenu.open()
        AppMenu {
            id: appMenu
            // Outside the ☰ itself, not outside the card. The default policy calls the mark's own press "outside" and
            // closes on it — and the click that follows the same press opens the card again, so the second press never
            // shuts anything (measured with an injected click — qmltestrunner, since a press cannot be put into the
            // app itself; the same reading `AppCombo.popup` was written from). Every press elsewhere in the window
            // still takes the card down, including the one on the band's own empty run: that run stops being the
            // platform's caption while the card stands (`WindowChrome.captionYielded`).
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
            // A full-height cell ends where the band does, so the card would otherwise open on top of the divider.
            y: menuButton.height + Theme.splitterWidth
            AppMenuItem {
                text: Words.openRepository
                onTriggered: tabStrip.openRepositoryRequested()
            }
            AppMenuItem {
                text: qsTr("Clone repository…")
                enabled: false
            }
            AppMenuSeparator {}
            // A local re-read (no network), for where the on-tick refresh cannot reach.
            AppMenuItem {
                text: qsTr("Reload")
                enabled: tabStrip.curPage !== null
                onTriggered: tabStrip.curPage.pageTab.refreshAll()
            }
            AppMenuSeparator {}
            AppMenuItem {
                text: qsTr("Identity…")
                onTriggered: tabStrip.identityEditRequested()
            }
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
        /// window, so opening one more tab may not squeeze it to nothing.
        readonly property real grabRun: tabStrip.captionMerged ? 2 * Theme.railWidth : 0
        /// The run the tabs share out between them; the width below and the cap both read this one expression.
        //
        // `tabs.grabRun` stays qualified: an unqualified name here reads whatever id happens to share it — ids outrank
        // the enclosing object's own properties — and an Item minus a number is NaN, which took the whole strip's width
        // with it once (no tabs drawn, nothing said why).
        readonly property real runAvail:
            Math.max(0, tabStrip.width - menuButton.width - plusButton.width - tabs.grabRun)
        onRunAvailChanged: tabStrip.settleTitleCap()
        /// The air a tab is set in (デザイン規約 §余白; the delegate carries the reasoning).
        readonly property int markAir: (Theme.iconLg - Theme.iconSm) / 2 + Theme.spaceXs
        readonly property real tabPadW: Theme.spaceSm + (Theme.spaceSm - tabs.markAir)
        /// What a tab costs before its name has a single letter in it — that air, and the mark at its end.
        readonly property real tabFixedW: tabs.tabPadW + Theme.iconLg
        width: Math.max(0, Math.min(contentWidth, tabs.runAvail))
        orientation: ListView.Horizontal
        // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
        boundsBehavior: Flickable.StopAtBounds
        clip: true
        // Wheel only. Left interactive, the view watches every press for a drag and steals the grab at the platform's
        // threshold (4px on Windows), so a click with a little sideways motion cancels the tab's own MouseArea instead
        // of switching (reported as "the tab stopped taking the first click").
        interactive: false
        // Every delegate stays alive however far the strip is scrolled: the automation walks the items (`tabPaths` /
        // `middleClickTab`), and a released delegate answers with null. Tabs are counted in ones, so this costs
        // nothing.
        cacheBuffer: 65536
        // A plain wheel only ever reports "vertical", so either axis moves the strip sideways.
        WheelHandler {
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            onWheel: event => {
                const step = event.angleDelta.x !== 0 ? event.angleDelta.x : event.angleDelta.y
                tabs.contentX = Math.max(0, Math.min(tabs.contentWidth - tabs.width, tabs.contentX - step))
            }
        }
        model: tabStrip.tabsModel
        delegate: TabItemDelegate {
            id: tabItem
            tabsModel: tabStrip.tabsModel
            titleCap: tabStrip.tabTitleCap
            padW: tabs.tabPadW
            markAir: tabs.markAir
            stripHeight: tabs.height
            held: tabCarry.heldId === tabItem.tab_id
            heldX: tabCarry.heldX
            onTabPressed: button => tabStrip.pressTab(tabItem.index, tabItem.tab_id, button)
            // `index` is read at the moment the hand reports, not at the one it took hold: the row this tab sits in is
            // what the drag has been changing all along.
            onTabTaken: grabX => tabCarry.takeTab(tabItem.index, grabX)
            onTabDragged: sceneX => tabCarry.carryTab(tabItem.index, sceneX)
            onTabDropped: tabCarry.dropTab()
        }
    }
    // The hand between the taking up and the setting down. Declared after the list it reads: what a carry measures
    // against is where the tabs sit, and the order it changes is the model's.
    TabCarry {
        id: tabCarry
        view: tabs
        tabsModel: tabStrip.tabsModel
    }
    // Opening one more. Drawn rather than typed: a `+` is in every family the chain names, so nothing here ever looked
    // broken — but its shape and the weight of its line were whatever the platform resolved, where every other mark in
    // the window holds `Metrics.iconStroke` wherever it stands (デザイン規約 §寸法「印はフォントの字に任せない」).
    //
    // The mark is a step below its seat. The band's own marks are its two ends — the ☰ and the window buttons, each a
    // `railWidth` cell the full height of it (§ウィンドウの縁) — and this is not one of those: it follows the last tab, so
    // what it is level with is the `✕` standing in the tabs beside it. That is the ink the typed `+` carried anyway
    // (measured on Windows, offscreen: 8px against `iconSm`'s 8.25), so the band reads as it did.
    //
    // The seat that ink sits in runs the band top to bottom, so a hand coming down the strip lands on the mark anywhere
    // in the band's depth — a bare `iconLg` box has to be aimed at (2026-08-22 ユーザー報告).
    // The one part of that depth the scene never sees is the resize edge Windows keeps at the top of a window that is
    // not maximised, which is the band's own affair rather than this button's (`winframe::hit_test`).
    // Only the hit area reaches that far: the wash keeps a square box of its own, since paint carried to the band's
    // edges would read as one of the two end cells the band does own (§当たり判定「広げるのは判定だけ」). That box is `iconXl`,
    // a step over the seat every icon button sits in — unlike the `✕` beside it, which stands inside a tab that washes
    // as a whole, this mark answers the pointer with nothing but itself.
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
        // The seat's width, held to the box exactly. The typed `+` came out a pixel wider than a box (Fusion's own
        // padding around a glyph — 21 measured), and three of this strip's width expressions read `plusButton` — as
        // does `grabArea.x` — so a seat that drifts moves the run the tabs share out and the grab run with it.
        implicitWidth: Theme.iconXl
        implicitHeight: Theme.iconXl
        // What the label was saying for it: a ToolButton names itself by its text, and this one no longer has any. The
        // words the menu's own row uses, since it raises the same request — the mark is idiomatic and the answer comes
        // straight out, so it gets a name and no tip, like the ☰ and the three at the far end (デザイン規約 §hover のツールチップ).
        Accessible.name: Words.openRepository
        contentItem: NavIcon {
            kind: "plus"
            tint: Theme.textPrimary
        }
        onClicked: tabStrip.openRepositoryRequested()
    }
    // The run of empty band past the last tab. The hit test answers HTCAPTION for this rectangle
    // (`winframe::hit_test`), so a press here never reaches the scene and every gesture is the platform's own. No
    // handlers — the scene's only job is saying where the run is.
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

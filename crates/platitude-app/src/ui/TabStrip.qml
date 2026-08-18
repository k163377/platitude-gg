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

    /// The tab in hand: which one it is, where inside it the hand took hold, and where its left edge has been carried
    /// to in the strip's own coordinates. Held by id rather than by position — the order changes under a drag, and the
    /// row the hand is on is the one thing about it that does not (デザイン規約 §タブの所作).
    property int heldId: -1
    property real heldGrabX: 0
    property real heldX: 0

    signal openRepositoryRequested()
    signal identityEditRequested()
    signal settingsRequested()
    /// The grab-run moved or changed size in the strip's own layout. `Main` folds in the shifts this strip cannot see
    /// from here (the maximised inset, the window resizing) and reports the strip on.
    signal captionStripMoved()

    /// The left button moves to the tab — and leaves a hand on it that may go on to carry it (`takeTab`) — the middle
    /// one closes it (デザイン規約 §タブの所作). The real press and the smoke hook both come through here.
    function pressTab(index, id, button) {
        if (button === Qt.MiddleButton)
            tabStrip.tabsModel.closeTab(id)
        else
            tabStrip.tabsModel.setCurrentIndex(index)
    }

    /// The tab at `index`, taken up: the hand has carried it past the platform's threshold and is now holding it.
    /// `grabX` is where inside the tab it took hold, which is what keeps that same point of the tab under the hand
    /// however far it travels.
    function takeTab(index, grabX) {
        const tab = tabs.itemAtIndex(index)
        if (!tab)
            return
        tabStrip.heldId = tab.tab_id
        tabStrip.heldGrabX = grabX
        tabStrip.heldX = tab.x
    }

    /// The hand, moved to `sceneX` with a tab in it. Scene coordinates in, the strip's own out: the tab is drawn where
    /// the hand is rather than where the row sits, so its own coordinates cannot say where the pointer got to.
    function carryTab(index, sceneX) {
        tabStrip.carryTo(index, tabs.contentItem.mapFromItem(null, sceneX, 0).x - tabStrip.heldGrabX)
    }

    /// The one place a carried tab is put anywhere: where it is drawn, and — for as long as it keeps passing them —
    /// which neighbours it has changed places with. The hand comes through `carryTab` and the smoke hook through
    /// `dragTabTo`, so neither can reach an order the other cannot.
    function carryTo(index, left) {
        const tab = tabs.itemAtIndex(index)
        if (!tab)
            return index
        // The strip is the whole of the run: a tab carried past either end stops there, the way everything else that
        // scrolls here stops (デザイン規約 §QML 実装ルール). Nothing is torn off into a window of its own.
        tabStrip.heldX = Math.max(0, Math.min(left, tabs.contentWidth - tab.width))
        let at = index
        // Bounded by the strip itself: each step passes one tab, so nothing can be passed more often than there are
        // tabs to pass.
        for (let step = 0; step < tabs.count; step++) {
            const next = tabStrip.stepOrder(at)
            if (next === at)
                break
            at = next
            // That step changed the order, so the places the next comparison reads have to be the ones the view has
            // just given the tabs rather than the ones they are leaving.
            tabs.forceLayout()
        }
        return at
    }

    /// One neighbour, passed or not: the carried tab changes places with whichever side it has taken half of.
    ///
    /// The leading edge against the neighbour's middle, rather than middle against middle. Tabs are of different widths
    /// — middles agree only where they are of one width, and a wide tab held against the end of the strip never reaches
    /// a narrow last tab's middle at all, which would leave the last place unreachable by hand.
    function stepOrder(at) {
        const tab = tabs.itemAtIndex(at)
        if (!tab)
            return at
        const left = at > 0 ? tabs.itemAtIndex(at - 1) : null
        if (left && tabStrip.heldX < left.x + left.width / 2) {
            tabStrip.tabsModel.moveTab(at, at - 1)
            return at - 1
        }
        const right = at + 1 < tabs.count ? tabs.itemAtIndex(at + 1) : null
        if (right && tabStrip.heldX + tab.width > right.x + right.width / 2) {
            tabStrip.tabsModel.moveTab(at, at + 1)
            return at + 1
        }
        return at
    }

    /// Set down. The order is already what it is going to be — the tab only stops being drawn away from its own row.
    function dropTab() {
        tabStrip.heldId = -1
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
        tabStrip.takeTab(from, tab.width / 2)
        tabStrip.carryTo(from, to > from ? dest.x + dest.width - tab.width : dest.x)
        tabStrip.dropTab()
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
        tabStrip.takeTab(index, tab.width / 2)
        // Towards the middle of the strip: the ends are where a carry stops, and the last tab carried further right
        // is drawn exactly where it already was.
        const toward = index + 1 < tabs.count ? tab.width / 2 : -tab.width / 2
        tabStrip.carryTo(index, tab.x + toward)
        return true
    }

    /// Automation: how far the tab in hand is actually drawn from its own row (`tab-hold`). Read off that tab, so a
    /// transform that came apart answers 0 — which is also what nothing being held answers.
    function heldTabShift() {
        for (let i = 0; i < tabs.count; i++) {
            const tab = tabs.itemAtIndex(i)
            if (tab && tab.tab_id === tabStrip.heldId)
                return Math.round(tab.shiftShown)
        }
        return 0
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
    // — `railWidth` wide, the rail cells' wash — so this mark and the section marks under it stand on one line (デザイン規約
    // §寸法).
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
        onClicked: appMenu.open()
        AppMenu {
            id: appMenu
            // A full-height cell ends where the band does, so the card would otherwise open on top of the divider.
            y: menuButton.height + Theme.splitterWidth
            AppMenuItem {
                text: qsTr("Open repository…")
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
            held: tabStrip.heldId === tabItem.tab_id
            heldX: tabStrip.heldX
            onTabPressed: button => tabStrip.pressTab(tabItem.index, tabItem.tab_id, button)
            // `index` is read at the moment the hand reports, not at the one it took hold: the row this tab sits in is
            // what the drag has been changing all along.
            onTabTaken: grabX => tabStrip.takeTab(tabItem.index, grabX)
            onTabDragged: sceneX => tabStrip.carryTab(tabItem.index, sceneX)
            onTabDropped: tabStrip.dropTab()
        }
    }
    HoverToolButton {
        id: plusButton
        x: tabs.x + tabs.width
        anchors.verticalCenter: parent.verticalCenter
        text: "+"
        font.pixelSize: Theme.fontLg
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

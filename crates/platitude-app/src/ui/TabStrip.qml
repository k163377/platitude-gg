pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The strip, placed by hand: the leftover is both what the tabs may grow into and where the window is taken hold of,
// and the selected tab's underline lands exactly on the band's bottom edge.
Item {
    id: tabStrip

    required property var tabsModel
    /// The band's ground behind the strip (`TopBar`): what a tab not in front shows behind its name, and what the name
    /// fades into under its mark (`TabItemDelegate.titleFade`).
    required property color bandColor
    /// Whether the band this strip sits in is the window's title bar. When it is, the strip keeps a run of band clear
    /// to take hold of (`tabs.grabRun`).
    property bool captionMerged: false
    /// The active tab's RepoPage (null while no tab is open), for the app menu.
    property var curPage: null
    /// The tab in front, as this list built it — pushed by the tab (`TabItemDelegate.frontChanged`), since a new row's
    /// item arrives only with the next layout. Null while no tab is open.
    property Item frontTab: null
    /// An ask from outside the strip to show the repository just put in front (`TabsModel`), and whether to travel
    /// there or start there (`askFrontTab`). Held, because at the moment of the ask the tab in front is still the one
    /// being left.
    property bool frontAsked: false
    property bool frontAskTravels: false
    /// The tab the open ask is for (`TabsModel::current_tab_id` at the ask). The ask stays open while that tab is in
    /// front and is answered again whenever what it was measured against moves: the view settles extent and widths
    /// after the item reports itself, and a one-shot answer is left off the run. It closes on the reader's hand on the
    /// strip (`takeRun`), a tab closed, the front moving, or — once answered — the next press anywhere (`pressLanded`).
    property int frontAskId: -1
    /// The open ask has answered once, so the next press ends it (`pressLanded`).
    property bool frontAskAnswered: false
    /// Whether the strip has held a tab in front since it was last empty
    /// (デザイン規約 §タブの所作「前に出ているタブを 1 枚も持っていなかった帯は、その席で始まる」). Not `frontTab` at the ask:
    /// across a move it can be null on both sides — the old tab lets go before the ask, the new one reports a layout
    /// later.
    property bool stood: false
    /// How many tabs the strip had when it last counted, so a drop reads as a tab closed (`tabs.onCountChanged`).
    property int tabsCounted: 0

    /// Automation (`tab-widths`): the run the tabs were handed and what they made of it — a picture cannot say which
    /// tabs gave way.
    readonly property int tabCount: tabs.count
    readonly property real runAvail: tabs.runAvail
    readonly property real contentWidth: tabs.contentWidth
    /// A broken term in the width expression makes it NaN, which draws as no tabs with nothing on stderr — this number
    /// catches it.
    readonly property real tabsWidth: tabs.width
    /// Band the tabs may not grow into, and the run itself for `Main` to measure in scene coordinates.
    readonly property real grabRun: tabs.grabRun
    readonly property Item grabRunItem: grabArea
    /// Whether the ☰'s card is open: the grab run stops being caption meanwhile, so a press on it reaches the scene and
    /// closes the card (`WindowChrome.captionYielded`).
    readonly property bool appMenuOpen: menuButton.menuOpen

    /// Automation (`tab-pin` / `tab-pin-go`): the stand-in's state, which a picture of a scrolled strip cannot tell
    /// apart. Read off the stand-in itself, so a broken binding answers with what is drawn.
    readonly property bool tabPinShown: tabPin.visible
    readonly property bool tabPinRidesLeft: tabPin.rideLeft
    readonly property bool frontTabWhole: tabPin.frontWhole
    readonly property bool runTravelling: tabRun.travelling
    /// Whether its name shows any letters (`TabPin.nameKept`) — a photograph cannot tell that from a short name in a
    /// narrow tab.
    readonly property bool tabPinNameKept: tabPin.nameKept

    /// The longest a name is drawn at (`TabShare.ceiling`). Pushed, like the three below.
    property int tabTitleMaxW: 0
    /// The floor, the easing width and the fade — measured off the font, so pushed
    /// (`TabMetrics.titleMinW` / `titleEaseFullW` / `fadeW`).
    property int tabTitleMinW: 0
    property real tabTitleEaseW: 0
    property real tabTitleFadeW: 0
    /// Every name's cap and every tab's mark room right now (`settleTitleCap`).
    property real tabTitleCap: tabStrip.tabTitleMaxW
    property real tabMarkRoom: tabMetrics.markRoomFull
    /// Whether names are cut and marks folded, said as the answer: the tab count that reaches either depends on the
    /// platform's font, so a run naming a count claims a state it never checked
    /// (rules-refs/app-ui.md「枚数ではなく状態を名乗らせ」).
    readonly property bool tabNamesCut: tabStrip.tabTitleCap < tabStrip.tabTitleMaxW
    readonly property bool tabMarksFolded: tabStrip.tabMarkRoom <= tabMetrics.markRoomMin
    /// What the strip would take with nothing cut (`settleTitleCap`), and its floor: two tabs (規約 §ウィンドウの縁).
    property real tabsWantWidth: 0
    readonly property int tabStripFloorW:
        menuButton.width + plusButton.width + tabs.grabRun
        + 2 * (tabMetrics.tabPadL + tabMetrics.markRoomMin + tabStrip.tabTitleMinW)

    signal openRepositoryRequested()
    signal cloneRepositoryRequested()
    signal settingsRequested()
    /// The ☰'s Exit row: the same road as the band's ✕ (`TopBar` folds it into `closeRequested`), so the close gate
    /// (`Main.qml`) has one door.
    signal exitRequested()
    /// The grab run moved or resized within the strip; `Main` adds the shifts it cannot see (maximised inset, window
    /// resize).
    signal captionStripMoved()

    /// Automation (`app-menu`): the ☰ pressed at the button, whose own handler holds the toggle.
    function clickAppMenu() {
        menuButton.clicked()
    }

    /// Automation (`clone-*`): the ☰'s `Clone repository…` row, through the row's own `triggered`.
    function clickCloneRow() {
        menuButton.clickCloneRow()
    }

    /// Left moves to the tab (a carry may follow, `takeTab`); middle closes it (デザイン規約 §タブの所作). Real presses
    /// and automation both come through here.
    function pressTab(index, id, button) {
        if (button === Qt.MiddleButton) {
            tabStrip.tabsModel.closeTab(id)
            return
        }
        tabStrip.takeRun()
        tabStrip.tabsModel.setCurrentIndex(index)
    }

    /// Automation (`tab-drag`): the tab at `from` carried to `to` and set down; past the pointer, a hand's road.
    /// Carried to exactly its resting place — trailing edge on the far edge of the target, or leading edge on the near
    /// one.
    function dragTabTo(from, to) {
        const tab = tabs.itemAtIndex(from)
        const dest = tabs.itemAtIndex(to)
        if (!tab || !dest || from === to)
            return false
        // Pressed first, as a hand does: the press is what moves to the tab.
        tabStrip.pressTab(from, tab.tab_id, Qt.LeftButton)
        tabCarry.takeTab(from, tab.width / 2)
        tabCarry.carryTo(from, to > from ? dest.x + dest.width - tab.width : dest.x)
        tabCarry.dropTab()
        return true
    }

    /// Automation (`tab-hold`): the tab at `index` taken up and carried half its width, not set down.
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

    /// Automation (`tab-hold`): how far the tab in hand is drawn from its row, read off the tab so a broken transform
    /// answers 0.
    function heldTabShift() {
        const at = tabCarry.heldIndex()
        const tab = at < 0 ? null : tabs.itemAtIndex(at)
        return tab ? Math.round(tab.shiftShown) : 0
    }

    /// Automation (`tab-edge`): the tab at `index` carried a tab width past the far end of the run and held there — the
    /// distance past is the drift speed.
    function carryTabPastEnd(index) {
        const tab = tabs.itemAtIndex(index)
        if (!tab)
            return false
        tabStrip.pressTab(index, tab.tab_id, Qt.LeftButton)
        tabCarry.takeTab(index, tab.width / 2)
        tabCarry.carryTab(index, tabs.mapToItem(null, tabs.width + tab.width, 0).x)
        return true
    }

    /// Automation (`tab-edge`): the hand's held index, and letting go.
    function heldIndex() {
        return tabCarry.heldIndex()
    }
    function dropTab() {
        tabCarry.dropTab()
    }

    /// What a press on the stand-in means: travel until the tab in front is whole (`TabRun.showTab`).
    function showFrontTab() {
        return tabRun.showTab(tabStrip.frontTab)
    }

    /// An outside ask to show the tab in front (デザイン規約 §タブの所作): travelled to if the strip has stood before,
    /// else started at (`stood`). The model has moved its front before asking, so its front is the tab asked for.
    function askFrontTab() {
        tabStrip.frontAskTravels = tabStrip.stood
        tabStrip.frontAskId = tabStrip.tabsModel.currentTabId
        tabStrip.frontAskAnswered = false
        tabStrip.frontAsked = true
        Qt.callLater(tabStrip.answerFrontAsk)
    }

    /// Answers the ask once the strip's front tab is the one asked for, and again while the ask is open.
    ///
    /// Always a turn late (`Qt.callLater`, which also folds repeats): a tab reports itself in front from
    /// `Component.onCompleted`, before the view lays it out, and read there it still stands at `x: 0`.
    function answerFrontAsk() {
        if (!tabStrip.frontAsked)
            return
        if (tabStrip.tabsModel.currentTabId !== tabStrip.frontAskId) {
            tabStrip.closeFrontAsk()
            return
        }
        // Its item has not reported itself in front yet; a later layout brings it.
        const tab = tabStrip.frontTab
        if (!tab || tab.tab_id !== tabStrip.frontAskId)
            return
        tabStrip.frontAskAnswered = true
        if (tabStrip.frontAskTravels)
            tabRun.showTab(tab)
        else
            tabRun.landOn(tab)
    }

    function closeFrontAsk() {
        tabStrip.frontAsked = false
        tabStrip.frontAskId = -1
        tabStrip.frontAskAnswered = false
    }

    /// The reader's hand on the strip (a press, the wheel, a carry): it outranks a travel in flight and an open ask
    /// alike (デザイン規約 §タブの所作「帯自身の所作は頼みの外」).
    function takeRun() {
        tabRun.halt()
        tabStrip.closeFrontAsk()
    }

    /// A press landed anywhere in the window (`FocusRelease.pressedAnywhere`): an answered ask ends with the reader's
    /// next act, or the strip would move on its own under them. An unanswered one goes on — its tab is still coming in.
    ///
    /// A press, not hover: Qt re-delivers hover whenever the scene moves under a still pointer, so hover is no act of
    /// the reader's.
    function pressLanded() {
        if (tabStrip.frontAskAnswered)
            tabStrip.closeFrontAsk()
    }

    /// Automation: the run's own answers (`TabRun`) — offset and far end (`tab-edge`), and sending it away from the
    /// tab in front (`tab-pin`).
    function runAtEnd() { return tabRun.atEnd() }
    function runOffset() { return tabRun.offset() }
    /// Outranks an open ask, as a reader's hand does (`takeRun`).
    function sendRunAway() {
        tabStrip.closeFrontAsk()
        return tabRun.sendAway(tabStrip.frontTab)
    }

    /// Automation (`tab-open-go`): the ask and what a travel to it is measured against, in one line. `to=` is where
    /// the strip would be sent now; a strip at rest whose `to=` is not its `cx=` has a tab it was not sent to.
    function frontAskAccount() {
        const tab = tabStrip.frontTab
        return "ask=" + tabStrip.frontAsked + " answered=" + tabStrip.frontAskAnswered + " for=" + tabStrip.frontAskId
            + " travels=" + tabStrip.frontAskTravels + " stood=" + tabStrip.stood
            + " front=" + (tab ? tab.tab_id + ":" + tab.index + "@" + Math.round(tab.x) + "+" + Math.round(tab.width)
                                : "none")
            + " cx=" + Math.round(tabs.contentX) + " ox=" + Math.round(tabs.originX) + " cw=" + Math.round(tabs.contentWidth)
            + " vw=" + Math.round(tabs.width) + " to=" + Math.round(tabRun.wholeAt(tab))
    }

    /// Automation (`tab-pin-go`): the stand-in pressed, at its own signal.
    function pressTabPin() {
        if (!tabPin.visible)
            return false
        tabPin.activated()
        return true
    }

    /// Automation (`middle-close`): a middle click on the tab at `index`. Answers whether an item was there to press —
    /// a new row's item arrives only with the next layout.
    function middleClickTab(index) {
        const tab = tabs.itemAtIndex(index)
        if (!tab)
            return false
        tabStrip.pressTab(index, tab.tab_id, Qt.MiddleButton)
        return true
    }

    /// Automation-only exposure (rules-refs/app-ui.md「製品の部品はハーネスへ答える相手を丸ごと渡す」): the list, read
    /// by `auto/TabProbe.qml`.
    readonly property alias tabsView: tabs

    /// Hands the run out (`TabMetrics.settle`). Settled by hand: the widths are read off a list of items, and a binding
    /// cannot see one arrive.
    function settleTitleCap() {
        let nat = []
        for (let i = 0; i < titleMeasure.count; i++) {
            const measured = titleMeasure.itemAt(i)
            if (measured)
                nat.push(Math.ceil(measured.natW))
        }
        // The strip asks for its natural width, not its floor, so the band shares its leftover with the state group in
        // proportion (asking for the floor leaves tabs at three characters beside whole badges). The ask must not
        // carry the ceiling: that is a share of the run, and this width is the strip's `implicitWidth`, which the band
        // answers with `Layout.fillWidth` — a loop (`TabShare.settle`).
        const run = Math.floor(tabs.runAvail)
        const settled = tabMetrics.settle(nat, run)
        tabStrip.tabTitleMinW = settled.minW
        tabStrip.tabTitleMaxW = settled.maxW
        tabStrip.tabTitleEaseW = settled.easeW
        tabStrip.tabTitleFadeW = tabMetrics.fadeW()
        tabStrip.tabMarkRoom = settled.markRoom
        tabStrip.tabTitleCap = settled.cap
        tabStrip.tabsWantWidth =
            menuButton.width + plusButton.width + tabs.grabRun + settled.wantNames
    }

    implicitWidth: tabStrip.tabsWantWidth
    // A new front tab may be the one an open ask waits for (`answerFrontAsk`).
    onFrontTabChanged: {
        if (tabStrip.frontTab)
            tabStrip.stood = true
        Qt.callLater(tabStrip.answerFrontAsk)
    }
    Connections {
        target: tabStrip.frontAsked ? tabs : null
        function onContentWidthChanged() {
            Qt.callLater(tabStrip.answerFrontAsk)
        }
        function onWidthChanged() {
            Qt.callLater(tabStrip.answerFrontAsk)
        }
    }
    Connections {
        target: tabStrip.frontAsked ? tabStrip.frontTab : null
        function onXChanged() {
            Qt.callLater(tabStrip.answerFrontAsk)
        }
        function onWidthChanged() {
            Qt.callLater(tabStrip.answerFrontAsk)
        }
    }
    Connections {
        target: tabStrip.frontAsked ? tabRun.travel : null
        function onStopped() {
            Qt.callLater(tabStrip.answerFrontAsk)
        }
    }

    TabMetrics {
        id: tabMetrics
    }

    /// The names at natural width, hidden: the strip's own labels are the ones being capped, so they cannot be what
    /// the cap is measured from. Same font and weight as drawn, and including a linked copy's run (`TabTreeMark`) —
    /// it is part of what the tab would draw whole.
    Repeater {
        id: titleMeasure
        model: tabStrip.tabsModel
        onCountChanged: tabStrip.settleTitleCap()
        delegate: Item {
            id: measured
            required property int index
            required property string title
            required property string copy_name

            /// Both runs together: the one number the run is handed out by.
            readonly property real natW: Math.ceil(nameLabel.implicitWidth) + treeMeasure.naturalWidth

            visible: false
            onNatWChanged: tabStrip.settleTitleCap()
            Component.onCompleted: tabStrip.settleTitleCap()

            Label {
                id: nameLabel
                text: measured.title
                // The tab's tracking, or the strip and the tab read one name at two widths.
                font.letterSpacing: tabMetrics.titleTracking(measured.title.length)
                font.weight: tabStrip.tabsModel.currentIndex === measured.index ? Theme.fontWeightStrong : Font.Normal
            }
            TabTreeMark {
                id: treeMeasure
                name: measured.copy_name
                metrics: tabMetrics
            }
        }
    }

    AppMenuButton {
        id: menuButton
        height: tabStrip.height
        curPage: tabStrip.curPage
        onOpenRepositoryRequested: tabStrip.openRepositoryRequested()
        onCloneRepositoryRequested: tabStrip.cloneRepositoryRequested()
        onSettingsRequested: tabStrip.settingsRequested()
        onExitRequested: tabStrip.exitRequested()
    }
    // As wide as its tabs, up to the strip's leftover; past that it scrolls, once the tabs can narrow no more
    // (`settleTitleCap`).
    ListView {
        id: tabs
        x: menuButton.width
        height: tabStrip.height
        /// Band kept clear of the tabs to take hold of the window by (デザイン規約 §ウィンドウの縁).
        readonly property real grabRun: tabStrip.captionMerged ? Theme.railWidth + Theme.spaceMd : 0
        /// The run the tabs share out; the width below and the cap both read this one expression.
        //
        // `tabs.grabRun` stays qualified: ids outrank the object's own properties, so a bare name can read an Item,
        // and the NaN takes the strip's width with it (no tabs drawn, nothing said why).
        readonly property real runAvail:
            Math.max(0, tabStrip.width - menuButton.width - plusButton.width - tabs.grabRun)
        onRunAvailChanged: tabStrip.settleTitleCap()
        width: Math.max(0, Math.min(contentWidth, tabs.runAvail))
        orientation: ListView.Horizontal
        // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
        boundsBehavior: Flickable.StopAtBounds
        clip: true
        // Wheel only: an interactive view steals the grab at the drag threshold, cancelling the tab's own MouseArea on
        // a click that drifts sideways.
        interactive: false
        // Every delegate stays alive: automation walks the items (`tabPaths` / `middleClickTab`), and a released one
        // answers null. Tabs are few, so this is free.
        cacheBuffer: 65536
        // A tab closed, by any road, ends the ask and its travel (デザイン規約 §タブの所作「閉じた後の帯はその場に留まる」);
        // an empty strip has stood nowhere again.
        onCountChanged: {
            if (tabs.count < tabStrip.tabsCounted)
                tabStrip.takeRun()
            if (tabs.count === 0)
                tabStrip.stood = false
            tabStrip.tabsCounted = tabs.count
        }
        // A plain wheel only ever reports "vertical", so either axis moves the strip sideways.
        WheelHandler {
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            onWheel: event => {
                const step = event.angleDelta.x !== 0 ? event.angleDelta.x : event.angleDelta.y
                tabStrip.takeRun()
                tabs.contentX = tabRun.clamp(tabs.contentX - step)
            }
        }
        model: tabStrip.tabsModel
        delegate: TabItemDelegate {
            id: tabItem
            tabsModel: tabStrip.tabsModel
            metrics: tabMetrics
            titleCap: tabStrip.tabTitleCap
            titleMinW: tabStrip.tabTitleMinW
            titleEaseW: tabStrip.tabTitleEaseW
            markRoom: tabStrip.tabMarkRoom
            fadeW: tabStrip.tabTitleFadeW
            bandColor: tabStrip.bandColor
            stripHeight: tabs.height
            held: tabCarry.heldId === tabItem.tab_id
            heldX: tabCarry.heldX
            onTabPressed: button => tabStrip.pressTab(tabItem.index, tabItem.tab_id, button)
            // `index` is read when the hand reports: the drag keeps changing it.
            onTabTaken: grabX => tabCarry.takeTab(tabItem.index, grabX)
            onTabDragged: sceneX => tabCarry.carryTab(tabItem.index, sceneX)
            onTabDropped: tabCarry.dropTab()
            // Order-free: the tab losing the front clears the seat only if nobody has claimed it since.
            onFrontChanged: front => {
                if (front)
                    tabStrip.frontTab = tabItem
                else if (tabStrip.frontTab === tabItem)
                    tabStrip.frontTab = null
            }
        }
    }
    // The hand between the taking up and the setting down.
    TabCarry {
        id: tabCarry
        view: tabs
        tabsModel: tabStrip.tabsModel
    }
    // The tab in front, standing at the edge its own row went out of — beside the list, not in it (`TabPin`), and
    // declared after it so it and its dissolve draw over the tabs.
    TabPin {
        id: tabPin
        tabsModel: tabStrip.tabsModel
        metrics: tabMetrics
        frontTab: tabStrip.frontTab
        titleCap: tabStrip.tabTitleCap
        titleMinW: tabStrip.tabTitleMinW
        titleEaseW: tabStrip.tabTitleEaseW
        markRoom: tabStrip.tabMarkRoom
        fadeW: tabStrip.tabTitleFadeW
        runX: tabs.x
        runWidth: tabs.width
        runOffset: tabs.contentX
        carrying: tabCarry.heldId >= 0
        stripHeight: tabStrip.height
        onActivated: tabStrip.showFrontTab()
    }
    // Where the strip stands in its run, and the travel that sends it somewhere.
    TabRun {
        id: tabRun
        view: tabs
    }
    // Every road into a repository asks through one signal (デザイン規約 §タブの所作「判定は 1 か所に置く」). Not
    // `currentIndex`: the strip's own press and carry move that too, and those leave the band where the reader put it.
    Connections {
        target: tabStrip.tabsModel
        function onFrontTabAsked() {
            tabStrip.askFrontTab()
        }
    }
    // Opening one more, drawn (デザイン規約 §タブの所作「揃える相手は隣の `✕`」): its ink is read off `markSeat` so it
    // cannot come out a step apart from the tabs' `✕`. Only the hit area runs the band's depth; the wash keeps its
    // `iconXl` box.
    //
    // Ink and wash are set as padding and inset: a `Control` re-lays its `contentItem` and `background` from those, so
    // a size written on either is lost on the next layout.
    HoverToolButton {
        id: plusButton
        x: tabs.x + tabs.width
        height: tabStrip.height
        padding: (Theme.iconXl - tabMetrics.markSeat) / 2
        topPadding: Math.round((plusButton.height - tabMetrics.markSeat) / 2)
        bottomPadding: plusButton.topPadding
        topInset: Math.round((plusButton.height - Theme.iconXl) / 2)
        bottomInset: plusButton.topInset
        // Held to the box exactly: three width expressions and `grabArea.x` read `plusButton.width`, so a drifting seat
        // moves the run and the grab run.
        implicitWidth: Theme.iconXl
        implicitHeight: Theme.iconXl
        // No text, so named here with the menu row's words (same request). No tip — the mark is idiomatic
        // (デザイン規約 §hover のツールチップ).
        Accessible.name: Words.openRepository
        contentItem: NavIcon {
            kind: "plus"
            tint: Theme.textPrimary
        }
        onClicked: tabStrip.openRepositoryRequested()
    }
    // The empty band past the last tab. `winframe::hit_test` answers HTCAPTION here, so presses never reach the scene;
    // the scene only says where it is.
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

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

Rectangle {
    id: topBar

    required property var tabsModel
    // The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// Whether this band is the window's title bar. When it is, the band carries the window's own buttons, and tells
    /// the platform where its empty runs sit — the gestures on those (drag, snap, the double-click, the window menu)
    /// are the platform's own, because the hit test calls them caption (`AppBackend.setCaptionStrips`).
    property bool captionMerged: false
    /// Which shape the middle button is in.
    property bool windowMaximized: false

    /// Automation: what the band came out to. A layout change can lose the window's own buttons, or the run of band
    /// left to take hold of, or push either off the end — and none of that shows in a screenshot taken where the
    /// platform draws no buttons at all. The strip's own readings come back through here because the hooks ask the band
    /// rather than the strip (`WindowAutoActDriver`).
    readonly property real bandGrabRun: tabStrip.grabRun
    /// …and the band's other run, which a picture holds no better: the divider is drawn the same whether or not the
    /// hit test was ever told about the stretch it stands in.
    readonly property real bandDividerRun: dividerRun.width
    readonly property real bandButtonsX: minimizeButton.x
    readonly property real bandRightMargin: bandRow.anchors.rightMargin
    readonly property real bandTabsWidth: tabStrip.tabsWidth
    readonly property int bandTabCount: tabStrip.tabCount
    readonly property real bandTabRun: tabStrip.runAvail
    readonly property real bandTabContent: tabStrip.contentWidth
    readonly property bool bandTabScrolls: tabStrip.contentWidth > tabStrip.tabsWidth
    /// Automation: the four things that can be the matter here. The group owns the conditions and the hidden
    /// measurements behind them; what the hooks ask the band for comes back through here (`BandStateGroup`).
    readonly property bool opBadgeShown: stateGroup.opBadgeShown
    readonly property bool conflictBadgeShown: stateGroup.conflictBadgeShown
    readonly property bool identityBadgeShown: stateGroup.identityBadgeShown
    readonly property bool oldGitBadgeShown: stateGroup.oldGitBadgeShown
    readonly property real stateBadgeMinW: stateGroup.stateBadgeMinW

    /// Automation: which of the group's three shapes is on screen, what the badges were narrowed to, and what the card
    /// came back with (`PG_AUTO_ACT=badges` / `badges-hover`). The group is what makes them; the hooks come to the band
    /// to read them (`BandStateGroup`).
    readonly property bool stateWordsShown: stateGroup.stateWordsShown
    readonly property bool stateMarkShown: stateGroup.stateMarkShown
    readonly property color stateMarkColor: stateGroup.stateMarkColor
    readonly property int stateCapW: stateGroup.stateCapW
    readonly property int stateGroupW: stateGroup.stateGroupW
    readonly property bool stateCardOpen: stateGroup.stateCardOpen
    readonly property string stateCardRows: stateGroup.stateCardRows
    readonly property string stateCardSize: stateGroup.stateCardSize
    /// Whether the window is standing on its floor. Handed in, because the floor is the larger of this band's and the
    /// page's and only `Main` has both. The group gives up its words there whatever else is true (2026-08-11 ユーザー指示).
    property bool windowAtFloor: false
    /// Stands in for the pointer where headless cannot put one, so the card can be photographed (`badges-hover` /
    /// `identity-tip`). The real hover writes this same one property — hover is the input that cannot be injected, so
    /// the card has to be answering a single question or the headless run proves nothing about it.
    property bool statePointedAt: false
    /// Whether the fetch button can be pressed at all — the edge `fetch-tip` waits on, since what it reads is only
    /// worth anything once the band has settled on an answer.
    readonly property bool fetchLive: fetchButton.enabled

    signal openRepositoryRequested()
    signal identityEditRequested()
    signal settingsRequested()
    signal maximizeToggleRequested()
    signal minimizeRequested()
    signal closeRequested()
    /// The grab-run moved or changed size in this band's own layout. `Main` folds in the shifts this band cannot see
    /// from here (the maximised inset, the window resizing) and reports the strip on.
    signal captionStripMoved()
    /// The two runs themselves, for `Main` to measure in scene coordinates.
    readonly property Item grabRunItem: tabStrip.grabRunItem
    readonly property Item dividerRunItem: dividerRun
    /// Whether the ☰'s card is standing (`TabStrip`). The runs are the parts of the band a press never reaches, so it
    /// hands them back to the scene while the card is up (`WindowChrome.captionYielded`).
    readonly property bool appMenuOpen: tabStrip.appMenuOpen

    /// The narrowest this band can be laid out at — one of the two numbers the window's floor is the larger of
    /// (`Main.floorWidth`). Read off the row's *minimum* rather than summed here, so a control added to the band later
    /// is counted without anybody remembering to.
    readonly property real floorWidth: bandRow.Layout.minimumWidth + bandRow.anchors.rightMargin

    /// Automation: run the push button's hold to its end. The busy visual is latched only after the real RepoTab
    /// reports that push started; this makes an intentionally intermediate screenshot causal even when the subprocess
    /// completes before grabToImage runs.
    property bool autoPushBusyLatched: false
    /// Automation: the same latch for the button the wait was designed on — the bare one. `framed` rides along in the
    /// report because "a button that never wore a frame grows none while it waits" (デザイン規約 §進行中・長押しの定数) is a
    /// claim about a line that is not there, and a picture cannot be judged on the absence of one.
    property bool autoFetchBusyLatched: false
    readonly property string pushMode: pushButton.mode
    /// Automation: what the Stash button read the working tree as, and the edge `stash-state` waits on.
    readonly property string stashMode: stashButton.mode
    /// PG_AUTO_ACT=stash-state: which answer the button settled on and what the band did with it.
    ///
    /// The reading is a binding over a HEAD and four counts, and a photograph of a dim button carries none of them —
    /// every refusal frames the same way. `tip=` is the string the button would open rather than a ToolTip caught
    /// open, for the reason `fetch-tip` gives below.
    function reportStashState() {
        AppBackend.report("stash_state mode=" + stashButton.mode
                          + " enabled=" + stashButton.enabled
                          + " tip=" + (stashButton.tip !== ""))
    }
    /// Automation: the Stash button, pressed (`PG_AUTO_ACT=stash`). Put in at the button rather than at what it calls,
    /// so what answers is the band's real wiring and not a second way in written for the run.
    ///
    /// **Answers whether the press went in.** The band refuses it while the tab is busy — the fetch a repository does
    /// on the way open is one — and a shot fired at nothing is not one to latch: the caller keeps offering it, the way
    /// a hand waits for the button to come alive.
    function stashNow() {
        if (!stashButton.enabled)
            return false
        stashButton.clicked()
        return true
    }
    function completePushHold() {
        pushButton.completeHold()
    }
    function reportPushBusy() {
        AppBackend.report("push_hold mode=" + pushButton.mode + " busy=" + topBar.autoPushBusyLatched)
    }
    function reportFetchBusy() {
        AppBackend.report("fetch_busy busy=" + topBar.autoFetchBusyLatched
                          + " fails=" + fetchButton.fails + " framed=" + fetchButton.framed)
    }
    /// PG_AUTO_ACT=fetch-tip: whether the fetch button has anything to say under a pointer.
    ///
    /// `tip=` is the string the button would open rather than a ToolTip caught open: a pointer cannot be injected,
    /// and a disabled control takes hover and opens its attached ToolTip like any other one (実測 —
    /// rules-refs/app-ui.md §hover), so the binding that decides is the whole of what a run can read here.
    function reportFetchTip() {
        AppBackend.report("fetch_tip enabled=" + fetchButton.enabled
                          + " tip=" + (fetchButton.tip !== "")
                          + " remotes=" + (topBar.curPage !== null ? topBar.curPage.pageTab.remoteCount : -1))
    }
    Connections {
        target: topBar.curPage ? topBar.curPage.pageTab : null
        function onBusyOpChanged() {
            const op = topBar.curPage.pageTab.busyOp
            if (AppBackend.autoAct === "force-push-hold" && op === "push")
                topBar.autoPushBusyLatched = true
            if (AppBackend.autoAct === "fetch-busy" && op === "fetch")
                topBar.autoFetchBusyLatched = true
        }
    }

    /// Automation: the strip's own hooks, handed on. What `Main` and `WindowAutoActDriver` hold is the band, so the way
    /// in stays here after the tabs themselves have gone (`TabStrip`).
    function clickAppMenu() { tabStrip.clickAppMenu() }
    function middleClickTab(index) { return tabStrip.middleClickTab(index) }
    function dragTabTo(from, to) { return tabStrip.dragTabTo(from, to) }
    function holdTabAt(index) { return tabStrip.holdTabAt(index) }
    function heldTabShift() { return tabStrip.heldTabShift() }
    function carryTabPastEnd(index) { return tabStrip.carryTabPastEnd(index) }
    function heldTabIndex() { return tabStrip.heldIndex() }
    function dropCarriedTab() { tabStrip.dropTab() }
    function runAtEnd() { return tabStrip.runAtEnd() }
    function runOffset() { return tabStrip.runOffset() }
    function tabPaths() { return tabStrip.tabPaths() }
    function tabItemCount() { return tabStrip.tabItemCount() }
    function hasTabPath(path) { return tabStrip.hasTabPath(path) }
    function tabPathAt(index) { return tabStrip.tabPathAt(index) }
    function tabWidths() { return tabStrip.tabWidths() }
    function pointAtTab(index) { tabStrip.pointAtTab(index) }
    function tabMarks() { return tabStrip.tabMarks() }

    /// The word all three toolbar buttons are measured for: one box for the set keeps any of them from shifting the
    /// others, and which wording is wider is a question about the installed fonts. Settled once rather than bound: a
    /// binding that reads four text metrics and feeds three button widths is a loop as far as the engine is concerned.
    property string widestAction: ""
    property bool widestActionCode: false
    Component.onCompleted: {
        let best = widest.fetchCodeWidest
        for (const m of [widest.fetchWordWidest, widest.pushCodeWidest, widest.stashCodeWidest])
            if (m.implicitWidth > best.implicitWidth)
                best = m
        topBar.widestAction = best.text
        topBar.widestActionCode = best.code
    }
    // The four candidate wordings, drawn invisibly the way a button would draw them (`BandWidest`).
    BandWidest {
        id: widest
    }

    /// Automation: what the strip made of the run it was handed, and the two ends it was settled between
    /// (`PG_AUTO_ACT=tab-widths` / `badges`). Re-exposed for the reason the band's other readings are.
    readonly property real tabTitleCap: tabStrip.tabTitleCap
    readonly property int tabTitleMinW: tabStrip.tabTitleMinW
    readonly property int tabTitleMaxW: tabStrip.tabTitleMaxW

    implicitHeight: Theme.toolbarHeight
    color: Theme.bgElevated

    RowLayout {
        id: bandRow
        anchors.fill: parent
        // The band's own right edge is not the window's: the client area reaches past what is drawn, so a row flush
        // with it puts the close button's last few pixels off screen and its wash reads as clipped (measured: the cell
        // ended 4.5px beyond the visible edge). `spaceXs` lands it flush instead.
        anchors.rightMargin: topBar.captionMerged ? Theme.spaceXs : Theme.spaceMd
        spacing: Theme.spaceXs
        TabStrip {
            id: tabStrip
            tabsModel: topBar.tabsModel
            captionMerged: topBar.captionMerged
            curPage: topBar.curPage
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: tabStrip.tabStripFloorW
            // The band's order when it runs short: the tab names narrow together, then the state group's words do, then
            // the strip scrolls, then the group becomes a mark (2026-08-11 ユーザー指示) — a stretch this much larger than
            // the group's puts the strip at the front of both queues.
            Layout.horizontalStretchFactor: 100
            onOpenRepositoryRequested: topBar.openRepositoryRequested()
            onIdentityEditRequested: topBar.identityEditRequested()
            onSettingsRequested: topBar.settingsRequested()
            onCaptionStripMoved: topBar.captionStripMoved()
        }

        BandStateGroup {
            id: stateGroup
            curPage: topBar.curPage
            windowAtFloor: topBar.windowAtFloor
            pointedAt: topBar.statePointedAt
            // The group folds off the strip's width as well as its own, and the two do not always change in the same
            // frame.
            tabContentWidth: tabStrip.contentWidth
            tabRunAvail: tabStrip.runAvail
            tabCount: tabStrip.tabCount
            // Measured off the pair beside it rather than written to tokens of its own: what sets how big a target is
            // here is the padding a Fusion `ToolButton` keeps around its content — a number the theme does not have.
            // The button's `padding` rather than the two sides it settles to: those carry the shared box's slack as
            // well (`ActionButton.slack`), so reading them would move this group every time the fetch button changed
            // its wording.
            controlPadding: fetchButton.padding
            controlHeight: fetchButton.implicitHeight
            Layout.fillWidth: true
            Layout.maximumWidth: stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
            // Second in both queues (the strip's comment carries the order).
            Layout.horizontalStretchFactor: 1
            onIdentityEditRequested: topBar.identityEditRequested()
        }
        // Fetch, and everything the network has to say about fetching (`BandFetchButton`).
        BandFetchButton {
            id: fetchButton
            curPage: topBar.curPage
            busyLatched: topBar.autoFetchBusyLatched
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
        }
        // Push, in whichever shape this branch's standing with its remote allows (`BandPushButton`).
        BandPushButton {
            id: pushButton
            curPage: topBar.curPage
            busyLatched: topBar.autoPushBusyLatched
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
        }
        // Everything uncommitted, set aside in one entry, on the press (`BandStashButton`).
        BandStashButton {
            id: stashButton
            curPage: topBar.curPage
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
        }
        // Where what the app owns ends and what the window owns begins.
        Rectangle {
            id: chromeDivider
            visible: topBar.captionMerged
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.borderWidth
            implicitHeight: Theme.iconMd
            color: Theme.borderDefault
        }
        // The window's own three, drawn here rather than left to the platform: the platform's cannot be styled and its
        // maximize mark never becomes a restore mark (P3-確認事項 §ウィンドウ chrome).
        WindowButton {
            id: minimizeButton
            visible: topBar.captionMerged
            kind: "window-minimize"
            Accessible.name: qsTr("Minimize")
            onTriggered: topBar.minimizeRequested()
        }
        WindowButton {
            visible: topBar.captionMerged
            kind: topBar.windowMaximized ? "window-restore" : "window-maximize"
            Accessible.name: topBar.windowMaximized ? qsTr("Restore") : qsTr("Maximize")
            onTriggered: topBar.maximizeToggleRequested()
        }
        WindowButton {
            visible: topBar.captionMerged
            kind: "close"
            danger: true
            Accessible.name: qsTr("Close")
            onTriggered: topBar.closeRequested()
        }
    }

    // The run the divider stands in: the line, and the band's own spacing either side of it — which together are the
    // whole stretch between the last thing the app can be asked and the first thing the window can. The hit test
    // answers HTCAPTION for it the way it does for the run past the last tab (`TabStrip.grabArea`), so a press here
    // is the platform's own gesture rather than a press that lands on nothing (2026-08-23 ユーザー指示: 掴み代と同じ
    // 挙動。Qt's own hit test called it caption until the window took the message over — 59a7754f).
    //
    // Outside the row, so that measuring the row's own layout does not become part of it: a child of a `RowLayout` is
    // laid out, and this one only wants to know where two of the row's items came to rest. `bandRow` fills the band,
    // so its children's coordinates are this item's.
    Item {
        id: dividerRun
        x: bandRow.x + chromeDivider.x - bandRow.spacing
        // Nothing at all while the band is not the title bar: the divider is not drawn there, and a `RowLayout` leaves
        // an item it is not laying out at whatever geometry it last had.
        width: chromeDivider.visible ? chromeDivider.width + 2 * bandRow.spacing : 0
        height: topBar.height
        onXChanged: topBar.captionStripMoved()
        onWidthChanged: topBar.captionStripMoved()
        Component.onCompleted: topBar.captionStripMoved()
    }
}

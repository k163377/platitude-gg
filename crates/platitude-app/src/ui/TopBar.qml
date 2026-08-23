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
    /// the platform where its empty run sits — the gestures on that run (drag, snap, the double-click, the window menu)
    /// are the platform's own, because the hit test calls it caption (`AppBackend.setCaptionStrip`).
    property bool captionMerged: false
    /// Which shape the middle button is in.
    property bool windowMaximized: false

    /// Automation: what the band came out to. A layout change can lose the window's own buttons, or the run of band
    /// left to take hold of, or push either off the end — and none of that shows in a screenshot taken where the
    /// platform draws no buttons at all. The strip's own readings come back through here because the hooks ask the band
    /// rather than the strip (`WindowAutoActDriver`).
    readonly property real bandGrabRun: tabStrip.grabRun
    readonly property real bandButtonsX: minimizeButton.x
    readonly property real bandRightMargin: bandRow.anchors.rightMargin
    readonly property real bandTabsWidth: tabStrip.tabsWidth
    readonly property int bandTabCount: tabStrip.tabCount
    readonly property real bandTabRun: tabStrip.runAvail
    readonly property real bandTabContent: tabStrip.contentWidth
    readonly property bool bandTabScrolls: tabStrip.contentWidth > tabStrip.tabsWidth
    /// Automation: what the command log's mark came out to — the band's own reading of the log and the error line
    /// together, and the colour it painted from it. Asked here rather than of the page, because what is being checked
    /// is that the page's news reached this mark at all (`PG_AUTO_ACT=commands-clear`).
    readonly property bool commandsWrong: commandsToggle.wrong
    readonly property color commandsMarkColor: commandsToggle.markColor
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
    /// The run itself, for `Main` to measure in scene coordinates.
    readonly property Item grabRunItem: tabStrip.grabRunItem

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
        let best = fetchCodeWidest
        for (const m of [fetchWordWidest, pushCodeWidest, stashCodeWidest])
            if (m.implicitWidth > best.implicitWidth)
                best = m
        topBar.widestAction = best.text
        topBar.widestActionCode = best.code
    }
    // `push` is inside `push -f`, fetch says the command in every shape but the stopped one, and stash has the one
    // wording, so four cover all seven states. `stash` never wins — five monospaced cells is what `fetch` already is —
    // but it is measured rather than reasoned about, so re-wording any of them cannot leave the box short. Labels
    // rather than TextMetrics: TextMetrics reports a few pixels tighter than a Label — the set could be ranked on one
    // measure and sized by another, and the loser could then be the wider of them.
    component Widest: Label {
        visible: false
        property bool code: false
        font.family: code ? Theme.monoFamily : Theme.uiFamily
        font.wordSpacing: code ? -Theme.spaceXs : 0
        font.pixelSize: Theme.fontMd
    }
    Widest {
        id: fetchCodeWidest
        code: true
        text: "fetch"
    }
    Widest {
        id: fetchWordWidest
        text: qsTr("Resume")
    }
    Widest {
        id: pushCodeWidest
        code: true
        text: "push -f"
    }
    Widest {
        id: stashCodeWidest
        code: true
        text: "stash"
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
            // Measured off the pair beside it rather than written to tokens of its own — `commandsToggle` below carries
            // the reasoning, and this is the same number.
            controlPadding: fetchButton.padding
            controlHeight: fetchButton.implicitHeight
            Layout.fillWidth: true
            Layout.maximumWidth: stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
            // Second in both queues (the strip's comment carries the order).
            Layout.horizontalStretchFactor: 1
            onIdentityEditRequested: topBar.identityEditRequested()
        }
        CommandsToggle {
            id: commandsToggle
            curPage: topBar.curPage
            controlPadding: fetchButton.padding
            controlHeight: fetchButton.implicitHeight
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
        // Everything uncommitted, set aside in one entry, on the press (デザイン規約 §変更を退避する). It stands on the band
        // rather than over the file list: what it sets aside is the working tree, which is there whichever pane is
        // open, and the pane that could otherwise host it is one row's selection away most of the time.
        ActionButton {
            id: stashButton
            /// What the working tree lets this button do (デザイン規約 §変更を退避する). The refusals are read where they
            /// were measured (`platitude_core::stash::standing` — unborn / conflicts / clean / ready); `closed` is
            /// this band's own half: nothing open here to read a working tree off.
            readonly property string mode:
                topBar.curPage === null || topBar.curPage.pageTab.state !== "open"
                    || !topBar.curPage.pageWt.loaded
                ? "closed" : topBar.curPage.pageWt.stashStanding
            kind: "stash"
            // The label is the command, like both buttons beside it (デザイン規約 §git 用語のコード表記). One wording in every
            // state: nothing here changes what the press costs, so there is no second shape to say.
            text: "stash"
            code: true
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            // Nothing worn on top of that: a stash destroys nothing, so no frame, no `!` and no hold (デザイン規約
            // §変更を退避する). No ring either — the ring names a wait on the network and this write is local (§進行中・
            // 長押しの定数); while it runs the band is busy and the button is down, like the commit button beside its
            // own write.
            enabled: stashButton.mode === "ready" && topBar.curPage.pageTab.busyCount === 0
            tip: {
                // Nothing open, or another git command already out. Neither is about stashing, and both are said on
                // this band already (デザイン規約 §無効). Said out loud because a disabled control still takes hover and
                // still opens its attached ToolTip (実測: rules-refs/app-ui.md §hover).
                if (stashButton.mode === "closed" || topBar.curPage.pageTab.busyCount > 0)
                    return ""
                // The three that *are* about the working tree each name what is missing (デザイン規約 §hover のツールチップ).
                // They speak where the fetch button's refusals stay silent, because the reason is not on screen the way
                // `REMOTES 0` is: a tree with conflicts in it looks exactly like one that could be stashed, and the
                // band's own conflict badge says the repository has them — not that they are what is holding this
                // button down.
                if (stashButton.mode === "unborn")
                    return qsTr("No commits yet — git cannot stash before the first one")
                if (stashButton.mode === "conflicts")
                    return qsTr("Settle the conflicts first — git will not stash an unmerged file")
                if (stashButton.mode === "clean")
                    return qsTr("Nothing to stash — the working tree is clean")
                // The breadth is what the label has no room for, and with no card to read it is the only place it is
                // said (デザイン規約 §hover のツールチップ).
                return qsTr("Set these changes aside, files git is not tracking yet included")
            }
            // The name comes from the page rather than being asked for here: a summary already written for these
            // changes is the name the reader would have given them anyway (デザイン規約 §変更を退避する).
            onActivated: topBar.curPage.pageTab.pushStash(topBar.curPage.pageStashName)
        }
        // Where what the app owns ends and what the window owns begins.
        Rectangle {
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
}

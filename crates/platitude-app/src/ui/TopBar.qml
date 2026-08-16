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
    /// Whether this band is the window's title bar. When it is, the band
    /// carries the window's own buttons, and tells the platform where
    /// its empty run sits — the gestures on that run (drag, snap, the
    /// double-click, the window menu) are the platform's own, because
    /// the hit test calls it caption (`AppBackend.setCaptionStrip`).
    property bool captionMerged: false
    /// Which shape the middle button is in.
    property bool windowMaximized: false

    /// Automation: what the band came out to. A layout change can lose the
    /// window's own buttons, or the run of band left to take hold of, or
    /// push either off the end — and none of that shows in a screenshot
    /// taken where the platform draws no buttons at all. The strip's own
    /// readings come back through here because the hooks ask the band
    /// rather than the strip (`WindowAutoActDriver`).
    readonly property real bandGrabRun: tabStrip.grabRun
    readonly property real bandButtonsX: minimizeButton.x
    readonly property real bandRightMargin: bandRow.anchors.rightMargin
    readonly property real bandTabsWidth: tabStrip.tabsWidth
    readonly property int bandTabCount: tabStrip.tabCount
    readonly property real bandTabRun: tabStrip.runAvail
    readonly property real bandTabContent: tabStrip.contentWidth
    readonly property bool bandTabScrolls: tabStrip.contentWidth > tabStrip.tabsWidth
    /// Automation: what the command log's mark came out to — the band's
    /// own reading of the log and the error line together, and the colour
    /// it painted from it. Asked here rather than of the page, because
    /// what is being checked is that the page's news reached this mark
    /// at all (`PG_AUTO_ACT=commands-clear`).
    readonly property bool commandsWrong: commandsToggle.wrong
    readonly property color commandsMarkColor: commandsMark.color
    /// Automation: the four things that can be the matter here. The group
    /// owns the conditions and the hidden measurements behind them; what
    /// the hooks ask the band for comes back through here
    /// (`BandStateGroup`).
    readonly property bool opBadgeShown: stateGroup.opBadgeShown
    readonly property bool conflictBadgeShown: stateGroup.conflictBadgeShown
    readonly property bool identityBadgeShown: stateGroup.identityBadgeShown
    readonly property bool oldGitBadgeShown: stateGroup.oldGitBadgeShown
    readonly property real stateBadgeMinW: stateGroup.stateBadgeMinW

    /// Automation: which of the group's three shapes is on screen, what
    /// the badges were narrowed to, and what the card came back with
    /// (`PG_AUTO_ACT=badges` / `badges-hover`). The group is what makes
    /// them; the hooks come to the band to read them (`BandStateGroup`).
    readonly property bool stateWordsShown: stateGroup.stateWordsShown
    readonly property bool stateMarkShown: stateGroup.stateMarkShown
    readonly property color stateMarkColor: stateGroup.stateMarkColor
    readonly property int stateCapW: stateGroup.stateCapW
    readonly property int stateGroupW: stateGroup.stateGroupW
    readonly property bool stateCardOpen: stateGroup.stateCardOpen
    readonly property string stateCardRows: stateGroup.stateCardRows
    readonly property string stateCardSize: stateGroup.stateCardSize
    /// Whether the window is standing on its floor. Handed in, because
    /// the floor is the larger of this band's and the page's and only
    /// `Main` has both. The group gives up its words there whatever else
    /// is true (2026-08-11 ユーザー指示).
    property bool windowAtFloor: false
    /// Stands in for the pointer where headless cannot put one, so the
    /// card can be photographed (`badges-hover` / `identity-tip`). The
    /// real hover writes this same one property — hover is the input that
    /// cannot be injected, so the card has to be answering a single
    /// question or the headless run proves nothing about it.
    property bool statePointedAt: false

    signal openRepositoryRequested()
    signal identityEditRequested()
    signal settingsRequested()
    signal maximizeToggleRequested()
    signal minimizeRequested()
    signal closeRequested()
    /// The grab-run moved or changed size in this band's own layout.
    /// `Main` folds in the shifts this band cannot see from here (the
    /// maximised inset, the window resizing) and reports the strip on.
    signal captionStripMoved()
    /// The run itself, for `Main` to measure in scene coordinates.
    readonly property Item grabRunItem: tabStrip.grabRunItem

    /// The narrowest this band can be laid out at — one of the two
    /// numbers the window's floor is the larger of (`Main.floorWidth`).
    /// Read off the row's *minimum* rather than summed here, so a control
    /// added to the band later is counted without anybody remembering to.
    readonly property real floorWidth:
        bandRow.Layout.minimumWidth + bandRow.anchors.rightMargin

    /// Automation: run the push button's hold to its end. Does nothing
    /// unless the button is in the shape that arms it.
    function completePushHold() {
        pushButton.completeHold()
        AppBackend.report("push_hold mode=" + pushButton.mode)
    }

    /// Automation: the strip's own hooks, handed on. What `Main` and
    /// `WindowAutoActDriver` hold is the band, so the way in stays here
    /// after the tabs themselves have gone (`TabStrip`).
    function middleClickTab(index) { tabStrip.middleClickTab(index) }
    function tabPaths() { return tabStrip.tabPaths() }
    function tabPathAt(index) { return tabStrip.tabPathAt(index) }
    function tabWidths() { return tabStrip.tabWidths() }
    function pointAtTab(index) { tabStrip.pointAtTab(index) }
    function tabMarks() { return tabStrip.tabMarks() }

    /// The word both toolbar buttons are measured for: one box for the
    /// pair keeps either from shifting the other, and which wording is
    /// wider is a question about the installed fonts. Settled once rather
    /// than bound: a binding that reads four text metrics and feeds two
    /// button widths is a loop as far as the engine is concerned.
    property string widestAction: ""
    property bool widestActionCode: false
    Component.onCompleted: {
        let best = fetchCodeWidest
        for (const m of [fetchWordWidest, pushCodeWidest])
            if (m.implicitWidth > best.implicitWidth)
                best = m
        topBar.widestAction = best.text
        topBar.widestActionCode = best.code
    }
    // `push` is inside `push -f`, and fetch says the command in every
    // shape but the stopped one, so three cover all six states. Labels
    // rather than TextMetrics: TextMetrics reports a few pixels tighter
    // than a Label — the pair could be ranked on one measure and sized by
    // another, and the loser could then be the wider of the two.
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

    /// Automation: what the strip made of the run it was handed, and the
    /// two ends it was settled between (`PG_AUTO_ACT=tab-widths` /
    /// `badges`). Re-exposed for the reason the band's other readings are.
    readonly property real tabTitleCap: tabStrip.tabTitleCap
    readonly property int tabTitleMinW: tabStrip.tabTitleMinW
    readonly property int tabTitleMaxW: tabStrip.tabTitleMaxW

    implicitHeight: Theme.toolbarHeight
    color: Theme.bgElevated

    RowLayout {
        id: bandRow
        anchors.fill: parent
        // The band's own right edge is not the window's: the client area
        // reaches past what is drawn, so a row flush with it puts the
        // close button's last few pixels off screen and its wash reads as
        // clipped (measured: the cell ended 4.5px beyond the visible
        // edge). `spaceXs` lands it flush instead.
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
            // The band's order when it runs short: the tab names narrow
            // together, then the state group's words do, then the strip
            // scrolls, then the group becomes a mark (2026-08-11
            // ユーザー指示) — a stretch this much larger than the group's
            // puts the strip at the front of both queues.
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
            // The group folds off the strip's width as well as its own,
            // and the two do not always change in the same frame.
            tabContentWidth: tabStrip.contentWidth
            tabRunAvail: tabStrip.runAvail
            tabCount: tabStrip.tabCount
            // Measured off the pair beside it rather than written to
            // tokens of its own — `commandsToggle` below carries the
            // reasoning, and this is the same number.
            controlPadding: fetchButton.padding
            controlHeight: fetchButton.implicitHeight
            Layout.fillWidth: true
            Layout.maximumWidth: stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
            // Second in both queues (the strip's comment carries the
            // order).
            Layout.horizontalStretchFactor: 1
            onIdentityEditRequested: topBar.identityEditRequested()
        }
        // The git commands this tab ran; the mark also carries the state
        // of the last one.
        Rectangle {
            id: commandsToggle
            readonly property var log: topBar.curPage !== null
                                       ? topBar.curPage.pageCommands : null
            // Asked of the log rather than of the page: closing a tab
            // takes the page's models down while the page itself is
            // still standing, so `curPage !== null` is true for a beat
            // after there is nothing left to read off it.
            readonly property bool wrong:
                commandsToggle.log !== null
                && (commandsToggle.log.failed
                    || topBar.curPage.pageTab.lastError !== "")
            readonly property bool open: topBar.curPage !== null
                                         && topBar.curPage.commandsOpen

            visible: topBar.curPage !== null
            // Measured off the pair beside it rather than written to
            // tokens of its own: what sets how big a target is here is
            // the padding a Fusion `ToolButton` keeps around its content
            // — a number the theme does not have (sized from the table it
            // came out 24x20 against the neighbours' 92x32). `padding`
            // rather than the two sides it settles to: those carry the
            // shared box's slack as well (`ActionButton.slack`), so
            // reading them would move this mark every time the fetch
            // button changed its wording.
            implicitWidth: commandsMark.implicitWidth
                           + 2 * fetchButton.padding
            implicitHeight: fetchButton.implicitHeight
            radius: Theme.radiusSm
            color: open ? Theme.bgSelected
                   : commandsMouse.containsMouse ? Theme.bgHover
                   : "transparent"
            border.width: commandsToggle.wrong ? Theme.borderWidth : 0
            border.color: Theme.danger
            Label {
                id: commandsMark
                anchors.centerIn: parent
                text: ">_"
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontMd
                color: commandsToggle.wrong ? Theme.danger
                       : commandsToggle.log !== null && commandsToggle.log.running
                         ? Theme.accent
                       : commandsToggle.open ? Theme.textPrimary
                       : Theme.textMuted
            }
            MouseArea {
                id: commandsMouse
                anchors.fill: parent
                hoverEnabled: true
                onClicked: topBar.curPage.toggleCommands()
                ToolTip.visible: containsMouse
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: commandsToggle.open
                              ? qsTr("Hide the git commands this window ran")
                              : commandsToggle.wrong
                                ? qsTr("The last command failed — read it here")
                                : qsTr("Show the git commands this window ran")
            }
        }
        // Fetch, and everything the network has to say about fetching
        // (デザイン規約 §リモートから取り込む).
        ActionButton {
            id: fetchButton
            /// Fetches that failed in a row, whoever asked for them.
            readonly property int fails: topBar.curPage !== null
                                         ? topBar.curPage.pageTab.fetchFailures : 0
            /// Enough of them that the timer was stopped. Only a hold on
            /// this button starts it again.
            readonly property bool stopped: topBar.curPage !== null
                                            && topBar.curPage.pageTab.autoFetchSuspended

            kind: "fetch"
            text: fetchButton.stopped ? qsTr("Resume") : "fetch"
            code: !fetchButton.stopped
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            // Only the stopped step takes a colour for its word: a run of
            // failures is said by the frame and the mark while the word
            // stays plain (デザイン規約 §長押し — 警告の色は語ではなく
            // 枠と印が持つ).
            tone: fetchButton.stopped ? Theme.danger : Theme.textPrimary
            // Read off the state rather than off `tone`: what the ring
            // stands in for during the wait is the frame, not the word
            // (デザイン規約 §暗く落とした段).
            toneDim: fetchButton.stopped ? Theme.dangerDim
                     : fetchButton.fails > 0 ? Theme.warningDim
                     : Theme.textMuted
            frameColor: fetchButton.stopped ? Theme.danger
                        : fetchButton.fails > 0 ? Theme.warning
                        : "transparent"
            // Only while the word is still `fetch`: once it reads
            // `Resume`, the word is the news.
            alert: fetchButton.fails > 0 && !fetchButton.stopped
            alertTone: Theme.warning
            holdMs: fetchButton.stopped ? Metrics.holdMs : 0
            // Whoever asked for it, the network shows here: a fetch on
            // the timer turns the button the way a clicked one does.
            busy: topBar.curPage !== null
                  && (topBar.curPage.pageTab.busyOp === "fetch"
                      || topBar.curPage.pageTab.autoFetchRunning)
            enabled: topBar.curPage !== null
                     && topBar.curPage.pageTab.remoteCount > 0
                     && (fetchButton.stopped
                         || topBar.curPage.pageTab.busyCount === 0)
            ToolTip.visible: hovered
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: {
                if (topBar.curPage === null)
                    return ""
                const what = fetchButton.stopped
                             ? qsTr("Automatic fetching stopped after %n failure(s). Hold to start it again.", "",
                                    fetchButton.fails)
                             : qsTr("Fetch all remotes and prune deleted branches")
                const why = topBar.curPage.pageTab.autoFetchError
                if (fetchButton.fails > 0 && why !== "")
                    return what + "\n\n" + why
                if (fetchButton.fails > 0)
                    return what
                return what + "\n" + (AppBackend.autoFetchMinutes > 0
                                      ? qsTr("Automatically every %n minute(s)", "",
                                             AppBackend.autoFetchMinutes)
                                      : qsTr("Automatic fetching is off"))
            }
            onActivated: topBar.curPage.pageTab.fetch("")
            onHeld: topBar.curPage.pageTab.resumeAutoFetch()
        }
        // Push, in whichever shape this branch's standing with its remote
        // allows (デザイン規約 §リモートへ送る). The counts behind it are
        // from the last fetch, so they are believed only where they refuse.
        ActionButton {
            id: pushButton
            readonly property string mode:
                topBar.curPage !== null ? topBar.curPage.pushState : "closed"
            /// The last go at sending this branch came back refused. No
            /// stopped step past it: nothing sends on its own to be
            /// stopped (デザイン規約 §リモートへ送る).
            readonly property bool failed: topBar.curPage !== null
                                           && topBar.curPage.pushFailed
            /// The frame's warning colour, for either of the two things
            /// that call for it: what an overwrite would do, and what the
            /// last go did. The word takes it for only one of them (below).
            readonly property bool warned: pushButton.mode === "diverged"
                                           || pushButton.failed
            kind: "push"
            busy: topBar.curPage !== null
                  && topBar.curPage.pageTab.busyOp === "push"
            // Two fixed wordings, no counts: a number here would make the
            // button a different width for every value it took
            // (デザイン規約 §リモートへ送る).
            text: mode === "diverged" ? "push -f" : "push"
            code: true
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            // The overwrite colours its word; the refusal does not — only
            // one of them changes what the press costs (デザイン規約
            // §長押し — 警告の色は語ではなく枠と印が持つ).
            tone: pushButton.mode === "diverged" ? Theme.warning
                                                 : Theme.textPrimary
            // The ring and the frame keep the warning through the wait, a
            // step down; while the wait lasts there is no word to read, so
            // what the ring stands in for is the frame (デザイン規約
            // §暗く落とした段).
            toneDim: pushButton.warned ? Theme.warningDim : Theme.textMuted
            frameColor: pushButton.warned ? Theme.warning : "transparent"
            // The frame's colour is worn by the diverged shape as well, so
            // on its own it would not tell "cannot land plainly" from
            // "did not land".
            alert: pushButton.failed
            alertTone: Theme.warning
            holdMs: mode === "diverged" ? Metrics.holdMs : 0
            enabled: topBar.curPage !== null
                     && (topBar.curPage.canPush
                         || (mode === "diverged"
                             && topBar.curPage.canForcePush))
            onHeld: topBar.curPage.forcePush()
            ToolTip.visible: hovered
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: {
                if (topBar.curPage === null)
                    return ""
                const to = topBar.curPage.pushTargetLabel
                const what = pushButton.mode === "publish"
                             ? qsTr("This branch has not been sent anywhere yet — asks where it goes")
                           : pushButton.mode === "ready"
                             ? qsTr("Push %n commit(s) to %1", "",
                                    topBar.curPage.pageWt.ahead).arg(to)
                           : pushButton.mode === "clean"
                             ? qsTr("Nothing to push — %1 is up to date")
                               .arg(to)
                           : pushButton.mode === "behind"
                             ? qsTr("Nothing to push — %1 has moved ahead")
                               .arg(to)
                           : pushButton.mode === "diverged"
                             ? qsTr("Hold to overwrite %1, dropping %n commit(s) it has (as of the last fetch)", "",
                                    topBar.curPage.pageWt.behind).arg(to)
                           : ""
                // What git said, under what the button would do next —
                // this is the one place with room for why.
                const why = topBar.curPage.pushFailReason
                if (pushButton.failed && why !== "")
                    return what + "\n\n" + why
                return what
            }
            onActivated: topBar.curPage.pushNow()
        }
        // Where what the app owns ends and what the window owns begins.
        Rectangle {
            visible: topBar.captionMerged
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.borderWidth
            implicitHeight: Theme.iconMd
            color: Theme.borderDefault
        }
        // The window's own three, drawn here rather than left to the
        // platform: the platform's cannot be styled and its maximize mark
        // never becomes a restore mark (P3-確認事項 §ウィンドウ chrome).
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

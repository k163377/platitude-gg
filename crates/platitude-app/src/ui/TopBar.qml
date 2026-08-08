pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Top toolbar: prominent tabs and the per-repository controls share
// one row. `curPage` is the active RepoPage; its transient state
// (operation badge, conflicts, errors, push shape) surfaces here.
Rectangle {
    id: topBar

    required property var tabsModel
    // The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// Whether this band is the window's title bar. When it is, the band
    /// carries what a title bar carries — the window's own buttons, and
    /// the two gestures its empty run answers.
    property bool captionMerged: false
    /// Which shape the middle button is in.
    property bool windowMaximized: false

    signal openRepositoryRequested()
    signal identityEditRequested()
    signal settingsRequested()
    signal windowDragRequested()
    signal maximizeToggleRequested()
    signal minimizeRequested()
    signal closeRequested()

    /// One of the window's own buttons. Narrower than the cell the app
    /// menu sits in, and carrying the smaller mark: this end of the band
    /// already holds fetch and push, which a browser's does not, so the
    /// three have to give up the room a browser can spend on them. Full
    /// band height still, so the column of hit areas is unbroken.
    ///
    /// The close button is the one exception to the wash — red under the
    /// pointer is a convention old enough that departing from it would
    /// read as a bug, not as a house style.
    component WindowButton: Rectangle {
        id: winBtn
        property string kind: ""
        property bool danger: false
        signal triggered()

        width: Theme.controlHeight
        height: parent ? parent.height : Theme.toolbarHeight
        color: !winBtnMouse.containsMouse ? "transparent"
               : winBtn.danger ? Theme.danger : Theme.bgHover
        NavIcon {
            anchors.centerIn: parent
            width: Theme.iconMd
            height: Theme.iconMd
            kind: winBtn.kind
            tint: winBtnMouse.containsMouse && winBtn.danger
                  ? Theme.textOnAccent : Theme.textPrimary
        }
        MouseArea {
            id: winBtnMouse
            anchors.fill: parent
            hoverEnabled: true
            onClicked: winBtn.triggered()
        }
    }

    /// Automation: run the push button's hold to its end. Does nothing
    /// unless the button is in the shape that arms it.
    function completePushHold() {
        pushButton.completeHold()
        AppBackend.report("push_hold mode=" + pushButton.mode)
    }

    /// What a press on a tab does, in one place: the left button picks
    /// the tab up, the middle one closes it (デザイン規約 §タブの所作).
    /// The real press and the smoke hook both come through here, so the
    /// two buttons are told apart once rather than twice.
    function pressTab(index, id, button) {
        if (button === Qt.MiddleButton)
            topBar.tabsModel.closeTab(id)
        else
            topBar.tabsModel.setCurrentIndex(index)
    }

    /// Automation: the middle click, landed on the tab at `index`
    /// (`PG_AUTO_ACT=middle-close`).
    function middleClickTab(index) {
        const tab = tabs.itemAtIndex(index)
        if (tab)
            topBar.pressTab(index, tab.tab_id, Qt.MiddleButton)
    }

    /// Automation: the strip as it stands now. A closed tab leaves
    /// nothing of itself behind, so which repositories are still open is
    /// what says the gesture took the tab it was aimed at — the paths
    /// rather than the titles, because every demo repository is called
    /// the same thing and a strip of one name proves nothing.
    function tabPaths() {
        let paths = []
        for (let i = 0; i < tabs.count; i++) {
            const tab = tabs.itemAtIndex(i)
            if (tab)
                paths.push(tab.repo_path)
        }
        return paths.join(",")
    }

    /// The word both toolbar buttons are measured for. They sit side by
    /// side and change wording independently, so one box for the pair is
    /// what keeps either of them from shifting the other — and which of
    /// the wordings is the wider one is a question about the installed
    /// fonts, not about their spelling. Two of the four are commands and
    /// are set in the mono family (デザイン規約 §git 用語のコード表記),
    /// so the winner carries which family it was measured in.
    ///
    /// Settled once rather than bound: a binding that reads four text
    /// metrics and feeds two button widths is a loop as far as the engine
    /// is concerned, and the answer cannot change while the app runs.
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
    // The longest each button can say, in each of the two voices it says
    // things in. `push` is inside `push -f`, and fetch says the command in
    // every shape but the stopped one, so three cover all six states —
    // push never leaves the command's voice (デザイン規約 §リモートへ送る).
    //
    // Labels rather than TextMetrics, and never drawn: the box these are
    // ranked for is a Label's, and TextMetrics reports a few pixels
    // tighter than one — enough that the pair could be ranked on one
    // measure and sized by another, and the loser could then be the wider
    // of the two (`ActionButton.widestText`).
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

    implicitHeight: Theme.toolbarHeight
    color: Theme.bgElevated

    RowLayout {
        anchors.fill: parent
        // The window's own buttons run to the very edge where the band is
        // the title bar: the corner of a maximised window is the one
        // target a pointer cannot overshoot.
        anchors.rightMargin: topBar.captionMerged ? 0 : Theme.spaceMd
        // The tighter step between this row's controls. A browser's band
        // carries the window's buttons and nothing else; this one carries
        // the command log, fetch and push as well, so the same spacing a
        // browser can afford would push the three off the end of what the
        // eye reads as one group.
        spacing: Theme.spaceXs
        // The strip: the app menu, the tabs, the way to open one more, and
        // whatever band is left over. Placed by hand rather than by a Row,
        // because the first three have to touch and the leftover has to be
        // measurable — it is both what the tabs may grow into and where
        // the window is taken hold of.
        //
        // No TabBar either: full control of the geometry is what puts the
        // selected tab's underline exactly on the band's bottom edge with
        // no styling leftovers beneath it.
        Item {
            id: tabStrip
            Layout.fillWidth: true
            Layout.fillHeight: true
            // App menu (Claude-Desktop-style hamburger); most entries are
            // placeholders until their phases land.
            //
            // The head of the folded sidebar's column rather than a member
            // of the tab strip: `railWidth` wide, the band's full height,
            // and the same square wash the rail's cells wear, so this mark
            // and the section marks under it stand on one line. `iconLg`
            // rather than `iconMd` because the step follows the height of
            // the band the mark sits in — what `menu` actually draws is
            // 10px wide inside a 16px box (P3-確認事項 §ハンバーガーのサイズ).
            ToolButton {
                id: menuButton
                width: Theme.railWidth
                height: tabStrip.height
                padding: 0
                Accessible.name: qsTr("Application menu")
                // Written out instead of borrowing HoverToolButton: that
                // one rounds its wash and the rail's cells do not. An open
                // menu keeps the wash — what is on screen has to say which
                // mark put it there (`NavRail`).
                background: Rectangle {
                    color: menuButton.hovered || appMenu.opened
                           ? Theme.bgHover : "transparent"
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
                    // A full-height cell ends where the band does, so the
                    // card would otherwise open on top of the divider.
                    y: menuButton.height + Theme.splitterWidth
                    AppMenuItem {
                        text: qsTr("Open repository…")
                        onTriggered: topBar.openRepositoryRequested()
                    }
                    AppMenuItem {
                        text: qsTr("Clone repository…")
                        enabled: false
                    }
                    AppMenuSeparator {}
                    // A local re-read (no network). The page re-reads
                    // itself on a tick while it is on screen, so this is
                    // here for where that cannot reach — a repository on a
                    // share that reads slowly, or a read that failed —
                    // rather than for everyday use, and it costs no
                    // toolbar room to keep.
                    AppMenuItem {
                        text: qsTr("Reload")
                        enabled: topBar.curPage !== null
                        onTriggered: topBar.curPage.pageTab.refreshAll()
                    }
                    AppMenuSeparator {}
                    AppMenuItem {
                        text: qsTr("Identity…")
                        onTriggered: topBar.identityEditRequested()
                    }
                    AppMenuItem {
                        text: qsTr("Settings…")
                        onTriggered: topBar.settingsRequested()
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
            // As wide as the tabs it holds, up to what the strip has left
            // once the menu and the `+` have their places. Past that it
            // scrolls: the alternative is the tabs pushing the repository's
            // own controls off the end of the band, and those are the ones
            // that have to stay where the hand expects them.
            //
            // Tabs keep their natural width for now — narrowing them needs
            // a floor to narrow towards, and that is a number the design
            // document does not have yet (P3-確認事項 §ウィンドウ chrome).
            ListView {
                id: tabs
                x: menuButton.width
                height: tabStrip.height
                /// Band the tabs may not grow into. Where this row is the
                /// title bar, the empty run past the last tab is the only
                /// place left to take hold of the window, and a run that
                /// can be squeezed to nothing by opening one more tab is
                /// not somewhere anyone would think to reach for. Two of
                /// the menu's cells is enough to read as a gap rather than
                /// as spacing.
                readonly property real grabRun: topBar.captionMerged
                                                ? 2 * Theme.railWidth : 0
                width: Math.max(0, Math.min(contentWidth,
                                            tabStrip.width - menuButton.width
                                            - plusButton.width - grabRun))
                orientation: ListView.Horizontal
                // Hard stop at the ends, as everywhere else that scrolls
                // (デザイン規約 §QML 実装ルール).
                boundsBehavior: Flickable.StopAtBounds
                clip: true
                // Every delegate stays alive however far the strip is
                // scrolled: the automation walks the items for their
                // paths and ids (`tabPaths` / `middleClickTab`), and a
                // released delegate answers those walks with null. Tabs
                // are counted in ones, so keeping them all costs nothing.
                cacheBuffer: 65536
                // A wheel over a strip that runs sideways should move it
                // sideways, whichever way the wheel itself reports: a
                // plain wheel only ever says "vertical", and it is the
                // only wheel most people have.
                WheelHandler {
                    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                    onWheel: event => {
                        const step = event.angleDelta.x !== 0
                                     ? event.angleDelta.x : event.angleDelta.y
                        tabs.contentX = Math.max(
                            0, Math.min(tabs.contentWidth - tabs.width,
                                        tabs.contentX - step))
                    }
                }
                model: topBar.tabsModel
                delegate: Rectangle {
                    id: tabItem
                    required property int index
                    required property int tab_id
                    required property string title
                    required property string repo_path
                    readonly property bool current: topBar.tabsModel.currentIndex === index
                    width: tabContent.implicitWidth + 2 * Theme.spaceSm
                    height: tabs.height
                    color: current ? Theme.bgSelected : "transparent"
                    // The middle button is taken here rather than on the
                    // `✕`: the whole tab answers to it, so closing one
                    // never asks the hand to find a 16px target — and the
                    // `✕` sits on top of this area without accepting the
                    // middle button, so a press that lands on the mark
                    // falls through to the same gesture.
                    MouseArea {
                        id: tabMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                        onClicked: mouse => topBar.pressTab(tabItem.index,
                                                            tabItem.tab_id,
                                                            mouse.button)
                    }
                    Rectangle {
                        anchors.fill: parent
                        color: Theme.bgHover
                        visible: tabMouse.containsMouse && !tabItem.current
                    }
                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        height: 2 * Theme.borderWidth
                        color: Theme.accent
                        visible: tabItem.current
                    }
                    RowLayout {
                        id: tabContent
                        anchors.fill: parent
                        anchors.leftMargin: Theme.spaceSm
                        anchors.rightMargin: Theme.spaceXs
                        spacing: Theme.spaceXs
                        Label {
                            text: tabItem.title
                            elide: Text.ElideRight
                            Layout.maximumWidth: 180
                            Layout.fillHeight: true
                            verticalAlignment: Text.AlignVCenter
                            font.weight: tabItem.current ? Font.DemiBold : Font.Normal
                            color: tabItem.current ? Theme.textPrimary
                                                   : Theme.textSecondary
                        }
                        HoverToolButton {
                            padding: 0
                            Layout.alignment: Qt.AlignVCenter
                            implicitWidth: Theme.iconLg
                            implicitHeight: Theme.iconLg
                            contentItem: Item {
                                NavIcon {
                                    anchors.centerIn: parent
                                    width: Theme.iconMd
                                    height: Theme.iconMd
                                    kind: "close"
                                    tint: Theme.textPrimary
                                }
                            }
                            onClicked: topBar.tabsModel.closeTab(tabItem.tab_id)
                        }
                    }
                }
            }
            HoverToolButton {
                id: plusButton
                x: tabs.x + tabs.width
                anchors.verticalCenter: parent.verticalCenter
                text: "+"
                font.pixelSize: Theme.fontLg
                onClicked: topBar.openRepositoryRequested()
            }
            // The run of empty band past the last tab. Where this band is
            // the title bar, it is also the only place left to take hold
            // of the window, so it answers a drag by moving it and a
            // double click by maximising it — the two things the bar it
            // replaced did. Neither is wired anywhere else: a press that
            // lands on a tab, a button or a badge belongs to that.
            Item {
                x: plusButton.x + plusButton.width
                width: Math.max(0, tabStrip.width - x)
                height: tabStrip.height
                DragHandler {
                    enabled: topBar.captionMerged
                    // Nothing here follows the pointer: the platform takes
                    // the press over and moves the window itself, which is
                    // the only way a window can be dragged without fighting
                    // the compositor.
                    target: null
                    onActiveChanged: if (active) topBar.windowDragRequested()
                }
                TapHandler {
                    enabled: topBar.captionMerged
                    // Lets go of the press as soon as it turns into a drag,
                    // so the handler above can have it.
                    gesturePolicy: TapHandler.DragThreshold
                    onDoubleTapped: topBar.maximizeToggleRequested()
                }
            }
        }

        // Transient state of the current repository. The badge says what
        // is stopped and how far it got; the ways out of it stand in the
        // working-tree pane, under the button that finishes things
        // (デザイン規約 §進行中の操作から出る).
        Rectangle {
            id: opBadge
            readonly property var wt: topBar.curPage !== null
                                      ? topBar.curPage.pageWt : null
            visible: opBadge.wt !== null && opBadge.wt.opText !== ""
            color: "transparent"
            border.color: Theme.warning
            border.width: Theme.borderWidth
            radius: Theme.radiusSm
            implicitHeight: Theme.iconLg
            implicitWidth: opLabel.implicitWidth + 2 * Theme.spaceXs
            Label {
                id: opLabel
                anchors.centerIn: parent
                // The count is the half a stopped rebase cannot say
                // without it; a merge steps through nothing and has none.
                text: opBadge.wt === null ? ""
                    : opBadge.wt.opSteps > 0
                      ? qsTr("%1 %2/%3").arg(opBadge.wt.opText)
                        .arg(opBadge.wt.opStep).arg(opBadge.wt.opSteps)
                      : opBadge.wt.opText
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
            }
        }
        Rectangle {
            visible: topBar.curPage !== null && topBar.curPage.pageWt.hasConflicts
            color: Theme.danger
            radius: Theme.radiusSm
            implicitHeight: Theme.iconLg
            implicitWidth: conflictLabel.implicitWidth + 2 * Theme.spaceXs
            Label {
                id: conflictLabel
                anchors.centerIn: parent
                text: qsTr("CONFLICTS")
                color: Theme.textOnAccent
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
            }
        }
        // Nothing to attribute commits to. Kept next to the other
        // repository-state badges so the way back to the setup screen
        // stays visible after "Not now".
        Rectangle {
            visible: AppBackend.identityState === "missing"
                     || (topBar.curPage !== null
                         && !topBar.curPage.pageTab.identityReady)
            color: "transparent"
            border.color: Theme.warning
            border.width: Theme.borderWidth
            radius: Theme.radiusSm
            implicitHeight: Theme.iconLg
            implicitWidth: identityBadge.implicitWidth + 2 * Theme.spaceXs
            Rectangle {
                anchors.fill: parent
                radius: Theme.radiusSm
                color: Theme.bgHover
                visible: identityBadgeMouse.containsMouse
            }
            Label {
                id: identityBadge
                anchors.centerIn: parent
                text: qsTr("SET IDENTITY")
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
            }
            MouseArea {
                id: identityBadgeMouse
                anchors.fill: parent
                hoverEnabled: true
                onClicked: topBar.identityEditRequested()
                ToolTip.visible: containsMouse
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("No name or email set for commits")
            }
        }
        // The git commands this tab ran. Closed, this mark is the whole
        // of the feature on screen; it is also where the state of the
        // last one shows, so the toolbar says something failed without
        // spending a line on a message nobody can read in 320px.
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
            implicitWidth: Theme.spaceXl
            implicitHeight: Theme.iconLg
            radius: Theme.radiusSm
            color: open ? Theme.bgSelected
                   : commandsMouse.containsMouse ? Theme.bgHover
                   : "transparent"
            border.width: commandsToggle.wrong ? Theme.borderWidth : 0
            border.color: Theme.danger
            Label {
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
        // Fetch, and everything the network has to say about fetching.
        // There is no indicator beside it: a second thing on the toolbar
        // saying the same three states was one place too many for the
        // reader to look (デザイン規約 §リモートから取り込む).
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
            // Stopped, the button is no longer about one fetch: it is the
            // way back to fetching on its own, and it says so — in words,
            // where every other shape of it says the command.
            text: fetchButton.stopped ? qsTr("Resume") : "fetch"
            code: !fetchButton.stopped
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            tone: fetchButton.stopped ? Theme.danger
                  : fetchButton.fails > 0 ? Theme.warning
                  : Theme.textPrimary
            // The same three, a step down, for the wait: a fetch that runs
            // while the last ones failed is still the button that failed
            // (デザイン規約 §暗く落とした段).
            toneDim: fetchButton.stopped ? Theme.dangerDim
                     : fetchButton.fails > 0 ? Theme.warningDim
                     : Theme.textMuted
            frameColor: fetchButton.stopped ? Theme.danger
                        : fetchButton.fails > 0 ? Theme.warning
                        : "transparent"
            // Only while the word is still `Fetch`: once it reads
            // `Resume`, the word is the news and a mark beside it is the
            // fourth thing on one button saying the same thing.
            alert: fetchButton.fails > 0 && !fetchButton.stopped
            holdMs: fetchButton.stopped ? Metrics.holdMs : 0
            // Whoever asked for it, the network shows here: a fetch on
            // the timer turns the button the way a clicked one does.
            busy: topBar.curPage !== null
                  && (topBar.curPage.pageTab.busyOp === "fetch"
                      || topBar.curPage.pageTab.autoFetchRunning)
            still: AppBackend.shotDir !== ""
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
                             ? qsTr("Automatic fetching stopped after %n failure(s)."
                                    + " Hold to start it again.", "",
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
            /// The last go at sending this branch came back refused. The
            /// same warning a fetch wears after one or two failures, and
            /// for the same reason; there is no stopped step past it,
            /// because nothing sends on its own to be stopped
            /// (デザイン規約 §リモートへ送る).
            readonly property bool failed: topBar.curPage !== null
                                           && topBar.curPage.pushFailed
            /// Warning colour, for either of the two things that call for
            /// it: what an overwrite would do, and what the last go did.
            readonly property bool warned: pushButton.mode === "diverged"
                                           || pushButton.failed
            kind: "push"
            busy: topBar.curPage !== null
                  && topBar.curPage.pageTab.busyOp === "push"
            still: AppBackend.shotDir !== ""
            // Two fixed wordings, no counts: how far ahead the branch is
            // stands in the sidebar and in this button's own tooltip, and
            // a number here would make the button a different width for
            // every value it took (デザイン規約 §リモートへ送る).
            //
            // The button says the command in every state, first push
            // included: what makes that one different is not the command
            // but that nothing here knows where it goes yet, and the
            // question that opens is where that is said.
            text: mode === "diverged" ? "push -f" : "push"
            code: true
            // Every shape of both buttons measured against the longest of
            // them, so the toolbar's right-hand end sits still while the
            // branch's standing with its remote changes under it.
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            tone: pushButton.warned ? Theme.warning : Theme.textPrimary
            // A force push on the wire is still a force push, and a second
            // go at one git turned down is still the button that was turned
            // down, so the ring and the frame keep the warning through the
            // wait, a step down (デザイン規約 §暗く落とした段).
            toneDim: pushButton.warned ? Theme.warningDim : Theme.textMuted
            frameColor: pushButton.warned ? Theme.warning : "transparent"
            // Said past the word, where a fetch says it: the frame's colour
            // is worn by the diverged shape as well, so on its own it would
            // not tell "cannot land plainly" from "did not land".
            alert: pushButton.failed
            // Diverged, the button stays live for the hold that is its
            // only gesture: a plain push cannot land there, so nothing
            // else is waiting on a click to be mistaken for.
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
                // Nothing to name a target with in the first state: where
                // it goes is what the press is about to ask.
                const what = pushButton.mode === "publish"
                             ? qsTr("This branch has not been sent anywhere "
                                    + "yet — asks where it goes")
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
                             ? qsTr("Hold to overwrite %1, dropping %n "
                                    + "commit(s) it has (as of the last "
                                    + "fetch)", "",
                                    topBar.curPage.pageWt.behind).arg(to)
                           : ""
                // What git said, under what the button would do next —
                // the mark says only that the last go failed, and this is
                // the one place with room for why (the same two-part
                // tooltip a failed fetch carries).
                const why = topBar.curPage.pushFailReason
                if (pushButton.failed && why !== "")
                    return what + "\n\n" + why
                return what
            }
            onActivated: topBar.curPage.pushNow()
        }
        // Where what the app owns ends and what the window owns begins.
        // Without it the two groups read as one row of controls with an
        // odd gap in it — the same mark a browser puts in the same place.
        // `iconMd` tall rather than the whole band: a rule that reached
        // the edges would be a second divider, and the one under the band
        // already says where it stops.
        Rectangle {
            visible: topBar.captionMerged
            Layout.alignment: Qt.AlignVCenter
            Layout.leftMargin: Theme.spaceXs
            implicitWidth: Theme.borderWidth
            implicitHeight: Theme.iconMd
            color: Theme.borderDefault
        }
        // The window's own three. Drawn here rather than left to the
        // platform: its own are a fixed 32px in a 40px band, with a hover
        // and glyphs that cannot be styled and a maximize mark that never
        // becomes a restore mark (P3-確認事項 §ウィンドウ chrome).
        WindowButton {
            visible: topBar.captionMerged
            Layout.leftMargin: Theme.spaceXs
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

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

    signal openRepositoryRequested()
    signal identityEditRequested()
    signal settingsRequested()

    implicitHeight: Theme.toolbarHeight
    color: Theme.bgElevated

    RowLayout {
        anchors.fill: parent
        anchors.rightMargin: Theme.spaceMd
        spacing: Theme.spaceSm
        // App menu (Claude-Desktop-style hamburger); most entries are
        // placeholders until their phases land.
        HoverToolButton {
            id: menuButton
            text: "☰"
            font.pixelSize: Theme.fontLg
            Layout.leftMargin: Theme.spaceXs
            Layout.alignment: Qt.AlignVCenter
            padding: 0
            implicitWidth: Theme.spaceXl
            implicitHeight: Theme.spaceXl
            onClicked: appMenu.open()
            AppMenu {
                id: appMenu
                y: menuButton.height
                AppMenuItem {
                    text: qsTr("Open repository…")
                    onTriggered: topBar.openRepositoryRequested()
                }
                AppMenuItem {
                    text: qsTr("Clone repository…")
                    enabled: false
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
                    text: qsTr("About platitude-gg")
                    enabled: false
                }
                AppMenuSeparator {}
                AppMenuItem {
                    text: qsTr("Exit")
                    onTriggered: Qt.quit()
                }
            }
        }
        // Plain Row tabs (no TabBar): full control of the geometry so
        // the selected underline sits exactly on the toolbar's bottom
        // edge with no styling leftovers beneath it.
        Row {
            id: tabRow
            Layout.fillHeight: true
            spacing: 0
            Repeater {
                model: topBar.tabsModel
                Rectangle {
                    id: tabItem
                    required property int index
                    required property int tab_id
                    required property string title
                    required property string repo_path
                    readonly property bool current: topBar.tabsModel.currentIndex === index
                    width: tabContent.implicitWidth + 2 * Theme.spaceSm
                    height: tabRow.height
                    color: current ? Theme.bgSelected : "transparent"
                    MouseArea {
                        id: tabMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: topBar.tabsModel.setCurrentIndex(tabItem.index)
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
                            text: "×"
                            padding: 0
                            Layout.alignment: Qt.AlignVCenter
                            implicitWidth: Theme.iconLg
                            implicitHeight: Theme.iconLg
                            onClicked: topBar.tabsModel.closeTab(tabItem.tab_id)
                        }
                    }
                }
            }
        }
        HoverToolButton {
            text: "+"
            font.pixelSize: Theme.fontLg
            onClicked: topBar.openRepositoryRequested()
        }
        Item { Layout.fillWidth: true }

        // Transient state of the current repository.
        Rectangle {
            visible: topBar.curPage !== null && topBar.curPage.pageWt.opText !== ""
            color: "transparent"
            border.color: Theme.warning
            border.width: Theme.borderWidth
            radius: Theme.radiusSm
            implicitHeight: Theme.iconLg
            implicitWidth: opLabel.implicitWidth + 2 * Theme.spaceXs
            Label {
                id: opLabel
                anchors.centerIn: parent
                text: topBar.curPage !== null ? topBar.curPage.pageWt.opText : ""
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
                ToolTip.delay: 600
                ToolTip.text: qsTr("git has no name or email to record on commits")
            }
        }
        Label {
            visible: topBar.curPage !== null && topBar.curPage.pageTab.lastError !== ""
            text: topBar.curPage !== null ? topBar.curPage.pageTab.lastError : ""
            color: Theme.danger
            elide: Text.ElideRight
            Layout.maximumWidth: 320
            font.pixelSize: Theme.fontSm
            background: Rectangle {
                color: Theme.bgHover
                visible: errorClearMouse.containsMouse
            }
            MouseArea {
                id: errorClearMouse
                anchors.fill: parent
                hoverEnabled: true
                onClicked: if (topBar.curPage !== null) topBar.curPage.pageTab.clearLastError()
            }
        }
        // Auto fetch: quiet by design. A machine that is simply offline
        // fails here once a minute, and that belongs in a tooltip
        // rather than in the error line.
        Label {
            readonly property bool failing: topBar.curPage !== null
                                            && topBar.curPage.pageTab.autoFetchError !== ""
            text: "↻"
            color: topBar.curPage !== null && topBar.curPage.pageTab.autoFetchRunning
                   ? Theme.accent
                   : failing ? Theme.warning
                   : AppBackend.autoFetchMinutes > 0 ? Theme.textMuted
                   : Theme.borderDefault
            font.pixelSize: Theme.fontMd
            background: Rectangle {
                radius: Theme.radiusSm
                color: Theme.bgHover
                visible: fetchIndicatorMouse.containsMouse
            }
            MouseArea {
                id: fetchIndicatorMouse
                anchors.fill: parent
                hoverEnabled: true
                onClicked: topBar.settingsRequested()
                ToolTip.visible: containsMouse
                ToolTip.delay: 600
                ToolTip.text: parent.failing
                              ? topBar.curPage.pageTab.autoFetchError
                              : AppBackend.autoFetchMinutes > 0
                                ? qsTr("Fetching every %n minute(s)", "",
                                       AppBackend.autoFetchMinutes)
                                : qsTr("Automatic fetching is off")
            }
        }
        // Reserved: search box (backlog)
        SlimField {
            enabled: false
            opacity: 0.35
            placeholderText: qsTr("Search")
            implicitWidth: 160
        }
        ActionButton {
            kind: "fetch"
            text: qsTr("Fetch")
            busy: topBar.curPage !== null
                  && topBar.curPage.pageTab.busyOp === "fetch"
            still: AppBackend.shotDir !== ""
            enabled: topBar.curPage !== null
                     && topBar.curPage.pageTab.remoteCount > 0
                     && topBar.curPage.pageTab.busyCount === 0
            ToolTip.visible: hovered
            ToolTip.delay: 600
            ToolTip.text: qsTr("Fetch every remote now, pruning branches "
                               + "they no longer have")
            onClicked: topBar.curPage.pageTab.fetch("")
        }
        // The remote moved past this branch, so a plain push cannot land.
        // The button itself only greys out, and a disabled button says
        // nothing on this palette — the state is announced beside it.
        Rectangle {
            visible: pushButton.mode === "diverged"
            color: "transparent"
            border.color: Theme.warning
            border.width: Theme.borderWidth
            radius: Theme.radiusSm
            implicitHeight: Theme.iconLg
            implicitWidth: divergedBadge.implicitWidth + 2 * Theme.spaceXs
            Label {
                id: divergedBadge
                anchors.centerIn: parent
                text: qsTr("REMOTE MOVED")
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
            }
        }
        // Push, in whichever shape this branch's standing with its remote
        // allows (デザイン規約 §リモートへ送る). The counts behind it are
        // from the last fetch, so they are believed only where they refuse.
        ActionButton {
            id: pushButton
            readonly property string mode:
                topBar.curPage !== null ? topBar.curPage.pushState : "closed"
            kind: "push"
            busy: topBar.curPage !== null
                  && topBar.curPage.pageTab.busyOp === "push"
            still: AppBackend.shotDir !== ""
            text: mode === "publish" ? qsTr("Publish")
                  : mode === "ready"
                    ? qsTr("Push ↑%1").arg(topBar.curPage.pageWt.ahead)
                    : qsTr("Push")
            enabled: topBar.curPage !== null && topBar.curPage.canPush
            ToolTip.visible: hovered
            ToolTip.delay: 600
            ToolTip.text: topBar.curPage === null ? ""
                          : mode === "publish"
                            ? qsTr("Publish this branch as %1")
                              .arg(topBar.curPage.pushTargetLabel)
                          : mode === "ready"
                            ? qsTr("Push this branch to %1")
                              .arg(topBar.curPage.pushTargetLabel)
                          : mode === "clean"
                            ? qsTr("%1 already has every commit on this branch")
                              .arg(topBar.curPage.pushTargetLabel)
                          : mode === "behind"
                            ? qsTr("%1 has moved on, and this branch has "
                                   + "nothing to add to it")
                              .arg(topBar.curPage.pushTargetLabel)
                          : mode === "diverged"
                            ? qsTr("%1 has commits this branch does not, as "
                                   + "seen at the last fetch, so only an "
                                   + "overwrite can put this branch there")
                              .arg(topBar.curPage.pushTargetLabel)
                          : ""
            onClicked: topBar.curPage.pushNow()
            // Overwriting a remote's history is the one push that needs
            // asking about, so it lives behind its own entry.
            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.RightButton
                onClicked: pushMenu.popup()
            }
            AppMenu {
                id: pushMenu
                AppMenuItem {
                    text: qsTr("Force push…")
                    enabled: topBar.curPage !== null
                             && topBar.curPage.canForcePush
                    onTriggered: topBar.curPage.forcePushNow()
                }
            }
        }
        // Local re-read only (no network).
        HoverToolButton {
            text: qsTr("Reload")
            enabled: topBar.curPage !== null
            ToolTip.visible: hovered
            ToolTip.delay: 600
            ToolTip.text: qsTr("Re-read this repository from disk (does not fetch)")
            onClicked: topBar.curPage.pageTab.refreshAll()
        }
    }
}

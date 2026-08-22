pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// What stands in the middle of the graph when there is no history to draw: the first load's turning mark, whatever went
// wrong instead, and the empty window's one thing worth doing.
Item {
    id: emptyState

    required property var graphModel
    /// No repository behind the pane at all.
    required property bool blank

    signal openRepositoryRequested()

    // First load, before any row exists. The drawn ring, not Fusion's BusyIndicator — the window has one turning mark
    // and this is it (規約 §進行中・長押しの定数).
    SpinnerIcon {
        anchors.centerIn: parent
        width: Theme.iconLg
        height: Theme.iconLg
        spinning: emptyState.graphModel.loading && emptyState.graphModel.rowTotal === 0
    }
    NoticeLine {
        anchors.centerIn: parent
        visible: emptyState.graphModel.error !== ""
        text: emptyState.graphModel.error
        color: Theme.danger
        width: parent.width - 2 * Theme.spaceXl
    }
    // Empty window: the one thing worth doing sits in the column that will hold the history.
    Column {
        anchors.centerIn: parent
        visible: emptyState.blank
        spacing: Theme.spaceLg
        Label {
            text: Words.appName
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        NoticeLine {
            text: qsTr("A thin, fast GUI over your installed git.")
            color: Theme.textSecondary
            // Its natural width while the column has room for it, and wrapped inside the column when it has not: at the
            // window's floor this pane is narrower than the line asks for, and unbounded it took the difference from
            // the panes on either side (規約 §窓の床). The same shape the error line above already has.
            width: Math.min(implicitWidth, emptyState.width - 2 * Theme.spaceXl)
            anchors.horizontalCenter: parent.horizontalCenter
        }
        NoticeButton {
            text: Words.openRepository
            onActivated: emptyState.openRepositoryRequested()
        }
    }
}

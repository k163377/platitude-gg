pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// What stands in the middle of the graph when there is no history to
// draw: the first load's turning mark, whatever went wrong instead, and
// the empty window's one thing worth doing.
Item {
    id: emptyState

    required property var graphModel
    /// No repository behind the pane at all.
    required property bool blank

    signal openRepositoryRequested()

    // First load, before any row exists. The drawn ring, not Fusion's
    // BusyIndicator — the window has one turning mark and this is it
    // (規約 §進行中・長押しの定数).
    SpinnerIcon {
        anchors.centerIn: parent
        width: Theme.iconLg
        height: Theme.iconLg
        spinning: emptyState.graphModel.loading
                  && emptyState.graphModel.rowTotal === 0
    }
    Label {
        anchors.centerIn: parent
        visible: emptyState.graphModel.error !== ""
        text: emptyState.graphModel.error
        color: Theme.danger
        width: parent.width - 2 * Theme.spaceXl
        wrapMode: Text.Wrap
        horizontalAlignment: Text.AlignHCenter
    }
    // Empty window: the one thing worth doing sits in the column that
    // will hold the history.
    Column {
        anchors.centerIn: parent
        visible: emptyState.blank
        spacing: Theme.spaceLg
        Label {
            text: qsTr("Platitude GG")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Label {
            text: qsTr("A thin, fast GUI over your installed git.")
            color: Theme.textSecondary
            // Its natural width while the column has room for it, and
            // wrapped inside the column when it has not: at the window's
            // floor this pane is narrower than the line asks for, and
            // unbounded it took the difference from the panes on either
            // side (規約 §窓の床). The same shape the error line above
            // already has.
            width: Math.min(implicitWidth,
                            emptyState.width - 2 * Theme.spaceXl)
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            anchors.horizontalCenter: parent.horizontalCenter
        }
        // A plain frame, not the accent: the accent is for the button
        // somebody came to press, and this page is what stands there when
        // nobody has opened anything yet. A frame all the same — bare is
        // the shape an answer takes beside a framed one, and there is
        // nothing beside this to read it against; under two lines of
        // centred text a button with no edge is a third line
        // (規約 §肯定側のボタン / §枠を持てる場所にだけ枠を出す).
        ActionButton {
            implicitHeight: Theme.controlHeight
            text: qsTr("Open repository…")
            frameColor: Theme.borderDefault
            activeFocusOnTab: true
            anchors.horizontalCenter: parent.horizontalCenter
            onActivated: emptyState.openRepositoryRequested()
        }
    }
}

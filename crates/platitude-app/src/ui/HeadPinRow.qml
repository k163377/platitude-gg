pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The current branch never leaves the viewport: while its own row is scrolled off, this stand-in rides the edge the row
// went out of, and it steps aside the moment the row itself is on screen — so the sidebar never shows the branch twice.
// A detached HEAD (and a branch a filter or a folded folder hides) has no row at all, so the stand-in stays on top.
//
// Whoever uses this has to adopt it onto the list itself (`parent:`), not declare it inside: a Flickable's declared
// children are taken by its content item and scroll away with it (app-ui.md).
Rectangle {
    id: headPin

    required property var branchesModel
    required property var workTree
    /// Where the list is standing and how tall it is — which edge this rides, and whether it is needed at all, is read
    /// off the two.
    required property real contentY
    required property real viewHeight

    /// The pointer stand-in the rows carry, for the row this one stands
    /// for (PG_AUTO_ACT=nav-tip head): hover cannot be injected, so what
    /// the pointer would light is written in the same one place the
    /// pointer's own arrival writes.
    property bool pointed: false

    signal activated(string oidHex)

    readonly property real rowTop: headPin.branchesModel.headRow * Theme.rowHeight
    readonly property bool rowAbove: headPin.branchesModel.headRow < 0 || headPin.rowTop < headPin.contentY
    readonly property bool rowBelow: headPin.rowTop + Theme.rowHeight > headPin.contentY + headPin.viewHeight

    visible: (headPin.branchesModel.headName !== "" || headPin.workTree.detached)
             && (headPin.rowAbove || headPin.rowBelow)
    height: Theme.rowHeight
    y: headPin.rowAbove ? 0 : headPin.viewHeight - height
    // Dressed as the row it stands for, down to the margins: the current branch's own highlight, not a header band.
    color: Theme.accentMuted
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: headRowMouse.containsMouse || headPin.pointed
    }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceMd
        anchors.rightMargin: Theme.spaceSm
        spacing: Theme.spaceXs
        // The rows' mark slot, left empty: the stand-in has no mark of its own, but its name has to begin in the same
        // column as the rows it rides above.
        Item {
            Layout.preferredWidth: Theme.iconMd
            Layout.preferredHeight: Theme.iconMd
            Layout.alignment: Qt.AlignVCenter
        }
        Label {
            Layout.fillWidth: true
            text: headPin.workTree.detached ? qsTr("DETACHED HEAD") : headPin.branchesModel.headName
            color: headPin.workTree.detached ? Theme.warning : Theme.textLink
            font.weight: Font.DemiBold
            font.pixelSize: Theme.fontMd
            elide: Text.ElideMiddle
        }
        HeadTrack {
            visible: !headPin.workTree.detached && headPin.workTree.upstream !== ""
            ahead: headPin.workTree.ahead
            behind: headPin.workTree.behind
            Layout.alignment: Qt.AlignVCenter
        }
        NavIcon {
            visible: !headPin.workTree.detached
                     && (headPin.branchesModel.headHasRemote || headPin.branchesModel.headHasPr)
            kind: headPin.branchesModel.headHasPr ? "pr" : "remote"
            tint: headPin.branchesModel.headHasPr ? Theme.success : Theme.textSecondary
            width: Theme.iconSm
            height: Theme.iconSm
        }
    }
    /// The name in full, as the row this one stands for would say it (デザイン規約 §hover のツールチップ). A detached HEAD has no
    /// name to spell, and the words standing in for one are not a name.
    readonly property string tipWords: headPin.workTree.detached ? "" : headPin.branchesModel.headName
    ToolTip.visible: (headRowMouse.containsMouse || headPin.pointed) && headPin.tipWords !== ""
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: headPin.tipWords

    // Hairline on the side the scrolled rows pass under.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        y: headPin.rowAbove ? parent.height - height : 0
        height: Theme.borderWidth
        color: Theme.borderSubtle
    }
    MouseArea {
        id: headRowMouse
        anchors.fill: parent
        hoverEnabled: true
        enabled: headPin.branchesModel.headOid !== ""
        onClicked: headPin.activated(headPin.branchesModel.headOid)
    }
}

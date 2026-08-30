pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The current branch never leaves the viewport: while its own row is scrolled off, this stand-in rides the edge the row
// went out of, and it steps aside the moment the row itself is on screen — so the sidebar never shows the branch twice.
// A branch a filter or a folded folder hides has no row at all, so there is no edge to ride: the stand-in takes a place
// of its own at the head of the list (`seated`) rather than covering the first row that is there.
//
// **A detached HEAD is not stood in for at all.** There is no branch to keep on screen, and the words for one are not
// a name; where HEAD is standing is said by the graph's pin, by the WORKTREES row and by the commit button's own
// wording (デザイン規約 §左メニューの所作).
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
    /// The branch is there but its row is not — a filter or a folded folder is holding it. Nothing to ride above, so
    /// this takes a row of its own and the list begins one row lower (`NavSections` reads it for `topMargin`). Read
    /// off the model alone: the list's own height answers to this, so reading its geometry back would be a loop, and
    /// a list with a top margin rests at a negative `contentY` — the edges below cannot be asked in that state.
    readonly property bool seated: headPin.branchesModel.headName !== "" && headPin.branchesModel.headRow < 0
    readonly property bool rowAbove: headPin.seated || headPin.rowTop < headPin.contentY
    readonly property bool rowBelow: !headPin.seated
                                     && headPin.rowTop + Theme.rowHeight > headPin.contentY + headPin.viewHeight

    visible: headPin.branchesModel.headName !== "" && (headPin.rowAbove || headPin.rowBelow)
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
        // The panel's own margin and the folds the row it stands for is nested by (`NavItemDelegate.rowInset` /
        // `nestStep`, which the list sets to the one value — see there). **The fold count comes from that row**
        // (`headDepth`): a branch carrying a `/` sits one step in and `main` sits at none, so a stand-in that always
        // took one step began its name in a column no row was in. While it is seated there is no row to follow and the
        // model answers 0, which is the column the list's own rows begin in.
        anchors.leftMargin: Theme.spaceSm + headPin.branchesModel.headDepth * Theme.spaceSm
        // The rows' own gutter, so the stand-in's ahead/behind and badge stand in the same column as theirs — the one
        // a pane's own bar takes (`NavItemDelegate` / `PaneScrollBar`).
        anchors.rightMargin: Theme.navBarGutter
        spacing: Theme.spaceXs
        // The rows' mark slot, left empty: the stand-in has no mark of its own, but its name has to begin in the same
        // column as the rows it rides above — so it takes their seat, which is the one a fold arrow and a state mark
        // stand in (`NameCell.seatSize` on a list with no change codes in it — the ink of an `iconSm` mark, not its box).
        Item {
            Layout.preferredWidth: Theme.iconXs
            Layout.preferredHeight: Theme.iconXs
            Layout.alignment: Qt.AlignVCenter
        }
        CutName {
            Layout.fillWidth: true
            text: headPin.branchesModel.headName
            color: Theme.textLink
            weight: Font.DemiBold
            pixelSize: Theme.fontMd
        }
        HeadTrack {
            visible: headPin.workTree.upstream !== ""
            ahead: headPin.workTree.ahead
            behind: headPin.workTree.behind
            Layout.alignment: Qt.AlignVCenter
        }
        NavIcon {
            visible: headPin.branchesModel.headHasRemote || headPin.branchesModel.headHasPr
            kind: headPin.branchesModel.headHasPr ? "pr" : "remote"
            tint: headPin.branchesModel.headHasPr ? Theme.success : Theme.textSecondary
            width: Theme.iconSm
            height: Theme.iconSm
        }
    }
    /// The name in full, as the row this one stands for would say it (デザイン規約 §hover のツールチップ).
    readonly property string tipWords: headPin.branchesModel.headName
    ToolTip.visible: headRowMouse.containsMouse || headPin.pointed
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: headPin.tipWords

    // Hairline on the side the scrolled rows pass under — its foot while it is seated, where the rows begin instead.
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

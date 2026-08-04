pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One sidebar row: ref / file / folder, shared by every section and the
// WIP file list. What a click means is the owner's business — the row
// only reports it.
Item {
    id: navRow
    required property int index
    required property string name
    required property string full
    required property string oid_hex
    required property string change
    required property string bucket
    required property string orig_path
    required property bool is_head
    required property bool has_remote
    required property bool has_pr
    required property int depth
    required property bool folder
    required property bool collapsed
    property string kindHint: "branch"
    property string headTrack: ""
    property real listWidth: 200
    // Shows the hover stage/unstage affordance (WIP view).
    property bool showStage: false

    signal refClicked(string oidHex)
    signal fileClicked(string bucket, string path, string origPath)
    signal folderClicked(string key)
    signal stageClicked(string bucket, string path)
    /// Right-click on a ref row; the page owns the menu because
    /// delegates are recycled out from under an open popup.
    signal refMenuRequested(string name, string oidHex)
    /// Right-click on a working-tree file row, for the same reason.
    signal fileMenuRequested(string bucket, string path)

    width: listWidth
    height: Theme.rowHeight

    // The current branch stays highlighted inside the list (the sidebar's
    // sticky row only stands in for it while this row is scrolled off).
    Rectangle {
        anchors.fill: parent
        color: Theme.accentMuted
        visible: navRow.is_head && !navRow.folder
    }
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: itemMouse.containsMouse
    }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceMd + navRow.depth * Theme.spaceMd
        anchors.rightMargin: Theme.spaceSm
        spacing: Theme.spaceXs
        // Every row opens with this slot, held open even when empty, so
        // that at a given depth all names begin in the same column: a
        // folder's fold arrow, a worktree file's change icon, and later
        // the mark for a hidden branch all live here. Letting the slot
        // collapse is what put a leaf's name to the *left* of the folder
        // it sits under (layouts drop invisible children entirely).
        Item {
            Layout.preferredWidth: Theme.iconSm + 2
            Layout.preferredHeight: Theme.iconSm + 2
            Layout.alignment: Qt.AlignVCenter
            Label {
                anchors.fill: parent
                visible: navRow.folder
                text: navRow.collapsed ? "▸" : "▾"
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            ChangeIcon {
                anchors.fill: parent
                visible: !navRow.folder && navRow.kindHint === "wt"
                change: navRow.change
                ToolTip.visible: wtHover.containsMouse
                ToolTip.delay: 600
                ToolTip.text: {
                    const c = navRow.change.length > 0 ? navRow.change[0] : ""
                    const what = navRow.change.length === 2 ? qsTr("Conflicted")
                               : c === "M" ? qsTr("Modified")
                               : c === "A" ? qsTr("Added")
                               : c === "D" ? qsTr("Deleted")
                               : c === "R" ? qsTr("Renamed")
                               : c === "C" ? qsTr("Copied")
                               : c === "T" ? qsTr("Type changed")
                               : c === "?" ? qsTr("Untracked") : navRow.change
                    const where = navRow.bucket === "staged" ? qsTr("staged")
                                : navRow.bucket === "unstaged" ? qsTr("unstaged")
                                : navRow.bucket === "untracked" ? qsTr("untracked")
                                : qsTr("conflict")
                    return what + " · " + where
                }
                MouseArea {
                    id: wtHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }
        }
        Label {
            Layout.fillWidth: true
            text: navRow.name
            elide: Text.ElideMiddle
            font.weight: navRow.is_head ? Font.DemiBold : Font.Normal
            color: navRow.folder ? Theme.textSecondary
                   : navRow.is_head ? Theme.textLink : Theme.textPrimary
            font.pixelSize: Theme.fontMd
        }
        // Worktree rows: checked-out branch on the right.
        Label {
            visible: !navRow.folder && navRow.kindHint === "worktree"
            text: navRow.bucket !== "" ? navRow.bucket : qsTr("detached")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            elide: Text.ElideMiddle
            Layout.maximumWidth: navRow.listWidth / 2
        }
        // Current branch's ahead/behind, left of the state icon.
        Label {
            visible: !navRow.folder && navRow.kindHint === "branch"
                     && navRow.is_head && navRow.headTrack !== ""
            text: navRow.headTrack
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
        // Branch remote state: nothing = local only, remote icon =
        // has a remote, PR icon = has a PR (real data in Phase 4;
        // PG_FAKE_PR previews the look). Remote-branch and worktree
        // rows show the PR state too.
        NavIcon {
            visible: !navRow.folder
                     && ((navRow.kindHint === "branch"
                          && (navRow.has_remote || navRow.has_pr))
                         || ((navRow.kindHint === "remote"
                              || navRow.kindHint === "worktree")
                             && navRow.has_pr))
            kind: navRow.has_pr ? "pr" : "remote"
            tint: navRow.has_pr ? Theme.success : Theme.textSecondary
            width: Theme.iconSm + 2
            height: Theme.iconSm + 2
            ToolTip.visible: remoteHover.containsMouse
            ToolTip.delay: 600
            ToolTip.text: navRow.has_pr ? qsTr("Has an open pull request")
                                        : qsTr("Has a remote branch")
            MouseArea {
                id: remoteHover
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.NoButton
            }
        }
    }
    MouseArea {
        id: itemMouse
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton) {
                // Only branch-like rows have operations behind them.
                if (!navRow.folder && navRow.oid_hex !== ""
                        && (navRow.kindHint === "branch"
                            || navRow.kindHint === "remote"))
                    navRow.refMenuRequested(
                        navRow.full !== "" ? navRow.full : navRow.name,
                        navRow.oid_hex)
                else if (!navRow.folder && navRow.kindHint === "wt")
                    navRow.fileMenuRequested(
                        navRow.bucket,
                        navRow.full !== "" ? navRow.full : navRow.name)
                return
            }
            if (navRow.folder)
                navRow.folderClicked(navRow.full)
            else if (navRow.kindHint === "wt")
                navRow.fileClicked(navRow.bucket,
                                   navRow.full !== "" ? navRow.full : navRow.name,
                                   navRow.orig_path)
            else if (navRow.kindHint === "worktree")
                navRow.fileClicked("worktree", navRow.full, "")
            else if (navRow.oid_hex !== "")
                navRow.refClicked(navRow.oid_hex)
        }
    }
    // Hover stage/unstage affordance.
    HoverToolButton {
        visible: navRow.showStage && !navRow.folder
                 && (itemMouse.containsMouse || hovered)
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        padding: 0
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        ToolTip.visible: hovered
        ToolTip.delay: 300
        ToolTip.text: navRow.bucket === "staged" ? qsTr("Unstage file")
                                                 : qsTr("Stage file")
        onClicked: navRow.stageClicked(
            navRow.bucket, navRow.full !== "" ? navRow.full : navRow.name)
        contentItem: NavIcon {
            kind: navRow.bucket === "staged" ? "minus" : "plus"
            tint: navRow.bucket === "staged" ? Theme.diffRemovedFg
                                             : Theme.diffAddedFg
        }
    }
    // Nested leaves show only their last segment; hover reveals the
    // full name.
    ToolTip.visible: itemMouse.containsMouse && !navRow.folder
                     && navRow.full !== "" && navRow.full !== navRow.name
    ToolTip.delay: 700
    ToolTip.text: navRow.full
}

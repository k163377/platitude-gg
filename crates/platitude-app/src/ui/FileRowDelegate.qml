pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A changed-file row (commit file list): tree view shows indented,
// collapsible directory rows with leaf names; path view shows flat
// full paths.
Item {
    id: fileRow
    required property var model
    property real listWidth: 200

    readonly property string changeText: model.change ?? ""
    readonly property string nameText: model.name ?? ""
    readonly property string pathText: model.path ?? ""
    readonly property string origPathText: model.orig_path ?? ""
    readonly property bool isFolder: (model.folder ?? false) === true

    /// Stands in for the pointer where headless cannot put one, so a
    /// cut-down row's tooltip can be photographed (PG_AUTO_ACT=path-tip).
    /// -1 points at no row.
    property int pointedTipRow: -1
    readonly property bool tipPointedAt:
        fileRow.pointedTipRow === fileRow.model.index

    signal activated(string bucket, string path, string origPath)
    signal folderToggled(string key)

    width: listWidth
    height: Theme.rowHeight

    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: fileMouse.containsMouse
    }

    // The mark and the name are the part both file lists say the same way
    // (`NameCell`), down to the arrow a rename puts between its two names.
    // The whole of this row is that part; what a click means is the only
    // thing it keeps to itself.
    NameCell {
        id: nameCell
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm + (fileRow.model.depth ?? 0) * Theme.spaceMd
        anchors.rightMargin: Theme.spaceSm
        folder: fileRow.isFolder
        change: fileRow.changeText
        name: fileRow.nameText
        origPath: fileRow.origPathText
    }
    MouseArea {
        id: fileMouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: {
            if (fileRow.isFolder)
                fileRow.folderToggled(fileRow.pathText)
            else
                fileRow.activated("", fileRow.pathText, fileRow.origPathText)
        }
    }
    // Tree leaves show only their file name; hover reveals the path. A
    // paths-view row the pane elided answers the same way — its display
    // name is the full path, so elision is the one thing that leaves
    // the whole name unsaid (デザイン規約 §hover のツールチップ). A
    // folder row is the elision case alone: the rows above it already
    // spell its prefix, so only a chain the pane cut short has anything
    // left to say.
    ToolTip.visible: (fileMouse.containsMouse || fileRow.tipPointedAt)
                     && (fileRow.isFolder
                         ? nameCell.truncated
                         : (fileRow.nameText !== fileRow.pathText
                            || nameCell.truncated))
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: fileRow.pathText
}

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

    signal activated(string bucket, string path, string origPath)
    signal folderToggled(string key)

    width: listWidth
    height: Theme.rowHeight

    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: fileMouse.containsMouse
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm + (fileRow.model.depth ?? 0) * Theme.spaceMd
        anchors.rightMargin: Theme.spaceSm
        spacing: Theme.spaceXs
        NavIcon {
            visible: fileRow.isFolder
            width: Theme.iconSm
            height: Theme.iconSm
            kind: "chevron"
            rotation: (fileRow.model.collapsed ?? false) ? 0 : 90
            tint: Theme.textSecondary
        }
        ChangeIcon {
            visible: !fileRow.isFolder
            change: fileRow.changeText
            width: Theme.iconSm + 2
            height: Theme.iconSm + 2
            ToolTip.visible: changeHover.containsMouse
            ToolTip.delay: 600
            ToolTip.text: {
                const c = fileRow.changeText.length > 0 ? fileRow.changeText[0] : ""
                return c === "M" ? qsTr("Modified")
                     : c === "A" ? qsTr("Added")
                     : c === "D" ? qsTr("Deleted")
                     : c === "R" ? qsTr("Renamed")
                     : c === "C" ? qsTr("Copied")
                     : c === "T" ? qsTr("Type changed") : fileRow.changeText
            }
            MouseArea {
                id: changeHover
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.NoButton
            }
        }
        Label {
            Layout.fillWidth: true
            text: fileRow.isFolder || fileRow.origPathText === ""
                  ? fileRow.nameText
                  : qsTr("%1 → %2").arg(fileRow.origPathText).arg(fileRow.nameText)
            elide: Text.ElideMiddle
            font.pixelSize: Theme.fontMd
            color: fileRow.isFolder ? Theme.textSecondary : Theme.textPrimary
        }
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
    // Tree leaves show only their file name; hover reveals the path.
    ToolTip.visible: fileMouse.containsMouse && !fileRow.isFolder
                     && fileRow.nameText !== fileRow.pathText
    ToolTip.delay: 700
    ToolTip.text: fileRow.pathText
}

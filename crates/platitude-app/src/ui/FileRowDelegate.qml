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
            // A folder row has no change code, so it keeps its fold state
            // in that field (`models::nav::FOLDED`).
            rotation: fileRow.changeText === "FOLDED" ? 0 : 90
            tint: Theme.textSecondary
        }
        ChangeIcon {
            visible: !fileRow.isFolder
            change: fileRow.changeText
            width: Theme.iconMd
            height: Theme.iconMd
        }
        // A rename is two names with the way between them drawn rather
        // than typed: U+2192 is East Asian Ambiguous, so the CJK families
        // this app names hold it in a full-width cell and each draws its
        // own arrow inside it (規約 §寸法「印はフォントの字に任せない」).
        // Both names give ground when the row is narrow. The old name is
        // capped at its own width so it shrinks without growing
        // (app-ui.md §`Layout.fillWidth` を書いていない子は縮まない床);
        // **the new one is not capped** — someone in the row has to take
        // the slack, and a row where every child refuses it hands the
        // leftover to the engine, which centres what it cannot fill
        // (measured: plain rows drifted to the middle of the pane).
        RowLayout {
            Layout.fillWidth: true
            spacing: 0
            Label {
                id: origName
                visible: !fileRow.isFolder && fileRow.origPathText !== ""
                Layout.fillWidth: true
                Layout.maximumWidth: origName.implicitWidth
                text: fileRow.origPathText
                elide: Text.ElideMiddle
                font.pixelSize: Theme.fontMd
                color: Theme.textPrimary
            }
            Item {
                visible: origName.visible
                Layout.preferredWidth: renameMark.inkWidth + Theme.spaceXs * 2
                Layout.preferredHeight: Theme.iconSm
                Layout.alignment: Qt.AlignVCenter
                NavIcon {
                    id: renameMark
                    anchors.centerIn: parent
                    kind: "arrow"
                    // Beside a word, so a step under the row's own mark,
                    // with the line taken down by the same ratio
                    // (app-ui.md §語の隣に立つ印).
                    width: Theme.iconSm
                    height: Theme.iconSm
                    stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                    tint: Theme.textSecondary
                }
            }
            Label {
                id: newName
                Layout.fillWidth: true
                text: fileRow.nameText
                elide: Text.ElideMiddle
                font.pixelSize: Theme.fontMd
                color: fileRow.isFolder ? Theme.textSecondary : Theme.textPrimary
            }
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
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: fileRow.pathText
}

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Center pane, diff mode: one file's unified diff with per-file,
// per-hunk and per-line staging affordances, plus image / binary
// previews. Which file is shown and what staging means is the owner's
// business — this pane reports intents.
Rectangle {
    id: diffPane

    required property var diffModel
    // Whether the shown diff is a working-tree file (stageable).
    property bool fromWorkTree: false
    // Whether it is the staged side (flips the affordance wording).
    property bool staged: false
    // A write is running: staging buttons disable.
    property bool busy: false

    signal closeRequested()
    /// Stage or unstage the whole file (direction follows `staged`).
    signal stageFileRequested()
    /// Stage or unstage one hunk (line < 0) or one line of it.
    signal stageSelectionRequested(int hunk, int line)

    color: Theme.bgSurface
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.headerHeight
            color: Theme.bgElevated
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceSm
                anchors.rightMargin: Theme.spaceSm
                spacing: Theme.spaceSm
                Label {
                    text: qsTr("DIFF · %1").arg(diffPane.diffModel.title)
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                    color: Theme.textSecondary
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
                HoverToolButton {
                    visible: diffPane.fromWorkTree
                    text: diffPane.staged ? qsTr("Unstage file")
                                          : qsTr("Stage file")
                    font.pixelSize: Theme.fontSm
                    enabled: !diffPane.busy
                    ToolTip.visible: hovered
                    ToolTip.delay: 300
                    ToolTip.text: diffPane.staged
                        ? qsTr("Take this whole file out of the next commit")
                        : qsTr("Put this whole file into the next commit")
                    onClicked: diffPane.stageFileRequested()
                }
                HoverToolButton {
                    text: "×"
                    implicitWidth: Theme.iconLg
                    implicitHeight: Theme.iconLg
                    padding: 0
                    onClicked: diffPane.closeRequested()
                }
            }
        }
        // -- content preview: binaries summarized by size, images
        //    rendered (added = After only, deleted = Before only,
        //    modified = both).
        Label {
            visible: diffPane.diffModel.previewKind === "binary"
                     || (diffPane.diffModel.isBinary
                         && diffPane.diffModel.previewKind === "")
            Layout.margins: Theme.spaceSm
            Layout.fillWidth: true
            elide: Text.ElideRight
            text: {
                const oldS = diffPane.diffModel.previewOldSize
                const newS = diffPane.diffModel.previewNewSize
                if (oldS !== "" && newS !== "")
                    return qsTr("Binary file · %1 → %2").arg(oldS).arg(newS)
                if (newS !== "")
                    return qsTr("Binary file · %1").arg(newS)
                if (oldS !== "")
                    return qsTr("Binary file removed · was %1").arg(oldS)
                return qsTr("Binary file — no text diff")
            }
            color: Theme.textMuted
        }
        RowLayout {
            visible: diffPane.diffModel.previewKind === "image"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: Theme.spaceSm
            spacing: Theme.spaceSm
            ImagePreviewCell {
                Layout.fillWidth: true
                Layout.fillHeight: true
                label: qsTr("Before · %1").arg(diffPane.diffModel.previewOldSize)
                url: diffPane.diffModel.previewOldUrl
                sizeText: diffPane.diffModel.previewOldSize
            }
            ImagePreviewCell {
                Layout.fillWidth: true
                Layout.fillHeight: true
                label: qsTr("After · %1").arg(diffPane.diffModel.previewNewSize)
                url: diffPane.diffModel.previewNewUrl
                sizeText: diffPane.diffModel.previewNewSize
            }
        }
        ListView {
            id: diffList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: diffPane.diffModel
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AutoScrollBar {}
            // An image with no text rows hands its space to the preview
            // (SVG edits keep both).
            visible: diffPane.diffModel.previewKind !== "image"
                     || count > 0
            delegate: Rectangle {
                id: diffRow
                required property string kind
                required property int old_no
                required property int new_no
                required property string text
                required property int hunk
                required property int line
                width: diffList.width
                height: Theme.rowHeight
                color: kind === "add" ? Theme.diffAddedBg
                       : kind === "del" ? Theme.diffRemovedBg
                       : kind === "hunk" ? Theme.diffHunkHeaderBg
                       : "transparent"
                Row {
                    anchors.fill: parent
                    spacing: 0
                    Label {
                        width: 42
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        text: diffRow.old_no >= 0 ? diffRow.old_no : ""
                        horizontalAlignment: Text.AlignRight
                        rightPadding: Theme.spaceXs
                        color: Theme.textMuted
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontSm
                    }
                    Label {
                        width: 42
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        text: diffRow.new_no >= 0 ? diffRow.new_no : ""
                        horizontalAlignment: Text.AlignRight
                        rightPadding: Theme.spaceXs
                        color: Theme.textMuted
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontSm
                    }
                    Label {
                        width: parent.width - 84
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        text: diffRow.text
                        elide: Text.ElideRight
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontMd
                        color: diffRow.kind === "add" ? Theme.diffAddedFg
                               : diffRow.kind === "del" ? Theme.diffRemovedFg
                               : diffRow.kind === "hunk" ? Theme.diffHunkHeaderFg
                               : diffRow.kind === "meta" ? Theme.textMuted
                               : Theme.textPrimary
                    }
                }
                MouseArea {
                    id: lineHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                    enabled: diffPane.fromWorkTree
                }
                // Hunk-level staging. The row carries the hunk index the
                // patch builder needs, so what is staged is exactly what
                // is shown.
                HoverToolButton {
                    visible: diffPane.fromWorkTree && diffRow.kind === "hunk"
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.spaceSm
                    anchors.verticalCenter: parent.verticalCenter
                    text: diffPane.staged ? qsTr("Unstage hunk")
                                          : qsTr("Stage hunk")
                    font.pixelSize: Theme.fontSm
                    enabled: !diffPane.busy
                    onClicked: diffPane.stageSelectionRequested(diffRow.hunk, -1)
                }
                // Line-level staging.
                Rectangle {
                    visible: diffPane.fromWorkTree && lineHover.containsMouse
                             && (diffRow.kind === "add"
                                 || diffRow.kind === "del")
                    x: Theme.spaceXs
                    anchors.verticalCenter: parent.verticalCenter
                    width: Theme.iconMd
                    height: Theme.iconMd
                    radius: Theme.radiusSm
                    color: Theme.bgElevated
                    border.color: Theme.borderStrong
                    border.width: Theme.borderWidth
                    ToolTip.visible: stageLineHover.containsMouse
                    ToolTip.delay: 300
                    ToolTip.text: diffPane.staged ? qsTr("Unstage this line")
                                                  : qsTr("Stage this line")
                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.radiusSm
                        color: Theme.bgHover
                        visible: stageLineHover.containsMouse
                    }
                    NavIcon {
                        anchors.centerIn: parent
                        width: Theme.iconSm
                        height: Theme.iconSm
                        kind: diffRow.kind === "add" ? "plus" : "minus"
                        tint: Theme.textPrimary
                    }
                    MouseArea {
                        id: stageLineHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: diffPane.stageSelectionRequested(diffRow.hunk,
                                                                    diffRow.line)
                    }
                }
            }
        }
    }
}

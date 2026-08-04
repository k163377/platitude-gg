pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Right pane, commit-details mode: message, author card, stash
// actions when the selected row is a stash, and the changed-file list.
ColumnLayout {
    id: detailsPane

    required property var details
    // Reflog selector when the selected row is a stash ("" otherwise).
    property string stashRef: ""

    signal fileActivated(string path, string origPath)
    signal parentClicked(string oidHex)
    signal copyRequested(string text)
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)

    spacing: 0

    PaneHeader {
        text: qsTr("COMMIT")
    }
    // Stash actions when the selected row is a stash.
    Rectangle {
        visible: detailsPane.stashRef !== ""
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            NavIcon {
                kind: "stash"
                tint: Theme.textSecondary
                width: Theme.iconSm + 2
                height: Theme.iconSm + 2
            }
            Label {
                text: detailsPane.stashRef
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
                color: Theme.textSecondary
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            HoverToolButton {
                text: qsTr("Apply")
                font.pixelSize: Theme.fontSm
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("Apply this stash, keeping it")
                onClicked: detailsPane.applyStashRequested(detailsPane.stashRef)
            }
            HoverToolButton {
                text: qsTr("Pop")
                font.pixelSize: Theme.fontSm
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("Apply this stash and drop it")
                onClicked: detailsPane.popStashRequested(detailsPane.stashRef)
            }
        }
    }
    // Inset on all four sides — the message box carries its own frame,
    // and flush against the header band the two borders read as one
    // welded block. Same inset as the pane edges, so the card sits
    // square in its well.
    ColumnLayout {
        Layout.fillWidth: true
        Layout.margins: Theme.spaceSm
        spacing: Theme.spaceXs
        visible: detailsPane.details.shaHex !== ""

        // -- message first, like the commit editor: a prominent summary
        // box and a dimmer description box --
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: subjectArea.implicitHeight + Theme.spaceSm
            color: Theme.bgBase
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            TextArea {
                id: subjectArea
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                readOnly: true
                wrapMode: TextArea.Wrap
                text: detailsPane.details.messageSubject
                font.pixelSize: Theme.fontLg
                font.weight: Font.DemiBold
                color: Theme.textPrimary
                background: null
                padding: 0
            }
        }
        // Always shown, even empty, and two lines tall from the start —
        // the pair mirrors the commit editor's fields.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(Math.max(bodyArea.implicitHeight,
                                                      2 * Theme.fontMdLine)
                                             + Theme.spaceSm, 120)
            color: Theme.bgBase
            radius: Theme.radiusMd
            border.color: Theme.borderSubtle
            border.width: Theme.borderWidth
            ScrollView {
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                // ScrollView keeps its Flickable private -- reach it
                // once it exists.
                Component.onCompleted:
                    contentItem.boundsBehavior = Flickable.StopAtBounds
                TextArea {
                    id: bodyArea
                    readOnly: true
                    wrapMode: TextArea.Wrap
                    text: detailsPane.details.messageBody
                    font.pixelSize: Theme.fontMd
                    color: Theme.textSecondary
                    background: null
                    padding: 0
                }
            }
        }
        // -- author card: avatar + name/date on the left, own hash over
        // parent hash on the right (rows aligned) --
        RowLayout {
            spacing: Theme.spaceSm
            IdentIcon {
                code: detailsPane.details.avatar
                width: Metrics.detailsAvatar
                height: Metrics.detailsAvatar
            }
            ColumnLayout {
                spacing: 0
                Layout.fillWidth: true
                Label {
                    id: authorLabel
                    text: detailsPane.details.authorName
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontMd
                    font.weight: Font.DemiBold
                    ToolTip.visible: authorHover.containsMouse
                    ToolTip.delay: 400
                    ToolTip.text: qsTr("Author: %1 <%2>\nCommitter: %3")
                                  .arg(detailsPane.details.authorName)
                                  .arg(detailsPane.details.authorEmail)
                                  .arg(detailsPane.details.committer)
                    MouseArea {
                        id: authorHover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                    }
                }
                Label {
                    id: detailsDate
                    text: Qt.formatDateTime(new Date(detailsPane.details.authorTime * 1000),
                                            "yyyy-MM-dd HH:mm")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
            }
            ColumnLayout {
                spacing: 0
                Layout.alignment: Qt.AlignRight
                // The hash is the button, not just the icon beside it —
                // a 16px glyph was too small to aim at. Hovering
                // underlines the hash and lights the icon so the whole
                // plate reads as one control.
                // Not a HoverToolButton: the style's panel would make
                // the plate taller than one line and drop this hash out
                // of step with the author name beside it, so it draws
                // the same wash over its own flat face.
                ToolButton {
                    id: hashCopy
                    Layout.alignment: Qt.AlignRight
                    text: detailsPane.details.sha8
                    leftPadding: Theme.spaceXs
                    rightPadding: Theme.spaceXs
                    topPadding: 0
                    bottomPadding: 0
                    readonly property bool lit: hovered || visualFocus
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: qsTr("Copy full hash")
                    onClicked: detailsPane.copyRequested(detailsPane.details.shaHex)
                    background: Rectangle {
                        radius: Theme.radiusSm
                        color: hashCopy.down ? Theme.bgPressed
                             : hashCopy.lit ? Theme.bgHover
                             : "transparent"
                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.NoButton
                            cursorShape: Qt.PointingHandCursor
                        }
                        // Drawn here rather than as the label's font
                        // underline so the rule runs under the icon too —
                        // the hash and the icon are one target, so they
                        // get one line.
                        Rectangle {
                            visible: hashCopy.lit
                            color: Theme.textPrimary
                            height: Theme.borderWidth
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.bottom: parent.bottom
                            anchors.leftMargin: hashCopy.leftPadding
                            anchors.rightMargin: hashCopy.rightPadding
                        }
                    }
                    contentItem: RowLayout {
                        spacing: Theme.spaceXs
                        Label {
                            text: hashCopy.text
                            font.family: Theme.monoFamily
                            font.pixelSize: Theme.fontMd
                            color: Theme.textPrimary
                            Layout.alignment: Qt.AlignVCenter
                        }
                        NavIcon {
                            kind: "copyicon"
                            tint: hashCopy.lit ? Theme.textPrimary
                                               : Theme.textSecondary
                            Layout.alignment: Qt.AlignVCenter
                        }
                    }
                }
                Label {
                    id: parentLink
                    visible: detailsPane.details.parentHex !== ""
                    Layout.alignment: Qt.AlignRight
                    text: "← " + detailsPane.details.parentHex.substring(0, 8)
                    font.family: Theme.monoFamily
                    color: Theme.textLink
                    font.pixelSize: Theme.fontSm
                    font.underline: parentHover.containsMouse
                    ToolTip.visible: parentHover.containsMouse
                    ToolTip.delay: 600
                    ToolTip.text: qsTr("Go to parent commit")
                    MouseArea {
                        id: parentHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: detailsPane.parentClicked(detailsPane.details.parentHex)
                    }
                }
            }
        }
    }
    // CHANGES header with the tree ⇄ path view toggle.
    Rectangle {
        visible: detailsPane.details.shaHex !== ""
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: qsTr("CHANGES (%1)").arg(detailsPane.details.fileTotal)
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                color: Theme.textSecondary
            }
            Item { Layout.fillWidth: true }
            HoverToolButton {
                padding: 0
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("Tree view")
                onClicked: detailsPane.details.setTreeView(true)
                contentItem: NavIcon {
                    kind: "hier"
                    tint: detailsPane.details.treeView ? Theme.accent
                                                       : Theme.textMuted
                }
            }
            HoverToolButton {
                padding: 0
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("Paths view")
                onClicked: detailsPane.details.setTreeView(false)
                contentItem: NavIcon {
                    kind: "list"
                    tint: detailsPane.details.treeView ? Theme.textMuted
                                                       : Theme.accent
                }
            }
        }
    }
    ListView {
        id: fileList
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        model: detailsPane.details
        reuseItems: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AutoScrollBar {}
        delegate: FileRowDelegate {
            listWidth: fileList.width
            onActivated: (bucket, path, origPath) =>
                detailsPane.fileActivated(path, origPath)
            onFolderToggled: key => detailsPane.details.toggleFolder(key)
        }
    }
}

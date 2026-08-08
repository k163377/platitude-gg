pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The git commands this tab ran, oldest first. Hidden until asked for
// (the toolbar's `>_`), and raised on its own when something the user
// asked for fails — until the error surfaces are built, this is where a
// failure is read in full.
//
// Nothing is written to disk and nothing survives the tab: the model
// keeps the last few hundred rows and drops the rest.
Rectangle {
    id: pane

    required property var commandsModel
    /// A failure that never became a command row (a background read that
    /// gave up). Shown in the header, cleared by clicking it.
    property string errorText: ""

    signal closeRequested()
    signal errorCleared()
    signal copyRequested(string text)

    color: Theme.bgBase

    /// Puts the newest row back in view — what the panel opens on.
    function showLatest() {
        list.follow = true
        list.positionViewAtEnd()
    }

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
                anchors.rightMargin: Theme.spaceMd
                spacing: Theme.spaceSm

                Label {
                    text: qsTr("GIT COMMANDS")
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                    color: Theme.textSecondary
                }
                Label {
                    text: "(" + list.count + ")"
                    font.pixelSize: Theme.fontSm
                    color: Theme.textMuted
                }
                // Why the panel is up, in git's own words. One line here;
                // the row that failed keeps the whole of it.
                Label {
                    Layout.fillWidth: true
                    visible: pane.errorText !== ""
                    text: "— " + pane.errorText
                    elide: Text.ElideRight
                    font.pixelSize: Theme.fontSm
                    color: Theme.danger
                    MouseArea {
                        anchors.fill: parent
                        onClicked: pane.errorCleared()
                    }
                }
                Item {
                    Layout.fillWidth: true
                    visible: pane.errorText === ""
                }

                CheckBox {
                    text: qsTr("Background reads")
                    font.pixelSize: Theme.fontSm
                    implicitHeight: Theme.iconLg
                    checked: pane.commandsModel.backgroundReads
                    // The reads a repository page makes on a timer are
                    // nobody's doing and would bury the rest, so they are
                    // off until asked for — and only from here on.
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: qsTr("Also record the reads this window "
                                       + "makes on its own, from now on")
                    onToggled: pane.commandsModel.setBackgroundReads(checked)
                }
                HoverToolButton {
                    text: qsTr("Clear")
                    font.pixelSize: Theme.fontSm
                    onClicked: pane.commandsModel.clear()
                }
                HoverToolButton {
                    padding: 0
                    implicitWidth: Theme.iconLg
                    implicitHeight: Theme.iconLg
                    contentItem: Item {
                        NavIcon {
                            anchors.centerIn: parent
                            width: Theme.iconMd
                            height: Theme.iconMd
                            kind: "close"
                            tint: Theme.textPrimary
                        }
                    }
                    onClicked: pane.closeRequested()
                }
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: pane.commandsModel
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            topMargin: Theme.spaceXs
            bottomMargin: Theme.spaceXs
            ScrollBar.vertical: AutoScrollBar {}

            /// Whether new rows pull the view along. Reading further up
            /// stops that until the end is reached again — the same rule
            /// the graph follows about not moving the ground under a
            /// reader.
            property bool follow: true
            onMovementEnded: list.follow = list.atYEnd
            onCountChanged: if (list.follow) list.positionViewAtEnd()

            delegate: CommandRowDelegate {
                width: list.width
                onCopyRequested: text => pane.copyRequested(text)
            }

            Label {
                // Children of a ListView are adopted by its contentItem,
                // which is 0×0 while the list is empty — pin the label to
                // the view itself so "empty" is said in its middle.
                parent: list
                anchors.centerIn: parent
                visible: list.count === 0
                text: qsTr("Nothing yet — the commands this window runs "
                           + "turn up here")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
            }
        }
    }
}

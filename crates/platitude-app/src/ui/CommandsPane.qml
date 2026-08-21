pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The git commands this tab ran, oldest first. Hidden until asked for (the toolbar's `>_`), and raised on its own when
// something the user asked for fails — until the error surfaces are built, this is where a failure is read in full.
//
// Nothing is written to disk and nothing survives the tab: the model keeps the last few hundred rows and drops the
// rest.
Rectangle {
    id: pane

    required property var commandsModel
    /// A failure that never became a command row (a background read that gave up). Shown in the header, cleared by
    /// clicking it — and by `Clear`, which takes everything the panel says at once.
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

    /// What `Clear` empties: the rows and the line in the header both. The toolbar's mark is red for either of them, so
    /// a Clear that left the line standing left the mark red over an empty panel, with nothing on screen left to
    /// explain it (2026-08-10 報告).
    ///
    /// And then the panel goes down with them (2026-08-21 ユーザー報告). A panel raised by a failure is read once; the
    /// press that says "I am done with this" is the same press that empties it, and what stays behind otherwise is a
    /// panel saying `Nothing yet` over the graph it pushed out of the way. Reopening is the toolbar's `>_`, where it
    /// always was.
    function clearPanel() {
        pane.commandsModel.clear()
        pane.errorCleared()
        pane.closeRequested()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.headerHeight
            color: Theme.bgElevated
            // The hairline every pane header closes with (see PaneHeader).
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: Theme.borderWidth
                color: Theme.borderSubtle
                z: 1
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceSm
                anchors.rightMargin: Theme.spaceMd
                spacing: Theme.spaceSm

                Label {
                    text: qsTr("GIT COMMANDS")
                    font.pixelSize: Theme.fontMd
                    font.weight: Font.DemiBold
                    color: Theme.textSecondary
                }
                Label {
                    text: "(" + list.count + ")"
                    font.pixelSize: Theme.fontMd
                    color: Theme.textMuted
                }
                // Why the panel is up, in git's own words. One line here; the row that failed keeps the whole of it.
                //
                // git's messages run to several lines, and eliding does not make text one line — it trims the last one.
                // Without a cap the band grows to fit them and the message is painted over the rows below it.
                Label {
                    Layout.fillWidth: true
                    visible: pane.errorText !== ""
                    text: "— " + pane.errorText
                    elide: Text.ElideRight
                    maximumLineCount: 1
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
                    font.pixelSize: Theme.fontMd
                    implicitHeight: Theme.iconLg
                    checked: pane.commandsModel.backgroundReads
                    // The reads a repository page makes on a timer are nobody's doing and would bury the rest, so they
                    // are off until asked for — and only from here on.
                    ToolTip.visible: hovered
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: qsTr("Also record the reads this window makes on its own, from now on")
                    onToggled: pane.commandsModel.setBackgroundReads(checked)
                }
                HoverToolButton {
                    text: qsTr("Clear")
                    font.pixelSize: Theme.fontMd
                    onClicked: pane.clearPanel()
                }
                HoverToolButton {
                    padding: 0
                    implicitWidth: Theme.iconLg
                    implicitHeight: Theme.iconLg
                    contentItem: Item {
                        // A step under what it closes (デザイン規約 §寸法).
                        NavIcon {
                            anchors.centerIn: parent
                            width: Theme.iconSm
                            height: Theme.iconSm
                            kind: "close"
                            tint: Theme.textSecondary
                        }
                    }
                    onClicked: pane.closeRequested()
                }
            }
        }

        AppListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: pane.commandsModel
            topMargin: Theme.spaceXs
            bottomMargin: Theme.spaceXs

            /// Whether new rows pull the view along. Reading further up stops that until the end is reached again — the
            /// same rule the graph follows about not moving the ground under a reader.
            property bool follow: true
            onMovementEnded: list.follow = list.atYEnd
            onCountChanged: if (list.follow) list.positionViewAtEnd()

            delegate: CommandRowDelegate {
                width: list.width
                onCopyRequested: text => pane.copyRequested(text)
            }

            Label {
                // Children of a ListView are adopted by its contentItem, which is 0×0 while the list is empty — pin the
                // label to the view itself so "empty" is said in its middle.
                parent: list
                anchors.centerIn: parent
                visible: list.count === 0
                text: qsTr("Nothing yet — the commands this window runs turn up here")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
            }
        }
    }
}

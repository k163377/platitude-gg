pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The git commands this tab ran, oldest first. Hidden until asked for, and raised on its own when something the user
// asked for fails — until the error surfaces are built, this is where a failure is read in full.
//
// **Its band is the left menu's last row, carried on across the window** (デザイン規約 §git が言ったことを読む場所): the
// same mark at the same step in the same column, the same name after it, and the panel's own controls filling the run
// the pane's foot had nothing in. Pressing that mark is what took the row down here, and pressing it again puts it
// back at the foot of the pane.
//
// Nothing is written to disk and nothing survives the tab: the model keeps the last few hundred rows and drops the
// rest.
Rectangle {
    id: pane

    required property var commandsModel
    /// The page this panel belongs to: the seat in its band reads the log and the error line off it, the same as the
    /// seat at the foot of the pane does while the panel is down.
    required property var curPage
    /// A failure that never became a command row (a background read that gave up). Shown in the header, cleared by
    /// clicking it — and by `Clear`, which takes everything the panel says at once.
    property string errorText: ""

    signal closeRequested()
    signal errorCleared()
    signal copyRequested(string text)

    color: Theme.bgBase

    /// Automation: the colour the `>_` in this band came out to (`PG_AUTO_ACT=commands-clear`).
    readonly property alias markColor: seat.markColor

    // The panel opens on its newest row, wherever it was raised from — the row that has just been added is the one
    // somebody came here to read. The page raises it; where it is looking when it comes up is this panel's own answer.
    onVisibleChanged: if (pane.visible) pane.showLatest()
    // …and a failure that arrives with the panel already up puts it back at the end as well, over a reader who had
    // scrolled away from it. The rows follow on their own while nobody has (`list.follow`), so this is only about the
    // reader who has: a failure is the one row worth taking them back to, which is the same call the page makes when
    // it raises the panel for one.
    Connections {
        target: pane.commandsModel
        function onFailure() {
            if (pane.visible)
                pane.showLatest()
        }
    }

    /// Puts the newest row back in view — what the panel opens on.
    function showLatest() {
        list.follow = true
        list.positionViewAtEnd()
    }

    /// What `Clear` empties: the rows and the line in the header both. The mark is red for either of them, so a Clear
    /// that left the line standing left the mark red over an empty panel, with nothing on screen left to explain it
    /// (2026-08-10 報告).
    ///
    /// And then the panel goes down with them (2026-08-21 ユーザー報告). A panel raised by a failure is read once; the
    /// press that says "I am done with this" is the same press that empties it, and what stays behind otherwise is a
    /// panel saying `Nothing yet` over the graph it pushed out of the way. Reopening is the `>_`, which goes back to
    /// the foot of the left menu with the panel.
    function clearPanel() {
        pane.commandsModel.clear()
        pane.errorCleared()
        pane.closeRequested()
    }

    /// What `Copy` takes: every row the panel is holding, oldest first, as the model writes them out
    /// (`CommandsModel.copyText`). The band's tools act on the whole panel — the row under the pointer is the row
    /// menu's business (`Copy command` / `Copy output`).
    ///
    /// The header's own line stays out of it: that place is for a failure that never became a row, and a write that
    /// failed already has one down here (P3-確認事項「エラー表示が仮置き」).
    function copyLog() {
        pane.copyRequested(pane.commandsModel.copyText())
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.headerHeight
            color: Theme.bgElevated
            BandRule { z: 1 }
            // No inset of its own at the near end — the seat carries the pane's, so the mark and the name stand in the
            // columns they stood in at the foot of the list (2026-08-23 ユーザー指示).
            RowLayout {
                anchors.fill: parent
                anchors.rightMargin: Theme.spaceMd
                // The sections' own step, now that this band opens with one of their rows: at `spaceSm` the count sat
                // twice as far from the name here as `(2)` does from `BRANCHES` (2026-08-23 ユーザー指示).
                spacing: Theme.spaceXs

                // The row this band is: the mark, and the name it has been carrying all along.
                CommandsToggle {
                    id: seat
                    Layout.fillHeight: true
                    captioned: true
                    ruled: false
                    curPage: pane.curPage
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

                AppCheckBox {
                    text: qsTr("Background reads")
                    // The panel's own band is shorter than a form row, so this box keeps the band's height rather
                    // than the control height `AppCheckBox` opens at.
                    implicitHeight: Theme.iconLg
                    checked: pane.commandsModel.backgroundReads
                    // The reads a repository page makes on a timer are nobody's doing and would bury the rest, so they
                    // are off until asked for — and only from here on.
                    ToolTip.visible: hovered
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: qsTr("Also record the reads this window makes on its own, from now on")
                    onToggled: pane.commandsModel.setBackgroundReads(checked)
                }
                // No object in the word, the same as `Clear` beside it: the band's controls are the panel's, and what
                // they act on is the panel (デザイン規約 §diff の中身をコピーする — 行が自分で示しているものは文言で
                // 言い直さない). Off over an empty one, where the press would put an empty clipboard out.
                HoverToolButton {
                    text: qsTr("Copy")
                    font.pixelSize: Theme.fontMd
                    enabled: list.count > 0
                    tip: qsTr("Copy every command in this log")
                    onClicked: pane.copyLog()
                }
                HoverToolButton {
                    text: qsTr("Clear")
                    font.pixelSize: Theme.fontMd
                    onClicked: pane.clearPanel()
                }
                CloseToolButton {
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

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One row of the interactive-rebase plan, newest first: the verb it carries, whose commit it is, and the reorder under
// the hand. Verbs and moves go straight to the plan model through the view's functions; which commit the page should
// read rides up as a signal (app-ui.md コンポーネント配線規約). Nothing here runs git — the plan is a draft until its
// one button is pressed (デザイン規約 §履歴を合流させる).
Item {
    id: planRow

    required property int index
    required property string oid_hex
    required property string author
    required property string author_email
    required property int avatar
    required property string avatar_url
    required property string shown
    required property string action
    required property string subject
    required property string msg_subject
    required property string msg_body

    readonly property var list: planRow.ListView.view
    readonly property bool selected: planRow.list !== null && planRow.list.selectedOid === planRow.oid_hex
    readonly property bool folded: planRow.action === "squash" || planRow.action === "fixup"
    readonly property bool dropped: planRow.action === "drop"

    width: planRow.list !== null ? planRow.list.width : 0
    height: Theme.graphRowHeight

    Rectangle {
        anchors.fill: parent
        color: planRow.selected ? Theme.bgSelected : "transparent"
    }
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: rowArea.containsMouse && !planRow.selected
    }

    // Declared before the content so the verb chip's own press area sits on top of it.
    MouseArea {
        id: rowArea
        property real pressedY: 0
        property bool dragging: false
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: mouse => {
            if (mouse.button === Qt.LeftButton) {
                rowArea.pressedY = mouse.y
                rowArea.dragging = false
            }
        }
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton)
                planRow.list.verbMenuRequested(planRow.index)
            else if (!rowArea.dragging)
                planRow.list.rowPicked(planRow.index, planRow.oid_hex)
        }
        // The reorder: past the drag threshold the row follows the pointer a row at a time. The model's move keeps
        // this very delegate alive under the hand (impl_move_notified), so the mapping below stays anchored to it.
        onPositionChanged: mouse => {
            if (!rowArea.pressed || planRow.list === null)
                return
            if (!rowArea.dragging && Math.abs(mouse.y - rowArea.pressedY) > Qt.styleHints.startDragDistance)
                rowArea.dragging = true
            if (!rowArea.dragging)
                return
            const yInList = planRow.mapToItem(planRow.list.contentItem, 0, mouse.y).y
            const target = Math.max(0, Math.min(planRow.list.count - 1, Math.floor(yInList / planRow.height)))
            if (target !== planRow.index)
                planRow.list.moveRequested(planRow.index, target)
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceMd
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceSm

        // The verb, in the todo's own spelling (デザイン規約 §git 用語のコード表記) — a press opens the verb menu.
        Item {
            Layout.preferredWidth: planRow.list !== null ? planRow.list.verbColW : 0
            Layout.fillHeight: true
            CodeChip {
                word: planRow.action
                size: Theme.fontChip
                tint: planRow.action === "drop" ? Theme.danger
                      : planRow.action === "edit" ? Theme.warning
                      : planRow.action === "pick" ? Theme.textSecondary
                      : Theme.accentHover
                anchors.verticalCenter: parent.verticalCenter
            }
            MouseArea {
                anchors.fill: parent
                onClicked: planRow.list.verbMenuRequested(planRow.index)
            }
        }

        IdentIcon {
            Layout.preferredWidth: Theme.iconMd
            Layout.preferredHeight: Theme.iconMd
            Layout.alignment: Qt.AlignVCenter
            code: planRow.avatar
            imageUrl: planRow.avatar_url
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            CutName {
                id: subjectCut
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width
                cutAt: "end"
                text: planRow.shown
                pixelSize: Theme.fontMd
                color: planRow.dropped ? Theme.textMuted : Theme.textPrimary
            }
            // A dropped row stays in its place with a line through it, so the mind can be changed (Fork draws the
            // same). CutName carries no strikeout of its own, so the line is drawn over its ink.
            Rectangle {
                visible: planRow.dropped
                anchors.verticalCenter: parent.verticalCenter
                x: 0
                width: subjectCut.inkWidth
                height: Theme.borderWidth
                color: Theme.textMuted
            }
        }

        Label {
            Layout.alignment: Qt.AlignVCenter
            text: planRow.oid_hex.substring(0, 8)
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontCode
            color: Theme.textMuted
        }
    }
}

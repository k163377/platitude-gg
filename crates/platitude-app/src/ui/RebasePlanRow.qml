import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One row of the interactive-rebase plan: its verb, its commit, and the reorder under the hand. Verbs and moves go to
// the plan model through the view's functions; the picked commit rides up to the page (rules/app-ui.md「配線」).
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
        // The reorder is this row's drag, so it keeps it (rules-refs/app-ui.md「行は渡されたドラッグを手放さない」).
        preventStealing: true
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
        // Both ends of the drag reach the model: the fold rule is applied at the release (`RebasePlanModel::end_move`).
        // `dragging` stays set until the next press, so the click after a release knows it was a drag.
        onReleased: mouse => {
            if (mouse.button === Qt.LeftButton && rowArea.dragging && planRow.list !== null)
                planRow.list.moveEnded()
        }
        // The grab was taken away (a popup, a window losing it): no release is coming, so the drag ends here.
        onCanceled: {
            if (rowArea.dragging && planRow.list !== null)
                planRow.list.moveEnded()
        }
        // Past the drag threshold the row follows the pointer a row at a time. The model's move keeps this delegate
        // alive under the hand (`impl_move_notified`), so the mapping below stays anchored to it.
        onPositionChanged: mouse => {
            if (!rowArea.pressed || planRow.list === null)
                return
            if (!rowArea.dragging && Math.abs(mouse.y - rowArea.pressedY) > Qt.styleHints.startDragDistance) {
                rowArea.dragging = true
                planRow.list.moveBegan()
            }
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
            // A dropped row stays, struck through; `CutName` has no strikeout, so the line is drawn over its ink.
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

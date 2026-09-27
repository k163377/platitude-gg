pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Ctrl+F over the graph: a card that hangs from the top-right corner of the list, over it (デザイン規約 §コミットを探す).
Rectangle {
    id: findBar

    property bool open: false
    /// Writable so a headless run can fill the box — keystrokes cannot be injected.
    property alias query: field.text
    /// A line is standing — the owner says what counts as one. Only then is there anything to count, `0 / 0` included.
    property bool searching: false
    /// How many rows the query matched, and which of them the view is on.
    property int matches: 0
    property int atMatch: 0
    /// Rows loaded: the widest the count can get, and the width its place is held at so the caret does not slide as
    /// the matches narrow (§コミットを探す).
    property int loaded: 0
    /// Something was typed that nothing answers. No state colour — nothing spills over and nothing waits for a hand
    /// (規約 §状態); the count says `0 / 0` and the reason waits in the tooltip.
    property bool unanswered: false
    property string unansweredTip: ""
    /// How wide the card may grow before it covers a subject's first character; the owner measures it.
    property real maxWidth: 0

    signal dismissed()
    /// Enter and Shift+Enter: on to the next match, back to the previous.
    signal nextRequested()
    signal previousRequested()

    /// Brings the card up with the caret in it; on a card already up, takes the focus back and selects the query.
    function raise() {
        findBar.open = true
        field.forceActiveFocus()
        field.selectAll()
    }
    function dismiss() {
        if (!findBar.open)
            return
        findBar.open = false
        findBar.dismissed()
    }
    /// Closes without `dismissed`: the press that landed elsewhere owns the keyboard, and the answer to `dismissed`
    /// would take the caret from the box that press landed in.
    function dropAway() {
        findBar.open = false
    }
    /// Whether a press at `scenePos` (scene coordinates) landed on this card, `✕` and count included. `null` — a
    /// headless press with no place — is never on it.
    function holds(scenePos) {
        if (!scenePos)
            return false
        const p = findBar.mapFromItem(null, scenePos)
        return p.x >= 0 && p.y >= 0 && p.x < findBar.width && p.y < findBar.height
    }

    // Hangs from the edge above, so only the lower corners are rounded.
    bottomLeftRadius: Theme.radiusMd
    bottomRightRadius: Theme.radiusMd
    color: Theme.bgElevated
    border.width: Theme.borderWidth
    border.color: Theme.borderDefault

    // Grows with the query up to `maxWidth`, and the minimum wins over the cap. Built from what the parts *want*:
    // reading the card's own width back through a child's `Layout.maximumWidth` is a loop Qt settles at the minimum.
    implicitWidth: Math.max(findBar.minWidth, Math.min(findRow.wantedWidth + 2 * Theme.spaceSm, findBar.maxWidth))
    implicitHeight: Theme.headerHeight
    readonly property real minWidth: findRow.minWidth + 2 * Theme.spaceSm

    // Fades whole: a height animating while the contents keep their size looks broken halfway through.
    visible: opacity > 0
    opacity: findBar.open ? 1 : 0
    Behavior on opacity {
        NumberAnimation { duration: 120 }
    }

    RowLayout {
        id: findRow
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceSm
        readonly property real minWidth: findRow.fieldMinWidth + findRow.fixedWidth
        readonly property real wantedWidth:
            Math.max(findRow.fieldMinWidth, field.contentWidth + field.leftPadding + field.rightPadding)
            + findRow.fixedWidth
        // Everything right of the box. The count's place is held with nothing typed and at no match alike, so crossing
        // either edge does not move the box's left edge — the card hangs from the right.
        readonly property real fixedWidth:
            widest.implicitWidth + findRow.spacing + closeButton.implicitWidth + findRow.spacing

        // A search box's fixed width (デザイン規約 §レイアウト初期値), here its floor.
        readonly property real fieldMinWidth: 160

        SlimField {
            id: field
            Layout.fillWidth: true
            Layout.minimumWidth: findRow.fieldMinWidth
            placeholderText: qsTr("Find commits")
            ToolTip.visible: findBar.unanswered && hovered
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: findBar.unansweredTip
            Keys.onEscapePressed: findBar.dismiss()
            Keys.onPressed: event => {
                if (event.key !== Qt.Key_Return && event.key !== Qt.Key_Enter)
                    return
                if (event.modifiers & Qt.ShiftModifier)
                    findBar.previousRequested()
                else
                    findBar.nextRequested()
                event.accepted = true
            }
        }
        // Emptied by its text, never hidden: a Layout skips an invisible item and its spacing with it.
        Label {
            id: count
            text: findBar.searching ? qsTr("%1 / %2").arg(findBar.atMatch).arg(findBar.matches) : ""
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            // Held at its widest (`loaded`) and right-aligned, so the numbers grow away from the box.
            Layout.preferredWidth: widest.implicitWidth
            horizontalAlignment: Text.AlignRight
            Label {
                id: widest
                visible: false
                font: count.font
                text: qsTr("%1 / %2").arg(findBar.loaded).arg(findBar.loaded)
            }
        }
        CloseToolButton {
            id: closeButton
            Layout.alignment: Qt.AlignVCenter
            Accessible.name: qsTr("Close the find bar")
            onClicked: findBar.dismiss()
        }
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One row of the stopped operation's card: `AppMenuItem`'s row, standing
// in a pane instead of dropping out of a menu.
//
// The vocabulary has to be the menu's exactly — the chip in git's own
// spelling, the sentence saying what it costs, the mark ahead of a row
// that is held (デザイン規約 §git 用語のコード表記, §長押し). A person
// who has learnt the reset submenu reads this card without being taught
// it twice. What it cannot borrow is `MenuItem` itself, which only lays
// out inside a Menu.
Item {
    id: opRow

    /// The git flag, in git's own spelling.
    property string code: ""
    /// What the flag does, in words.
    property string text: ""
    /// A short tag said after the words, the way a menu row says one
    /// (`AppMenuItem.note`). The row still runs — this is what it costs,
    /// or in the one case here what it does not.
    property string note: ""
    /// Held rather than clicked, for a row that takes something away.
    /// Zero is an ordinary row.
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    property real holdProgress: 0
    property color holdTone: Theme.danger
    property bool enabled: true
    /// The width this row's chip asks the card's shared column to hold,
    /// so every sentence starts on the same x.
    readonly property real codeColSeat: codeLabel.implicitWidth
    /// The column the card settled on, set by the card.
    property real codeColW: 0
    /// How far the words are pushed in to leave room for the mark — the
    /// same on every row, held or not, so the card reads down one column
    /// of first letters.
    property real holdIndent: 0

    /// Run, whichever gesture this row takes.
    signal picked()

    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        if (opRow.holdMs > 0) {
            backAnim.stop()
            holdAnim.restart()
        }
    }
    readonly property bool holding: opRow.holdProgress > 0

    implicitHeight: opRow.visible ? Theme.rowHeight : 0
    Accessible.description: opRow.holdMs > 0 ? qsTr("Hold to activate") : ""

    // A row says its whole line when the pane has narrowed enough to cut
    // it, the way a menu row does — the elision is the pane running out
    // of width, not the row having less to say (`AppMenuItem`).
    ToolTip.visible: rowHover.containsMouse && rowLabel.truncated
    ToolTip.delay: 600
    ToolTip.text: opRow.code + " " + opRow.text

    // One colour for every word in the row, so the chip cannot disagree
    // with the sentence it sits in. A held row wears its cost before it
    // is touched (デザイン規約 §状態).
    readonly property color wordColor: !opRow.enabled ? Theme.textMuted
                                     : opRow.holding ? Theme.textOnAccent
                                     : opRow.holdMs > 0 ? opRow.holdTone
                                     : Theme.textPrimary

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusSm
        color: rowHover.containsMouse && opRow.enabled && !opRow.holding
               ? Theme.bgHover : "transparent"
        // The hold filling the row from the left, the report every held
        // control in this app gives (デザイン規約 §長押し).
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: opRow.holding
                   ? Math.max(Metrics.holdFillMin, parent.width * opRow.holdProgress)
                   : 0
            radius: Theme.radiusSm
            color: opRow.holdTone
            visible: opRow.holding
        }
    }

    HoldIcon {
        anchors.left: parent.left
        anchors.leftMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        progress: opRow.holdProgress
        tint: opRow.wordColor
        visible: opRow.holdMs > 0
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs + opRow.holdIndent
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceSm
        Item {
            implicitWidth: Math.max(opRow.codeColW, codeLabel.implicitWidth)
            implicitHeight: codeLabel.implicitHeight
            Rectangle {
                anchors.fill: codeLabel
                // Half a gap of tint outside the glyphs, the way the menu
                // chips sit: a whole one would reach back to the mark and
                // the row would read as one run of ink.
                anchors.leftMargin: -Theme.spaceXs / 2
                anchors.rightMargin: -Theme.spaceXs / 2
                radius: Theme.radiusSm
                color: Theme.bgHover
            }
            Label {
                id: codeLabel
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: opRow.code
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontMd
                color: opRow.wordColor
            }
        }
        Label {
            id: rowLabel
            Layout.fillWidth: true
            text: opRow.text
            font.pixelSize: Theme.fontMd
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            color: opRow.wordColor
        }
        Label {
            visible: opRow.note !== ""
            text: opRow.note
            verticalAlignment: Text.AlignVCenter
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
    }

    MouseArea {
        id: rowHover
        anchors.fill: parent
        hoverEnabled: true
        enabled: opRow.enabled
        onPressed: {
            if (opRow.holdMs > 0) {
                backAnim.stop()
                holdAnim.restart()
            }
        }
        // Released anywhere, or dragged off the row: both call it off. A
        // plain row runs on the release that lands on it.
        onReleased: mouse => {
            if (opRow.holdMs > 0) {
                holdAnim.stop()
                return
            }
            if (mouse.x >= 0 && mouse.y >= 0
                    && mouse.x <= width && mouse.y <= height)
                opRow.picked()
        }
        onCanceled: holdAnim.stop()
        onPositionChanged: if (!containsMouse) holdAnim.stop()
    }
    // The same row from the keyboard, the one alternative a hold has
    // anywhere in this app: focus it, then hold Space or Enter.
    // Auto-repeat is dropped on both edges, or the fill restarts from
    // zero for as long as the key is down and can never complete.
    activeFocusOnTab: opRow.enabled
    Keys.onPressed: event => {
        if (!holdKey(event.key) || event.isAutoRepeat)
            return
        if (opRow.holdMs <= 0) {
            opRow.picked()
            event.accepted = true
            return
        }
        backAnim.stop()
        holdAnim.restart()
        event.accepted = true
    }
    Keys.onReleased: event => {
        if (opRow.holdMs <= 0 || event.isAutoRepeat || !holdKey(event.key))
            return
        holdAnim.stop()
        event.accepted = true
    }
    function holdKey(key) {
        return key === Qt.Key_Space || key === Qt.Key_Return
                || key === Qt.Key_Enter
    }

    NumberAnimation {
        id: holdAnim
        target: opRow
        property: "holdProgress"
        from: 0
        to: 1
        duration: Math.max(opRow.holdMs, 1)
        // A press that stopped short slides back out: a held row reports
        // no click, so without this the answer to a plain click on one is
        // nothing happening at all (デザイン規約 §長押し).
        onStopped: {
            if (opRow.holdProgress >= 1)
                opRow.holdProgress = 0
            else if (opRow.holdProgress > 0)
                backAnim.restart()
        }
        onFinished: opRow.picked()
    }
    NumberAnimation {
        id: backAnim
        target: opRow
        property: "holdProgress"
        to: 0
        duration: Metrics.holdBackMs
        easing.type: Easing.OutCubic
    }
}

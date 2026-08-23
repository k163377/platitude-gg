import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One row of the stopped operation's card: `AppMenuItem`'s row, standing in a pane instead of dropping out of a menu.
//
// The vocabulary has to be the menu's exactly — the chip in git's own spelling, the sentence saying what it costs, the
// mark ahead of a row that is held (デザイン規約 §git 用語のコード表記, §長押し). A person who has learnt the reset submenu reads this
// card without being taught it twice. What it cannot borrow is `MenuItem` itself, which only lays out inside a Menu.
Item {
    id: opRow

    /// The git flag, in git's own spelling.
    property string code: ""
    /// What the flag does, in words.
    property string text: ""
    /// A short tag said after the words, the way a menu row says one (`AppMenuItem.note`). The row still runs — this is
    /// what it costs, or in the one case here what it does not.
    property string note: ""
    /// Held rather than clicked, for a row that takes something away. Zero is an ordinary row.
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    property color holdTone: Theme.danger
    property bool enabled: true
    /// The width this row's chip asks the card's shared column to hold, so every sentence starts on the same x.
    readonly property real codeColSeat: codeLabel.implicitWidth
    /// The column the card settled on, set by the card.
    property real codeColW: 0
    /// How far the words are pushed in to leave room for the mark — the same on every row, held or not, so the card
    /// reads down one column of first letters.
    property real holdIndent: 0

    /// Run, whichever gesture this row takes.
    signal picked()

    /// Automation: run the row to its end without a press behind it — the hold where there is one, and the plain press
    /// where there is not. **`HoldDriver.begin()` is a no-op on a row with no hold** (holdMs 0 disarms it), so a run
    /// that only called it waited out the watchdog in silence on `--continue`, `--quit` and the free `--skip`.
    function completeHold() {
        if (opRow.holdMs <= 0) {
            opRow.picked()
            return
        }
        holdDrive.begin()
    }
    readonly property bool holding: opRow.holdProgress > 0

    implicitHeight: opRow.visible ? Theme.rowHeight : 0
    Accessible.description: opRow.holdMs > 0 ? Words.holdToActivate : ""

    // A row says its whole line when the pane has narrowed enough to cut it, the way a menu row does — the elision is
    // the pane running out of width, not the row having less to say (`AppMenuItem`).
    ToolTip.visible: rowHover.containsMouse && rowLabel.truncated
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: opRow.code + " " + opRow.text

    // One colour for every word in the row, so the chip cannot disagree with the sentence it sits in. A held row wears
    // its cost before it is touched (デザイン規約 §状態).
    readonly property color wordColor: !opRow.enabled ? Theme.textMuted
                                     : opRow.holding ? Theme.textOnAccent
                                     : opRow.holdMs > 0 ? opRow.holdTone
                                     : Theme.textPrimary

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusSm
        color: rowHover.containsMouse && opRow.enabled && !opRow.holding ? Theme.bgHover : "transparent"
        HoldFill {
            progress: opRow.holdProgress
            tone: opRow.holdTone
        }
    }

    // At the row's own left edge, the way a menu row's mark stands (`AppMenuItem`): the card's padding is to its left,
    // and it overhangs the seat the words leave for it on both sides.
    HoldIcon {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        progress: opRow.holdProgress
        tint: opRow.wordColor
        visible: opRow.holdMs > 0
    }

    // The menu row's own padding (`AppMenuItem` padding: spaceSm), so the card's rows and a menu's read as the same
    // row.
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm + opRow.holdIndent
        anchors.rightMargin: Theme.spaceSm
        spacing: Theme.spaceSm
        Item {
            implicitWidth: Math.max(opRow.codeColW, codeLabel.implicitWidth)
            implicitHeight: codeLabel.implicitHeight
            Rectangle {
                anchors.fill: codeLabel
                // Half a gap of tint outside the glyphs, the way the menu chips sit: a whole one would reach back to
                // the mark and the row would read as one run of ink.
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
        onPressed: holdDrive.begin()
        // Released anywhere, or dragged off the row: both call it off. A plain row runs on the release that lands on
        // it.
        onReleased: mouse => {
            if (opRow.holdMs > 0) {
                holdDrive.letUp()
                return
            }
            if (mouse.x >= 0 && mouse.y >= 0 && mouse.x <= width && mouse.y <= height)
                opRow.picked()
        }
        onCanceled: holdDrive.letUp()
        onPositionChanged: if (!containsMouse) holdDrive.letUp()
    }
    // The same row from the keyboard, the one alternative a hold has anywhere in this app: focus it, then hold Space or
    // Enter (`HoldDriver.pressKey`).
    activeFocusOnTab: opRow.enabled
    Keys.onPressed: event => {
        if (holdDrive.pressKey(event))
            return
        // A row with no hold on it runs on the same key, outright.
        if (holdDrive.ownsKey(event)) {
            opRow.picked()
            event.accepted = true
        }
    }
    Keys.onReleased: event => holdDrive.releaseKey(event)

    HoldDriver {
        id: holdDrive
        holdMs: opRow.holdMs
        onFinished: opRow.picked()
    }
}

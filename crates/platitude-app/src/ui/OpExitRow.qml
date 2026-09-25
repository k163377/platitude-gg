import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One row of the stopped operation's card: `AppMenuItem`'s row standing in a pane — the menu's chip, sentence and hold
// mark exactly, so the menu's reader reads it untaught. Not `MenuItem` itself, which only lays out inside a Menu.
Item {
    id: opRow

    /// The git flag, in git's own spelling.
    property string code: ""
    /// What the flag does, in words.
    property string text: ""
    /// A short tag after the words, as `AppMenuItem.note`: what running the row costs, or that it costs nothing.
    property string note: ""
    /// Held, for a row that takes something away. Zero is an ordinary row.
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// The length the press under way was given, else the live one (`HoldDriver.armedMs`). A tag worked out from the
    /// same answer reads this, so the wording holds under a hand already on the row.
    readonly property alias armedMs: holdDrive.armedMs
    property color holdTone: Theme.danger
    property bool enabled: true
    /// The width this row's chip asks the card's shared column to hold, so every sentence starts on the same x.
    readonly property real codeColSeat: codeLabel.implicitWidth
    /// The column the card settled on.
    property real codeColW: 0
    /// How far the words are pushed in to leave room for the mark — the same on every row, held or not.
    property real holdIndent: 0

    /// Run, whichever gesture this row takes.
    signal picked()

    /// Automation: run the row to its end — the hold where there is one, the plain press where not
    /// (`HoldDriver.begin()` does nothing at `holdMs` 0).
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

    // The whole line where the pane has cut it, as a menu row (`AppMenuItem`).
    ToolTip.visible: rowHover.containsMouse && rowLabel.truncated
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: opRow.code + " " + opRow.text

    // One colour for every word, so the chip cannot disagree with its sentence; a held row wears its cost before it is
    // touched (デザイン規約 §状態).
    readonly property color wordColor: !opRow.enabled ? Theme.textMuted
                                     : opRow.holding ? Theme.textOnAccent
                                     : opRow.armedMs > 0 ? opRow.holdTone
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

    // At the row's own left edge, overhanging the seat the words leave for it, as a menu row's mark (`AppMenuItem`).
    HoldIcon {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        progress: opRow.holdProgress
        tint: opRow.wordColor
        visible: opRow.armedMs > 0
    }

    // `AppMenuItem`'s padding, so the card's rows and a menu's read alike.
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
            // Against the length **this press** was given, and nothing where that has moved since
            // (`HoldDriver.armedMs` / `stale`): git answering mid-press must not change the gesture (デザイン規約 §長押し).
            const plain = holdDrive.armedMs <= 0 && !holdDrive.stale
                    && mouse.x >= 0 && mouse.y >= 0 && mouse.x <= width && mouse.y <= height
            holdDrive.letUp()
            if (plain)
                opRow.picked()
        }
        onCanceled: holdDrive.letUp()
        onPositionChanged: if (!containsMouse) holdDrive.letUp()
    }
    // Keyboard: focus it, then hold Space or Enter (`HoldDriver.pressKey`).
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
    // A key let go after the focus has moved is answered somewhere else (`HoldDriver.focusLost`).
    onActiveFocusChanged: if (!opRow.activeFocus) holdDrive.focusLost()

    HoldDriver {
        id: holdDrive
        holdMs: opRow.holdMs
        onFinished: opRow.picked()
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One row of the stopped operation's card: `AppMenuItem`'s row standing in a pane — the menu's chip, sentence and hold
// mark exactly, so the menu's reader reads it untaught. Not `MenuItem` itself, which only lays out inside a Menu.
// **The sentence wraps where a menu's is cut** (デザイン規約 §進行中の操作から出る): a menu is as wide as its widest
// row and says the rest on hover, while this card is only as wide as the pane — and what a row does is the one thing
// it is there to say. The chip, the mark and the note stand on the first line.
Item {
    id: opRow

    /// The git flag, in git's own spelling.
    property string code: ""
    /// What the flag does, in words.
    property string text: ""
    /// A short tag after the words, as `AppMenuItem.note`: what running the row costs, or that it costs nothing.
    property string note: ""
    /// Pressed this many times, for a row that takes something away. Zero is an ordinary row (デザイン規約 §回数押し).
    property int holdPresses: 0
    /// How far into the presses the row has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// The count the gesture under way was given, else the live one (`HoldDriver.armedPresses`). A tag worked out from
    /// the same answer reads this, so the wording holds between presses.
    readonly property alias armedPresses: holdDrive.armedPresses
    /// `warning` unless the row names `danger` itself — the card's held rows do: what they throw away is the work of
    /// the stopped operation, which the discard record does not keep (デザイン規約 §回数押し の色の表).
    property color holdTone: Theme.warning
    /// The width this row's chip asks the card's shared column to hold, so every sentence starts on the same x.
    readonly property real codeColSeat: codeLabel.implicitWidth
    /// The column the card settled on.
    property real codeColW: 0
    /// How far the words are pushed in to leave room for the mark — the same on every row, confirmed or not.
    property real holdIndent: 0
    /// How far the fill reaches past the row into the card's side padding (`HoldFill.bleed`).
    property real fillBleed: 0
    /// The row's work is out (the tab is busy): the full fill waits on it past the floor, with the ring in the mark's
    /// seat (デザイン規約 §回数押し).
    property bool waiting: false

    /// Run, whichever gesture this row takes.
    signal picked()

    /// Automation: run the row to its end — every press at once where it takes several, the plain press where not. A
    /// row turned off answers it as it answers a hand: not at all. Answers whether the presses went in.
    function completeHold() {
        if (!opRow.enabled)
            return false
        if (opRow.holdPresses <= 0)
            opRow.picked()
        else
            holdDrive.runOut()
        return true
    }
    readonly property bool holding: opRow.holdProgress > 0
    /// A gesture is under way, its completion included (`HoldDriver.gesturing`).
    readonly property alias gesturing: holdDrive.gesturing

    /// One line of the sentence; the row is a menu row's height for the first and grows by each one after it.
    readonly property real lineHeight: rowLabel.lineCount > 0 ? rowLabel.implicitHeight / rowLabel.lineCount
                                                              : rowLabel.implicitHeight
    /// The first line's band: the taller of the sentence's line and the chip, each piece of the first line centred in
    /// it. The chip's family and the sentence's differ by OS, so a margin worked out from one alone can go negative.
    readonly property real firstBand: Math.max(opRow.lineHeight, codeLabel.implicitHeight)
    implicitHeight: opRow.visible ? Theme.rowHeight + rowLabel.implicitHeight - opRow.lineHeight : 0
    Accessible.description: opRow.holdPresses > 0 ? Words.pressToActivate(opRow.holdPresses) : ""

    // One colour for every word, so the chip cannot disagree with its sentence; a confirmed row wears its cost before
    // it is touched (デザイン規約 §状態). Over the fill the words lift, the wait included: the row goes off while its
    // own work runs.
    readonly property color wordColor: opRow.holding && (opRow.enabled || holdDrive.done) ? Theme.textOnAccent
                                     : !opRow.enabled ? Theme.textMuted
                                     : opRow.armedPresses > 0 ? opRow.holdTone
                                     : Theme.textPrimary
    /// The row cannot be pressed: grey words, and the press mark keeps its tone a step down, as a menu row's does
    /// (`AppMenuItem.holdOff`, which says why a fill keeps it out).
    readonly property bool holdOff: !opRow.enabled && !opRow.holding
    readonly property color holdInk: opRow.holdOff ? opRow.holdTone : opRow.wordColor

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusSm
        color: rowHover.containsMouse && opRow.enabled && !opRow.holding ? Theme.bgHover : "transparent"
        HoldFill {
            progress: opRow.holdProgress
            tone: opRow.holdTone
            dimmed: holdDrive.dimmed
            bleed: opRow.fillBleed
        }
    }

    // At the row's own left edge, overhanging the seat the words leave for it, as a menu row's mark (`AppMenuItem`).
    // Centred on the first line's band.
    HoldIcon {
        id: holdMark
        anchors.left: parent.left
        y: (Theme.rowHeight - holdMark.height) / 2 + Metrics.opticalDrop
        progress: opRow.holdProgress
        presses: opRow.armedPresses
        tint: opRow.holdInk
        off: opRow.holdOff
        visible: opRow.armedPresses > 0 && !holdDrive.loading
    }
    // The ring the row's work turns in the mark's seat once the floor has passed (デザイン規約 §回数押し).
    SpinnerIcon {
        x: holdMark.x
        y: holdMark.y
        width: Theme.iconSm
        height: Theme.iconSm
        tint: opRow.wordColor
        spinning: holdDrive.loading
    }

    // `AppMenuItem`'s padding, so the card's rows and a menu's read alike. Hung from the top with the first line
    // centred in a menu row's band, so a one-line row stands where it always did.
    RowLayout {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: Theme.spaceXs + opRow.holdIndent
        anchors.rightMargin: Theme.spaceXs
        anchors.topMargin: (Theme.rowHeight - opRow.firstBand) / 2
        spacing: Theme.spaceSm
        Item {
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: (opRow.firstBand - codeLabel.implicitHeight) / 2
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
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: (opRow.firstBand - opRow.lineHeight) / 2
            text: opRow.text
            font.pixelSize: Theme.fontMd
            wrapMode: Text.Wrap
            color: opRow.wordColor
        }
        Label {
            id: noteLabel
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: (opRow.firstBand - noteLabel.implicitHeight) / 2
            visible: opRow.note !== ""
            text: opRow.note
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
    }

    MouseArea {
        id: rowHover
        anchors.fill: parent
        hoverEnabled: true
        onPressed: holdDrive.begin()
        // A plain row runs on the release that lands on it.
        onReleased: mouse => {
            // Against the count **this gesture** was given, and nothing where that has moved since
            // (`HoldDriver.armedPresses` / `stale`): git answering between presses must not change the gesture
            // (デザイン規約 §回数押し).
            const plain = holdDrive.armedPresses <= 0 && !holdDrive.stale
                    && mouse.x >= 0 && mouse.y >= 0 && mouse.x <= width && mouse.y <= height
            holdDrive.letUp()
            if (plain)
                opRow.picked()
        }
        onCanceled: holdDrive.letUp()
        // Dragged off the row, or the hand left it part way through its presses: back to zero.
        onPositionChanged: if (!containsMouse) holdDrive.leave()
        onExited: holdDrive.leave()
    }
    // Keyboard: focus it, then press Space or Enter as many times (`HoldDriver.pressKey`). Turned off — `enabled`, its
    // own or an ancestor's — the row drops the focus and the pointer's grab, so neither a key nor a release reaches
    // it, and the tab passes it by.
    activeFocusOnTab: true
    // Presses under way when it turns off are dropped (as `ActionButton.live`); the last press's fill stays for its
    // work (`HoldDriver.blank`).
    onEnabledChanged: if (!opRow.enabled) holdDrive.blank()
    Keys.onPressed: event => {
        if (holdDrive.pressKey(event))
            return
        // A row that takes one press runs on the same key, outright.
        if (holdDrive.ownsKey(event)) {
            opRow.picked()
            event.accepted = true
        }
    }
    Keys.onReleased: event => holdDrive.releaseKey(event)
    // Keys counted elsewhere are another gesture: the focus moved away drops them (`HoldDriver.focusLost`).
    onActiveFocusChanged: if (!opRow.activeFocus) holdDrive.focusLost()

    HoldDriver {
        id: holdDrive
        holdPresses: opRow.holdPresses
        waiting: opRow.waiting
        onFinished: opRow.picked()
    }
}

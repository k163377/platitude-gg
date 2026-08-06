import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One AppMenu row: the list row height and body size used everywhere
// else, and its own words as its width. A row elides only where the
// menu has run out of window to grow into; hovering an elided row says
// the whole line.
MenuItem {
    id: menuItem

    /// A short warning said after the row's words ("already pushed").
    /// The row still runs on click — this is the tag that says what it
    /// costs, the same shape the amend editor uses.
    property string note: ""

    /// A git term said in git's own spelling — lowercase, mono, on a
    /// faint chip — instead of dressed up as a sentence word (デザイン
    /// 規約 §git 用語のコード表記). Never translated: it is the command,
    /// not a phrase about it. It sits ahead of `text`, which carries
    /// whatever of the sentence is left ("this file"), often nothing.
    property string code: ""

    /// Held rather than clicked, for a row that would otherwise have to
    /// raise a question of its own (デザイン規約 §長押し). Zero is an
    /// ordinary row. A hold row reports no click at all — the press is
    /// taken before the button behind it can see it, which is also what
    /// keeps the menu from closing under the hold.
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    property real holdProgress: 0
    /// The colour the hold fills the row with.
    property color holdTone: Theme.danger
    /// Held all the way down.
    signal held()
    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        if (menuItem.holdMs > 0)
            rowHoldAnim.restart()
    }
    readonly property bool holding: menuItem.holdProgress > 0

    padding: Theme.spaceSm
    topPadding: 0
    bottomPadding: 0
    // A row this menu is not offering takes no room. The list lays its
    // rows out by height, so an invisible one that keeps a height leaves
    // an empty row behind — a hole where the reader looks for the row
    // that is missing (measured on the file menu, whose two destructive
    // rows are one per bucket).
    implicitHeight: menuItem.visible ? Theme.rowHeight : 0
    implicitWidth: (menuItem.code !== ""
                    ? codeChip.implicitWidth + Theme.spaceSm : 0)
                   + itemLabel.implicitWidth
                   + (menuItem.note !== ""
                      ? noteLabel.implicitWidth + Theme.spaceSm : 0)
                   + menuItem.leftPadding + menuItem.rightPadding
    font.pixelSize: Theme.fontMd

    // The one colour every word in the row follows, so the chip cannot
    // disagree with the sentence it sits in. A held row says what it
    // costs in its own colour before it is touched at all — it is the
    // one row in the menu that takes something away (デザイン規約
    // §状態); over the fill the words cross the tone itself and lift
    // clear of it.
    readonly property color wordColor: !menuItem.enabled ? Theme.textMuted
                                     : menuItem.holding ? Theme.textOnAccent
                                     : menuItem.holdMs > 0 ? menuItem.holdTone
                                     : menuItem.highlighted ? Theme.textOnAccent
                                                            : Theme.textPrimary

    ToolTip.visible: menuItem.hovered && itemLabel.truncated
    ToolTip.delay: 600
    ToolTip.text: menuItem.code !== ""
                  ? menuItem.code + " " + menuItem.text : menuItem.text

    contentItem: RowLayout {
        spacing: Theme.spaceSm
        Rectangle {
            id: codeChip
            visible: menuItem.code !== ""
            implicitWidth: codeLabel.implicitWidth + 2 * Theme.spaceXs
            implicitHeight: codeLabel.implicitHeight
            radius: Theme.radiusSm
            // A faint lift off whatever the row is showing underneath —
            // the menu card at rest, the accent under the pointer, the
            // hold tone mid-hold — the way inline code sits in prose.
            color: Theme.bgHover
            Label {
                id: codeLabel
                anchors.centerIn: parent
                text: menuItem.code
                font.family: Theme.monoFamily
                font.pixelSize: menuItem.font.pixelSize
                color: menuItem.wordColor
            }
        }
        Label {
            id: itemLabel
            Layout.fillWidth: true
            text: menuItem.text
            font: menuItem.font
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            // Clear of the arrow the style paints over the row's right
            // edge on a row that opens a submenu (the note, when there
            // is one, is what sits last instead).
            rightPadding: !noteLabel.visible && menuItem.subMenu && menuItem.arrow
                          ? menuItem.arrow.width + Theme.spaceXs : 0
            color: menuItem.wordColor
        }
        Label {
            id: noteLabel
            visible: menuItem.note !== ""
            text: menuItem.note
            verticalAlignment: Text.AlignVCenter
            rightPadding: menuItem.subMenu && menuItem.arrow
                          ? menuItem.arrow.width + Theme.spaceXs : 0
            color: Theme.warning
            font.pixelSize: Theme.fontSm
        }
    }

    background: Rectangle {
        radius: Theme.radiusSm
        // A held row hovers to a wash rather than to the solid accent:
        // its words are the tone, and the accent underneath them would
        // both fight the colour and take the warning away at the exact
        // moment the pointer is on the row (the same reason the pill
        // hovers to `bgHover`).
        color: !menuItem.highlighted ? "transparent"
             : menuItem.holdMs > 0 ? Theme.bgHover : Theme.accent
        // The hold filling the row from the left, the same report the
        // pill and the toolbar button give (デザイン規約 §長押し), and
        // never thinner than `holdFillMin` while it runs.
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: menuItem.holding
                   ? Math.max(Metrics.holdFillMin,
                              parent.width * menuItem.holdProgress)
                   : 0
            radius: Theme.radiusSm
            color: menuItem.holdTone
            visible: menuItem.holding
        }
    }

    NumberAnimation {
        id: rowHoldAnim
        target: menuItem
        property: "holdProgress"
        from: 0
        to: 1
        duration: Math.max(menuItem.holdMs, 1)
        // Letting go part way leaves nothing behind.
        onStopped: menuItem.holdProgress = 0
        onFinished: menuItem.held()
    }
    // Takes the press before the MenuItem underneath can: a click here
    // would emit `triggered`, which both runs the row and closes the menu
    // — and the menu has to stay open for as long as the hold lasts.
    // Hover is left alone (this one accepts none), so the row still
    // highlights the way every other row does.
    MouseArea {
        anchors.fill: parent
        enabled: menuItem.holdMs > 0 && menuItem.enabled
        onPressed: rowHoldAnim.restart()
        // Released anywhere, or dragged off the row: both call it off.
        onReleased: rowHoldAnim.stop()
        onCanceled: rowHoldAnim.stop()
        onPositionChanged: if (!containsMouse) rowHoldAnim.stop()
    }
    // The same row from the keyboard: walk to it and hold Space or Enter.
    // Accepting the key keeps the menu from triggering the row outright,
    // and auto-repeat is dropped on both edges (デザイン規約 §長押し).
    Keys.onPressed: event => {
        if (menuItem.holdMs <= 0 || event.isAutoRepeat || !holdKey(event.key))
            return
        rowHoldAnim.restart()
        event.accepted = true
    }
    Keys.onReleased: event => {
        if (menuItem.holdMs <= 0 || event.isAutoRepeat || !holdKey(event.key))
            return
        rowHoldAnim.stop()
        event.accepted = true
    }
    function holdKey(key) {
        return key === Qt.Key_Space || key === Qt.Key_Return
                || key === Qt.Key_Enter
    }
}

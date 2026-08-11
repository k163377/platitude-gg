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

    /// Whether this row is on offer for the thing the menu was opened on
    /// — false where it cannot be chosen, and then it is not drawn at all
    /// rather than greyed (デザイン規約 §メニュー).
    ///
    /// Kept apart from `visible`, which it drives, because `visible`
    /// cannot be *asked*: a menu's list releases the rows it is not
    /// showing and turns their visibility off behind the binding, so
    /// every row of a closed menu reads as invisible. The menu and its
    /// dividers have to know what is on offer before it opens.
    property bool offered: true
    visible: menuItem.offered

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
    /// The width this row asks the menu's shared chip column to hold:
    /// its chip's own glyphs, whether or not words follow them. A chip
    /// that ends its row asks too — the column has to clear the widest
    /// command, or it would end past where the other rows' words begin
    /// (デザイン規約 §git 用語のコード表記). Only a row with no chip
    /// asks for nothing.
    readonly property real codeColSeat:
        menuItem.code !== "" ? codeLabel.implicitWidth : 0

    /// Whether the row's words bid for the menu's width. Off for a row
    /// whose text is data rather than sentence — the name a delete row
    /// re-states: the menu is sized by its other rows, the name takes
    /// what is left and elides, and the elided row's hover already says
    /// the whole line (デザイン規約 §メニュー).
    property bool growsForText: true

    /// Held rather than clicked, for a row that would otherwise have to
    /// raise a question of its own (デザイン規約 §長押し). Zero is an
    /// ordinary row. A hold row reports no click at all — the press is
    /// taken before the button behind it can see it, which is also what
    /// keeps the menu from closing under the hold.
    ///
    /// The row says so with the mark ahead of its words, not in them: the
    /// menu leaves the same seat for it on every row, so a held row reads
    /// down the same column as the rest (`AppMenu.holdIndent`).
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// The colour the hold fills the row with.
    property color holdTone: Theme.danger
    /// Held all the way down.
    signal held()
    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        holdDrive.begin()
    }
    readonly property bool holding: menuItem.holdProgress > 0

    /// A row that runs on a click but leaves the menu standing, for the
    /// one thing here that git answers rather than this app: `branch -d`
    /// refuses while the branch holds commits nothing else does, and the
    /// refusal is the question worth asking (デザイン規約 §左メニューの所作).
    /// The menu has to outlive the click for that answer to have somewhere
    /// to land — the row turns into a held one where the hand already is,
    /// instead of a bar coming down over the graph.
    property bool staysOpen: false
    /// Clicked, on a row that stays open. `triggered` never fires for one:
    /// the press is taken before the button behind it can see it, which is
    /// what keeps the menu up.
    signal picked()

    /// How far this menu's rows are pushed in to leave room for the hold
    /// mark — the same on every row, held or not, so a menu still reads
    /// down one column of first letters (`AppMenu.holdIndent`).
    readonly property real holdIndent:
        menuItem.menu !== null && menuItem.menu.holdIndent !== undefined
        ? menuItem.menu.holdIndent : 0

    padding: Theme.spaceSm
    leftPadding: Theme.spaceSm + menuItem.holdIndent
    topPadding: 0
    bottomPadding: 0
    // A row this menu is not offering takes no room. The list lays its
    // rows out by height, so an invisible one that keeps a height leaves
    // an empty row behind — a hole where the reader looks for the row
    // that is missing (measured on the file menu, whose two destructive
    // rows are one per bucket).
    implicitHeight: menuItem.offered ? Theme.rowHeight : 0
    implicitWidth: (menuItem.code !== ""
                      ? codeChip.implicitWidth + Theme.spaceSm : 0)
                   + (menuItem.growsForText ? itemLabel.implicitWidth : 0)
                   + (menuItem.note !== ""
                      ? noteLabel.implicitWidth + Theme.spaceSm : 0)
                   + menuItem.leftPadding + menuItem.rightPadding
    font.pixelSize: Theme.fontMd
    // The words no longer say the gesture, so this is where it is left
    // for a reader who cannot see the mark.
    Accessible.description: menuItem.holdMs > 0 ? qsTr("Hold to activate") : ""

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
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: menuItem.code !== ""
                  ? menuItem.code + " " + menuItem.text : menuItem.text

    // The mark, inside the padding the whole menu carries for it rather
    // than in the row's layout: it stands against the card's own padding
    // with nothing but air to its left, and the words follow at the
    // distance `holdIndent` sets — not at that distance plus the layout's
    // gap (デザイン規約 §長押し). A row of the menu that is not held
    // leaves the same space empty, so the column of first letters holds.
    HoldIcon {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        progress: menuItem.holdProgress
        tint: menuItem.wordColor
        visible: menuItem.holdMs > 0
    }

    contentItem: RowLayout {
        spacing: Theme.spaceSm
        // The chip spends no width of its own beyond the column: the
        // layout sees the menu's shared chip column — never less than
        // its own glyphs — so every row's words start on the same x
        // (デザイン規約 §git 用語のコード表記). The word
        // itself starts where every other row starts its words, and the
        // tint hangs outside its glyphs — left into the row padding,
        // right into the gap — half tint, half air; the column's spare
        // width stays air.
        Item {
            id: codeChip
            visible: menuItem.code !== ""
            implicitWidth: {
                const own = codeLabel.implicitWidth
                if (!menuItem.menu)
                    return own
                const col = menuItem.menu.codeColW
                return col !== undefined ? Math.max(col, own) : own
            }
            implicitHeight: codeLabel.implicitHeight
            Rectangle {
                anchors.fill: codeLabel
                // Half a gap of tint outside the glyphs, not a whole one:
                // a chip that follows the hold mark would otherwise reach
                // back far enough to sit against it, and the row would
                // read as one run of ink with no air between the two
                // things it is saying.
                anchors.leftMargin: -Theme.spaceXs / 2
                anchors.rightMargin: -Theme.spaceXs / 2
                radius: Theme.radiusSm
                // A faint lift off whatever the row is showing under it —
                // the menu card at rest, the accent under the pointer,
                // the hold tone mid-hold — the way inline code sits in
                // prose.
                color: Theme.bgHover
            }
            Label {
                id: codeLabel
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: menuItem.code
                font.family: Theme.monoFamily
                // A command and its flag are one thing said (`push
                // --delete`), and a mono space is wider than the air the
                // chip keeps at its own ends — left alone the two drift
                // apart and the chip reads as two words on one ground.
                font.wordSpacing: -Theme.spaceXs
                font.pixelSize: menuItem.font.pixelSize
                color: menuItem.wordColor
            }
        }
        Label {
            id: itemLabel
            Layout.fillWidth: true
            text: menuItem.text
            font: menuItem.font
            // Pinned, not left to `AutoText`. Rows carry text nobody here
            // chose — branch names, paths, commit subjects — plus one
            // deliberate placeholder in angle brackets, and AutoText
            // decides by guessing whether a string looks like markup. A
            // branch called `<b>` should read as its name, not vanish.
            textFormat: Text.PlainText
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

    HoldDriver {
        id: holdDrive
        holdMs: menuItem.holdMs
        onFinished: menuItem.held()
    }
    // Takes the press before the MenuItem underneath can: a click here
    // would emit `triggered`, which both runs the row and closes the menu
    // — and the menu has to stay open for as long as the hold lasts.
    // Hover is left alone (this one accepts none), so the row still
    // highlights the way every other row does.
    MouseArea {
        id: rowPress
        anchors.fill: parent
        enabled: (menuItem.holdMs > 0 || menuItem.staysOpen) && menuItem.enabled
        onPressed: holdDrive.begin()
        // Released anywhere, or dragged off the row: both call it off.
        // A stays-open row has no fill to call off — letting go on the row
        // is its click, and letting go outside it is not.
        onReleased: mouse => {
            holdDrive.letUp()
            if (menuItem.holdMs <= 0 && menuItem.staysOpen
                    && mouse.x >= 0 && mouse.y >= 0
                    && mouse.x <= width && mouse.y <= height)
                menuItem.picked()
        }
        onCanceled: holdDrive.letUp()
        onPositionChanged: if (!containsMouse) holdDrive.letUp()
    }
    // The same row from the keyboard: walk to it and hold Space or Enter.
    // Accepting the key keeps the menu from triggering the row outright,
    // and auto-repeat is dropped on both edges (デザイン規約 §長押し).
    Keys.onPressed: event => {
        if (menuItem.holdMs <= 0 || event.isAutoRepeat
                || !holdDrive.holdKey(event.key))
            return
        holdDrive.begin()
        event.accepted = true
    }
    Keys.onReleased: event => {
        if (menuItem.holdMs <= 0 || event.isAutoRepeat
                || !holdDrive.holdKey(event.key))
            return
        holdDrive.letUp()
        event.accepted = true
    }
}

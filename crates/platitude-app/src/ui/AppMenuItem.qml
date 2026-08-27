import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One AppMenu row: the list row height and body size used everywhere else, and its own words as its width. A row elides
// only where the menu has run out of window to grow into; hovering an elided row says the whole line.
MenuItem {
    id: menuItem

    /// Whether this row is on offer for the thing the menu was opened on — false where it cannot be chosen, and then it
    /// is not drawn at all rather than greyed (デザイン規約 §メニュー).
    ///
    /// Kept apart from `visible`, which it drives, because `visible` cannot be *asked*: a menu's list releases the rows
    /// it is not showing and turns their visibility off behind the binding, so every row of a closed menu reads as
    /// invisible. The menu and its dividers have to know what is on offer before it opens.
    property bool offered: true
    visible: menuItem.offered

    /// A short warning said after the row's words ("already pushed"). The row still runs on click — this is the tag
    /// that says what it costs, the same shape the amend editor uses.
    property string note: ""

    /// A git term said in git's own spelling — lowercase, mono, on a faint chip — instead of dressed up as a sentence
    /// word (デザイン 規約 §git 用語のコード表記). Never translated: it is the command, not a phrase about it. It sits ahead of
    /// `text`, which carries whatever of the sentence is left ("this file"), often nothing.
    property string code: ""
    /// The mark a row wears instead of a chip, and its colour: the `NavIcon` kind of the thing the rows behind this one
    /// act on (デザイン規約 §メニュー の入れ子). Only the rows that open a submenu carry one — a row that runs a command says
    /// which by its chip, and a row that opens a card of them has no command to name.
    ///
    /// **A row with a mark is the sidebar's section band, in a menu**: the mark where the rows start their words, its
    /// own word a hair behind it, spelled the way a section spells its name. The word is out of the chip column
    /// entirely — this row names the card below it rather than taking a turn in the table.
    property string markKind: ""
    property color markTint: Theme.textSecondary
    readonly property bool heads: menuItem.markKind !== ""

    /// The width this row asks the menu's shared chip column to hold: its chip's own glyphs, whether or not words
    /// follow them. A chip that ends its row asks too — the column has to clear the widest command, or it would end
    /// past where the other rows' words begin (デザイン規約 §git 用語のコード表記). Only a row with no chip asks for nothing —
    /// and a row that names a card asks for nothing either, standing outside the column.
    readonly property real codeColSeat: menuItem.code !== "" ? codeLabel.implicitWidth : 0

    /// Whether the row's words bid their whole width from the menu. Off for a row whose text is data rather than
    /// sentence — the name a delete row re-states: it bids at most the seat a ref name gets in the graph (`labelColW`),
    /// elides past that into the hover that already says the whole line, and stretches further only into width the
    /// other rows have paid for (デザイン規約 §メニュー). A floor rather than nothing: a sparse menu at `menuMinW` with a wide
    /// frozen chip column otherwise leaves the name zero width.
    property bool growsForText: true

    /// Why this row cannot be chosen right now, in one line — and, by being non-empty, that it cannot. A blocked row is
    /// greyed the way a disabled one is (§無効) but stays hoverable, because the line is the whole point: a row of a
    /// fixed table that says nothing about why it is out is worse than no row at all (デザイン規約 §メニュー の削除の表). Presses and
    /// holds do nothing.
    property string blockedReason: ""
    readonly property bool blocked: menuItem.blockedReason !== ""
    /// Automation: show the tooltip with no pointer behind it. The same property the real hover drives, so a run that
    /// never reached the row photographs a row without one (verify-ui §hover の絵の撮り方).
    property bool tipForced: false

    /// Held rather than clicked, for a row that would otherwise have to raise a question of its own (デザイン規約 §長押し). Zero
    /// is an ordinary row. A hold row reports no click at all — the press is taken before the button behind it can see
    /// it, which is also what keeps the menu from closing under the hold.
    ///
    /// The row says so with the mark ahead of its words, not in them: the menu leaves the same seat for it on every
    /// row, so a held row reads down the same column as the rest (`AppMenu.holdIndent`).
    property int holdMs: 0
    /// Whether pressing this row raises a question instead of doing what it says — a `!` in the mark's seat, which is
    /// the row's one place for "read this before you press" (デザイン規約 §進行中の操作から出る). Never both this and a
    /// hold: the hold is how a row that acts is confirmed, and a row that asks is confirmed on the bar it raises.
    property bool asks: false
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

    /// A row that runs on a click but leaves the menu standing, for the one thing here that git answers rather than
    /// this app: `branch -d` refuses while the branch holds commits nothing else does, and the refusal is the question
    /// worth asking (デザイン規約 §左メニューの所作). The menu has to outlive the click for that answer to have somewhere to land —
    /// the row turns into a held one where the hand already is, instead of a bar coming down over the graph.
    property bool staysOpen: false
    /// Clicked, on a row that stays open. `triggered` never fires for one: the press is taken before the button behind
    /// it can see it, which is what keeps the menu up.
    signal picked()

    /// How far this menu's rows are pushed in to leave room for the hold mark — the same on every row, held or not, so
    /// a menu still reads down one column of first letters (`AppMenu.holdIndent`).
    readonly property real holdIndent:
        menuItem.menu !== null && menuItem.menu.holdIndent !== undefined ? menuItem.menu.holdIndent : 0

    padding: Theme.spaceSm

    /// Where a card's name starts its mark — **the mark's ink, not its box**. Shifted left by exactly the seat the
    /// menu leaves for the hold ring and the `!`: where there is one the mark stands in it, where there is none it
    /// starts on the same x the other rows start their words (2026-08-26 ユーザー指示).
    readonly property real markX: Theme.spaceSm - menuItem.holdIndent

    // A row that names a card carries its whole name in the padding, seated **to the mark's ink and not its box**:
    // `spaceXs` of air behind the ink and no more, with the box's own air on either side not spent a second time. The
    // same seat the copy mark takes beside a hash and the face takes beside a name (`HashPlate`, `CommitAuthorRow` —
    // デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」, 2026-08-26 ユーザー指示).
    leftPadding: menuItem.heads ? menuItem.markX + rowMark.inkWidth + Theme.spaceXs
                                : Theme.spaceSm + menuItem.holdIndent
    topPadding: 0
    bottomPadding: 0
    // A row this menu is not offering takes no room. The list lays its rows out by height, so an invisible one that
    // keeps a height leaves an empty row behind — a hole where the reader looks for the row that is missing (measured
    // on the file menu, whose two destructive rows are one per bucket).
    implicitHeight: menuItem.offered ? Theme.rowHeight : 0
    implicitWidth: (codeChip.visible ? codeChip.implicitWidth + Theme.spaceSm : 0)
                   + (menuItem.growsForText
                      ? itemLabel.implicitWidth : Math.min(itemLabel.implicitWidth, Metrics.labelColW))
                   + (menuItem.note !== "" ? noteLabel.implicitWidth + Theme.spaceSm : 0)
                   + menuItem.leftPadding + menuItem.rightPadding
    font.pixelSize: Theme.fontMd
    // The words do not say the gesture, so this is where it is left for a reader who cannot see the mark.
    Accessible.description: menuItem.holdMs > 0 ? Words.holdToActivate : ""

    // The one colour every word in the row follows, so the chip cannot disagree with the sentence it sits in. A held
    // row says what it costs in its own colour before it is touched at all — it is the one row in the menu that takes
    // something away (デザイン規約 §状態); over the fill the words cross the tone itself and lift clear of it.
    readonly property color wordColor: !menuItem.enabled || menuItem.blocked ? Theme.textMuted
                                     : menuItem.holding ? Theme.textOnAccent
                                     : menuItem.holdMs > 0 ? menuItem.holdTone : Theme.textPrimary

    // A blocked row's line is what the hover is for, so it comes before the elision's. Nothing else changes: one
    // tooltip, one delay.
    ToolTip.visible: (menuItem.hovered || menuItem.tipForced)
                     && (menuItem.blocked || itemLabel.truncated)
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: menuItem.blocked ? menuItem.blockedReason
                : menuItem.code !== "" ? menuItem.code + " " + menuItem.text : menuItem.text

    // The mark, inside the padding the whole menu carries for it rather than in the row's layout: it stands against the
    // card's own padding with nothing but air to its left, and the words follow at the distance `holdIndent` sets — not
    // at that distance plus the layout's gap (デザイン規約 §長押し). A row of the menu that is not held leaves the same space
    // empty, so the column of first letters holds.
    HoldIcon {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        progress: menuItem.holdProgress
        tint: menuItem.wordColor
        visible: menuItem.holdMs > 0
    }
    // **The other thing that can stand in that seat**: this row does not do what it says on its own — it raises a
    // question first. A row cannot be both (a question is not answered by holding the row that raises it), so the two
    // share the seat rather than crowding it (デザイン規約 §進行中の操作から出る).
    //
    // **It is not drawn the way the ring is, though — it is drawn the way every other `!` in this app is**: raised by
    // a gap and half a gap into the word's own bearing, hanging off the words rather than sitting centred in a column
    // of its own (`ActionButtonLabel`, `NameCell`, the toolbar's `push -f`). A mark that reads the same wherever it is
    // met is the whole point of having one shape for it (2026-08-22 ユーザー判断). At the head of the row because the
    // words run to the right of it — the same end `ActionButtonLabel` puts it on for a phrase.
    NavIcon {
        x: Theme.spaceXs / 2
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop - Theme.spaceXs
        kind: "bang"
        tint: Theme.warning
        width: Theme.iconSm
        height: Theme.iconSm
        visible: menuItem.asks && menuItem.holdMs <= 0
    }
    // The kind's own mark, standing where the row begins. Out in the card's padding rather than in the row's layout
    // for the same reason the ring is: the word behind it has to sit against the mark, not against the mark plus the
    // layout's gap. What is put at `markX` is the **ink**, so the box hangs half its own air either side of that —
    // air that is already the card's padding on the left and the word's gap on the right, and would otherwise be
    // spent twice.
    NavIcon {
        id: rowMark
        x: menuItem.markX - (rowMark.width - rowMark.inkWidth) / 2
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        width: Theme.iconMd
        height: Theme.iconMd
        visible: menuItem.heads
        kind: menuItem.heads ? menuItem.markKind : "branch"
        tint: menuItem.markTint
        // A `Canvas` in the overlay layer can miss its first chance to paint — the card is built before it is shown,
        // and a mark that never painted frames as one nobody wired.
        Component.onCompleted: rowMark.requestPaint()
        onVisibleChanged: if (visible) rowMark.requestPaint()
    }

    contentItem: RowLayout {
        spacing: Theme.spaceSm
        // The chip spends no width of its own beyond the column: the layout sees the menu's shared chip column — never
        // less than its own glyphs — so every row's words start on the same x (デザイン規約 §git 用語のコード表記). The word itself
        // starts where every other row starts its words, and the tint hangs outside its glyphs — left into the row
        // padding, right into the gap — half tint, half air; the column's spare width stays air.
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
                anchors.left: codeLabel.left
                anchors.right: codeLabel.right
                // The word's own step rather than the label's height, which is the mono family's line box — the one
                // part of this dress each OS settles differently (`ActionButtonLabel` carries the measurements). Here
                // it also has a line of its own to keep to: the row's words are set in the UI family, and a ground
                // taking the mono box stands taller than the sentence it is a word of.
                anchors.verticalCenter: codeLabel.verticalCenter
                height: codeLabel.font.pixelSize + Theme.spaceXs / 2
                // Half a gap of tint outside the glyphs, not a whole one: a chip that follows the hold mark would
                // otherwise reach back far enough to sit against it, and the row would read as one run of ink with no
                // air between the two things it is saying.
                anchors.leftMargin: -Theme.spaceXs / 2
                anchors.rightMargin: -Theme.spaceXs / 2
                radius: Theme.radiusSm
                // A faint lift off whatever the row is showing under it — the menu card at rest, the accent under the
                // pointer, the hold tone mid-hold — the way inline code sits in prose.
                color: Theme.bgHover
            }
            Label {
                id: codeLabel
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: menuItem.code
                font.family: Theme.monoFamily
                // A command and its flag are one thing said (`push --delete`), and a mono space is wider than the air
                // the chip keeps at its own ends — left alone the two drift apart and the chip reads as two words on
                // one ground.
                font.wordSpacing: -Theme.spaceXs
                font.pixelSize: menuItem.font.pixelSize
                color: menuItem.wordColor
            }
        }
        Label {
            id: itemLabel
            Layout.fillWidth: true
            text: menuItem.text
            // Written out rather than taken as a group, so the one row that changes its weight can (a group assignment
            // and a `font.weight` on the same Label is "already assigned").
            font.family: Theme.uiFamily
            font.pixelSize: menuItem.font.pixelSize
            // Pinned, not left to `AutoText`. Rows carry text nobody here chose — branch names, paths, commit subjects
            // — plus one deliberate placeholder in angle brackets, and AutoText decides by guessing whether a string
            // looks like markup. A branch called `<b>` should read as its name, not vanish.
            textFormat: Text.PlainText
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            // Clear of the arrow the style paints over the row's right edge on a row that opens a submenu (the note,
            // when there is one, is what sits last instead).
            rightPadding: !noteLabel.visible && menuItem.subMenu && menuItem.arrow
                          ? menuItem.arrow.width + Theme.spaceXs : 0
            // The section band's own spelling, for a row that names a card: the same weight and colour the sidebar
            // gives BRANCHES and TAGS, so the two read as one kind of thing wherever they are met (NavHeader).
            font.weight: menuItem.heads ? Font.DemiBold : menuItem.font.weight
            color: menuItem.heads ? Theme.textSecondary : menuItem.wordColor
        }
        Label {
            id: noteLabel
            visible: menuItem.note !== ""
            text: menuItem.note
            verticalAlignment: Text.AlignVCenter
            rightPadding: menuItem.subMenu && menuItem.arrow ? menuItem.arrow.width + Theme.spaceXs : 0
            color: Theme.warning
            font.pixelSize: Theme.fontSm
        }
    }

    background: Rectangle {
        radius: Theme.radiusSm
        // Every row hovers to the same wash (デザイン規約 §メニュー). The held rows could never take the solid accent — their
        // words are the tone, and a face under them would both fight the colour and take the warning away at the moment
        // the pointer arrives — and one menu holding two strengths of highlight read as two different states rather
        // than one pointer (2026-08-11 ユーザー報告). The list the combo drops has said it this way all along (§選ぶ欄と打つ欄).
        color: menuItem.highlighted ? Theme.bgHover : "transparent"
        HoldFill {
            progress: menuItem.holdProgress
            tone: menuItem.holdTone
        }
        // Under the hand, a row that names a card is underlined in its mark's colour, right across the card. The wash
        // says *where the hand is* — every row gets that — and this says **what is about to open**, which is the one
        // thing this row does that no other row does (デザイン規約 §メニュー の入れ子). Across the whole row rather than
        // under the word: it is the card below that is being named, not the two words themselves.
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: Theme.borderWidth
            visible: menuItem.heads && (menuItem.hovered || menuItem.highlighted)
            color: menuItem.markTint
        }
    }

    HoldDriver {
        id: holdDrive
        holdMs: menuItem.holdMs
        onFinished: menuItem.held()
    }
    // Takes the press before the MenuItem underneath can: a click here would emit `triggered`, which both runs the row
    // and closes the menu — and the menu has to stay open for as long as the hold lasts. Hover is left alone (this one
    // accepts none), so the row still highlights the way every other row does.
    MouseArea {
        id: rowPress
        anchors.fill: parent
        // A blocked row takes the press and does nothing with it: the row underneath would otherwise run and close the
        // menu, and the line explaining why it is out would never be read.
        enabled: menuItem.blocked || ((menuItem.holdMs > 0 || menuItem.staysOpen) && menuItem.enabled)
        onPressed: if (!menuItem.blocked) holdDrive.begin()
        // Released anywhere, or dragged off the row: both call it off. A stays-open row has no fill to call off —
        // letting go on the row is its click, and letting go outside it is not.
        onReleased: mouse => {
            holdDrive.letUp()
            if (!menuItem.blocked && menuItem.holdMs <= 0 && menuItem.staysOpen
                    && mouse.x >= 0 && mouse.y >= 0 && mouse.x <= width && mouse.y <= height)
                menuItem.picked()
        }
        onCanceled: holdDrive.letUp()
        onPositionChanged: if (!containsMouse) holdDrive.letUp()
    }
    // The same row from the keyboard: walk to it and hold Space or Enter. Accepting the key keeps the menu from
    // triggering the row outright, and auto-repeat is dropped on both edges (デザイン規約 §長押し).
    Keys.onPressed: event => {
        // Same for the keyboard: the row is walked to and reads its line, and the key that would run it is taken and
        // dropped.
        if (menuItem.blocked) {
            event.accepted = holdDrive.holdKey(event.key)
            return
        }
        holdDrive.pressKey(event)
    }
    Keys.onReleased: event => holdDrive.releaseKey(event)
}

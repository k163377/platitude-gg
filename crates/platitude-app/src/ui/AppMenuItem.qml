import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One AppMenu row: the list row height and body size used everywhere else, and its own words as its width. A row elides
// only where the menu has run out of window to grow into; hovering an elided row says the whole line.
MenuItem {
    id: menuItem

    /// Whether this row is on offer for the thing the menu was opened on — false where it cannot be chosen, and then
    /// the row goes (デザイン規約 §メニュー).
    ///
    /// Kept apart from `visible`, which it drives, because `visible` cannot be *asked*: a menu's list releases the rows
    /// it is not showing and turns their visibility off behind the binding, so every row of a closed menu reads as
    /// invisible. The menu and its dividers have to know what is on offer before it opens.
    property bool offered: true
    visible: menuItem.offered

    /// A short warning said after the row's words ("already pushed"). The row still runs on click — this is the tag
    /// that says what it costs, the same shape the amend editor uses.
    property string note: ""

    /// A git term said in git's own spelling — lowercase, mono, on a faint chip
    /// (デザイン 規約 §git 用語のコード表記). Left untranslated: it is the command itself. It sits ahead of
    /// `text`, which carries whatever of the sentence is left ("this file"), often nothing.
    property string code: ""
    /// **A ref name inside the row's words**, and the sentence with the name's own seat left in it ("into %1"). The
    /// name is drawn in the colour it wears everywhere else it is met: these sentences take
    /// one name and it is always the branch the working tree is on (`merge` into main, `rebase` main onto it, `reset`
    /// main here), and that name is `textLink` wherever it appears — the left pane's row, the chip, the hover card
    /// (デザイン規約 §ref の種別「現在のブランチは、名前が出る場所すべてで textLink」).
    ///
    /// The seat is cut out of the sentence: a branch called
    /// `it` would otherwise be found in the first word of `into it`.
    property string refSentence: ""
    property string refName: ""
    text: menuItem.refSentence !== "" ? menuItem.refSentence.arg(menuItem.refName) : ""
    /// The mark a row wears, and its colour: the `NavIcon` kind of the thing the rows behind this one
    /// act on (デザイン規約 §メニュー の入れ子). Only the rows that open a submenu carry one — a row that runs a command says
    /// which by its chip, and a row that opens a card of them has no command to name.
    ///
    /// **A row with a mark is the sidebar's section band, in a menu**: the mark where the rows start their words, its
    /// own word a hair behind it, spelled the way a section spells its name. The word is out of the chip column
    /// entirely — this row names the card below it.
    property string markKind: ""
    property color markTint: Theme.textSecondary
    readonly property bool heads: menuItem.markKind !== ""

    /// **A name inside the row's words, with the mark its kind is read by against it** — a working copy's folder
    /// behind the WORKTREES mark, wherever a row leads to one (`RefRowMenu` の `Open`). `text` carries the words
    /// before it and **the name ends them**: a mark is drawn, not written, so it cannot be put in a sentence's seat
    /// the way a coloured name can (`refWords`) — and a row that led somewhere *through* a name would be naming two
    /// things at once.
    property string nameMark: ""
    property string markName: ""
    property color nameMarkTint: Theme.textSecondary
    readonly property bool namesMark: menuItem.nameMark !== "" && menuItem.markName !== ""
    /// The step in front of that mark. **A mark inside a word is a letter, not a thing standing in a row**
    /// (デザイン規約 §余白 / §タブの所作): the word's own step, less the air the mark's box already holds — and
    /// between the mark and the name it is about, nothing is spent at all.
    readonly property real markWordGap:
        Theme.spaceXs - (Theme.iconSm - nameMarkIcon.inkWidth) / 2

    /// The width this row asks the menu's shared chip column to hold: its chip's own glyphs, whether or not words
    /// follow them. A chip that ends its row asks too — the column has to clear the widest command, or it would end
    /// past where the other rows' words begin (デザイン規約 §git 用語のコード表記). Only a row with no chip asks for nothing —
    /// and a row that names a card asks for nothing either, standing outside the column.
    readonly property real codeColSeat: menuItem.code !== "" ? codeLabel.implicitWidth : 0

    /// Whether the row's words bid their whole width from the menu. Off for a row whose text is data — the name a
    /// delete row re-states: it bids at most the seat a ref name gets in the graph (`labelColW`),
    /// elides past that into the hover that already says the whole line, and stretches further only into width the
    /// other rows have paid for (デザイン規約 §メニュー). A floor: a sparse menu at `menuMinW` with a wide
    /// frozen chip column otherwise leaves the name zero width.
    property bool growsForText: true

    /// Why this row cannot be chosen right now, in one line — and, by being non-empty, that it cannot. A blocked row is
    /// greyed the way a disabled one is (§無効) but stays hoverable, because the line is the whole point: a row of a
    /// fixed table that says nothing about why it is out is worse than no row at all (デザイン規約 §メニュー の削除の表). Presses and
    /// holds do nothing.
    property string blockedReason: ""
    /// The same, said once for the whole menu (`AppMenu.heldReason`) — read off the menu the way `holdIndent` is.
    /// **Only the rows that act read it**: what is held is what the card holds, and the rows in there carry the
    /// line themselves (the card sets its own `heldReason`).
    readonly property string menuHeldReason:
        menuItem.subMenu ? ""
        : menuItem.menu !== null && menuItem.menu.heldReason !== undefined ? menuItem.menu.heldReason : ""
    /// Why this row is out, whichever of the two said so. **The row's own answer wins**: that one is about the ref it
    /// names, and the menu's is about right now — the more particular line is the one worth reading.
    readonly property string blockedWhy:
        menuItem.blockedReason !== "" ? menuItem.blockedReason : menuItem.menuHeldReason
    readonly property bool blocked: menuItem.blockedWhy !== ""
    /// A word inside that line which is a working copy's folder — the shared tooltip reads it off this row and
    /// stands the tree mark against it (`SharedToolTip.tipMarkWord`). A copy's name wears that mark wherever it is
    /// said (デザイン規約 §ref の種別), and a tooltip is the one place the name is a string rather than something with
    /// a seat beside it.
    property string tipMarkWord: ""

    /// Automation: show the tooltip with no pointer behind it. The same property the real hover drives, so a run that
    /// never reached the row photographs a row without one (verify-ui §hover の絵の撮り方).
    property bool tipForced: false

    /// Held, for a row that would otherwise have to raise a question of its own (デザイン規約 §長押し). Zero
    /// is an ordinary row. A hold row reports no click at all — the press is taken before the button behind it can see
    /// it, which is also what keeps the menu from closing under the hold.
    ///
    /// The row says so with the mark ahead of its words: the menu leaves the same seat for it on every
    /// row, so a held row reads down the same column as the rest (`AppMenu.holdIndent`).
    property int holdMs: 0
    /// Whether pressing this row raises a question — a `!` in the mark's seat, which is
    /// the row's one place for "read this before you press" (デザイン規約 §進行中の操作から出る). A row asks or holds:
    /// the hold is how a row that acts is confirmed, and a row that asks is confirmed on the bar it raises.
    property bool asks: false
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// **The length the press under way was given** (`HoldDriver.armedMs`), which is the live one while no press is
    /// under way. A row whose chip, note or tone is worked out from the same answer the length is reads this instead,
    /// so the row cannot change what it says — or what it runs — under a hand that is already holding
    /// (デザイン規約 §長押し「長押しか否かは押した瞬間に確定」).
    readonly property alias armedMs: holdDrive.armedMs
    /// **A gesture is under way**, the fill's slide back included (`HoldDriver.gesturing`) — what a row latches on
    /// when what it dresses itself from is an answer other than the length (`RefBranchMenu` の `Delete both`).
    readonly property alias gesturing: holdDrive.gesturing
    /// The colour the hold fills the row with.
    property color holdTone: Theme.danger
    /// Held all the way down.
    signal held()
    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        holdDrive.begin()
    }
    readonly property bool holding: menuItem.holdProgress > 0

    /// A row that runs on a click but leaves the menu standing, for the one thing here that git answers:
    /// `branch -d` refuses while the branch holds commits nothing else does, and the refusal is the question
    /// worth asking (デザイン規約 §左メニューの所作). The menu has to outlive the click for that answer to have somewhere to land —
    /// the row turns into a held one where the hand already is.
    property bool staysOpen: false
    /// Clicked, on a row that stays open. `triggered` never fires for one: the press is taken before the button behind
    /// it can see it, which is what keeps the menu up.
    signal picked()

    /// How far this menu's rows are pushed in to leave room for the hold mark — the same on every row, held or not, so
    /// a menu still reads down one column of first letters (`AppMenu.holdIndent`).
    readonly property real holdIndent:
        menuItem.menu !== null && menuItem.menu.holdIndent !== undefined ? menuItem.menu.holdIndent : 0

    padding: Theme.spaceSm

    /// Where a card's name starts its mark — **the mark's ink**. Shifted left by exactly the seat the
    /// menu leaves for the hold ring and the `!`: where there is one the mark stands in it, where there is none it
    /// starts on the same x the other rows start their words.
    readonly property real markX: Theme.spaceSm - menuItem.holdIndent

    // A row that names a card carries its whole name in the padding, seated **to the mark's ink**:
    // `spaceXs` of air behind the ink and no more, with the box's own air on either side not spent a second time. The
    // same seat the copy mark takes beside a hash and the face takes beside a name (`HashPlate`, `CommitAuthorRow` —
    // デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」).
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
                   // The marked name is data — a folder can be called anything — so it bids the seat a name gets
                   // and elides past it, whatever the words in front of it are allowed (デザイン規約 §メニュー).
                   + (menuItem.namesMark
                      ? menuItem.markWordGap + Theme.iconSm
                        + Math.min(markNameLabel.implicitWidth, Metrics.labelColW) : 0)
                   + (menuItem.note !== "" ? noteLabel.implicitWidth + Theme.spaceSm : 0)
                   + menuItem.leftPadding + menuItem.rightPadding
    font.pixelSize: Theme.fontMd
    // The gesture is said here, for a reader who cannot see the mark.
    Accessible.description: menuItem.holdMs > 0 ? Words.holdToActivate : ""

    // The one colour every word in the row follows, so the chip cannot disagree with the sentence it sits in. A held
    // row says what it costs in its own colour before it is touched at all — it is the one row in the menu that takes
    // something away (デザイン規約 §状態); over the fill the words cross the tone itself and lift clear of it.
    readonly property color wordColor: !menuItem.enabled || menuItem.blocked ? Theme.textMuted
                                     : menuItem.holding ? Theme.textOnAccent
                                     : menuItem.armedMs > 0 ? menuItem.holdTone : Theme.textPrimary
    /// The colour the name inside those words takes instead. **Its own only while the row has nothing of its own to
    /// say**: a row that cannot be chosen is grey to its last letter (§無効), and one that is held says what it costs
    /// across the whole line (§長押し) — a name lit blue in either would read as the one part of the row still
    /// answering.
    readonly property color refColor: !menuItem.enabled || menuItem.blocked || menuItem.armedMs > 0
                                      ? menuItem.wordColor : Theme.textLink
    /// Those words as the markup `Text.StyledText` reads: the sentence in the row's colour, the name in its own. One
    /// label, so the line elides, measures and hovers the way every other row's does.
    ///
    /// **Every piece is escaped on the way in.** The sentence is this app's and the name is the repository's — a
    /// branch called `<b>` has to read as its name. Spaces go as `&nbsp;` for the reason a diff line's do
    /// (`encode::markup`): rich text folds a run of them the way HTML does.
    readonly property string refWords: {
        const seat = menuItem.refSentence.indexOf("%1")
        if (seat < 0)
            return ""
        return menuItem.inked(menuItem.refSentence.substring(0, seat))
             + "<font color=\"" + menuItem.refColor + "\">" + menuItem.inked(menuItem.refName) + "</font>"
             + menuItem.inked(menuItem.refSentence.substring(seat + 2))
    }
    function inked(words) {
        return words.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/ /g, "&nbsp;")
    }

    // A blocked row's line is what the hover is for, so it comes before the elision's. Nothing else changes: one
    // tooltip, one delay.
    ToolTip.visible: (menuItem.hovered || menuItem.tipForced)
                     && (menuItem.blocked || itemLabel.truncated
                         || (menuItem.namesMark && markNameLabel.truncated))
    ToolTip.delay: Metrics.tipDelayMs
    // The whole line, marked name included: the mark is not a word, so what the hover hands over is the sentence
    // with the name in it — and the copy the row leads to is the one thing a cut name takes away.
    ToolTip.text: menuItem.blocked ? menuItem.blockedWhy
                : menuItem.code !== "" ? menuItem.code + " " + menuItem.text
                : menuItem.namesMark ? menuItem.text + " " + menuItem.markName
                : menuItem.text

    // The mark, inside the padding the whole menu carries for it: it stands against the card's own padding with nothing
    // but air to its left, and the words follow at the distance `holdIndent` sets (デザイン規約 §長押し). A row of the menu that
    // is not held leaves the same space empty, so the column of first letters holds.
    HoldIcon {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        progress: menuItem.holdProgress
        tint: menuItem.wordColor
        visible: menuItem.armedMs > 0
    }
    // **The other thing that can stand in that seat**: this row raises a question first. A row is one or the other (a
    // question is not answered by holding the row that raises it), so the two share the seat (デザイン規約 §進行中の操作から出る).
    //
    // **It is drawn the way every other `!` in this app is**: raised by a gap and half a gap into the word's own
    // bearing, hanging off the words (`ActionButtonLabel`, `NameCell`, the toolbar's `push -f`). A mark that reads the
    // same wherever it is met is the whole point of having one shape for it. At the head of the row because the words
    // run to the right of it — the same end `ActionButtonLabel` puts it on for a phrase.
    NavIcon {
        x: Theme.spaceXs / 2
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop - Theme.spaceXs
        kind: "bang"
        tint: Theme.warning
        width: Theme.iconSm
        height: Theme.iconSm
        // Only on a row that can be pressed: the mark says "read this before you press", and there is no
        // press to read it before — what the row has to say then is the line under the pointer (`blockedWhy`).
        visible: menuItem.asks && menuItem.armedMs <= 0 && !menuItem.blocked
    }
    // The kind's own mark, standing where the row begins. Out in the card's padding
    // for the same reason the ring is: the word behind it has to sit against the mark.
    // What is put at `markX` is the **ink**, so the box hangs half its own air either side of that —
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
        // **The steps are written on the items, not on the row.** All but one of them is the gap between two things
        // the row says and takes `spaceSm`; the marked name is the last word of one sentence and takes a word's step
        // (`markWordGap`), which a shared spacing could not tell apart.
        spacing: 0
        // The chip spends no width of its own beyond the column: the layout sees the menu's shared chip column — never
        // less than its own glyphs — so every row's words start on the same x (デザイン規約 §git 用語のコード表記). The word itself
        // starts where every other row starts its words, and the tint hangs outside its glyphs — left into the row
        // padding, right into the gap — half tint, half air; the column's spare width stays air.
        Item {
            id: codeChip
            visible: menuItem.code !== ""
            // The row's own step, off the item now that the layout keeps none. An invisible item is out of the
            // layout altogether, margin and all, which is what a shared spacing did for it before.
            Layout.rightMargin: Theme.spaceSm
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
                // The word's own step — the label's height is the mono family's line box, the one
                // part of this dress each OS settles differently (`ActionButtonLabel` carries the measurements). Here
                // it also has a line of its own to keep to: the row's words are set in the UI family, and a ground
                // taking the mono box stands taller than the sentence it is a word of.
                anchors.verticalCenter: codeLabel.verticalCenter
                height: codeLabel.font.pixelSize + Theme.spaceXs / 2
                // Half a gap of tint outside the glyphs: a chip that follows the hold mark would
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
            // The width goes to whichever of the two carries the name: a row with a marked name has its data there,
            // and these words are the fixed part in front of it.
            Layout.fillWidth: !menuItem.namesMark
            // The row's own words, or the same line spelled as markup where one of them is a name in a colour of its
            // own (`refWords`). What the row *says* is `text` either way — the tooltip and the reader who cannot see
            // it are handed the sentence.
            text: menuItem.refWords !== "" ? menuItem.refWords : menuItem.text
            // Written out, so the one row that changes its weight can (a group assignment
            // and a `font.weight` on the same Label is "already assigned").
            font.family: Theme.uiFamily
            font.pixelSize: menuItem.font.pixelSize
            // Pinned. Rows carry text nobody here chose — branch names, paths, commit subjects
            // — plus one deliberate placeholder in angle brackets, and AutoText decides by guessing whether a string
            // looks like markup. A branch called `<b>` reads as its name. The one line that is
            // markup says so because this file wrote it, and escaped everything that went into it.
            textFormat: menuItem.refWords !== "" ? Text.StyledText : Text.PlainText
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
        // The name the row leads to, with its mark set in front of it as a letter of the same word
        // (`markWordGap`): one mark for one idea, the very one the WORKTREES rows, the tab and the graph's chips
        // wear for a working copy (デザイン規約 §ref の種別).
        RowLayout {
            visible: menuItem.namesMark
            Layout.fillWidth: menuItem.namesMark
            Layout.leftMargin: menuItem.namesMark ? menuItem.markWordGap : 0
            spacing: 0
            Item {
                Layout.preferredWidth: Theme.iconSm
                Layout.preferredHeight: Theme.iconSm
                Layout.alignment: Qt.AlignVCenter
                NavIcon {
                    id: nameMarkIcon
                    anchors.centerIn: parent
                    width: Theme.iconSm
                    height: Theme.iconSm
                    // The 16-grid scales with the seat and the line does not, so a mark set in a word would
                    // otherwise carry more weight than the letters beside it (`NavIcon.stroke`).
                    stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                    // `NavIcon` paints on a kind it is given; an empty one would be a shape nobody asked for.
                    kind: menuItem.namesMark ? menuItem.nameMark : "tree"
                    tint: menuItem.blocked || !menuItem.enabled || menuItem.armedMs > 0
                          ? menuItem.wordColor : menuItem.nameMarkTint
                    // A `Canvas` in the overlay layer can miss its first chance to paint — the card is built before
                    // it is shown, and a mark that never painted frames as one nobody wired.
                    Component.onCompleted: nameMarkIcon.requestPaint()
                    onVisibleChanged: if (visible) nameMarkIcon.requestPaint()
                }
            }
            Label {
                id: markNameLabel
                Layout.fillWidth: true
                text: menuItem.markName
                font.family: Theme.uiFamily
                font.pixelSize: menuItem.font.pixelSize
                elide: Text.ElideRight
                verticalAlignment: Text.AlignVCenter
                color: menuItem.wordColor
            }
        }
        Label {
            id: noteLabel
            visible: menuItem.note !== ""
            // The row's own step, for the same reason the chip's is on the chip.
            Layout.leftMargin: Theme.spaceSm
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
        // than one pointer. The list the combo drops has said it this way all along (§選ぶ欄と打つ欄).
        color: menuItem.highlighted ? Theme.bgHover : "transparent"
        HoldFill {
            progress: menuItem.holdProgress
            tone: menuItem.holdTone
        }
        // Under the hand, a row that names a card is underlined in its mark's colour, right across the card. The wash
        // says *where the hand is* — every row gets that — and this says **what is about to open**, which is the one
        // thing this row does that no other row does (デザイン規約 §メニュー の入れ子). Across the whole row:
        // it is the card below that is being named.
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
        // **The length this press was given**, so a row that turns into a held one under the hand does not take the
        // area out from under the gesture it is in the middle of — the release would then never arrive here and the
        // fill would run on to fire a hold nobody was still making.
        enabled: menuItem.blocked || ((holdDrive.armedMs > 0 || menuItem.staysOpen) && menuItem.enabled)
        onPressed: if (!menuItem.blocked) holdDrive.begin()
        // Released anywhere, or dragged off the row: both call it off. A stays-open row has no fill to call off —
        // letting go on the row is its click, and letting go outside it is not.
        onReleased: mouse => {
            // Worked out before the latch opens, and nothing at all where the answer the press was made under has
            // gone since (`HoldDriver.stale`): this row's plain press and its hold are two different commands, and a
            // press that outlived its own premise is not either of them (デザイン規約 §長押し).
            const pick = !menuItem.blocked && holdDrive.armedMs <= 0 && !holdDrive.stale && menuItem.staysOpen
                    && mouse.x >= 0 && mouse.y >= 0 && mouse.x <= width && mouse.y <= height
            holdDrive.letUp()
            if (pick)
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
    // A key let go after the focus has moved is answered somewhere else (`HoldDriver.focusLost`).
    onActiveFocusChanged: if (!menuItem.activeFocus) holdDrive.focusLost()
}

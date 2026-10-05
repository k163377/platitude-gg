import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One AppMenu row: as wide as its own words, eliding only where the menu has run out of window (デザイン規約 §メニュー).
MenuItem {
    id: menuItem

    /// Whether this row is on offer — false where it cannot be chosen, and then the row goes (デザイン規約 §メニュー).
    /// Kept apart from `visible`, which it drives: a closed menu's list turns its rows' `visible` off behind the
    /// binding, and the menu and its dividers read this before it opens.
    property bool offered: true
    visible: menuItem.offered

    /// A short warning tag after the row's words ("already pushed"); the row still runs on click.
    property string note: ""

    /// A second name after the row's own — the working copy a tab is in, drawn the way the tab draws it
    /// (デザイン規約 §タブの所作). Empty draws none.
    property string trail: ""

    /// A git term in git's own spelling, on a chip (デザイン規約 §git 用語のコード表記). It sits ahead of `text`, which
    /// carries whatever of the sentence is left ("this file"), often nothing.
    property string code: ""
    /// A ref name inside the row's words, and the sentence with the name's seat left in it ("into %1"). These sentences
    /// only ever name the current branch, so the name is `textLink`
    /// (デザイン規約 §ref の種別「現在のブランチは、名前が出る場所すべてで textLink」).
    /// The seat is cut out of the sentence, not searched for in `text`: a branch `it` would be found in `into it`.
    property string refSentence: ""
    property string refName: ""
    text: menuItem.refSentence !== "" ? menuItem.refSentence.arg(menuItem.refName) : ""
    /// The mark a row wears, and its colour. On a row that opens a card it is the `NavIcon` kind of what the card acts
    /// on (デザイン規約 §メニュー の入れ子), and the row becomes a heading: the sidebar's section band in a menu, its
    /// word out of the chip column. A row with `headed: false` wears a mark about itself instead.
    property string markKind: ""
    property color markTint: Theme.textSecondary
    readonly property bool heads: menuItem.markKind !== ""
    /// Whether that mark makes this row a heading. False for a mark about the row itself (a copy's padlock, the house,
    /// the tree — the marks `NavRowBody.seatMark` gives): dressed as a heading it reads as a title with nothing under.
    property bool headed: true
    readonly property bool titled: menuItem.heads && menuItem.headed
    /// Whether this row's menu keeps the left menu's seat on every row (`AppMenu.seatWorn`), so every name on the card
    /// begins on one x as in `NameCell`; a mark about the row stands in it at `iconSm`.
    readonly property bool seated:
        menuItem.menu !== null && menuItem.menu.seatWorn !== undefined && menuItem.menu.seatWorn === true
    readonly property bool wearsSeat: menuItem.heads && !menuItem.headed
    readonly property real seatX: Theme.spaceSm + menuItem.holdIndent
    /// Read by `AppMenu.headingSeat` to line a card's headings up.
    readonly property real headInk: menuItem.titled ? rowMark.inkWidth : 0
    /// The step every heading on such a card starts its words at, marked or not; zero on any other card.
    readonly property real headingSeat:
        menuItem.subMenu !== null && menuItem.menu !== null && menuItem.menu.headingSeat !== undefined
        ? menuItem.menu.headingSeat : 0
    /// How far this row's branch stands from its upstream (`HeadTrack`); both zero draws nothing (§左メニューの所作).
    property int ahead: 0
    property int behind: 0
    /// The badge the left menu's row ends with (`NavRowBody.remoteSeat`): on a remote, a pull request open, or the
    /// upstream gone.
    property bool remoteBadge: false
    property bool badgePr: false
    property bool badgeGone: false
    readonly property bool badged: menuItem.remoteBadge || menuItem.badgePr || menuItem.badgeGone

    /// A name ending the row's words, with its kind's mark against it — a working copy's folder behind the WORKTREES
    /// mark (`RefRowMenu`'s `Open`). It always ends them: a mark is drawn, not written, so it cannot take a sentence's
    /// seat the way a coloured name can (`refWords`).
    property string nameMark: ""
    property string markName: ""
    property color nameMarkTint: Theme.textSecondary
    readonly property bool namesMark: menuItem.nameMark !== "" && menuItem.markName !== ""
    /// Automation: whether that name lost its tail — a picture a pixel short and one whole read alike at a glance.
    readonly property bool nameCut: menuItem.namesMark && markNameLabel.truncated
    /// A fact about the row in the left menu's far column — the branch a working copy has out
    /// (`NavRowBody.branchSeat`). Empty draws none.
    property string sideName: ""
    /// The step in front of `nameMark`: a mark inside a word is a letter (デザイン規約 §余白), so a word's step less the
    /// air its box already holds — and nothing between the mark and its name.
    readonly property real markWordGap:
        Theme.spaceXs - (Theme.iconSm - nameMarkIcon.inkWidth) / 2

    /// The width this row asks of the menu's shared chip column: its chip's glyphs, even where the chip ends the row —
    /// the column has to clear the widest command (デザイン規約 §git 用語のコード表記).
    readonly property real codeColSeat: menuItem.code !== "" ? codeLabel.implicitWidth : 0

    /// Whether the row's words bid their whole width. Off for a row whose text is data (the name a delete row
    /// re-states): it bids at most `labelColW` and elides past that into the hover (デザイン規約 §メニュー). Not zero:
    /// a sparse menu at `menuMinW` with a wide frozen chip column would leave the name no width.
    property bool growsForText: true

    /// Why this row cannot be chosen right now — and, by being non-empty, that it cannot. Greyed like a disabled row
    /// (§無効) but still hoverable, since the line is the point (デザイン規約 §メニュー の削除の表). Presses and holds
    /// do nothing.
    property string blockedReason: ""
    /// The same, said once for the whole menu (`AppMenu.heldReason`). A card's title row skips it: the rows inside
    /// carry the line (the card sets its own `heldReason`).
    readonly property string menuHeldReason:
        menuItem.subMenu ? ""
        : menuItem.menu !== null && menuItem.menu.heldReason !== undefined ? menuItem.menu.heldReason : ""
    /// The row's own reason wins over the menu's: it is about the ref the row names, the more particular line.
    readonly property string blockedWhy:
        menuItem.blockedReason !== "" ? menuItem.blockedReason : menuItem.menuHeldReason
    readonly property bool blocked: menuItem.blockedWhy !== ""
    /// A working copy's folder inside the row's own reason (`blockedReason`); empty for none.
    property string reasonMarkWord: ""
    /// The word the shared tooltip stands the tree mark against (`SharedToolTip.tipMarkWord`, デザイン規約 §ref の種別
    /// 「ツールチップだけは字の中」): the folder in the reason, or the folder this row names behind the tree mark, which the
    /// hover gives back whole when it is cut.
    readonly property string tipMarkWord: menuItem.blocked ? menuItem.reasonMarkWord
                                        : menuItem.nameMark === "tree" ? menuItem.markName : ""

    /// Automation: show the tooltip with no pointer behind it, through the binding the hover drives
    /// (verify-ui §hover の絵の撮り方).
    property bool tipForced: false

    /// Held, for a row that would otherwise have to raise a question of its own (デザイン規約 §長押し). Zero is an
    /// ordinary row. A hold row reports no click (`rowPress`).
    property int holdMs: 0
    /// Pressing this row raises a question: a `!` in the hold mark's seat (デザイン規約 §進行中の操作から出る). A row
    /// asks or holds, never both — a row that asks is confirmed on the bar it raises.
    property bool asks: false
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// The length the press under way was given (`HoldDriver.armedMs`; the live one between presses). Anything the row
    /// dresses from the same answer reads this, so it cannot change under a holding hand
    /// (rules-refs の「長押しか否かは押した瞬間に確定する」の行).
    readonly property alias armedMs: holdDrive.armedMs
    /// A gesture is under way, the fill's slide back included (`HoldDriver.gesturing`) — the latch for a row dressed
    /// from an answer other than the length (`RefBranchMenu`'s `Delete both`).
    readonly property alias gesturing: holdDrive.gesturing
    /// What a held row wears — word, chip, mark and fill. `warning`: what a hold here throws away is brought back from
    /// the discard record. **A row that cannot be brought back names `danger` itself** (デザイン規約 §長押し の色の表) —
    /// so a new held row never takes the stronger warning by leaving this out.
    property color holdTone: Theme.warning
    /// Held all the way down.
    signal held()
    /// Automation: run the hold to its end without a press behind it — on a row a hand could hold.
    function completeHold() {
        if (menuItem.blocked || !menuItem.enabled)
            return
        holdDrive.begin()
    }
    // Blocked under a hand: the hold under way blanks rather than running out to fire, as a row blocked from the start
    // drops the press (`rowPress`).
    onBlockedChanged: if (menuItem.blocked) holdDrive.blank()
    readonly property bool holding: menuItem.holdProgress > 0

    /// A row that runs on a click but leaves the menu standing: `branch -d` refuses while the branch holds commits
    /// nothing else does, and the refusal has to land where the hand already is — the row turns into a held one
    /// (デザイン規約 §左メニューの所作).
    property bool staysOpen: false
    /// Clicked, on a row that stays open; `triggered` never fires for one (`rowPress` takes the press).
    signal picked()

    /// The hold mark's seat, the same on every row of the menu (`AppMenu.holdIndent`).
    readonly property real holdIndent:
        menuItem.menu !== null && menuItem.menu.holdIndent !== undefined ? menuItem.menu.holdIndent : 0

    padding: Theme.spaceSm

    /// Where a heading's mark ink starts: the x other rows start their words at, shifted left into the hold / `!` seat
    /// where the menu has one (デザイン規約 §メニュー「印の始まりは文字に合わせ」).
    readonly property real markX: Theme.spaceSm - menuItem.holdIndent

    // A heading's words sit `spaceXs` past the mark's ink, not its box
    // (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」). On a card that keeps the seat, every row's
    // words start past it, marked or not (`NameCell`'s spacing).
    leftPadding: menuItem.headingSeat > 0 ? menuItem.markX + menuItem.headingSeat
               : menuItem.titled ? menuItem.markX + rowMark.inkWidth + Theme.spaceXs
               : menuItem.seated ? menuItem.seatX + Theme.iconXs + Theme.spaceXs
               : menuItem.heads ? menuItem.markX + rowMark.inkWidth + Theme.spaceXs
               : Theme.spaceSm + menuItem.holdIndent
    topPadding: 0
    bottomPadding: 0
    // A row this menu is not offering takes no room: the list lays rows out by height, so an invisible row that keeps
    // one leaves a hole.
    implicitHeight: menuItem.offered ? Theme.rowHeight : 0
    // The chip and the marked name are bid in whole pixels, as the layout hands them out: the row's `RowLayout` rounds
    // each of those up, and a sum rounded once comes up the fractions short — the marked name, laid out last, elides
    // (a Linux font's `worktree remove` + folder).
    implicitWidth: (codeChip.visible ? Math.ceil(codeChip.implicitWidth) + Theme.spaceSm : 0)
                   + (menuItem.growsForText
                      ? itemLabel.implicitWidth : Math.min(itemLabel.implicitWidth, Metrics.labelColW))
                   // The marked name is data: it bids at most `labelColW` and elides past it (デザイン規約 §メニュー).
                   + (menuItem.namesMark
                      ? menuItem.markWordGap
                        + Math.ceil(Theme.iconSm + Math.min(markNameLabel.implicitWidth, Metrics.labelColW)) : 0)
                   + (menuItem.note !== "" ? noteLabel.implicitWidth + Theme.spaceSm : 0)
                   // Everything drawn after the words bids too, or the row that sets the menu's width cuts its own
                   // name. Asked of what the row holds, not `visible`: a closed card's rows answer false, and the
                   // card is measured before it opens.
                   + (menuItem.sideName !== "" ? sideLabel.implicitWidth + Theme.spaceLg : 0)
                   + (menuItem.ahead > 0 || menuItem.behind > 0 ? rowTrack.implicitWidth + Theme.spaceSm : 0)
                   + (menuItem.badged ? Theme.iconSm + Theme.spaceXs : 0)
                   + (menuItem.trail !== "" ? Theme.spaceXs + Theme.iconXs + trailLabel.implicitWidth : 0)
                   + menuItem.leftPadding + menuItem.rightPadding
    font.pixelSize: Theme.fontMd
    Accessible.description: menuItem.holdMs > 0 ? Words.holdToActivate : ""

    // The one colour every word in the row follows, so the chip cannot disagree with its sentence. A held row wears
    // its tone before it is touched at all (デザイン規約 §状態).
    readonly property color wordColor: !menuItem.enabled || menuItem.blocked ? Theme.textMuted
                                     : menuItem.holding ? Theme.textOnAccent
                                     : menuItem.armedMs > 0 ? menuItem.holdTone : Theme.textPrimary
    /// The colour the name inside those words takes: its own only while the row has no colour of its own
    /// (デザイン規約 §メニュー「文の中に入った名前も、色は名前のもの」).
    readonly property color refColor: !menuItem.enabled || menuItem.blocked || menuItem.armedMs > 0
                                      ? menuItem.wordColor : Theme.textLink
    /// Those words as `Text.StyledText` markup, in one label so the line elides, measures and hovers like any row's.
    /// Every piece is escaped: the name is the repository's (a branch called `<b>`), and spaces go as `&nbsp;` because
    /// rich text folds a run of them (`encode::markup`).
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

    /// Out of the menu, never over its other rows — on the right first, where a sub-menu opens, so a parent menu on the
    /// left stays readable (`SharedToolTip.tipRowSide`).
    readonly property string tipRowSide: "right"
    // A blocked row's line comes before the elision's: one tooltip, one delay.
    ToolTip.visible: (menuItem.hovered || menuItem.tipForced)
                     && (menuItem.blocked || itemLabel.truncated
                         || (menuItem.namesMark && markNameLabel.truncated))
    ToolTip.delay: Metrics.tipDelayMs
    // The whole line, marked name included — a cut name is what the hover is there to give back, behind a chip too
    // (`worktree remove`).
    ToolTip.text: menuItem.blocked ? menuItem.blockedWhy
                : [menuItem.code, menuItem.text, menuItem.namesMark ? menuItem.markName : ""]
                  .filter(words => words !== "").join(" ")

    // The hold mark, in the seat `holdIndent` leaves on every row (デザイン規約 §長押し).
    HoldIcon {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        progress: menuItem.holdProgress
        tint: menuItem.wordColor
        visible: menuItem.armedMs > 0
    }
    // The `!` of a row that asks, sharing the hold mark's seat — a row is one or the other (`asks`). Ahead of the word,
    // so it is the seat's icon (デザイン規約 §git 用語のコード表記「`!` の席」): the step a menu's icons take, centred
    // where the hold mark is — at the mark's step a stroke this thin reads as punctuation.
    NavIcon {
        x: (Theme.iconSm - width) / 2
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        kind: "bang"
        tint: Theme.warning
        width: Theme.iconMd
        height: Theme.iconMd
        // Only on a row that can be pressed; a blocked row says `blockedWhy` instead.
        visible: menuItem.asks && menuItem.armedMs <= 0 && !menuItem.blocked
    }
    // A heading's mark. Its ink, not its box, stands at `markX`: the box's air hangs either side into the card's
    // padding and the word's gap, which would otherwise be spent twice.
    NavIcon {
        id: rowMark
        x: menuItem.markX - (rowMark.width - rowMark.inkWidth) / 2
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        width: Theme.iconMd
        height: Theme.iconMd
        visible: menuItem.heads && (menuItem.titled || !menuItem.seated)
        kind: menuItem.heads ? menuItem.markKind : "branch"
        tint: menuItem.markTint
    }
    // A mark about the row, in the left menu's seat and drawn the way that seat draws it (`NameCell.seatNudge`):
    // `iconSm`, its box hanging past the seat into the step after it. `iconMd` would stand taller than the left menu's.
    NavIcon {
        id: seatMark
        x: menuItem.seatX
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        width: Theme.iconSm
        height: Theme.iconSm
        visible: menuItem.heads && !menuItem.titled && menuItem.seated
        kind: menuItem.heads ? menuItem.markKind : "branch"
        tint: !menuItem.enabled || menuItem.blocked ? Theme.textMuted : menuItem.markTint
    }

    contentItem: RowLayout {
        // Steps are written on the items: the marked name takes a word's step (`markWordGap`), not the `spaceSm`
        // between two things the row says, and a shared spacing could not tell them apart.
        spacing: 0
        // The chip takes the menu's shared chip column (never less than its glyphs), so every row's words start on one
        // x; its tint hangs outside the glyphs into the padding and the gap (デザイン規約 §git 用語のコード表記).
        Item {
            id: codeChip
            visible: menuItem.code !== ""
            // An invisible item leaves the layout margin and all, so the step on it goes with it.
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
                // Height off the glyph size, not the label: the mono line box differs per OS and stands taller than
                // the UI-family sentence (デザイン規約 §git 用語のコード表記).
                anchors.verticalCenter: codeLabel.verticalCenter
                height: codeLabel.font.pixelSize + Theme.spaceXs / 2
                // Half a gap, no more: a chip after the hold mark would otherwise reach back against it and the two
                // would read as one run of ink.
                anchors.leftMargin: -Theme.spaceXs / 2
                anchors.rightMargin: -Theme.spaceXs / 2
                radius: Theme.radiusSm
                color: Theme.bgHover
            }
            Label {
                id: codeLabel
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: menuItem.code
                font.family: Theme.monoFamily
                // A mono space would split `push --delete` into two words on one ground (デザイン規約 §git 用語のコード表記).
                font.wordSpacing: -Theme.spaceXs
                font.pixelSize: menuItem.font.pixelSize
                color: menuItem.wordColor
            }
        }
        Label {
            id: itemLabel
            // The width goes to the marked name where there is one; these words are the fixed part before it.
            Layout.fillWidth: !menuItem.namesMark
            // `text` stays the plain sentence either way: the tooltip and accessibility read that.
            text: menuItem.refWords !== "" ? menuItem.refWords : menuItem.text
            // Written out: a group `font:` assignment and `font.weight` on one Label fail to load ("already assigned").
            font.family: Theme.uiFamily
            font.pixelSize: menuItem.font.pixelSize
            // Pinned: rows carry text nobody here chose (a branch called `<b>`), and AutoText guesses at markup. The
            // one markup line is `refWords`, escaped on the way in.
            textFormat: menuItem.refWords !== "" ? Text.StyledText : Text.PlainText
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            // Clear of the card's arrow (`arrow`) at the right edge (the note, when there is one, sits last instead).
            rightPadding: !noteLabel.visible && menuItem.subMenu && menuItem.arrow
                          ? menuItem.arrow.width + Theme.spaceXs : 0
            // A heading takes the sidebar section band's weight, not its ink: a row that opens a card to its right is
            // never drawn darker than the rows beside it (デザイン規約 §メニュー の入れ子).
            font.weight: menuItem.titled ? Theme.fontWeightStrong : menuItem.font.weight
            color: menuItem.wordColor
        }
        // `sideName`, pushed to the right end by the words taking the rest, a column's step off them: at a word's
        // step the two read as one run.
        Label {
            id: sideLabel
            visible: menuItem.sideName !== ""
            Layout.leftMargin: Theme.spaceLg
            Layout.alignment: Qt.AlignVCenter
            text: menuItem.sideName
            textFormat: Text.PlainText
            font.family: Theme.uiFamily
            font.pixelSize: Theme.fontSm
            color: !menuItem.enabled || menuItem.blocked ? menuItem.wordColor : Theme.textSecondary
        }
        // Ahead / behind, in the left menu's seat and order (`NavRowBody`'s `trackSeat`).
        HeadTrack {
            id: rowTrack
            visible: menuItem.ahead > 0 || menuItem.behind > 0
            Layout.leftMargin: Theme.spaceSm
            Layout.alignment: Qt.AlignVCenter
            ahead: menuItem.ahead
            behind: menuItem.behind
        }
        // …and the badge after it, as the left menu's row ends (`NavRowBody`'s `remoteSeat`).
        GoneBadge {
            visible: menuItem.badged
            Layout.leftMargin: Theme.spaceXs
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: Theme.iconSm
            Layout.preferredHeight: Theme.iconSm
            pullRequest: menuItem.badgePr
            gone: menuItem.badgeGone
        }
        // `nameMark` + `markName`, the mark set as a letter of the word (`markWordGap`).
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
                    // The 16-grid scales with the seat and the line does not; unscaled, the mark outweighs the letters
                    // beside it (`NavIcon.stroke`).
                    stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                    // `NavIcon` paints on a kind it is given; an empty one would be a shape nobody asked for.
                    kind: menuItem.namesMark ? menuItem.nameMark : "tree"
                    tint: menuItem.blocked || !menuItem.enabled || menuItem.armedMs > 0
                          ? menuItem.wordColor : menuItem.nameMarkTint
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
        // `trail`: the tree mark and folder as one run, the way the tab draws it (`TabTreeMark` / `OpsPicker.trail`).
        NavIcon {
            visible: menuItem.trail !== ""
            Layout.leftMargin: Theme.spaceXs
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: Theme.iconXs
            Layout.preferredHeight: Theme.iconXs
            kind: "tree"
            stroke: Metrics.iconStroke * Theme.iconXs / Theme.iconMd
            tint: Theme.textSecondary
        }
        Label {
            id: trailLabel
            visible: menuItem.trail !== ""
            text: menuItem.trail
            verticalAlignment: Text.AlignVCenter
            color: Theme.textSecondary
            font.family: Theme.uiFamily
            font.pixelSize: Theme.fontSm
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

    // A row that opens a card ends in `>`: `chevron` as drawn, the mark `AppCombo` turns down for its list, on the
    // left menu's row grid (デザイン規約 §メニュー の入れ子). Its ink ends where the right-set words of other rows end
    // (a note, a track): on the right padding. The item is the room the words of such a row keep clear, `iconLg`
    // wide — where that row is a card's widest, the card's width is measured off it. The mark is built only on the
    // rows that wear it (rules-refs の「行のデリゲートが見せない部品は消す」): a card builds a row per branch.
    arrow: Item {
        x: menuItem.width - width - menuItem.rightPadding
        y: (menuItem.height - height) / 2
        width: Theme.iconLg
        height: Theme.iconSm
        Loader {
            id: arrowSeat
            active: menuItem.subMenu !== null
            x: parent.width - (arrowSeat.item ? arrowSeat.item.inkRight : 0)
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: Metrics.opticalDrop
            sourceComponent: NavIcon {
                width: Theme.iconSm
                height: Theme.iconSm
                kind: "chevron"
                tint: !menuItem.enabled || menuItem.blocked ? Theme.textMuted : Theme.textSecondary
            }
        }
    }

    background: Rectangle {
        radius: Theme.radiusSm
        // Every row hovers to the same wash, held rows included (デザイン規約 §メニュー).
        color: menuItem.highlighted ? Theme.bgHover : "transparent"
        HoldFill {
            progress: menuItem.holdProgress
            tone: menuItem.holdTone
        }
        // Under the hand, a heading is underlined in its mark's colour across the row: what is about to open
        // (デザイン規約 §メニュー の入れ子). Headings only — a mark about the row names no card.
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: Theme.borderWidth
            visible: menuItem.titled && (menuItem.hovered || menuItem.highlighted)
            color: menuItem.markTint
        }
    }

    HoldDriver {
        id: holdDrive
        holdMs: menuItem.holdMs
        onFinished: menuItem.held()
    }
    // Takes the press before the MenuItem can emit `triggered`, which would run the row and close the menu under a
    // hold. It accepts no hover, so the row still highlights like any other.
    MouseArea {
        id: rowPress
        anchors.fill: parent
        // A blocked row takes the press and drops it, or the row underneath would run and close the menu. `armedMs`,
        // not the live length: disabling the area mid-gesture would lose the release and fire a hold nobody is making.
        enabled: menuItem.blocked || ((holdDrive.armedMs > 0 || menuItem.staysOpen) && menuItem.enabled)
        onPressed: if (!menuItem.blocked) holdDrive.begin()
        // Letting go or dragging off calls the hold off; on a stays-open row, letting go on the row is its click.
        onReleased: mouse => {
            // Read before `letUp()` opens the latch; a press whose premise has gone (`HoldDriver.stale`) is neither
            // the click nor the hold.
            const pick = !menuItem.blocked && holdDrive.armedMs <= 0 && !holdDrive.stale && menuItem.staysOpen
                    && mouse.x >= 0 && mouse.y >= 0 && mouse.x <= width && mouse.y <= height
            holdDrive.letUp()
            if (pick)
                menuItem.picked()
        }
        onCanceled: holdDrive.letUp()
        onPositionChanged: if (!containsMouse) holdDrive.letUp()
    }
    // Keyboard: hold Space or Enter on the row (デザイン規約 §長押し). Accepting the key keeps the menu from
    // triggering the row outright.
    Keys.onPressed: event => {
        // A blocked row takes the key that would run it and drops it.
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

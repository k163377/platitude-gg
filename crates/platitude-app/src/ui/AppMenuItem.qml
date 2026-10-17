import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One AppMenu row: as wide as its own words, eliding only where the menu has run out of window (デザイン規約 §メニュー).
MenuItem {
    id: menuItem

    /// Whether this row is on offer — false where it cannot be chosen, and then the row goes (デザイン規約 §メニュー).
    /// Kept apart from `visible`, which `present` drives: a closed menu's list turns its rows' `visible` off behind the
    /// binding, and the menu and its dividers read these before it opens.
    property bool offered: true
    /// Set while the menu keeps its shape (`AppMenu.holdShape`): the row stays as it was then — on show as
    /// `shownAtHold` says — whatever `offered` turns to. A row gone from under the hand would put another in its place,
    /// a row come would push the rest down.
    property bool shapeHeld: false
    property bool shownAtHold: false
    /// On show. What the menu lays its rows out and measures by.
    readonly property bool present: menuItem.shapeHeld ? menuItem.shownAtHold : menuItem.offered
    /// On show though no longer on offer — the row itself, or the whole card it is on (`AppMenu.applies`: a title row
    /// greyed for it still opens the card): greyed, saying the menu has to be raised again (デザイン規約 §メニュー).
    readonly property bool withdrawn:
        menuItem.shapeHeld && menuItem.shownAtHold
        && (!menuItem.offered || menuItem.menu !== null && menuItem.menu.applies === false)
    visible: menuItem.present

    /// A short warning tag after the row's words ("already pushed"); the row still runs on click.
    property string note: ""
    /// That tag's colour: the warning, grey with the words on a row that cannot be chosen — it is one of the row's
    /// words, and a lit tag in a grey row reads as the part that still works (デザイン規約 §無効).
    readonly property color noteColor: menuItem.greyed ? Theme.textMuted : Theme.warning

    /// A second name after the row's own — the worktree a tab is in, drawn the way the tab draws it
    /// (デザイン規約 §タブの所作). Empty draws none.
    property string trail: ""

    /// A git term in git's own spelling, on a chip (デザイン規約 §git 用語のコード表記). It sits ahead of `text`, which
    /// carries whatever of the sentence is left ("this file"), often nothing.
    property string code: ""
    /// A ref name inside the row's words, and the sentence with the name's seat left in it ("into %1"). The name wears
    /// its kind's colour (`refTint`): `textLink` for the current branch, which is what these sentences mostly name
    /// (デザイン規約 §ref の種別「現在のブランチは、名前が出る場所すべてで textLink」).
    /// The seat is cut out of the sentence, not searched for in `text`: a branch `it` would be found in `into it`.
    property string refSentence: ""
    property string refName: ""
    /// That name's own colour — `textPrimary` for a local branch the worktree is not on (`pull` into it).
    property color refTint: Theme.textLink
    text: menuItem.refSentence !== "" ? menuItem.refSentence.arg(menuItem.refName) : ""
    /// The mark a row wears, and its colour. On a row that opens a card it is the `NavIcon` kind of what the card acts
    /// on (デザイン規約 §メニュー の入れ子), and the row becomes a heading: the sidebar's section band in a menu, its
    /// word out of the chip column. A row with `headed: false` wears a mark about itself instead.
    property string markKind: ""
    property color markTint: Theme.textSecondary
    readonly property bool heads: menuItem.markKind !== ""
    /// Whether that mark makes this row a heading. False for a mark about the row itself (a worktree's padlock, the
    /// house, the tree — the marks `NavRowBody.seatMark` gives): dressed as a heading it reads as a title with nothing
    /// under.
    property bool headed: true
    readonly property bool titled: menuItem.heads && menuItem.headed
    /// Whether this row's menu keeps the left menu's seat on every row (`AppMenu.seatWorn`), so every name on the card
    /// begins on one x as in `NameCell`; a mark about the row stands in it, drawn as the head of the row's words
    /// (`set: "lead"`).
    readonly property bool seated:
        menuItem.menu !== null && menuItem.menu.seatWorn !== undefined && menuItem.menu.seatWorn === true
    readonly property bool wearsSeat: menuItem.heads && !menuItem.headed
    readonly property real seatX: menuItem.padding + menuItem.holdIndent
    /// The room a heading's mark takes, zero on any other row: read by `AppMenu.headingSeat` to line a card's headings
    /// up.
    readonly property real headingRoom: menuItem.titled ? menuItem.headSeat : 0
    /// How wide a heading's mark seat is: as wide as the widest heading mark's ink (11 squares of its grid and its
    /// line), so the words start one step past any of them.
    readonly property real headSeat: Math.ceil(rowMark.unit * 11 + rowMark.pen)
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

    /// A name ending the row's words, with its kind's mark against it — a worktree's folder behind the WORKTREES
    /// mark (`RefRowMenu`'s `Open`). It always ends them: a mark is drawn, not written, so it cannot take a sentence's
    /// seat the way a coloured name can (`refWords`).
    property string nameMark: ""
    property string markName: ""
    property color nameMarkTint: Theme.textSecondary
    readonly property bool namesMark: menuItem.nameMark !== "" && menuItem.markName !== ""
    /// Automation: whether that name lost its tail — a picture a pixel short and one whole read alike at a glance.
    readonly property bool nameCut: menuItem.namesMark && markNameLabel.truncated
    /// A fact about the row in the left menu's far column — the branch a worktree has out
    /// (`NavRowBody.branchSeat`). Empty draws none.
    property string sideName: ""
    /// The step in front of `nameMark`: a word's. The mark is a sign on its name, so nothing stands between
    /// it and the name but its cell's own air (デザイン規約 §アイコンの席 の修飾).
    readonly property real markWordGap: Theme.spaceXs
    /// Where the row's words stand, in the row's y: every mark ahead of them takes their line for its box
    /// (デザイン規約 §アイコンの席 の文頭記号). Read off the label, since the layout and an anchor round a half pixel
    /// apart.
    readonly property real wordsTop: menuItem.contentItem ? menuItem.contentItem.y + itemLabel.y : 0

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
    /// Whether pressing this row starts git — itself, or through the box or bar it opens — which is what the menu's
    /// held doors and a command already running hold back (`AppMenu.rowsHeldWhy`). A row that only reads what is on
    /// screen (`Copy path`) is never held.
    property bool startsCommand: true
    /// The same, said once for the whole menu (`AppMenu.rowsHeldWhy`). A card's title row skips it: the rows inside
    /// carry the line (the card holds and waits on its own).
    readonly property string menuHeldReason:
        menuItem.subMenu || !menuItem.startsCommand ? ""
        : menuItem.menu !== null && menuItem.menu.rowsHeldWhy !== undefined ? menuItem.menu.rowsHeldWhy : ""
    /// What a row that stopped applying while its menu stood says (`withdrawn`).
    readonly property string withdrawnWhy: qsTr("The command that ran changed this — open the menu again")
    /// A row out of offer says only that; else the row's own reason wins over the menu's: it is about the ref the row
    /// names, the more particular line.
    readonly property string blockedWhy:
        menuItem.withdrawn ? menuItem.withdrawnWhy
        : menuItem.blockedReason !== "" ? menuItem.blockedReason : menuItem.menuHeldReason
    readonly property bool blocked: menuItem.blockedWhy !== ""
    /// The row cannot be chosen — turned off, or `blocked`: its words grey to the last letter (デザイン規約 §無効).
    readonly property bool greyed: !menuItem.enabled || menuItem.blocked
    /// A worktree's folder inside the row's own reason (`blockedReason`); empty for none.
    property string reasonMarkWord: ""
    /// The mark that folder wears: the house for the repository's own worktree, else the tree.
    property string reasonMarkKind: "tree"
    /// The word the shared tooltip stands the worktree's mark against (`SharedToolTip.tipMarkWord`, デザイン規約
    /// §ref の種別「ツールチップだけは字の中」): the folder in the reason, or the folder this row names behind its mark,
    /// which the hover gives back whole when it is cut.
    readonly property string tipMarkWord: menuItem.blocked ? menuItem.reasonMarkWord
                                        : menuItem.nameMark === "tree" || menuItem.nameMark === "home"
                                          ? menuItem.markName : ""
    /// …and which mark (`SharedToolTip.tipMarkKind`).
    readonly property string tipMarkKind: menuItem.blocked ? menuItem.reasonMarkKind : menuItem.nameMark

    /// Automation: show the tooltip with no pointer behind it, through the binding the hover drives
    /// (verify-ui §hover の絵の撮り方).
    property bool tipForced: false

    /// Pressed this many times, for a row that would otherwise have to raise a question of its own (デザイン規約
    /// §回数押し). Zero is an ordinary row. Such a row reports no click (`rowPress`).
    property int holdPresses: 0
    /// Pressing this row raises a question: a `!` in the hold mark's seat (デザイン規約 §進行中の操作から出る). A row
    /// asks or holds, never both — a row that asks is confirmed on the bar it raises.
    property bool asks: false
    /// How far into the presses the row has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// The count the gesture under way was given (`HoldDriver.armedPresses`; the live one between gestures). Anything
    /// the row dresses from the same answer reads this, so it cannot change between presses
    /// (rules-refs の「回数押しか否かは最初の押下で確定する」の行).
    readonly property alias armedPresses: holdDrive.armedPresses
    /// A gesture is under way, the fill's slide back included (`HoldDriver.gesturing`) — the latch for a row dressed
    /// from an answer other than the count (`RefBranchMenu`'s `Delete both`).
    readonly property alias gesturing: holdDrive.gesturing
    /// What a confirmed row wears — word, chip, mark and fill. `warning`: what the row throws away is brought back from
    /// the discard record. **A row that cannot be brought back names `danger` itself**, and a plain delete
    /// `textPrimary` (デザイン規約 §回数押し の色の表) — so a new row never takes the stronger warning by leaving this out.
    property color holdTone: Theme.warning
    /// [`closeMenu`] asked under the floor: the menu goes when it has passed.
    property bool closesAfterFloor: false
    /// Pressed through: the handler runs the row now, at the last press, while what the presses were for still holds.
    /// The menu stays for the floor — the handler closes it with [`closeMenu`], never the menu's own `dismiss()`.
    signal held()
    /// Closes the menu this row is in, every level of it (Qt's `dismiss()`): at once, or — under a row just pressed
    /// through — once its floor has passed, so the full fill is seen (デザイン規約 §回数押し). A menu closed some other
    /// way meanwhile is not closed again: what opens next is not this row's. Where only this row's level went (the
    /// hand moving onto another row of the menu above), the rest stays: the hand is in it.
    function closeMenu() {
        if (menuItem.menu === null)
            return
        if (holdDrive.settling) {
            menuItem.closesAfterFloor = true
            return
        }
        menuItem.menu.dismiss()
    }
    /// Automation: every press at once without a hand behind them — on a row a hand could press. Answers whether
    /// they went in, so a run that met the row out of reach waits rather than counting presses nothing landed.
    function completeHold() {
        if (!menuItem.present || menuItem.greyed)
            return false
        holdDrive.runOut()
        return true
    }
    // Blocked under a hand: the hold under way blanks rather than running out to fire, as a row blocked from the start
    // drops the press (`rowPress`).
    onBlockedChanged: if (menuItem.blocked) holdDrive.blank()
    readonly property bool holding: menuItem.holdProgress > 0

    /// The hold mark's seat, the same on every row of the menu (`AppMenu.holdIndent`).
    readonly property real holdIndent:
        menuItem.menu !== null && menuItem.menu.holdIndent !== undefined ? menuItem.menu.holdIndent : 0

    // The row's air either side of its words: the step the left menu keeps between a mark and its name (デザイン規約
    // §メニュー). The card's own `spaceXs` outside it only keeps a highlight off the rounded corners.
    padding: Theme.spaceXs

    /// Where a heading's mark box stands: its ink then starts where the other rows start their words (the row's
    /// padding), so the card's left edge reads as one column (デザイン規約 §メニュー) — or, where the menu has a hold /
    /// `!` seat, the box is centred on that seat as the `!` is.
    readonly property real markX: menuItem.holdIndent > 0 ? (menuItem.holdIndent - Theme.iconMd) / 2
                                  : menuItem.padding - Math.round((Theme.iconMd - rowMark.inkAcross) / 2)

    // On a card that keeps the heading seat, every row's words start past it, marked or not (`NameCell`'s spacing).
    leftPadding: menuItem.headingSeat > 0 ? menuItem.padding + menuItem.headingSeat
               : menuItem.seated && !menuItem.titled ? menuItem.seatX + Theme.iconXs + Theme.spaceXs
               : menuItem.heads
                 ? (menuItem.holdIndent > 0 ? menuItem.padding + menuItem.holdIndent
                    : menuItem.padding + menuItem.headSeat + Theme.spaceXs)
               : menuItem.padding + menuItem.holdIndent
    topPadding: 0
    bottomPadding: 0
    // A row this menu is not showing takes no room: the list lays rows out by height, so an invisible row that keeps
    // one leaves a hole.
    implicitHeight: menuItem.present ? Theme.rowHeight : 0
    // The chip and the marked name are bid in whole pixels, as the layout hands them out: the row's `RowLayout` rounds
    // each of those up, and a sum rounded once comes up the fractions short — the marked name, laid out last, elides
    // (a Linux font's `worktree remove` + folder).
    implicitWidth: (codeChip.visible ? Math.ceil(codeChip.implicitWidth) + Theme.spaceSm : 0)
                   + (menuItem.growsForText
                      ? itemLabel.implicitWidth : Math.min(itemLabel.implicitWidth, Metrics.labelColW))
                   // The marked name is data: it bids at most `labelColW` and elides past it (デザイン規約 §メニュー).
                   + (menuItem.namesMark
                      ? menuItem.markWordGap
                        + Math.ceil(nameMarkIcon.width + Math.min(markNameLabel.implicitWidth, Metrics.labelColW)) : 0)
                   + (menuItem.note !== "" ? noteLabel.implicitWidth + Theme.spaceSm : 0)
                   // Everything drawn after the words bids too, or the row that sets the menu's width cuts its own
                   // name. Asked of what the row holds, not `visible`: a closed card's rows answer false, and the
                   // card is measured before it opens.
                   + (menuItem.sideName !== "" ? sideLabel.implicitWidth + Theme.spaceLg : 0)
                   + (menuItem.ahead > 0 || menuItem.behind > 0 ? rowTrack.implicitWidth + Theme.spaceSm : 0)
                   + (menuItem.badged ? Theme.iconSm + Theme.spaceXs : 0)
                   + trailRun.naturalWidth
                   + menuItem.leftPadding + menuItem.rightPadding
    font.pixelSize: Theme.fontMd
    Accessible.description: menuItem.holdPresses > 0 ? Words.pressToActivate(menuItem.holdPresses) : ""

    // The one colour the row's chip and sentence follow, so the chip cannot disagree with its sentence. A held row wears
    // its tone before it is touched at all (デザイン規約 §状態).
    readonly property color wordColor: menuItem.greyed ? Theme.textMuted
                                     : menuItem.holding ? Theme.textOnAccent
                                     : menuItem.armedPresses > 0 ? menuItem.holdTone : Theme.textPrimary
    /// The row cannot be pressed: the grey words say it is off, and the press mark keeps the tone it wears when the row
    /// can be pressed, a step down — what pressing it would cost (デザイン規約 §回数押しの色). Not over a fill: there the
    /// mark lifts with the words, and a step down would sink into the fill's own dimmed step.
    readonly property bool holdOff: menuItem.greyed && !menuItem.holding
    /// The press mark's colour: the words', or while [`holdOff`] the row's tone (`HoldIcon.off` steps it down).
    readonly property color holdInk: menuItem.holdOff ? menuItem.holdTone : menuItem.wordColor
    /// Whether the row's colour takes the names in it and their marks too: greyed, under the fill, and on a held row
    /// that warns. **A plain delete's words keep the ordinary rules** — its warning is the press mark's alone
    /// (デザイン規約 §回数押しの色), so a name there wears its own colour.
    readonly property bool inkTakesNames: menuItem.greyed || menuItem.holding
                                          || (menuItem.armedPresses > 0
                                              && !Qt.colorEqual(menuItem.holdTone, Theme.textPrimary))
    /// The colour the name inside those words takes: its own only while the row has no colour of its own
    /// (デザイン規約 §メニュー「文の中に入った名前も、色は名前のもの」).
    readonly property color refColor: menuItem.inkTakesNames ? menuItem.wordColor : menuItem.refTint
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

    // The press mark, in the seat `holdIndent` leaves on every row (デザイン規約 §回数押し): the head of the row's words,
    // as tall as their capitals and on their middle (デザイン規約 §アイコンの席 の文頭記号).
    HoldIcon {
        id: rowHold
        // Centred on the seat, where the `!` and a heading's mark stand.
        x: Math.round((menuItem.holdIndent - rowHold.width) / 2)
        y: Math.round(menuItem.wordsTop + itemLabel.baselineOffset - WordFace.capitalMiddle(rowHold.size)
                      - rowHold.height / 2)
        size: itemLabel.font.pixelSize
        progress: menuItem.holdProgress
        presses: menuItem.armedPresses
        tint: menuItem.holdInk
        off: menuItem.holdOff
        visible: menuItem.armedPresses > 0
    }
    // The `!` of a row that asks, sharing the hold mark's seat — a row is one or the other (`asks`). Ahead of the words
    // it heads them (デザイン規約 §アイコンの席 の文頭記号): drawn to their letters and centred on their capitals'
    // middle, its box their line (`wordsTop`).
    NavIcon {
        x: 0
        y: menuItem.wordsTop
        kind: "bang"
        set: "lead"
        size: itemLabel.font.pixelSize
        wordBase: itemLabel.baselineOffset
        tint: Theme.warning
        width: Theme.iconSm
        height: itemLabel.height
        // Only on a row that can be pressed; a blocked row says `blockedWhy` instead.
        visible: menuItem.asks && menuItem.armedPresses <= 0 && !menuItem.blocked
    }
    // A heading's mark, its box at `markX`: a step over the heading's capitals.
    NavIcon {
        id: rowMark
        x: menuItem.markX
        y: menuItem.wordsTop
        set: "head"
        size: itemLabel.font.pixelSize
        wordBase: itemLabel.baselineOffset
        width: Theme.iconMd
        height: itemLabel.height
        visible: menuItem.heads && (menuItem.titled || !menuItem.seated)
        kind: menuItem.heads ? menuItem.markKind : "branch"
        tint: menuItem.markTint
    }
    // A mark about the row, in the left menu's seat and drawn the way that seat draws it (`NameCell`), centred on the
    // seat.
    NavIcon {
        id: seatMark
        x: menuItem.seatX - (Theme.iconSm - Theme.iconXs) / 2
        y: menuItem.wordsTop
        set: "lead"
        size: itemLabel.font.pixelSize
        wordBase: itemLabel.baselineOffset
        width: Theme.iconSm
        height: itemLabel.height
        visible: menuItem.heads && !menuItem.titled && menuItem.seated
        kind: menuItem.heads ? menuItem.markKind : "branch"
        tint: menuItem.greyed ? Theme.textMuted : menuItem.markTint
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
            color: menuItem.greyed ? menuItem.wordColor : Theme.textSecondary
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
            id: rowBadge
            size: itemLabel.font.pixelSize
            wordBase: itemLabel.y + itemLabel.baselineOffset - rowBadge.y
            visible: menuItem.badged
            Layout.leftMargin: Theme.spaceXs
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: Theme.iconSm
            Layout.preferredHeight: Theme.iconSm
            pullRequest: menuItem.badgePr
            gone: menuItem.badgeGone
        }
        // `nameMark` + `markName`, the mark a sign on the name (`markWordGap`).
        RowLayout {
            visible: menuItem.namesMark
            Layout.fillWidth: menuItem.namesMark
            Layout.leftMargin: menuItem.namesMark ? menuItem.markWordGap : 0
            spacing: 0
            // A sign on the name, its box the name's line (デザイン規約 §アイコンの席 の修飾).
            NavIcon {
                id: nameMarkIcon
                Layout.preferredWidth: nameMarkIcon.wordCell
                Layout.preferredHeight: markNameLabel.implicitHeight
                width: nameMarkIcon.wordCell
                height: markNameLabel.implicitHeight
                set: "sign"
                size: markNameLabel.font.pixelSize
                wordBase: markNameLabel.baselineOffset
                // `NavIcon` paints on a kind it is given; an empty one would be a shape nobody asked for.
                kind: menuItem.namesMark ? menuItem.nameMark : "tree"
                tint: menuItem.inkTakesNames ? menuItem.wordColor : menuItem.nameMarkTint
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
        // `trail`: the tree mark and folder as one run, the tab's own (`TabTreeMark`) — the mark a sign on the
        // folder's name (デザイン規約 §アイコンの席 の修飾).
        TabTreeMark {
            id: trailRun
            visible: menuItem.trail !== ""
            Layout.preferredWidth: trailRun.naturalWidth
            Layout.preferredHeight: trailRun.implicitHeight
            name: menuItem.trail
            gap: Theme.spaceXs
        }
        Label {
            id: noteLabel
            visible: menuItem.note !== ""
            // The row's own step, for the same reason the chip's is on the chip.
            Layout.leftMargin: Theme.spaceSm
            text: menuItem.note
            verticalAlignment: Text.AlignVCenter
            rightPadding: menuItem.subMenu && menuItem.arrow ? menuItem.arrow.width + Theme.spaceXs : 0
            color: menuItem.noteColor
            font.pixelSize: Theme.fontSm
        }
    }

    // A row that opens a card ends in `>`: `chevron` as drawn, the mark `AppCombo` turns down for its list, on the
    // left menu's row grid (デザイン規約 §メニュー の入れ子). Its ink ends where the right-set words of other rows end
    // (a note, a track): on the right padding. The item is the mark's ink and nothing more, so the words of such a row
    // stop a word's step short of the ink, as a heading's start one past its mark's (デザイン規約 §余白「印が自分で
    // 持っている余白は、隣の詰めに数える」) — where that row is a card's widest, the card's width is measured off it.
    // The mark is built only on the rows that wear it (rules-refs の「行のデリゲートが見せない部品は消す」): a card builds a
    // row per branch.
    arrow: Item {
        x: menuItem.width - width - menuItem.rightPadding
        y: (menuItem.height - height) / 2
        width: arrowSeat.item ? arrowSeat.item.inkWidth : 0
        height: Theme.iconSm
        Loader {
            id: arrowSeat
            active: menuItem.subMenu !== null
            x: parent.width - (arrowSeat.item ? arrowSeat.item.inkRight : 0)
            anchors.verticalCenter: parent.verticalCenter
            sourceComponent: NavIcon {
                width: Theme.iconSm
                height: Theme.iconSm
                kind: "chevron"
                tint: menuItem.greyed ? Theme.textMuted : Theme.textSecondary
            }
        }
    }

    background: Rectangle {
        radius: Theme.radiusSm
        // Every row hovers to the same wash, held rows included (デザイン規約 §メニュー).
        color: menuItem.highlighted ? Theme.bgHover : "transparent"
        // Out over the card's side padding to its frame, so the mark at the row's edge has as much fill on its left as
        // its word's step gives it on its right (デザイン規約 §回数押し).
        HoldFill {
            progress: menuItem.holdProgress
            tone: menuItem.holdTone
            dimmed: holdDrive.dimmed
            bleed: menuItem.menu !== null ? menuItem.menu.leftPadding - Theme.borderWidth : 0
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

    // The row runs at the last press; only the menu's going waits out the floor (`closeMenu`).
    HoldDriver {
        id: holdDrive
        holdPresses: menuItem.holdPresses
        onFinished: menuItem.held()
        onLanded: {
            if (!menuItem.closesAfterFloor)
                return
            menuItem.closesAfterFloor = false
            menuItem.menu.dismiss()
        }
    }
    Connections {
        target: menuItem.menu
        // Whoever closed the menu, the row's completion went with it.
        function onAboutToHide() {
            menuItem.closesAfterFloor = false
            holdDrive.settle()
        }
    }
    // The hand left the row part way through its presses: back to zero (デザイン規約 §回数押し).
    onHoveredChanged: if (!menuItem.hovered) holdDrive.leave()
    // Takes the press before the MenuItem can emit `triggered`, which would run the row and close the menu on the
    // first press. It accepts no hover, so the row still highlights like any other.
    MouseArea {
        id: rowPress
        anchors.fill: parent
        // A blocked row takes the press and drops it, or the row underneath would run and close the menu.
        // `armedPresses`, not the live count: disabling the area mid-gesture would lose the release.
        enabled: menuItem.blocked || (holdDrive.armedPresses > 0 && menuItem.enabled)
        onPressed: if (!menuItem.blocked) holdDrive.begin()
        onReleased: holdDrive.letUp()
        onCanceled: holdDrive.letUp()
        // Dragged off the row: as the hand leaving it.
        onPositionChanged: if (!containsMouse) holdDrive.leave()
    }
    // Keyboard: press Space or Enter on the row, as many times (デザイン規約 §回数押し). Accepting the key keeps the
    // menu from triggering the row outright.
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

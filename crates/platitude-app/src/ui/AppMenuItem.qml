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
    /// The hand on this row, by a handler of its own (`AppMenu.pointerInside`): the row's `hovered` drops the moment
    /// the card goes (`QQuickControl::itemChange`), before Qt hands the hover to what is under the hand; a handler
    /// keeps it until Qt takes it.
    readonly property bool handOn: rowHand.hovered

    /// A short warning tag after the row's words ("already pushed"); the row still runs on click.
    property string note: ""
    /// That tag's colour: the warning, and the words' colour where they have one of their own — grey on a row that
    /// cannot be chosen (a lit tag in a grey row reads as the part that still works, デザイン規約 §無効), lifted over
    /// the fill (in the warning it would sink into it).
    readonly property color noteColor: menuItem.greyed ? Theme.textMuted
                                     : menuItem.holding ? Theme.textOnAccent : Theme.warning

    /// A second name after the row's own — the worktree a tab is in, drawn the way the tab draws it
    /// (デザイン規約 §タブの所作). Empty draws none.
    property string trail: ""

    /// A git term in git's own spelling, on a chip (デザイン規約 §git 用語のコード表記). It sits ahead of `text`, which
    /// carries whatever of the sentence is left ("this file"), often nothing.
    property string code: ""
    /// A ref name inside the row's words, and the sentence with the name's seat left in it ("into %1"). The name is set
    /// as code (デザイン規約 §ref の種別「文の中の名前」), in its own colour where it says where the reader is
    /// (`refTint`). The seat is cut out of the sentence, not searched for in `text`: a branch `it` would be found in
    /// `into it`.
    property string refSentence: ""
    property string refName: ""
    /// That name's own colour: `textLink` for the current branch, which is what these sentences mostly name
    /// (デザイン規約 §ref の種別「現在のブランチは、名前が出る場所すべてで textLink」); `textPrimary` for any other.
    property color refTint: Theme.textLink
    /// The mark that name wears in front of it — its kind's `NavIcon` (デザイン規約 §ref の種別「名前の印」), a sign on
    /// the name. Empty for none.
    property string refMark: ""
    /// The room the markup opens for that mark, the seat (`seatRoom`): the sentence's own space before the name stays
    /// the gap (as `CardText.marks`).
    readonly property string refRoom1: menuItem.refMark === "" || menuItem.refSentence.indexOf("%1") < 0 ? ""
                                       : menuItem.seatRoom
    /// The seat in the row's words: as many of the row's own spaces as hold the widest mark (`MarkSeat`). The size is
    /// the row's own, not read back off the ruler's font: on Linux that read loops the binding on every row.
    readonly property string seatRoom: MarkSeat.room(refSpace.advanceWidth, menuItem.font.pixelSize)
    /// A room's width as the row lays it out — one width whatever the mark, so a card's names start on one x, a name
    /// ending a row too (`codeNameSeat`). The mark stands in its middle, so a card's marks share one upright axis.
    readonly property real refCell: menuItem.seatRoom.length * refSpace.advanceWidth
    readonly property int refMarkRoom: menuItem.refRoom1.length
    /// A second name in the same sentence, at its `%2` seat ("%1 from %2": a tag and the remote it goes from), with
    /// its colour and mark as the first's. Empty for a sentence with one name.
    property string refName2: ""
    property color refTint2: Theme.textPrimary
    property string refMark2: ""
    readonly property string refRoom2: menuItem.refMark2 === "" || menuItem.refSentence.indexOf("%2") < 0 ? ""
                                       : menuItem.seatRoom
    readonly property int refMarkRoom2: menuItem.refRoom2.length
    text: menuItem.refSentence === "" ? ""
        : menuItem.refSentence.indexOf("%2") < 0 ? menuItem.refSentence.arg(menuItem.refName)
        : menuItem.refSentence.arg(menuItem.refName).arg(menuItem.refName2)
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
    /// How wide a heading's mark seat is: as wide as the tag's ink, its mitred corners included — the widest of the
    /// cards' heading marks but GitHub's cloud, which runs past it — so the words start one step past the others.
    readonly property real headSeat: Math.ceil(rowMark.inkAcrossOf("tag"))
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

    /// A name ending the row's words, as code behind its kind's mark — a worktree's folder (`RefRowMenu`'s `Open`), the
    /// branch a delete row deletes — or bare where the card's title wears the mark (`TopBar`'s `WORKTREE` card). Data,
    /// not words: it bids at most `labelColW`, or the name ceiling where it is the row's one name (`textNames`), and
    /// gives way past it — in the middle, or a remote's at its end (デザイン規約 §寸法「upstream 名だけは末尾で切る」).
    property string nameMark: ""
    property string codeName: ""
    readonly property bool codesName: menuItem.codeName !== ""
    /// Automation: whether that name was cut — a picture a pixel short and one whole read alike at a glance.
    readonly property bool nameCut: codeNameSeat.item !== null && codeNameSeat.item.cut
    /// The worktree after the words was cut — at the name ceiling (`TabTreeMark.cappedWidth`) or by the window.
    readonly property bool trailCut: trailSeat.item !== null && trailSeat.item.cutting
    /// A fact about the row in the left menu's far column — the branch a worktree has out
    /// (`NavRowBody.branchSeat`). Empty draws none.
    property string sideName: ""
    /// That name's width, held under the name ceiling (`Theme.nameMaxChars`), and whether it was cut — there or by the
    /// window.
    readonly property real sideWidth: sideSeat.item === null ? 0
        : Math.min(sideSeat.item.wantWidth, sideSeat.item.markRoom + sideSeat.item.ceilingWidth)
    readonly property bool sideCut: sideSeat.item !== null && sideSeat.item.cut
    /// The step in front of `codeName`: a word's after the row's words; none where the name opens the row, so it starts
    /// where the words would, as a name opening a sentence does. The mark is a sign on its name, so nothing stands
    /// between it and the name but its seat's air (デザイン規約 §アイコンの席 の修飾).
    readonly property real markWordGap: menuItem.text !== "" ? Theme.spaceXs : 0
    /// Where the row's words stand, in the row's y: every mark ahead of them takes their line for its box
    /// (デザイン規約 §アイコンの席 の文頭記号). Read off the label, since the layout and an anchor round a half pixel
    /// apart.
    readonly property real wordsTop: menuItem.contentItem ? menuItem.contentItem.y + itemLabel.y : 0

    /// The width this row asks of the menu's shared chip column: its chip's glyphs, even where the chip ends the row —
    /// the column has to clear the widest command (デザイン規約 §git 用語のコード表記).
    readonly property real codeColSeat: menuItem.code !== "" ? codeLabel.implicitWidth : 0

    /// Whether the row's words bid their whole width. Off for a row whose words carry data (`Unset upstream` and the
    /// upstream it names): the names bid at most `labelColW`, on top of the words round them — capped whole, the
    /// sentence would leave a card of such rows alone too narrow for any name —, and give way past that into the hover
    /// (デザイン規約 §メニュー). Not zero: a sparse menu at `menuMinW` with a wide frozen chip column would leave the
    /// name no width. A name behind its mark (`codeName`) bids the same of its own accord.
    property bool growsForText: true
    /// The row is one name (a repository, a worktree) — its words, or its code name (`codeName`): it bids at most the
    /// name ceiling (`Theme.nameMaxChars`) in the face it is drawn in and gives way in the middle, as a name in a
    /// sentence does (`refCuts`).
    property bool textNames: false

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
    /// The names in the row's own reason seated by the caller, where there are several or a ref's, as the tip's field
    /// takes them (`Words.listMarks`, `TipAsk.marksNow`). Wins over `reasonMarkWord`.
    property var reasonMarks: []
    /// The word the tooltip stands the worktree's mark against (`TipAsk.markWordNow`, デザイン規約 §ref の種別
    /// 「ツールチップだけは字の中」): the folder in a turned-down row's reason. A row's own names, the folder among them,
    /// are `tipMarks`.
    readonly property string tipMarkWord: menuItem.blocked ? menuItem.reasonMarkWord : ""
    /// …and which mark (`TipAsk.markKindNow`).
    readonly property string tipMarkKind: menuItem.blocked ? menuItem.reasonMarkKind : ""
    /// The names the hover's whole line says (`tip.words`), each behind its mark, seated part by part as that line
    /// joins them — the sentence's (`refSentence`), the code name, the worktree after the words, the far column's
    /// branch (`TipAsk.marksNow` reads them off the row).
    readonly property var tipMarks: {
        if (menuItem.blocked)
            return menuItem.blockedWhy === menuItem.blockedReason ? menuItem.reasonMarks : []
        const marks = []
        let at = menuItem.code !== "" ? menuItem.code.length + 1 : 0
        if (menuItem.text !== "") {
            let shift = 0
            for (const seat of menuItem.refSeats) {
                const mark = seat.key === 1 ? menuItem.refMark : menuItem.refMark2
                const name = seat.key === 1 ? menuItem.refName : menuItem.refName2
                if (mark !== "")
                    marks.push([at + seat.at + shift, mark, seat.key === 1 ? menuItem.refTint : menuItem.refTint2,
                                name.length])
                shift += name.length - 2
            }
            at += menuItem.text.length + 1
        }
        const runs = [[menuItem.codesName ? menuItem.codeName : "", menuItem.nameMark],
                      [menuItem.trail, "tree"], [menuItem.sideName, "branch"]]
        for (const run of runs) {
            if (run[0] === "")
                continue
            marks.push([at, run[1], undefined, run[0].length])
            at += run[0].length + 1
        }
        return marks
    }

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
    /// from an answer other than the count (`BranchBothDeleteRow`, the delete table's `Delete both`).
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
    /// `!` seat, the box is centred on that seat as the `!` is: the seat is the box's width (`AppMenu.holdIndent`), so
    /// the widest heading mark, the cloud, stands inside the row's wash (`tst_menuedge.qml`).
    readonly property real markX: menuItem.holdIndent > 0
                                  ? (menuItem.holdIndent - Theme.iconMd) / 2
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
    implicitWidth: (!menuItem.growsForText
                    ? Math.min(menuItem.wordsWant,
                               // The sentence, not `refSeats`: those carry the names as cut, off this width.
                               (menuItem.refSentence.indexOf("%1") >= 0 || menuItem.refSentence.indexOf("%2") >= 0
                                ? Math.ceil(refFixedRuler.advanceWidth) : 0)
                               + Metrics.labelColW)
                    : menuItem.textNames
                    ? Math.min(menuItem.wordsWant, Math.ceil(nameCapRuler.advanceWidth) + itemLabel.rightPadding)
                    : menuItem.wordsWant)
                   + menuItem.besideWords
    /// Everything the row bids beside its words. The words' room is the row less this (`refRoom`) — read off the
    /// bids, never off the words' own label: that one's width answers its text, which the room cuts, and the loop
    /// closes wherever the layout answers in the same pass. The chip and the code name are bid in whole pixels, as
    /// the layout hands them out: the row's `RowLayout` rounds each of those up, and a sum rounded once comes up the
    /// fractions short — the code name, laid out last, elides (a Linux font's `worktree remove` + folder).
    readonly property real besideWords: (codeChip.visible ? Math.ceil(codeChip.implicitWidth) + Theme.spaceSm : 0)
                   // The code name is data: it bids at most `labelColW` and elides past it (デザイン規約 §メニュー) —
                   // the name ceiling where it is the row's one name.
                   + (codeNameSeat.item !== null
                      ? menuItem.markWordGap
                        + Math.ceil(codeNameSeat.item.markRoom
                                    + Math.min(codeNameSeat.item.wantWidth - codeNameSeat.item.markRoom,
                                               menuItem.textNames ? Math.ceil(codeCapRuler.advanceWidth)
                                                                  : Metrics.labelColW)) : 0)
                   + (menuItem.note !== "" ? noteLabel.implicitWidth + Theme.spaceSm : 0)
                   // Everything drawn after the words bids too, or the row that sets the menu's width cuts its own
                   // name. Asked of what the row holds, not `visible`: a closed card's rows answer false, and the
                   // card is measured before it opens.
                   + (menuItem.sideName !== "" ? menuItem.sideWidth + Theme.spaceLg : 0)
                   + (menuItem.ahead > 0 || menuItem.behind > 0 ? rowTrack.implicitWidth + Theme.spaceSm : 0)
                   + (menuItem.badged ? Math.ceil(rowBadge.implicitWidth) + Theme.spaceXs : 0)
                   + (trailSeat.item !== null ? trailSeat.item.cappedWidth : 0)
                   + menuItem.leftPadding + menuItem.rightPadding
    font.pixelSize: Theme.fontMd
    // A row that is one code name says it, as the words would.
    Accessible.name: menuItem.text !== "" ? menuItem.text : menuItem.codeName
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
    /// The colour each name in the row takes (`Words.nameInk`): plain, or its own where it says where the reader is —
    /// on a held row's tone too, which the name's ground keeps it apart from — and the row's on a greyed row or under
    /// the fill, which take everything on them (デザイン規約 §メニュー「文の中に入った名前も、色は名前のもの」).
    readonly property color refColor: Words.nameInk(menuItem.wordColor, menuItem.refTint)
    readonly property color refColor2: Words.nameInk(menuItem.wordColor, menuItem.refTint2)
    readonly property color codeNameColor: Words.nameInk(menuItem.wordColor, undefined)
    /// The room the sentence is drawn in: where it is narrower than the sentence, its names give way — in the middle,
    /// a remote's at its end (デザイン規約 §寸法「upstream 名だけは末尾で切る」) — never the words round them (a
    /// `from ☁origin` cut off reads as nothing), and never with a cut mark in another colour than the name it cuts. -1
    /// while the row has not been laid out.
    readonly property real refRoom: menuItem.width > 0
                                    ? menuItem.width - menuItem.besideWords - itemLabel.rightPadding - 1 : -1
    /// What the sentence asks for whole, names uncut but for the name ceiling — what the row bids, so a cut never
    /// shrinks the menu that caused it. Whether there are seats is the sentence's to say, not `refSeats`', which
    /// carry the names as cut.
    readonly property real wordsWant: menuItem.refSentence.indexOf("%1") >= 0 || menuItem.refSentence.indexOf("%2") >= 0
                                      ? menuItem.refWant + itemLabel.rightPadding : itemLabel.implicitWidth
    readonly property real refWant: Math.ceil(refFixedRuler.advanceWidth
                                              + (menuItem.refSentence.indexOf("%1") >= 0 ? menuItem.refHeld(1) : 0)
                                              + (menuItem.refSentence.indexOf("%2") >= 0 ? menuItem.refHeld(2) : 0))
                                    + 1
    /// A name's width held under the name ceiling (`Theme.nameMaxChars`): a menu grows with its words, and its own
    /// ceiling (the window's edge) is far past where a name stops being read.
    function refHeld(which) {
        return Math.min((which === 1 ? refName1Whole : refName2Whole).advanceWidth, codeCapRuler.advanceWidth)
    }
    /// How wide each name is cut to (-1: whole): held under the ceiling, then, where the room is short, the longer
    /// gives way first, down to an even share.
    readonly property var refCuts: {
        const has1 = menuItem.refSentence.indexOf("%1") >= 0
        const has2 = menuItem.refSentence.indexOf("%2") >= 0
        if (!has1 && !has2)
            return [-1, -1]
        let w1 = has1 ? menuItem.refHeld(1) : 0
        let w2 = has2 ? menuItem.refHeld(2) : 0
        const room = menuItem.refRoom < 0 ? Number.MAX_VALUE
                   : Math.max(0, menuItem.refRoom - refFixedRuler.advanceWidth)
        if (w1 + w2 > room) {
            const small = Math.min(w1, w2)
            if (!has1 || !has2) {
                w1 = has1 ? room : 0
                w2 = has2 ? room : 0
            } else if (2 * small <= room) {
                if (w1 > w2)
                    w1 = room - w2
                else
                    w2 = room - w1
            } else {
                w1 = room / 2
                w2 = room / 2
            }
        }
        return [has1 && w1 < refName1Whole.advanceWidth ? w1 : -1, has2 && w2 < refName2Whole.advanceWidth ? w2 : -1]
    }
    readonly property string refShown: menuItem.refCuts[0] < 0 ? menuItem.refName : refName1Ruler.elidedText
    readonly property string refShown2: menuItem.refCuts[1] < 0 ? menuItem.refName2 : refName2Ruler.elidedText
    /// A name was cut: the hover gives the whole line back.
    readonly property bool refCut: menuItem.refCuts[0] >= 0 || menuItem.refCuts[1] >= 0
    /// The sentence's name seats in its own order — a translation may put `%2` first: `{at, name, tint, room, key}`,
    /// `room` the mark's (`refRoom1` / `refRoom2`).
    readonly property var refSeats: {
        const seats = []
        const first = menuItem.refSentence.indexOf("%1")
        if (first >= 0)
            seats.push({ at: first, name: menuItem.refShown, tint: menuItem.refColor, room: menuItem.refRoom1, key: 1 })
        const second = menuItem.refSentence.indexOf("%2")
        if (second >= 0)
            seats.push({ at: second, name: menuItem.refShown2, tint: menuItem.refColor2, room: menuItem.refRoom2,
                         key: 2 })
        return seats.sort((a, b) => a.at - b.at)
    }
    /// Those words as rich-text markup, in one label so the line measures and hovers like any row's: the words in the
    /// row's face, each name as code behind its mark's room (`Words.codeMarkup`, `Words.inked`). Rich text, as a name's
    /// face is not one styled text can set — it cannot elide, so the names give way first (`refCuts`). Every piece is
    /// escaped: the name is the repository's (a branch called `<b>`), and spaces go as `&nbsp;` because rich text folds
    /// a run of them (`encode::markup`).
    readonly property string refWords: {
        if (menuItem.refSeats.length === 0)
            return ""
        let out = ""
        let from = 0
        for (const seat of menuItem.refSeats) {
            out += menuItem.inked(menuItem.refSentence.substring(from, seat.at)) + Words.inked(seat.room)
                 + Words.codeMarkup(seat.name, seat.tint)
            from = seat.at + 2
        }
        return out + menuItem.inked(menuItem.refSentence.substring(from))
    }
    /// Where each name begins in the label, its room before it: the words and rooms ahead of it in the row's face, a
    /// name ahead of it in the code face — as the markup lays them (`refHead` / `refBetween`, `refShown1Code` /
    /// `refShown2Code`).
    readonly property real refLead1: menuItem.leadOf(1)
    readonly property real refLead2: menuItem.leadOf(2)
    function leadOf(key) {
        const seats = menuItem.refSeats
        if (seats.length === 0 || seats[0].key === key)
            return refHead.advanceWidth
        const first = seats[0].key === 1 ? refShown1Code : refShown2Code
        return seats.length > 1 ? refHead.advanceWidth + first.advanceWidth + refBetween.advanceWidth : 0
    }
    function inked(words) {
        return words.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/ /g, "&nbsp;")
    }

    /// The row's ask for the window's tooltip (`TipAsk`), for the harness and the tests to read. The names behind
    /// their marks and the one word a mark stands against are the row's own (`tipMarks`, `tipMarkWord`), read off the
    /// row by the seat.
    readonly property alias tip: rowTip
    readonly property TipAsk rowTip: TipAsk {
        id: rowTip
        target: menuItem
        // A blocked row's line comes before the elision's: one tooltip, one delay.
        asked: (menuItem.hovered || menuItem.tipForced)
               && (menuItem.blocked || itemLabel.truncated || menuItem.refCut || menuItem.trailCut
                   || menuItem.sideCut || menuItem.nameCut)
        // The whole line, code name included — a cut name is what the hover is there to give back, behind a chip too
        // (`worktree remove`).
        words: menuItem.blocked ? menuItem.blockedWhy
             : [menuItem.code, menuItem.text, menuItem.codesName ? menuItem.codeName : "", menuItem.trail,
                menuItem.sideName]
               .filter(words => words !== "").join(" ")
        // Out of the menu, never over its other rows — on the right first, where a sub-menu opens, so a parent menu
        // on the left stays readable.
        rowSide: "right"
    }

    // The press mark, in the seat `holdIndent` leaves on every row (デザイン規約 §回数押し): the head of the row's words,
    // as tall as their capitals and on their middle (デザイン規約 §アイコンの席 の文頭記号).
    HoldIcon {
        id: rowHold
        // Centred on the seat, where the `!` and a heading's mark stand.
        x: Math.round((menuItem.holdIndent - rowHold.width) / 2)
        y: Math.round(menuItem.wordsTop + itemLabel.baselineOffset - WordFace.drawnCapitalMiddle(rowHold.size)
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
        // The seat's width, so the `!` stands in its middle as the hold mark does — never sizeless while the menu has
        // no seat yet: a canvas born without a size is never asked to paint once one arrives (`GoneBadge`).
        width: Math.max(Theme.iconSm, menuItem.holdIndent)
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
        // Steps are written on the items: the code name takes a word's step (`markWordGap`), not the `spaceSm`
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
            CodeGround {
                anchors.left: codeLabel.left
                anchors.right: codeLabel.right
                // Height off the glyph size, not the label: the mono line box differs per OS and stands taller than
                // the UI-family sentence (デザイン規約 §git 用語のコード表記).
                anchors.verticalCenter: codeLabel.verticalCenter
                size: codeLabel.font.pixelSize
                // Half a gap, no more: a chip after the hold mark would otherwise reach back against it and the two
                // would read as one run of ink.
                anchors.leftMargin: -Theme.spaceXs / 2
                anchors.rightMargin: -Theme.spaceXs / 2
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
            // The width goes to the code name where there is one; these words are the fixed part before it.
            Layout.fillWidth: !menuItem.codesName
            // A name for words is drawn no further than the name ceiling, however wide another row makes the card.
            Layout.maximumWidth: menuItem.textNames ? Math.ceil(nameCapRuler.advanceWidth) + itemLabel.rightPadding
                                                    : Number.POSITIVE_INFINITY
            // `text` stays the plain sentence either way: the tooltip and accessibility read that.
            text: menuItem.refWords !== "" ? menuItem.refWords : menuItem.text
            // Written out: a group `font:` assignment and `font.weight` on one Label fail to load ("already assigned").
            font.family: Theme.uiFamily
            font.pixelSize: menuItem.font.pixelSize
            // Pinned: rows carry text nobody here chose (a branch called `<b>`), and AutoText guesses at markup. The
            // one markup line is `refWords`, escaped on the way in.
            textFormat: menuItem.refWords !== "" ? Text.RichText : Text.PlainText
            elide: menuItem.textNames ? Text.ElideMiddle : Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            // Clear of the card's arrow (`arrow`) at the right edge (the note, when there is one, sits last instead).
            rightPadding: !noteLabel.visible && menuItem.subMenu && menuItem.arrow
                          ? menuItem.arrow.width + Theme.spaceXs : 0
            // A heading takes the sidebar section band's weight, not its ink: a row that opens a card to its right is
            // never drawn darker than the rows beside it (デザイン規約 §メニュー の入れ子).
            font.weight: menuItem.titled ? Theme.fontWeightStrong : menuItem.font.weight
            color: menuItem.wordColor
            /// The markup ahead of the names in the row's face (`leadOf`): the words and room before the first, and the
            /// words and room between it and the second.
            TextMetrics {
                id: refHead
                font: itemLabel.font
                text: menuItem.refSeats.length > 0
                      ? menuItem.refSentence.substring(0, menuItem.refSeats[0].at) + menuItem.refSeats[0].room : ""
            }
            TextMetrics {
                id: refBetween
                font: itemLabel.font
                text: menuItem.refSeats.length > 1
                      ? menuItem.refSentence.substring(menuItem.refSeats[0].at + 2, menuItem.refSeats[1].at)
                        + menuItem.refSeats[1].room : ""
            }
            TextMetrics {
                id: refSpace
                font: itemLabel.font
                text: "\u00a0"
            }
            // The name ceiling: in the row's face for words that are one name (`textNames`), in the code face for the
            // names set as code — each laid out only on a row that reads it.
            TextMetrics {
                id: nameCapRuler
                font: itemLabel.font
                text: menuItem.textNames ? "n".repeat(Theme.nameMaxChars) : ""
            }
            /// The code face at the row's size (`WordFace.groundEnd`).
            FontMetrics {
                id: codeFace
                font.family: Theme.monoFamily
                font.pixelSize: itemLabel.font.pixelSize
            }
            TextMetrics {
                id: codeCapRuler
                font.family: Theme.monoFamily
                font.pixelSize: itemLabel.font.pixelSize
                text: menuItem.refSentence !== "" || menuItem.codesName ? "n".repeat(Theme.nameMaxChars) : ""
            }
            // The names in the code face: whole, as cut (`refCuts`), and as drawn (`leadOf`, the grounds); and the
            // sentence without them, rooms and all. **Whole and cut on two rulers**: a `TextMetrics` signals one change
            // for all it answers, so a cut read back off the ruler it writes reads its own answer and loops.
            TextMetrics {
                id: refName1Whole
                font.family: Theme.monoFamily
                font.pixelSize: itemLabel.font.pixelSize
                text: menuItem.refName
            }
            TextMetrics {
                id: refName2Whole
                font.family: Theme.monoFamily
                font.pixelSize: itemLabel.font.pixelSize
                text: menuItem.refName2
            }
            TextMetrics {
                id: refName1Ruler
                font.family: Theme.monoFamily
                font.pixelSize: itemLabel.font.pixelSize
                text: menuItem.refName
                elide: menuItem.refMark === "remote" ? Text.ElideRight : Text.ElideMiddle
                elideWidth: Math.max(0, menuItem.refCuts[0])
            }
            TextMetrics {
                id: refName2Ruler
                font.family: Theme.monoFamily
                font.pixelSize: itemLabel.font.pixelSize
                text: menuItem.refName2
                elide: menuItem.refMark2 === "remote" ? Text.ElideRight : Text.ElideMiddle
                elideWidth: Math.max(0, menuItem.refCuts[1])
            }
            TextMetrics {
                id: refShown1Code
                font.family: Theme.monoFamily
                font.pixelSize: itemLabel.font.pixelSize
                text: menuItem.refShown
            }
            TextMetrics {
                id: refShown2Code
                font.family: Theme.monoFamily
                font.pixelSize: itemLabel.font.pixelSize
                text: menuItem.refShown2
            }
            TextMetrics {
                id: refFixedRuler
                font: itemLabel.font
                text: menuItem.refSentence.replace("%1", menuItem.refRoom1).replace("%2", menuItem.refRoom2)
            }
            // The names' marks and code grounds, built on a row whose words hold a name (`refSentence`). Under the
            // words: a ground lies beneath the name it holds, and a mark stands in a room the words leave it.
            Loader {
                id: refInk
                anchors.fill: parent
                z: -1
                active: menuItem.refSentence !== ""
                sourceComponent: Item {
                    // Each name's code ground: from the head of its mark's room, which holds the mark's air — or half a
                    // gap before the name, behind no mark — to half a gap past its end (`WordFace.groundStart` /
                    // `groundEnd`). Gone where the words are cut before the name.
                    CodeGround {
                        visible: menuItem.refSentence.indexOf("%1") >= 0 && menuItem.refShown !== ""
                                 && menuItem.refLead1 < itemLabel.width - itemLabel.rightPadding
                        x: menuItem.refMarkRoom > 0 ? menuItem.refLead1 - menuItem.refCell
                                                    : WordFace.groundStart(menuItem.refLead1)
                        width: (menuItem.refMarkRoom > 0
                                ? WordFace.groundEnd(codeFace, menuItem.refShown,
                                                     menuItem.refLead1 + refShown1Code.advanceWidth)
                                : menuItem.refLead1 + refShown1Code.advanceWidth + Theme.spaceXs / 2) - x
                        anchors.verticalCenter: parent.verticalCenter
                        size: itemLabel.font.pixelSize
                    }
                    CodeGround {
                        visible: menuItem.refSentence.indexOf("%2") >= 0 && menuItem.refShown2 !== ""
                                 && menuItem.refLead2 < itemLabel.width - itemLabel.rightPadding
                        x: menuItem.refMarkRoom2 > 0 ? menuItem.refLead2 - menuItem.refCell
                                                     : WordFace.groundStart(menuItem.refLead2)
                        width: (menuItem.refMarkRoom2 > 0
                                ? WordFace.groundEnd(codeFace, menuItem.refShown2,
                                                     menuItem.refLead2 + refShown2Code.advanceWidth)
                                : menuItem.refLead2 + refShown2Code.advanceWidth + Theme.spaceXs / 2) - x
                        anchors.verticalCenter: parent.verticalCenter
                        size: itemLabel.font.pixelSize
                    }
                    // `refMark`, in the middle of the room the markup opened in front of the name: a sign on the name,
                    // its box the words' line (デザイン規約 §アイコンの席 の修飾), so a card's marks share one upright
                    // axis (§メニュー). Gone where the words are cut before the name.
                    NavIcon {
                        id: refMarkIcon
                        visible: menuItem.refMarkRoom > 0
                                 && menuItem.refLead1 < itemLabel.width - itemLabel.rightPadding
                        x: Math.round(menuItem.refLead1 - menuItem.refCell)
                        y: 0
                        width: menuItem.refCell
                        height: itemLabel.height
                        set: "sign"
                        size: itemLabel.font.pixelSize
                        wordBase: itemLabel.baselineOffset
                        nameAir: WordFace.nameAir(codeFace, menuItem.refShown)
                        kind: menuItem.refMark !== "" ? menuItem.refMark : "branch"
                        tint: refMarkIcon.inkBeside(menuItem.refColor)
                    }
                    NavIcon {
                        id: refMarkIcon2
                        visible: menuItem.refMarkRoom2 > 0
                                 && menuItem.refLead2 < itemLabel.width - itemLabel.rightPadding
                        x: Math.round(menuItem.refLead2 - menuItem.refCell)
                        y: 0
                        width: menuItem.refCell
                        height: itemLabel.height
                        set: "sign"
                        size: itemLabel.font.pixelSize
                        wordBase: itemLabel.baselineOffset
                        nameAir: WordFace.nameAir(codeFace, menuItem.refShown2)
                        kind: menuItem.refMark2 !== "" ? menuItem.refMark2 : "remote"
                        tint: refMarkIcon2.inkBeside(menuItem.refColor2)
                    }
                }
            }
        }
        // `nameMark` + `codeName`: the name as code, its mark on the same ground in the middle of the room a sentence
        // opens (`refCell`), so a card's names start on one x and its marks stand on one axis (デザイン規約 §メニュー);
        // with no mark, the name alone. A word's step after the row's words; with none, where the words would start
        // (`markWordGap`). Built on a row that ends in one.
        Loader {
            id: codeNameSeat
            active: menuItem.codesName
            visible: menuItem.codesName
            Layout.fillWidth: menuItem.codesName
            Layout.leftMargin: menuItem.codesName ? menuItem.markWordGap : 0
            // Bid at `labelColW` — the name ceiling where it is the row's one name (`besideWords`) — drawn as far as
            // the card lets it, up to the name ceiling, and cut to what it is given.
            Layout.maximumWidth: codeNameSeat.item !== null
                                 ? codeNameSeat.item.markRoom + Math.ceil(codeCapRuler.advanceWidth) : 0
            Layout.preferredHeight: codeNameSeat.implicitHeight
            sourceComponent: CodeChip {
                cutToWidth: true
                word: menuItem.codeName
                mark: menuItem.nameMark
                markCell: menuItem.refCell
                cutAt: menuItem.nameMark === "remote" ? "end" : "middle"
                size: menuItem.font.pixelSize
                tint: menuItem.codeNameColor
            }
        }
        // The run's spare width, at its end: a row with room past the name's ceiling hands none of it out in front of
        // the name.
        Item {
            visible: menuItem.codesName
            Layout.fillWidth: true
        }
        // `sideName`, pushed to the right end by the words — or the code name's spare width — taking the rest, a
        // column's step off them: at a word's step the two read as one run. A branch, so as code behind the branch mark
        // (デザイン規約 §ref の種別「文の中の名前」), a step down with the column (`Words.nameInk`); held under the
        // name ceiling and cut in the middle past it (`sideCut`). Built on a row that has one.
        Loader {
            id: sideSeat
            active: menuItem.sideName !== ""
            visible: menuItem.sideName !== ""
            Layout.leftMargin: Theme.spaceLg
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: menuItem.sideWidth
            Layout.preferredHeight: sideSeat.implicitHeight
            sourceComponent: CodeChip {
                id: sideChip
                /// The name ceiling in the chip's face (`sideWidth`).
                readonly property real ceilingWidth: Math.ceil(sideCapRuler.advanceWidth)
                cutToWidth: true
                word: menuItem.sideName
                mark: "branch"
                size: Theme.fontSm
                tint: Words.nameInk(menuItem.greyed ? menuItem.wordColor : Theme.textSecondary)
                TextMetrics {
                    id: sideCapRuler
                    font.family: Theme.monoFamily
                    font.pixelSize: sideChip.size
                    text: "n".repeat(Theme.nameMaxChars)
                }
            }
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
            Layout.preferredHeight: Theme.iconSm
            pullRequest: menuItem.badgePr
            gone: menuItem.badgeGone
        }
        // `trail`: the tree mark and folder as one run, the tab's own (`TabTreeMark`) — the mark a sign on the
        // folder's name (デザイン規約 §アイコンの席 の修飾). Built on a row that has one.
        Loader {
            id: trailSeat
            active: menuItem.trail !== ""
            visible: menuItem.trail !== ""
            Layout.preferredWidth: trailSeat.item !== null ? trailSeat.item.cappedWidth : 0
            Layout.preferredHeight: trailSeat.implicitHeight
            sourceComponent: TabTreeMark {
                name: menuItem.trail
                gap: Theme.spaceXs
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
    readonly property HoldDriver holdDrive: HoldDriver {
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

    HoverHandler {
        id: rowHand
    }
}

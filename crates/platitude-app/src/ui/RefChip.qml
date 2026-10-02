import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The front card of a commit's names: the first record drawn whole, and `+N` for the rest (the sheets behind it are
// `RefChipStack`'s). One channel each (デザイン規約 §ref の種別): the frame carries the kind (`kindColourFor`), the
// name carries where the ref is — ordinary text in this repository, grey only on a remote.
Rectangle {
    id: chip
    property var records: []
    property real maxWidth: Metrics.labelColW
    /// Whether a name too long for `maxWidth` runs on to more lines of the same frame. On only in the list a chip
    /// unstacks into, the one place the name is shown in order to be read (規約 §hover のツールチップ).
    property bool wrapped: false
    /// A second click is waiting out the double-click window before this becomes a name box
    /// (デザイン規約 §グラフ行のダブルクリック): worn as the hover wash, one step over whatever the chip wears.
    property bool waiting: false
    /// What this name reads, or what reads it, drawn inside this frame under the name —
    /// `{mark, markTint, text, tone, ahead, behind}`, or null. Only the card a chip unfolds into sets it
    /// (`RefListPopup`): a graph row is one chip tall.
    property var mate: null
    /// Whether that line leads somewhere (`mate.to` — `NavFacts.place`): a press goes to the commit the name stands
    /// on. Said by the pointer as the left panel's lines say it (`NavFactLine.goes`): the underline and the hand.
    readonly property bool mateGoes: chip.mate !== null && !!chip.mate.to
    /// Stands in for the pointer on those words where headless cannot put one (PGG_AUTO_ACT=ref-list-follow-lit).
    property bool matePointedAt: false
    readonly property bool mateHandOn: mateSeat.item !== null && mateSeat.item.containsMouse
    readonly property bool mateAimed: chip.mateGoes && (chip.mateHandOn || chip.matePointedAt)
    /// Whether the band is drawn — what a run reads back, rather than the ask above (PGG_AUTO_ACT=ref-list-follow-lit).
    readonly property bool mateLit: mateBand.visible
    /// The name on that line was pressed: where it goes (`mate.to`).
    signal mateFollowed(var to)
    /// What the hand's handler calls, and the one way in for a run (verify-ui §壊れない動詞の実装と反復).
    function followMate() {
        if (chip.mateGoes)
            chip.mateFollowed(chip.mate.to)
    }
    /// The height that line adds: a line box the same as the name's, so the room over the name matches the room
    /// under the line (measured off the ink instead, the frame closes tighter under a `fontSm` line).
    readonly property real mateRoom: chip.mate ? Theme.fontChipLine : 0
    /// The width that line asks of the frame; the measure rides whichever line names the local branch
    /// ([`trackOnName`]).
    readonly property real mateWidth: chip.mate ? mateRow.implicitWidth : 0
    readonly property real trackRoom: mateTrack.active ? Theme.spaceSm + mateTrack.width : 0
    /// Whether the ahead/behind measure stands on the chip's own line: it rides the line that names the local
    /// branch, as on the left panel's rows (デザイン規約 §左メニューの所作).
    readonly property bool trackOnName: chip.recKind === "branch"

    visible: records.length > 0
    // The name's line box plus the frame: two under the commit node, so the chip is not the loudest thing on the row.
    // Extra wrapped lines take the family's own line spacing, which in CJK families is taller than the box.
    height: Theme.fontChipLine + Math.max(0, chip.nameLines - 1) * chipFont.lineSpacing + 2 * Theme.borderWidth
            + chip.mateRoom
    // Whole pixels: every term inside is fractional, and a width just past a whole pixel loses the right border
    // altogether (the chip clips its own frame). Rounded up, the extra pixel going into the end padding (§余白).
    width: Math.min(Math.ceil(Math.max(chipContent.implicitWidth + (chip.trackOnName ? chip.trackRoom : 0),
                                       chip.mateWidth + (chip.trackOnName ? 0 : chip.trackRoom)))
                    + 2 * Theme.spaceXs, maxWidth)
    readonly property int nameLines: chip.wrapped ? Math.max(1, nameLabel.lineCount) : 1
    /// Everything in the chip but the name, each with its gap, counted only while drawn: room not taken would cut the
    /// name short of the frame.
    readonly property real furnitureW: (chip.hasBadge ? chip.badgeInk + Theme.spaceXs / 2 : 0)
                                       + (chip.hasCount ? chip.countW + Theme.spaceXs / 2 : 0)
                                       + (chip.hasMark ? chip.markInk + Theme.spaceXs / 2 : 0)
    /// What the contents come to; `width` clamps this to `maxWidth` and the frame clips the rest, so [`furnitureW`]
    /// must leave the name exactly its room (`tst_refstack.qml` holds it).
    readonly property real contentW: chipContent.implicitWidth
    /// Lays the contents out now, for a caller that changed this chip's room and reads its width back in the same
    /// turn (`RefListPopup.layOutRows`). The rows inside are positioners, which answer their width a polish later:
    /// unforced, the chip answers with the width it had in the graph's column (`tst_refstack.qml` holds it).
    function layOutNow() {
        chipContent.forceLayout()
        mateRow.forceLayout()
    }
    readonly property bool hasCount: chip.records.length > 1
    /// The count's seat, measured off the label that draws it: a seat priced for one digit lets the frame clip a
    /// `+41` to `+4`, a wrong number.
    readonly property real countW: countLabel.implicitWidth
    /// What each mark's ink spans (`NavIcon.inkWidth`). Both marks are seated to their ink: the air in their box
    /// counts as the gap beside them (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」).
    readonly property real markInk: nameMark.inkWidth
    /// The cloud's ink starts 1.7 of its sixteen in; seated on its box, that air would add to the row's spacing.
    readonly property real badgeInk: badgeMark.inkWidth
    /// Where the badge and the count stand, read off the laid-out items: swapping them changes no width, so only
    /// these can say the drawn order (`tst_refstack.qml`).
    readonly property real badgeX: badgeSeat.x
    readonly property real countX: countLabel.x
    /// Where the count and the ahead/behind measure came out: they share a seat (`tests/qml/tst_refstack.qml`).
    readonly property real countY: countLabel.y
    readonly property real trackY: mateTrack.y
    readonly property real nameRoom: chip.maxWidth - 2 * Theme.spaceXs - chip.furnitureW
    radius: Theme.radiusSm
    clip: true

    /// Stand-in record while there are none: not drawn (`visible`), but every colour below is still asked.
    readonly property var noChip: ({ "kind": "branch", "name": "", "isHead": false, "hasRemote": false,
                                     "hasPr": false, "here": true, "held": false, "locked": false, "remote": "",
                                     "key": "" })
    /// The record the card draws: the first of the row's (`encode::Chip`).
    readonly property var rec: records.length > 0 ? records[0] : chip.noChip
    readonly property string recKind: rec.kind
    readonly property bool recHead: rec.isHead
    readonly property bool recRemote: rec.hasRemote
    readonly property bool recPr: rec.hasPr
    readonly property bool recHere: rec.here
    // Another working copy has this branch out (the green frame). Read off the record, which is rebuilt with the ref
    // joins, so the chip repaints with them.
    readonly property bool recHeld: rec.held
    // `git worktree lock` is on the copy standing here; only records a copy stands on carry it.
    readonly property bool recLocked: rec.locked
    // Name, and the remotes it was read from when it was not read here.
    readonly property string recName: rec.name
    readonly property string recWhere: rec.remote
    // One slot, one mark: the cloud, or the PR mark instead (規約 §グラフ行のダブルクリック).
    readonly property bool hasBadge: recRemote || recPr
    /// The padlock ahead of the name: the copy standing here is locked (デザイン規約 §ref の種別). That a copy stands
    /// here at all is the frame's green. Same mark and colour as the WORKTREES row (`NavRowBody`).
    readonly property bool hasLock: chip.recLocked
    /// The tree ahead of a name that is a working copy's folder, not a ref: bare, it would read as a branch
    /// (デザイン規約 §ref の種別). Never with the padlock, which already says "working copy".
    readonly property bool hasTree: chip.recKind === "worktree" && !chip.recLocked
    /// One seat for the padlock or the tree.
    readonly property bool hasMark: chip.hasLock || chip.hasTree
    /// Every colour a record can wear, which bounds the sheets (`RefChipStack.maxSheets`). A key added to
    /// `kindKeyOf` has to be added here.
    readonly property var kindKeys: ["head", "local", "worktree", "remote", "tag", "tagdim"]
    /// Which frame colour a record wears, as a word: the stack counts colours and `color` values do not compare, so
    /// [`kindColourFor`] turns the word into ink. A branch another working copy holds wears the copy's colour.
    function kindKeyOf(rec) {
        if (rec.held)
            return "worktree"
        if (rec.kind === "tag")
            return rec.here ? "tag" : "tagdim"
        if (rec.kind === "remote")
            return "remote"
        if (rec.kind === "head")
            return "head"
        // A copy standing here with no branch out shares the held branch's key: one sheet per colour, and a key of
        // its own over the same ink would draw two sheets nobody can tell apart (§重ね表示).
        if (rec.kind === "worktree")
            return "worktree"
        return "local"
    }
    /// A tag this repository does not hold drops a step (§暗く落とした段). Only tags dim: other kinds say where they
    /// are in their frame colour or their name (`origin/main`).
    function kindColourFor(key) {
        return key === "worktree" ? Theme.success
             : key === "tag" ? Theme.refTag
             : key === "tagdim" ? Theme.refTagDim
             : key === "remote" ? Theme.textSecondary
             : key === "head" ? Theme.warning
             : Theme.accent
    }
    function kindGroundFor(key) {
        return key === "tag" || key === "tagdim" ? Theme.bgElevated : "transparent"
    }
    readonly property string kindKey: chip.kindKeyOf(chip.rec)
    readonly property color kindColor: chip.kindColourFor(chip.kindKey)
    // Where it is. Grey, not textMuted, for what this repository does not hold: it can still be reached (§無効 is for
    // what cannot). The detached HEAD keeps its state colour; the current branch is `textLink`, as in the sidebar. A
    // branch another copy holds keeps the ordinary ink: the frame already says where the copy is.
    readonly property color nameColor: recKind === "head" ? Theme.warning
                                       : recHead ? Theme.textLink
                                       : !recHere ? Theme.textSecondary
                                       : Theme.textPrimary

    color: chip.kindGroundFor(chip.kindKey)
    border.color: kindColor
    border.width: Theme.borderWidth

    /// Where the ink starts, in whole pixels off the chip's top: the room the first line's box leaves around the ink,
    /// halved, the odd pixel going up. Asked of the family, since each keeps a different room over its ascender
    /// (rules-refs/app-ui.md「チップの中身はインクで置く」); headless draws in another font, so check on the real ones.
    function inkTop(ink) {
        const room = Theme.fontChipLine - ink.tightBoundingRect.height
        return Theme.borderWidth + Math.ceil(room / 2)
    }
    /// Where a label goes to put its ink there, in whole pixels (a half pixel smears antialiasing into the margin).
    /// Each label asks for itself: they differ in size.
    function inkY(label, ink) {
        return Math.round(chip.inkTop(ink) - label.baselineOffset - ink.tightBoundingRect.y)
    }

    // A probe reaching every Latin extreme, so chips stand level whatever their names hold.
    TextMetrics {
        id: nameInk
        font: nameLabel.font
        text: "Hbxp"
    }
    // The count's own probe: it is smaller than the name (see `inkY`).
    TextMetrics {
        id: countInk
        font: countLabel.font
        text: "Hbxp"
    }
    // Line spacing for a wrapped name's height; `TextMetrics` measures a string and cannot answer it.
    FontMetrics {
        id: chipFont
        font: nameLabel.font
    }

    // The wait's wash, a layer of its own: one step over a tag's fill or a branch's none.
    Rectangle {
        anchors.fill: parent
        radius: chip.radius
        color: Theme.bgHover
        visible: chip.waiting
    }
    // The band under the hand: the left panel's lines' look (`NavFacts.aimFill`), on `mateSeat`'s rect.
    Rectangle {
        id: mateBand
        visible: chip.mateAimed
        x: Theme.borderWidth
        y: mateRow.y
        width: chip.width - 2 * Theme.borderWidth
        height: mateRow.height
        radius: Theme.radiusSm
        color: NavFacts.aimFill
        border.width: NavFacts.aimRim.a > 0 ? Theme.borderWidth : 0
        border.color: NavFacts.aimRim
    }
    // Mark, name, badge, then the count: the first three are this ref, the count is about the others, so it stands
    // after them (デザイン規約 §重ね表示).
    Row {
        id: chipContent
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.leftMargin: Theme.spaceXs
        // A whole gap from the frame, half a gap inside: the contents are one phrase, and at a whole gap each mark
        // stood as far from its name as the name from the frame.
        spacing: Theme.spaceXs / 2
        // The padlock or the tree ([`hasMark`]). Unlike the sidebar row's, the seat comes and goes: held open, it
        // would push every name on the graph one mark right. Seated to its ink ([`markInk`]).
        Item {
            visible: chip.hasMark
            // Asked of the mark, not a number: the padlock and the tree span different ink.
            width: chip.markInk
            height: Theme.iconSm
            // On the first line's box, for the reason the badge at the other end is.
            y: Theme.borderWidth + Math.round((Theme.fontChipLine - Theme.iconSm) / 2)
            NavIcon {
                id: nameMark
                anchors.verticalCenter: parent.verticalCenter
                anchors.left: parent.left
                anchors.leftMargin: -(Theme.iconSm - nameMark.inkWidth) / 2
                kind: chip.hasLock ? "lock" : "tree"
                tint: Theme.textSecondary
                width: Theme.iconSm
                height: Theme.iconSm
            }
        }
        Label {
            id: nameLabel
            y: chip.inkY(nameLabel, nameInk)
            text: chip.recName
            color: chip.nameColor
            font.pixelSize: Theme.fontChip
            font.weight: chip.recHead ? Theme.fontWeightStrong : Font.Normal
            elide: chip.wrapped ? Text.ElideNone : Text.ElideRight
            // `Text.Wrap`: a ref name has no spaces, and this breaks anywhere when there is no word boundary.
            wrapMode: chip.wrapped ? Text.Wrap : Text.NoWrap
            // Also when wrapped: handed its whole room, every chip in a list would be as wide as the widest name.
            width: Math.min(implicitWidth, chip.nameRoom)
        }
        // Remote / PR badge: its width is reserved in `furnitureW`, so it survives any elision. Seated to its ink
        // ([`badgeInk`]).
        Item {
            id: badgeSeat
            visible: chip.hasBadge
            width: chip.badgeInk
            height: Theme.iconSm
            // On the first line's box: centred on a wrapped chip, it would leave its name.
            y: Theme.borderWidth + Math.round((Theme.fontChipLine - Theme.iconSm) / 2)
            NavIcon {
                id: badgeMark
                anchors.verticalCenter: parent.verticalCenter
                // The ink's right edge on the seat's; the air left of the ink overhangs into the row's spacing.
                anchors.right: parent.right
                anchors.rightMargin: -(badgeMark.width - badgeMark.inkRight)
                kind: chip.recPr ? "pr" : "remote"
                tint: Theme.textSecondary
                width: Theme.iconSm
                height: Theme.iconSm
            }
        }
        // How many more names: row meta, in the meta colour (author, date).
        Label {
            id: countLabel
            y: chip.inkY(countLabel, countInk)
            visible: chip.hasCount
            text: "+" + (chip.records.length - 1)
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
    }
    // The [`mate`] line, as the left panel's rows open theirs: mark, name, measure (デザイン規約 §左メニューの所作).
    Row {
        id: mateRow
        visible: chip.mate !== null
        // At the frame's padding, mark first, like the panel's lines: stepped in, it read as a second name.
        x: Theme.spaceXs
        // Straight under the first line box; the border is already in each seat's own y (`inkY`).
        y: Theme.fontChipLine + Math.max(0, chip.nameLines - 1) * chipFont.lineSpacing
        height: Theme.fontChipLine
        // The panel's whole gap (`NavFactLine`), not the first line's half: this mark sits on its box, whose air
        // would eat a half gap.
        spacing: Theme.spaceXs
        // The panel's mark seat (`NavFactLine`), on its line's box like the badge.
        Item {
            width: Theme.iconXs
            height: Theme.iconXs
            y: Theme.borderWidth + Math.round((Theme.fontChipLine - Theme.iconXs) / 2)
            NavIcon {
                anchors.centerIn: parent
                anchors.horizontalCenterOffset: (Theme.iconSm - Theme.iconXs) / 2
                kind: chip.mate ? chip.mate.mark : ""
                tint: chip.mate ? chip.mate.markTint : Theme.textSecondary
                width: Theme.iconSm
                height: Theme.iconSm
            }
        }
        // Seated to its ink (`inkY`): centred on the box, a `fontSm` line hangs low.
        Label {
            id: mateText
            y: chip.inkY(mateText, countInk)
            text: chip.mate ? chip.mate.text : ""
            color: chip.mate ? chip.mate.tone : Theme.textSecondary
            font.pixelSize: Theme.fontSm
            font.underline: chip.mateAimed
        }
    }
    // The hand on that line: its whole box across the frame, the band's own rect (デザイン規約 §当たり判定「端に接しない
    // ものは判定と塗りを一致させる」). Over the row's handlers, so a press here is never the row's. Built only where
    // the line leads somewhere (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
    Loader {
        id: mateSeat
        active: chip.mateGoes
        x: Theme.borderWidth
        y: mateRow.y
        width: chip.width - 2 * Theme.borderWidth
        height: mateRow.height
        sourceComponent: MouseArea {
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: chip.followMate()
        }
    }
    // Ahead/behind at the frame's right end, on the line naming the branch ([`trackOnName`]), as the panel's rows
    // draw it (`HeadTrack`).
    Loader {
        id: mateTrack
        active: chip.mate !== null && (chip.mate.ahead > 0 || chip.mate.behind > 0)
        visible: mateTrack.active
        x: chip.width - Theme.spaceXs - mateTrack.width
        // The count's seat, both being `fontSm` digits; centred on the box, it hangs below the name.
        y: (chip.trackOnName ? 0 : mateRow.y) + chip.inkY(countLabel, countInk)
        sourceComponent: HeadTrack {
            ahead: chip.mate ? chip.mate.ahead : 0
            behind: chip.mate ? chip.mate.behind : 0
        }
    }
}

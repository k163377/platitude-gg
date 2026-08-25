import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One aggregated chip: primary name + "+N". There are two things to read off it and they get one channel each — the
// frame carries the kind (local = accent, remote = secondary grey, detached HEAD = warning, which is a state rather
// than a kind, tag = refTag with a fill behind it), and the name carries where the ref is: ordinary text for one that
// is in this repository, grey for one that is only on the remote (デザイン規約 §ref の種別). The sidebar already reads that way
// — names in textPrimary, kind in the section icon — and on a graph row the name is the thing most worth reading, so it
// is not the place to spend a colour on something the frame is already saying. The kind is the first record's own, not
// something the caller sets: a row hands over everything on it in one list, branches ahead of tags, and the chip shows
// the head of that list. The one icon is the remote/PR badge, same mark and same single slot as the sidebar rows — for
// tags too, which is how "this one is only here" reads.
Rectangle {
    id: chip
    property var records: []
    property real maxWidth: Metrics.labelColW
    // Nowhere to go from here (the branch already under the working tree): §無効 — the words drop to the muted colour,
    // frame included, since the frame is how a branch chip is read at all.
    property bool muted: false
    /// Whether a name too long for `maxWidth` runs on to another line instead of being cut. Off everywhere the chip
    /// stands in a row of its own size — the graph row, the menus — and on in the list a chip unstacks into, which is
    /// the one place the name is shown *in order to be read* (規約 §hover のツールチップ). **One frame either way**: the
    /// lines are a single label inside a single border, so a wrapped name is one chip that got taller, not two chips.
    property bool wrapped: false
    /// The same answer the record itself can carry: a branch another working copy has out is nowhere a move can go
    /// either (git refuses it outright), so it reads the way the caller's own `muted` does. It carries a mark of its
    /// own as well — the muting says a move cannot land here, the mark says where the branch went instead.
    readonly property bool dulled: chip.muted || chip.recHeld
    /// This chip has taken a second click and is waiting out the double-click window before it becomes a name box
    /// (デザイン規約 §グラフ行のダブルクリック). **The wash the pointer uses**, one step over whatever the row is already wearing
    /// — the gesture's own beat is the one place in the app where a press has landed and nothing has happened yet,
    /// and the reader is looking straight at this chip while it does (2026-08-26 ユーザー判断).
    property bool waiting: false

    visible: records.length > 0
    // The name's own line box, and the frame drawn around it — nothing else is in the box, so nothing else sets its
    // height. That comes to eighteen, two under the commit node it stands beside: near enough that the row reads as one
    // band, low enough that the chip is not the loudest thing on it (2026-08-21 ユーザー判断 — the frame was the node's own
    // twenty for a day, and against a subject at `fontMd` the chip won the row).
    //
    // A wrapped name adds the family's own line spacing per extra line rather than another `fontChipLine`: the first
    // line keeps the box the token names, and the lines under it are spaced the way the family spaces them, so nothing
    // is clipped in a family whose lines are taller than the box (the CJK ones are). **A one-line chip is the same
    // eighteen it always was** — the term is zero.
    height: Theme.fontChipLine + Math.max(0, chip.nameLines - 1) * chipFont.lineSpacing + 2 * Theme.borderWidth
    // **The frame is drawn on whole pixels.** Every term inside is fractional — glyph advances, and a mark's seat is
    // its ink — so the box lands wherever the sum does, and a box whose width stops just past a whole pixel **loses its
    // right border altogether**: the top and bottom rules and both corners are drawn, and the straight run between them
    // is not (2026-08-22 実測 — `main +4` came to 71.04 and drew three sides; the same chip at 72 draws four. The chip
    // clips, which is what puts its own frame under the cut). Rounded up, so the box is never narrower than what it
    // holds; the pixel that buys goes where a layout's remainder goes anyway, into the padding at the end (§余白).
    width: Math.min(Math.ceil(chipContent.implicitWidth) + 2 * Theme.spaceXs, maxWidth)
    /// How many lines the name came out on. Only a wrapped chip can answer more than one.
    readonly property int nameLines: chip.wrapped ? Math.max(1, nameLabel.lineCount) : 1
    /// Everything in the chip that is not the name: the `+N` seat, the badge with its gap, and the held mark with its
    /// own. Each is counted only while it is drawn — the two marks come and go, and a name measured against room that
    /// is not taken would be cut short of the frame.
    readonly property real furnitureW: (chip.records.length > 1 ? Theme.spaceLg : 0)
                                       + (chip.hasBadge ? chip.badgeInk + Theme.spaceXs / 2 : 0)
                                       + (chip.recHeld ? chip.heldInk + Theme.spaceXs / 2 : 0)
    /// What each mark's ink actually spans (`NavIcon.inkWidth`). **Both marks are seated to that rather than to their
    /// square**: the air a box holds past its ink is the mark's own, and belongs to the gap beside it (デザイン規約 §余白).
    /// Seated so, the chip reads the same figures from both ends — a whole gap between the frame and the mark, half a
    /// one between the mark and the word.
    readonly property real heldInk: heldMark.inkWidth
    /// The cloud needs the seat more than the tree does: it is drawn 2.0 of the sixteen in from its own left edge, so
    /// the gap before it was that air on top of the row's spacing — the one place in the chip where two spacings added
    /// up (2026-08-22 ユーザー報告).
    readonly property real badgeInk: badgeMark.inkWidth
    /// What is left for the name inside `maxWidth`.
    readonly property real nameRoom: chip.maxWidth - 2 * Theme.spaceXs - chip.furnitureW
    radius: Theme.radiusSm
    clip: true

    readonly property string rec: records.length > 0 ? records[0] : "L00010"
    readonly property string recKind: rec[0]
    readonly property bool recHead: rec[1] === "1"
    readonly property bool recRemote: rec.length > 2 && rec[2] === "1"
    readonly property bool recPr: rec.length > 3 && rec[3] === "1"
    readonly property bool recHere: rec.length > 4 && rec[4] === "1"
    // Another working copy has this branch out, so git refuses a move onto it (2026-08-21 実測). Read off the record
    // rather than asked of a model: the record is rebuilt whenever the ref joins are, so the chip repaints with the
    // rest of them instead of hanging a binding off a slot (app-ui.md 「QML バインディングはプロパティにしか反応しない」).
    readonly property bool recHeld: rec.length > 5 && rec[5] === "1"
    readonly property bool tagStyle: recKind === "T"
    // Name, and the remotes it was read from when it was not read here. The separator is absent whenever there are
    // none, so the name runs to the end of the record (see encode.rs).
    readonly property var recFields: rec.substring(6).split("\u001E")
    readonly property string recName: chip.recFields[0]
    readonly property string recWhere: chip.recFields.length > 1 ? chip.recFields[1] : ""
    // One slot, one mark: on the remote, or on the remote with a PR open (規約 §グラフ行のダブルクリック — the two never stack).
    readonly property bool hasBadge: recRemote || recPr
    // A tag this repository does not hold keeps the tag hue and only drops a step (§暗く落とした段): still a tag, read
    // somewhere else. Only tags dim, because only tags need it — every other kind says where it is in its own frame
    // colour (a remote branch is grey) or in its name (`origin/main` carries the remote in the name itself).
    readonly property color kindColor: chip.dulled ? Theme.textMuted
                                       : tagStyle ? (recHere ? Theme.refTag : Theme.refTagDim)
                                       : recKind === "R" ? Theme.textSecondary
                                       : recKind === "H" ? Theme.warning
                                       : Theme.accent
    // Where it is, not what it is. Grey is the name of something this repository does not hold — a remote branch, or a
    // tag only a remote has. Dropping further, to textMuted, would claim it cannot be reached, and a double-click on a
    // remote branch row goes there (§無効 is for what is actually unavailable). The detached HEAD marker keeps its state
    // colour in the name too: it is the one chip whose colour is not a kind. The branch the working tree stands on is
    // the nearest answer this colour has — "here" — and the sidebar already writes it that way, so the chip does too.
    readonly property color nameColor: chip.dulled ? Theme.textMuted
                                       : recKind === "H" ? Theme.warning
                                       : recHead ? Theme.textLink
                                       : !recHere ? Theme.textSecondary
                                       : Theme.textPrimary

    color: tagStyle ? Theme.bgElevated : "transparent"
    border.color: kindColor
    border.width: Theme.borderWidth

    /// Where the ink starts inside the frame, in whole pixels off the top of the chip: the room the **first line's**
    /// box leaves, halved, **with the odd pixel going up**.
    ///
    /// The line box rather than the frame's own height, which are the same thing until a name wraps: measured off the
    /// frame, a two-line chip would centre its first line halfway down the box and leave the last one on the border.
    ///
    /// A line box is not where a family puts its ink: it keeps more room above its ascender than below its descender,
    /// so centring the box inside the frame spent that room there and sat the descenders of `g` and `/` on the border
    /// (2026-08-20 ユーザー報告). How much room is the family's own — a fixed lift squares one family and opens a gap under
    /// the other — so this asks the family instead. When what is left over will not halve, the pixel goes above, where
    /// every ascender is, rather than below the one descender a name may not even have. **Not to be measured off the
    /// headless picture**, which is drawn in a family neither OS uses (verify-ui スキル §Windows での実行・デバッグの罠).
    function inkTop(ink) {
        const room = Theme.fontChipLine - ink.tightBoundingRect.height
        return Theme.borderWidth + Math.ceil(room / 2)
    }
    /// Where a label goes to put its ink there. Whole pixels: a label laid out on a half one spreads its antialiasing
    /// into a row it does not own, and that row is the margin. **Each label asks for itself** — the two here are
    /// different sizes, and one offset for the row hung the smaller from the taller one's top edge (2026-08-20 ユーザー報告).
    function inkY(label, ink) {
        return Math.round(chip.inkTop(ink) - label.baselineOffset - ink.tightBoundingRect.y)
    }

    // Measured off a probe reaching every extreme Latin ink has, rather than off the name itself, so a chip does not
    // stand differently from its neighbour because the name it carries happens to have no descender in it.
    TextMetrics {
        id: nameInk
        font: nameLabel.font
        text: "Hbxp"
    }
    TextMetrics {
        id: countInk
        font: countLabel.font
        text: "Hbxp"
    }
    // How far apart the family sets its lines, for the height a wrapped name takes. `TextMetrics` cannot answer this
    // one — it measures a string, and line spacing is the family's.
    FontMetrics {
        id: chipFont
        font: nameLabel.font
    }

    // The wash the wait wears. A layer of its own rather than a colour on the frame: a tag already has a fill and a
    // branch has none, and the wash has to read as one step over whichever of the two is underneath.
    Rectangle {
        anchors.fill: parent
        radius: chip.radius
        color: Theme.bgHover
        visible: chip.waiting
    }
    Row {
        id: chipContent
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.leftMargin: Theme.spaceXs
        // **A whole gap from the frame, half a gap between the things inside it** (2026-08-22 ユーザー判断). Everything in
        // here — the mark, the name, the count, the badge — is one phrase about one commit, and the frame's own padding
        // is the only wide space in the box; at a whole gap throughout, the `+N` stood off from the name it counts for
        // as far as the name stands off from the frame. The two marks are then seated to their plain ink, since it is
        // this spacing that is already the half gap their box-air would otherwise take out of a whole one.
        spacing: Theme.spaceXs / 2
        // Ahead of the name, and only when there is one to draw: another working copy has this branch out (2026-08-21
        // ユーザー判断 — the seat is added on the left only when it applies). Same mark and same meaning as the sidebar row's
        // (`NavItemDelegate`), which is the WORKTREES section's own.
        //
        // **The seat is not held open** the way the sidebar row's is: a chip is measured to its own contents rather
        // than laid out in a column of them, so an empty seat on every chip would walk every name on the graph one
        // mark to the right for a state almost none of them are in.
        //
        // **And it is seated to its ink, not to its box.** The mark is a head on a stem and fills half the sixteen it
        // is drawn on; a square seat would add that air to the gaps on both sides, and the pair read as a name pushed
        // away from a mark that sat tight against the frame (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」;
        // 2026-08-21 ユーザー報告, measured 114 -> 110 -> 108).
        Item {
            visible: chip.recHeld
            // The seat is the ink, so the frame keeps its whole gap to the mark and the row's own half gap is all
            // that stands between the mark and the name.
            width: chip.heldInk
            height: Theme.iconSm
            // On the first line's box, for the reason the badge at the other end is.
            y: Theme.borderWidth + Math.round((Theme.fontChipLine - Theme.iconSm) / 2)
            NavIcon {
                id: heldMark
                anchors.verticalCenter: parent.verticalCenter
                anchors.left: parent.left
                anchors.leftMargin: -(Theme.iconSm - heldMark.inkWidth) / 2
                kind: "tree"
                tint: chip.dulled ? Theme.textMuted : Theme.textSecondary
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
            font.weight: chip.recHead ? Font.DemiBold : Font.Normal
            // Cut, or carried on to the next line — never both, and which one is the owner's to say (`wrapped`).
            elide: chip.wrapped ? Text.ElideNone : Text.ElideRight
            // A ref name has no spaces to break at, so the break has to be allowed anywhere; `Text.Wrap` takes the word
            // boundary when there is one and breaks anywhere when there is not.
            wrapMode: chip.wrapped ? Text.Wrap : Text.NoWrap
            // The narrower of what the name wants and what it is given. **The same expression either way**: a wrapped
            // label handed its whole room would make every chip in a list as wide as the widest name.
            width: Math.min(implicitWidth, chip.nameRoom)
        }
        // How many more names the card has, which is meta about the row rather than one of the names — the colour the
        // row's other meta (author, date) is written in.
        Label {
            id: countLabel
            y: chip.inkY(countLabel, countInk)
            visible: chip.records.length > 1
            text: "+" + (chip.records.length - 1)
            color: chip.dulled ? Theme.textMuted : Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
        // Remote / PR badge: reserved width above, so it survives any elision.
        //
        // **Seated to its ink, like the mark at the other end**, and for the reason that one is: a square seat hands
        // the mark's own air to the gaps on both sides of it, and the cloud carries two of the sixteen on its left.
        // The gap before it would then read as the row's spacing **plus** that — the one place in the chip where two
        // spacings add up, wider than the same token spends anywhere else in the same frame (デザイン規約 §余白; 2026-08-22
        // ユーザー報告).
        Item {
            visible: chip.hasBadge
            width: chip.badgeInk
            height: Theme.iconSm
            // On the first line's box, not on the middle of the frame — the two are the same height until a name wraps,
            // and a badge that centres itself on a three-line chip has left the name it belongs to.
            y: Theme.borderWidth + Math.round((Theme.fontChipLine - Theme.iconSm) / 2)
            NavIcon {
                id: badgeMark
                anchors.verticalCenter: parent.verticalCenter
                // The ink's right edge on the seat's, which is where the frame's own padding starts: the air the box
                // holds past the ink comes off here rather than widening that padding, and the ink runs half a gap out
                // the other side into the row's spacing.
                anchors.right: parent.right
                anchors.rightMargin: -(badgeMark.width - badgeMark.inkRight)
                kind: chip.recPr ? "pr" : "remote"
                tint: chip.dulled ? Theme.textMuted : chip.recPr ? Theme.success : Theme.textSecondary
                width: Theme.iconSm
                height: Theme.iconSm
            }
        }
    }
}

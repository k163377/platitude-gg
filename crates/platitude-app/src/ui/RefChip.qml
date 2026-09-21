import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The front card of a commit's names: the first one drawn whole, and how many more the row carries (`+N`). What kind
// those others are is said behind the card, one sheet per colour, by the stack that seats this (`RefChipStack`) — this
// file draws the card and nothing else. There are two things to read off the name and they get one channel each — the
// frame carries the kind (local = accent, remote = secondary grey, detached HEAD = warning, which is a state rather
// than a kind, a working copy standing on this commit = the WORKTREES section's own green, whether it is holding a
// branch here or standing on no branch at all, tag = refTag with a fill behind it), and the name carries where the ref
// is: ordinary text for one that is in this repository, grey for one that is only on the remote
// (デザイン規約 §ref の種別). The sidebar already reads that way
// — names in textPrimary, kind in the section icon — and on a graph row the name is the thing most worth reading, so
// its colour is spent on where the ref is and the frame keeps the kind. The kind is the first record's own: a row
// hands over everything on it in one list, branches ahead of tags, and the chip shows the head of that list. The one
// icon is the remote/PR badge, same mark and same single slot as the sidebar rows — for tags too, which is how
// "this one is only here" reads. The one mark at the other end is the padlock, and only a locked working copy wears
// it.
Rectangle {
    id: chip
    property var records: []
    property real maxWidth: Metrics.labelColW
    /// Whether a name too long for `maxWidth` runs on to another line. Off everywhere the chip stands in a row of its
    /// own size — the graph row, the menus — and on in the list a chip unstacks into, which is the one place the name
    /// is shown *in order to be read* (規約 §hover のツールチップ). **One frame either way**: the lines are a single
    /// label inside a single border, so a wrapped name is one chip that got taller.
    property bool wrapped: false
    /// This chip has taken a second click and is waiting out the double-click window before it becomes a name box
    /// (デザイン規約 §グラフ行のダブルクリック). **The wash the pointer uses**, one step over whatever the row is already wearing
    /// — the gesture's own beat is the one place in the app where a press has landed and nothing has happened yet,
    /// and the reader is looking straight at this chip while it does.
    property bool waiting: false
    /// What this name reads, or what reads it, drawn **inside this frame** under the name —
    /// `{mark, markTint, text, tone, ahead, behind}`, or null where there is nothing to say. Only the card a chip
    /// unfolds into sets it (`RefListPopup`): a graph row is one chip tall, and the card is where this frame has the
    /// room. **One frame** — what the line names is this ref's own reading, so it belongs in the box the name is in,
    /// the way a wrapped name does.
    property var mate: null
    /// What that line takes off the frame's floor: **a line box of its own, the same one the name has**. The frame
    /// then holds two boxes of one height, so the room over the name is the room under the line — measured off the
    /// box rather than the words, whose own ink leaves less under a `fontSm` line than over a `fontChip` one
    /// (observed: the frame closed tighter under the second line than it opened over the first).
    readonly property real mateRoom: chip.mate ? Theme.fontChipLine : 0
    /// And what each line asks of the frame's width — the measure rides whichever of the two names the local
    /// branch ([`trackOnName`]).
    readonly property real mateWidth: chip.mate ? mateRow.implicitWidth : 0
    readonly property real trackRoom: mateTrack.active ? Theme.spaceSm + mateTrack.width : 0
    /// Whether the measure stands on the chip's own line. **It rides the line that names the local branch**, the way
    /// the left panel's rows draw it: a branch's own row carries it, and a remote-tracking row carries it on the line
    /// that names the branch reading it (デザイン規約 §左メニューの所作). So it is the chip's own line when the chip is
    /// the branch, and the line under it when the chip is what that branch reads.
    readonly property bool trackOnName: chip.recKind === "branch"

    visible: records.length > 0
    // The name's own line box, and the frame drawn around it — nothing else is in the box, so nothing else sets its
    // height. That comes to eighteen, two under the commit node it stands beside: near enough that the row reads as one
    // band, low enough that the chip is not the loudest thing on it (by design — the frame was the node's own
    // twenty for a day, and against a subject at `fontMd` the chip won the row).
    //
    // A wrapped name adds the family's own line spacing per extra line: the first line keeps the box the token
    // names, and the lines under it are spaced the way the family spaces them, so nothing is clipped in a family
    // whose lines are taller than the box (the CJK ones are). **A one-line chip is the same eighteen it always
    // was** — the term is zero.
    height: Theme.fontChipLine + Math.max(0, chip.nameLines - 1) * chipFont.lineSpacing + 2 * Theme.borderWidth
            + chip.mateRoom
    // **The frame is drawn on whole pixels.** Every term inside is fractional — glyph advances, and a mark's seat is
    // its ink — so the box lands wherever the sum does, and a box whose width stops just past a whole pixel **loses its
    // right border altogether**: the top and bottom rules and both corners are drawn, and the straight run between them
    // is not (measured — `main +4` came to 71.04 and drew three sides; the same chip at 72 draws four. The chip
    // clips, which is what puts its own frame under the cut). Rounded up, so the box is at least as wide as what it
    // holds; the pixel that buys goes where a layout's remainder goes anyway, into the padding at the end (§余白).
    width: Math.min(Math.ceil(Math.max(chipContent.implicitWidth + (chip.trackOnName ? chip.trackRoom : 0),
                                       chip.mateWidth + (chip.trackOnName ? 0 : chip.trackRoom)))
                    + 2 * Theme.spaceXs, maxWidth)
    /// How many lines the name came out on. Only a wrapped chip can answer more than one.
    readonly property int nameLines: chip.wrapped ? Math.max(1, nameLabel.lineCount) : 1
    /// Everything in the chip that is not the name: the badge with its gap, the count with its own, and the mark in
    /// front of the name with its own. Each is counted only while it is drawn — all three come and go, and a name
    /// measured against room that is not taken would be cut short of the frame.
    readonly property real furnitureW: (chip.hasBadge ? chip.badgeInk + Theme.spaceXs / 2 : 0)
                                       + (chip.hasCount ? chip.countW + Theme.spaceXs / 2 : 0)
                                       + (chip.hasMark ? chip.markInk + Theme.spaceXs / 2 : 0)
    /// What the frame's contents actually come to. `width` is this clamped to `maxWidth`, and whatever it runs over by
    /// is what the frame clips off its own right-hand end — so everything inside is held to what the name's room
    /// leaves, which is the whole of what [`furnitureW`] is measured for (`tst_refstack.qml` holds it).
    readonly property real contentW: chipContent.implicitWidth
    /// Lays the frame's contents out now, for a caller that has just changed the room this chip is given and is about
    /// to read its width back in the same turn (`RefListPopup.layOutRows`).
    ///
    /// **The row inside the frame is a positioner, and a positioner answers its width in the polish after the turn it
    /// was changed in.** The label re-lays itself out where it stands — its line count and its own width are current
    /// the moment `maxWidth` moves — but what the frame is measured by is the row's sum, and that one is a pass
    /// behind: a chip handed the card's room and read in the same breath answers with the width it had in the graph's
    /// column, whole columns narrower than the name it is already drawing (`tst_refstack.qml` holds it).
    function layOutNow() {
        chipContent.forceLayout()
        mateRow.forceLayout()
    }
    /// Whether the row carries names this card is not showing, which is the whole of what the count is for.
    readonly property bool hasCount: chip.records.length > 1
    /// The count's own seat, measured off the label that draws it. **A seat priced for a single digit is a seat the
    /// two-digit rows overrun**: what it does not cover is handed to the name, and the frame clips its own
    /// right-hand end off the far side of the row — **the count itself first**, since it stands last, and a `+41`
    /// cut to `+4` is a wrong number (measured on `JetBrains/kotlin`: 20 commits carry ten or more refs, and the
    /// deepest wears 42).
    readonly property real countW: countLabel.implicitWidth
    /// What each mark's ink actually spans (`NavIcon.inkWidth`). **Both marks are seated to that**: the air a box
    /// holds past its ink is the mark's own, and belongs to the gap beside it (デザイン規約 §余白). Seated so, the chip
    /// reads the same figures from both ends — a whole gap between the frame and the mark, half a one between the
    /// mark and the word.
    readonly property real markInk: nameMark.inkWidth
    /// The cloud needs the seat more than the padlock does: it is drawn 1.7 of the sixteen in from its own left edge,
    /// so the gap before it was that air on top of the row's spacing — the one place in the chip where two spacings
    /// added up.
    readonly property real badgeInk: badgeMark.inkWidth
    /// Where the two things after the name stand. The badge is this ref's own state and belongs beside the name it
    /// describes; the count is how many *others* the row carries and belongs after both (デザイン規約 §重ね表示).
    /// **Read off the laid-out items**, since nothing in the frame's arithmetic moves when the two swap — the contents
    /// come to the same width either way, so this is the only thing that can say they are in the drawn order.
    readonly property real badgeX: badgeSeat.x
    readonly property real countX: countLabel.x
    /// And where the measure under, or beside, the name came out — the seat it shares with the count, which nothing
    /// in the arithmetic above says out loud (`tests/qml/tst_refstack.qml`).
    readonly property real countY: countLabel.y
    readonly property real trackY: mateTrack.y
    /// What is left for the name inside `maxWidth`.
    readonly property real nameRoom: chip.maxWidth - 2 * Theme.spaceXs - chip.furnitureW
    radius: Theme.radiusSm
    clip: true

    /// The chip a card with nothing on it reads as: a local branch with no name, here and unmarked. The frame is
    /// not drawn then (`visible`), but every colour below is still asked.
    readonly property var noChip: ({ "kind": "branch", "name": "", "isHead": false, "hasRemote": false,
                                     "hasPr": false, "here": true, "held": false, "locked": false, "remote": "",
                                     "key": "" })
    /// The record the card draws — the first of the row's (`encode::Chip`: `kind`, `name`, `isHead`, `hasRemote`,
    /// `hasPr`, `here`, `held`, `locked`, `remote`, `key`).
    readonly property var rec: records.length > 0 ? records[0] : chip.noChip
    readonly property string recKind: rec.kind
    readonly property bool recHead: rec.isHead
    readonly property bool recRemote: rec.hasRemote
    readonly property bool recPr: rec.hasPr
    readonly property bool recHere: rec.here
    // Another working copy has this branch out, which is what puts the green frame on it — and also why git refuses
    // a move onto it (measured). Read off the record: the record is rebuilt whenever the ref joins are, so the chip
    // repaints with the rest of them (app-ui.md 「QML バインディングはプロパティにしか反応しない」).
    readonly property bool recHeld: rec.held
    // And `git worktree lock` is on that copy — the branch's holder, or the copy this marker is about. Only the two
    // records a copy is standing on ever carry it, so the padlock cannot turn up on a chip nobody is standing on.
    readonly property bool recLocked: rec.locked
    // Name, and the remotes it was read from when it was not read here.
    readonly property string recName: rec.name
    readonly property string recWhere: rec.remote
    // One slot, one mark: on the remote, or on the remote with a PR open (規約 §グラフ行のダブルクリック — the two never stack).
    readonly property bool hasBadge: recRemote || recPr
    /// Whether the padlock stands ahead of the name: `git worktree lock` is on the copy standing here
    /// (デザイン規約 §ref の種別). **That a copy is standing here at all is the frame's to say** — the green is on
    /// every one of them — so this is a state and not a kind, and a chip with no padlock is a copy nobody has
    /// locked. Same mark and same colour as the WORKTREES row's own (`NavRowBody`), which is the other place this
    /// state is read.
    readonly property bool hasLock: chip.recLocked
    /// And whether the name itself is a working copy's folder rather than a ref's — the one chip that is not
    /// naming a ref at all. **The mark belongs to the name**, not to the state: every other chip in the column
    /// spells a branch, a remote or a tag, and a bare folder name among them reads as one of those
    /// (デザイン規約 §ref の種別). It is the same mark the pane and the sidebar section write a copy's name with, so
    /// the three places spell it one way — and it takes the ink every mark in this frame takes, the padlock and the
    /// badge included: what a mark says is its shape, and a colour on top of it would be a second answer.
    ///
    /// **One mark in front of the name, never two.** A padlock is already a working copy's own state — nothing
    /// else in this column can be locked — so it says what the tree was there to say, and the pair only crowded a
    /// frame eighteen pixels tall.
    readonly property bool hasTree: chip.recKind === "worktree" && !chip.recLocked
    /// Which is why there is one seat, and the two above only decide what stands in it.
    readonly property bool hasMark: chip.hasLock || chip.hasTree
    /// Every colour a record can wear, which is how many cards one commit's names can ever come to
    /// (`RefChipStack.maxSheets`). **The list itself** — a kind added below is counted here by adding it here.
    readonly property var kindKeys: ["head", "local", "worktree", "remote", "tag", "tagdim"]
    /// Which of the frame colours a record wears, as a name. **The stack behind the card counts colours**
    /// (`RefChipStack`) and two colours cannot be told apart by comparing `color` values, so the rule answers in
    /// words and [`kindColourFor`] turns one into ink. A branch another working copy holds answers with where that
    /// copy is: standing on this commit, which is the thing the reader has to see first.
    function kindKeyOf(rec) {
        if (rec.held)
            return "worktree"
        if (rec.kind === "tag")
            return rec.here ? "tag" : "tagdim"
        if (rec.kind === "remote")
            return "remote"
        if (rec.kind === "head")
            return "head"
        // A working copy standing here with no branch out reads as the branch chip a copy *does* hold: the same
        // green frame and a name in the same ink, the only difference being which name there is to show
        // (デザイン規約 §ref の種別). The two say one thing — a copy is on this commit — so they share one key:
        // the sheets behind the card are one per colour (§重ね表示), and a key of its own over the same ink would
        // draw two nobody can tell apart.
        if (rec.kind === "worktree")
            return "worktree"
        return "local"
    }
    /// A tag this repository does not hold keeps the tag hue and only drops a step (§暗く落とした段): still a tag, read
    /// somewhere else. Only tags dim, because only tags need it — every other kind says where it is in its own frame
    /// colour (a remote branch is grey) or in its name (`origin/main` carries the remote in the name itself).
    function kindColourFor(key) {
        return key === "worktree" ? Theme.success
             : key === "tag" ? Theme.refTag
             : key === "tagdim" ? Theme.refTagDim
             : key === "remote" ? Theme.textSecondary
             : key === "head" ? Theme.warning
             : Theme.accent
    }
    /// The ground a card of that kind stands on. Only tags carry one.
    function kindGroundFor(key) {
        return key === "tag" || key === "tagdim" ? Theme.bgElevated : "transparent"
    }
    readonly property string kindKey: chip.kindKeyOf(chip.rec)
    readonly property color kindColor: chip.kindColourFor(chip.kindKey)
    // Where it is. Grey is the name of something this repository does not hold — a remote branch, or a tag only a
    // remote has. Dropping further, to textMuted, would claim it cannot be reached, and a double-click on a remote
    // branch row goes there (§無効 is for what is actually unavailable). The detached HEAD marker keeps its state
    // colour in the name too: it is the one chip whose colour is not a kind. The branch the working tree stands on is
    // the nearest answer this colour has — "here" — and the sidebar already writes it that way, so the chip does too.
    //
    // **A branch another copy holds writes its name in the ordinary ink**: the name is the
    // branch's, the branch is in this repository, and where the copy is is the frame's to say. Dulling the words as
    // well spent a second channel on a fact the frame already carries, and left the row's most-read text as the
    // faintest thing on it.
    readonly property color nameColor: recKind === "head" ? Theme.warning
                                       : recHead ? Theme.textLink
                                       : !recHere ? Theme.textSecondary
                                       : Theme.textPrimary

    color: chip.kindGroundFor(chip.kindKey)
    border.color: kindColor
    border.width: Theme.borderWidth

    /// Where the ink starts inside the frame, in whole pixels off the top of the chip: the room the **first line's**
    /// box leaves, halved, **with the odd pixel going up**.
    ///
    /// The line box, which is the frame's own height until a name wraps: measured off the
    /// frame, a two-line chip would centre its first line halfway down the box and leave the last one on the border.
    ///
    /// A line box is not where a family puts its ink: it keeps more room above its ascender than below its descender,
    /// so centring the box inside the frame spent that room there and sat the descenders of `g` and `/` on the
    /// border. How much room is the family's own — a fixed lift squares one family and opens a gap under the other —
    /// so this asks the family instead. When what is left over will not halve, the pixel goes above, where every
    /// ascender is. **Measured on the real families**, the headless picture being drawn in one neither OS
    /// uses (verify-ui スキル §Windows での実行・デバッグの罠).
    function inkTop(ink) {
        const room = Theme.fontChipLine - ink.tightBoundingRect.height
        return Theme.borderWidth + Math.ceil(room / 2)
    }
    /// Where a label goes to put its ink there. Whole pixels: a label laid out on a half one spreads its antialiasing
    /// into a row it does not own, and that row is the margin. **Each label asks for itself** — the two here are
    /// different sizes, and one offset for the row hung the smaller from the taller one's top edge.
    function inkY(label, ink) {
        return Math.round(chip.inkTop(ink) - label.baselineOffset - ink.tightBoundingRect.y)
    }

    // Measured off a probe reaching every extreme Latin ink has, so a chip stands level with its neighbour whatever
    // the name it carries happens to have in it.
    TextMetrics {
        id: nameInk
        font: nameLabel.font
        text: "Hbxp"
    }
    // The count's own probe. It is set smaller than the name and in a colour of its own, so the two are seated
    // separately: one lift for the row would hang the count off the name's top edge (see `inkY`).
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

    // The wash the wait wears. A layer of its own: a tag already has a fill and a branch has none, and the wash has
    // to read as one step over whichever of the two is underneath.
    Rectangle {
        anchors.fill: parent
        radius: chip.radius
        color: Theme.bgHover
        visible: chip.waiting
    }
    // **The mark, the name and the badge, and the count after them** (デザイン規約 §重ね表示). The first three are the
    // one ref this card is showing — where it is checked out, what it is called, and whether it is on a remote or has
    // a PR open — and the count is the only thing in the frame that is not about that ref at all: it is how many
    // *others* the row is carrying behind it. So it stands after the phrase it is not part of, and the card reads as
    // a branch and its state, and then how many more are under it.
    Row {
        id: chipContent
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.leftMargin: Theme.spaceXs
        // **A whole gap from the frame, half a gap between the things inside it**. Everything in
        // here — the mark, the name, the badge — is one phrase about one commit, and the frame's own padding is the
        // only wide space in the box; at a whole gap throughout, each mark stood off from the name it belongs to as
        // far as the name stands off from the frame. The two marks are then seated to their plain ink, since it is
        // this spacing that is already the half gap their box-air would otherwise take out of a whole one.
        spacing: Theme.spaceXs / 2
        // Ahead of the name, and only when there is one to draw: the padlock of a locked working copy, or the mark
        // that says the name itself is a copy's folder ([`hasLock`], [`hasTree`] — the two never meet). Same marks
        // and same colour as the sidebar's own (`NavRowBody`), which is the other place they are read.
        //
        // **The seat comes and goes** where the sidebar row's is held open: a chip is measured to its own contents,
        // so an empty seat on every chip would walk every name on the graph one mark to the right for a state almost
        // none of them are in.
        //
        // **And it is seated to its ink.** Neither mark fills the sixteen it is drawn on; a
        // square seat would add that air to the gaps on both sides, and the pair read as a name pushed away from a
        // mark that sat tight against the frame (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」;
        // observed, measured 114 -> 110 -> 108).
        Item {
            visible: chip.hasMark
            // The seat is the ink, so the frame keeps its whole gap to the mark and the row's own half gap is all
            // that stands between the mark and the name. **The two kinds do not span the same ink** (the padlock's
            // body is nine of the sixteen, the tree's head eight), so the seat is asked of the mark rather than
            // told a number.
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
            font.weight: chip.recHead ? Font.DemiBold : Font.Normal
            // Cut, or carried on to the next line — one of the two, and which is the owner's to say (`wrapped`).
            elide: chip.wrapped ? Text.ElideNone : Text.ElideRight
            // A ref name has no spaces to break at, so the break has to be allowed anywhere; `Text.Wrap` takes the word
            // boundary when there is one and breaks anywhere when there is not.
            wrapMode: chip.wrapped ? Text.Wrap : Text.NoWrap
            // The narrower of what the name wants and what it is given. **The same expression either way**: a wrapped
            // label handed its whole room would make every chip in a list as wide as the widest name.
            width: Math.min(implicitWidth, chip.nameRoom)
        }
        // Remote / PR badge: reserved width above, so it survives any elision.
        //
        // **Seated to its ink, like the mark at the other end**, and for the reason that one is: a square seat hands
        // the mark's own air to the gaps on both sides of it, and the cloud carries 1.7 of the sixteen on its left.
        // The gap before it would then read as the row's spacing **plus** that — the one place in the chip where two
        // spacings add up, wider than the same token spends anywhere else in the same frame (デザイン規約 §余白;
        // observed).
        Item {
            id: badgeSeat
            visible: chip.hasBadge
            width: chip.badgeInk
            height: Theme.iconSm
            // On the first line's box — the two are the same height until a name wraps, and a badge that centres
            // itself on a three-line chip has left the name it belongs to.
            y: Theme.borderWidth + Math.round((Theme.fontChipLine - Theme.iconSm) / 2)
            NavIcon {
                id: badgeMark
                anchors.verticalCenter: parent.verticalCenter
                // The ink's right edge on the seat's: the air the box holds past the ink comes off here, and the ink
                // runs half a gap out the other side into the row's spacing.
                anchors.right: parent.right
                anchors.rightMargin: -(badgeMark.width - badgeMark.inkRight)
                kind: chip.recPr ? "pr" : "remote"
                tint: Theme.textSecondary
                width: Theme.iconSm
                height: Theme.iconSm
            }
        }
        // How many more names the row carries, which is meta about the row — the colour the row's other meta
        // (author, date) is written in.
        //
        // **The sheets behind the card say which colours they are; this says how many there are** (デザイン規約 §重ね表示).
        // The two answer different halves of the same question and neither can be read off the other: a colour is one
        // sheet however many names wear it, so a commit wearing forty tags draws exactly the fan a commit wearing two
        // draws — which is the row a reader most needs the number on.
        Label {
            id: countLabel
            y: chip.inkY(countLabel, countInk)
            visible: chip.hasCount
            text: "+" + (chip.records.length - 1)
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
    }
    // What this name reads, or what reads it, under the name and inside the same frame ([`mate`]) — the same line the
    // left panel's rows open under themselves, in the same order: the mark that stands for the fact, the name, and
    // the measure that name is worth (デザイン規約 §左メニューの所作).
    //
    // **A step in from the name**, so the line hangs off it rather than standing beside it as a second name would.
    Row {
        id: mateRow
        visible: chip.mate !== null
        // Hard against the frame's own padding, the mark first: the panel's lines lead with the mark and this is the
        // same line (デザイン規約 §左メニューの所作). Stepped in from the name above, it read as a second name rather than
        // as what that name reads.
        x: Theme.spaceXs
        // The second line box, straight under the first — the border is already inside what each seat below asks
        // for (`inkY`), so this is the box's own step and nothing more.
        y: Theme.fontChipLine + Math.max(0, chip.nameLines - 1) * chipFont.lineSpacing
        height: Theme.fontChipLine
        // The panel's own gap between a line's mark and its words (`NavFactLine`) — the mark here is seated the way
        // that one is, on its box rather than to its ink, so the half gap the first line spends is taken up by the
        // mark's own air and the cloud lands against the name.
        spacing: Theme.spaceXs
        // The seat the panel's lines keep whether or not a mark stands in it, so the words begin in one column
        // (`NavFactLine`). Here it always has one — a line with nothing to stand for is a line with nothing to say.
        // **On its line's box**, the way the badge sits on the first one.
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
        // Seated to its ink in that box, the way every other word in this frame is (`inkY`): centred on the box, a
        // `fontSm` line hangs low against the name over it, since a line box keeps more room over its ascender than
        // under its descender.
        Label {
            id: mateText
            y: chip.inkY(mateText, countInk)
            text: chip.mate ? chip.mate.text : ""
            color: chip.mate ? chip.mate.tone : Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
    }
    // How far that branch stands from what it reads. **At the frame's right-hand end, on the line that names the
    // branch** ([`trackOnName`]) — the same seat and the same measure the panel's rows draw at their own right edge
    // (`HeadTrack`), so the two places read as one answer.
    Loader {
        id: mateTrack
        active: chip.mate !== null && (chip.mate.ahead > 0 || chip.mate.behind > 0)
        visible: mateTrack.active
        x: chip.width - Theme.spaceXs - mateTrack.width
        // On its line, seated where that line's digits are. **The measure is a `fontSm` digit and its mark**, which
        // is what the count at the end of the name already is, so it takes the count's own seat — centred on the box
        // instead, it hangs below the name it is measuring, and the room that leaves makes the frame's own padding
        // read as uneven.
        y: (chip.trackOnName ? 0 : mateRow.y) + chip.inkY(countLabel, countInk)
        sourceComponent: HeadTrack {
            ahead: chip.mate ? chip.mate.ahead : 0
            behind: chip.mate ? chip.mate.behind : 0
        }
    }
}

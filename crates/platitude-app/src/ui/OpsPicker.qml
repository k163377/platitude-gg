import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One of the three things the window is standing in — the repository, the working copy, the branch — and the door to
// another of them. **The chevron in front of the name is the affordance**, drawn and turned the way the left panel's
// rows draw theirs (`NameCell`): pointing right while the list is shut, down while it is open. Where the name is a
// kind of thing rather than the window's own subject, a smaller mark for that kind stands between the two — a step
// under the chevron and a step from the name, so the pair reads as one thing in front of one name. Nothing follows
// the name but what the name is measured against.
//
// How loudly the name is said is the caller's (`gap` / `pixelSize` / `weight` / `tone`), because what the three of
// them say between them is which one holds the others. The name is cut in the middle like every other name in a
// column (`CutName`): both of its ends are how one working copy is told from another.
//
// Nothing is bound to a repository from here — the picker says what it is handed and reports the press. What a press
// opens belongs to whoever put the picker in a bar (`TopBar`).
HoverToolButton {
    id: picker

    /// The name on the face.
    property string name: ""
    /// What stands there while there is no name — a repository that has not answered yet, an unborn HEAD. Said in the
    /// muted ink, so "nothing yet" never reads as something's name (デザイン規約 §テキスト).
    property string blank: ""
    /// The one thing about this name that is not part of it — the ref a branch is measured against. **Written on a
    /// line of its own under the name**, a step down (`noteSize`), in `Font.Normal` and the muted ink, behind the mark
    /// that says it is a remote's (規約 §操作パネル). **Never folded into `name`** — spelt into the string it would take
    /// the name's weight and colour, and read as part of what the branch is called. Empty says nothing, and the picker
    /// is one line.
    property string note: ""
    /// Whether the name is one the reader chose rather than a state git is in — a detached HEAD is the second kind,
    /// and takes the state's colour the way its chip does (規約 §ref の種別).
    property color tone: Theme.textPrimary
    /// Whether the list this opens is standing. Which way the chevron points, and nothing else: the same question the
    /// sidebar's folder rows answer with the same mark (`NameCell.folded`, inverted — a row says whether it is shut).
    property bool opened: false
    /// What kind of thing this name is, as a `NavIcon` kind — drawn between the chevron and the name, **a step under
    /// the chevron's own size**, so the pair reads as one thing in front of one name rather than as two marks. Empty
    /// draws none, which is what the name that opens from the window's own edge does: nothing stands beside the ☰.
    property string kind: ""
    property int kindSize: Theme.iconSm
    /// The kind mark's ink, which is the left panel's for the same kind — a branch is the accent there, a working
    /// copy's section is `success` (規約 §ref の種別 / `NavSections`).
    property color kindTint: Theme.textSecondary
    /// The ink the upstream's run is said in. **Warning where that upstream has gone from the remote** — the left
    /// panel turns the same reading the same colour (§左メニューの所作「消えていれば段ごと `warning`」), and a name
    /// that is still drawn in the quiet ink would have the panel saying a branch is measured against something that
    /// is not there any more.
    property color noteTone: Theme.textMuted
    /// The step the upstream is written at — one under the name above it, and fixed rather than following it: the
    /// name can be raised to say it is what the row is read for, and the upstream rising with it would take that
    /// back.
    property int noteSize: Theme.fontMd
    /// Where the branch stands against what it follows, counted the way the left panel counts it (`HeadTrack`).
    /// Both zero draws nothing: level with the upstream is not a number worth a seat (§左メニューの所作).
    property int ahead: 0
    property int behind: 0
    /// A second name set after the first in one line — the working copy this window is standing in, drawn the way the
    /// tab's own node draws it (デザイン規約 §タブの所作): the mark the WORKTREES section is read by, then that copy's
    /// folder name, **a step down and a shade quieter** because the row is read for the name in front of it. **The
    /// mark is a letter of the run** — nothing is written either side of it, and what opens around it is the air its
    /// own box holds (§余白). Empty draws none.
    property string trail: ""
    /// How much narrower than whole this picker has been asked to be, in pixels. **Written from outside** — what a
    /// word costs is this picker's question, but the order the two pickers give way in is the row's (`TopBar`
    /// §譲る順). **One figure, handed out here**: the two lines are one box, so what the box gives up comes off
    /// whichever line is the longer — a line that is not the longer one frees nothing by being cut.
    ///
    /// **Nothing is dropped, only cut**: a run that went
    /// missing at one width and came back at another would have the panel saying different things about the same
    /// repository, and the reader who is short of room is the one who needs the mark to still be there.
    property real given: 0
    /// What each can give up before it reaches its own floor — both ends of a name and the mark between them.
    readonly property real trailSlack:
        picker.trail === "" ? 0
        : Math.max(0, Math.ceil(trailRuler.advanceWidth) - Math.ceil(trailFloorRuler.advanceWidth))
    readonly property real nameSlack:
        Math.max(0, Math.ceil(nameRuler.advanceWidth) - Math.ceil(floorRuler.advanceWidth))
    readonly property real noteSlack:
        picker.note === "" ? 0
        : Math.max(0, Math.ceil(noteRuler.advanceWidth) - Math.ceil(noteFloorRuler.advanceWidth))
    /// The width the lines are cut to: the longer line whole, less what was asked for — and never under the widest
    /// either line comes to at its own floor.
    readonly property real textRoom: picker.textWhole - Math.min(Math.max(0, picker.given), picker.slack)
    /// …and what that takes off each run. **The first line gives its copy's run before its name** (§譲る順); the
    /// second line is the upstream and nothing else.
    readonly property real lineOneShort: Math.max(0, picker.lineOneWhole - picker.textRoom)
    readonly property real trailGiven: Math.min(picker.lineOneShort, picker.trailSlack)
    readonly property real nameGiven: Math.min(picker.lineOneShort - picker.trailGiven, picker.nameSlack)
    readonly property real noteGiven:
        Math.min(Math.max(0, picker.lineTwoWhole - picker.textRoom), picker.noteSlack)
    /// What this picker comes out at once it has given up what it was asked for.
    readonly property real drawnWidth: picker.bareBox + picker.textRoom

    // ---- which register this one is said in ----------------------------
    // Three pickers stand in a row and they are not three of a kind: one of them holds the other two. The panel says
    // so where a heading would — in size, weight and ink (規約 §タイポグラフィ「見出しは重み・色・大文字が作る」) — and each
    // use site names its own register rather than a rank word being decided in here.
    /// **The gap is between the mark's ink and the word**, not between their boxes: a mark is drawn inside its square
    /// with air of its own, and a step measured box to box lands on screen as that step plus the air
    /// (デザイン規約 §余白). What the layout is given is `markGap` below.
    property int gap: Theme.spaceXs
    /// The step after the chevron's ink, where it is not the same as the others. A row that has to line its first name
    /// up with something outside the picker sets this one and leaves the rest alone (`TopBar`).
    property real markStep: picker.gap
    /// The step between the kind mark's ink and the name. **Zero** — the mark's own air is the whole of it: the
    /// chevron's seat is what opens this group, and a second amount of air inside the group would read against it
    /// rather than with it (デザイン規約 §余白 — 印が自分で持っている余白は、隣の詰めに数える).
    property real kindStep: 0
    /// How big the chevron is — the mark a hand goes for, so it is the larger of the two a picker wears.
    property int markSize: Theme.iconMd
    /// The slot it stands in, centred. Wider than the mark where the mark has a **column** of its own rather than a
    /// place in a row: the one at the window's edge stands in the ☰'s cell and what follows begins at that cell's
    /// edge, the way the tabs begin at it in the band above. A column is not a gap, so nothing is measured across it —
    /// the step below comes out zero on its own, because the seat already holds more air than a step asks for.
    property int markSeat: picker.markSize
    property int pixelSize: Theme.fontMd
    property int weight: Font.DemiBold
    /// How tall the two lines stand when both are written, **whatever is written now**: read off the faces, not the
    /// words. The depth the panel's buttons are drawn to — their own two lines are there whether or not this branch
    /// has an upstream to write under its name, and a picker still waiting on its repository writes neither.
    readonly property real pairHeight: nameMetrics.height + noteMetrics.height
    /// **Where a mark stands among the letters it is set in**, measured down from the top of the line they are set
    /// on: **standing on the baseline the way a letter does**, and where it is taller than the capitals, centred on
    /// them. Centred in the line box instead it rides above the letters — the box holds far more under the baseline
    /// than over them. Centred on the x-height, a mark taller than the capitals hangs under the baseline by what it
    /// has over them, and reads as sitting low beside its word (observed: the branch's lower ring under the line).
    /// **Each answers to the face it stands in**, the way its size already does (§寸法), and to its own ink — a mark's
    /// box holds air of its own above and below (`NavIcon.inkTallGrid`).
    function markMiddle(seat, metrics, mark) {
        const ink = mark.inkTallGrid / 16 * mark.height + mark.stroke
        const stands = Math.min(Math.max(ink, metrics.xHeight), metrics.capitalHeight)
        return Math.max(0, Math.round(metrics.ascent - stands / 2 - seat / 2))
    }
    readonly property real kindDrop: picker.markMiddle(picker.kindSize, nameMetrics, kindMark)
    readonly property real noteMarkDrop: picker.markMiddle(picker.noteSeat, noteMetrics, noteMark)

    readonly property bool saying: picker.name !== ""
    readonly property string shown: picker.saying ? picker.name : picker.blank

    /// The box with no words in it — the paddings, the chevron's seat and the step after it, paid whatever the row
    /// can afford.
    readonly property real bareBox:
        picker.leftPadding + picker.rightPadding + picker.markSeat + picker.markGap
    /// The column the kind mark stands in at the head of each line, and the step after it.
    readonly property real kindColumn: picker.kind === "" ? 0 : picker.markColumn + picker.kindGap
    /// What the copy's run costs whole: **one step off what it follows, its mark, and its own name** — the mark is a
    /// letter of the run, so nothing is counted on either side of it (§余白).
    readonly property real trailWidth:
        picker.trail === "" ? 0 : picker.gap + picker.trailSeat + Math.ceil(trailRuler.advanceWidth)
    /// Whether there are counts beside the name, and what they cost: the step off the name and the counts' own
    /// width. **The step is a word's-worth rather than a letter's** — the counts are a figure about the branch, not
    /// a letter of its name, and set at a letter's step the two read as one word (observed: `main↑2`).
    readonly property bool tracked: picker.ahead > 0 || picker.behind > 0
    property int trackGap: Theme.spaceSm
    readonly property real trackRun: picker.tracked ? picker.trackGap + Math.ceil(trackSeat.implicitWidth) : 0
    /// **Each line is measured as the line it is.** The upstream is written under the name rather than after it, so
    /// the box is as wide as the longer of the two — never the two laid end to end, which holds a whole upstream's
    /// width of air past both of them.
    readonly property real lineOneWhole:
        picker.kindColumn + Math.ceil(nameRuler.advanceWidth) + picker.trailWidth + picker.trackRun
    readonly property real lineTwoWhole:
        picker.note === "" ? 0 : picker.kindColumn + Math.ceil(noteRuler.advanceWidth)
    readonly property real textWhole: Math.max(picker.lineOneWhole, picker.lineTwoWhole)
    /// …and each at its floor: the runs and the name cut down as far as they go, the counts whole.
    readonly property real lineOneFloor: picker.lineOneWhole - picker.trailSlack - picker.nameSlack
    readonly property real lineTwoFloor: picker.lineTwoWhole - picker.noteSlack
    /// The width this picker asks the row for before anything gives way. **Read without the answer in it** — the row
    /// decides what it can afford from this, so a figure that already counted the decision would close the ring.
    readonly property real wholeWidth: picker.bareBox + picker.textWhole
    /// …and the narrowest it may be laid out at: every name at **both of its ends** — three characters each side and
    /// the mark between them, which is the floor a tab's title stands on (規約 §レイアウト初期値「両端に 3 文字ずつ +
    /// `…`」). A name told apart by its tail is not told apart by a head alone.
    readonly property real foldWidth: picker.bareBox + Math.max(picker.lineOneFloor, picker.lineTwoFloor)
    /// What lies between the two — the most this picker can give.
    readonly property real slack: picker.wholeWidth - picker.foldWidth
    /// The two runs' marks, each at the step its own words are set in (規約 §寸法「語の中に組む印は、その語の段に
    /// 従う」): the copy's name is a step down, so its mark is too; the upstream is set at the row's own step.
    readonly property int trailSeat: Theme.iconXs
    readonly property int noteSeat: Theme.iconSm
    /// **The one column both lines' marks stand in**, so the words after them begin on the same x
    ///. The wider of the two, centred: a column sized to one of them would put the other's
    /// word a pixel or two along from its neighbour above.
    readonly property int markColumn: Math.max(picker.kindSize, picker.noteSeat)
    /// The air the mark keeps inside its own square, on each side — **measured on the mark at rest**. It is turned a
    /// quarter while the list is open, and a turned chevron spans more sideways than a resting one: read off the mark
    /// as drawn, the air changes as the card comes up, the steps worked out from it change with it, and every name in
    /// the row moves under the hand that has just pressed one. The ink turns inside its seat; nothing else moves.
    ///
    /// Measured from the middle out rather than off `inkRight`: that one answers where the far edge of the ink falls
    /// and only the kinds set against a corner carry an entry for it, so a mark centred in its square reports its
    /// whole box and the step comes out **wider** than it was asked for (observed — the gap grew by half a stroke).
    readonly property real markAir:
        (picker.markSeat - (mark.inkGrid / 16 * mark.width + mark.stroke)) / 2
    /// The air between the box's leading edge and the first ink in it — what a hand sees between the wash's edge and
    /// the chevron.
    readonly property real leadAir: picker.leftPadding + picker.markAir
    /// The same for the kind mark, which keeps more of it: it is drawn on a grid a step smaller and none of the family
    /// reaches its own edge.
    readonly property real kindAir: picker.kind === "" ? 0 : (kindMark.width - kindMark.inkWidth) / 2
    /// Whole pixels, because everything a row hands out is: a box asking for a fraction is laid out a fraction short,
    /// and what a name does when its column is a fraction short is cut itself (observed). **Both of the marks' airs
    /// come off the step between them** — each of them holds some, and a step that paid for one would set the pair
    /// further apart than either stands from the name.
    readonly property real markGap:
        Math.max(0, Math.ceil(picker.markStep - picker.markAir - picker.kindAir))
    /// The step after the kind mark, where there is one.
    readonly property real kindGap:
        picker.kind === "" ? 0 : Math.max(0, Math.ceil(picker.kindStep - picker.kindAir))
    /// Automation: what the picker came out saying, whether the row had to cut it, and which way the mark is turned.
    /// A photograph of a cut name and one of a short name read the same, and a run cannot go green with the mark
    /// unwired (`NameCell.foldTurn` takes the same reading).
    readonly property bool nameCut: nameCell.cutting
    readonly property real foldTurn: mark.rotation
    /// …and where the counts came out, which a picture cannot be read for to the pixel: `after` the name at their own
    /// step, carried out to the `end` of a longer second line, or `off` both — `none` where there are none. Read off
    /// the items as laid out, so a run cannot go green on the arithmetic alone.
    readonly property string trackPlace: {
        if (!picker.tracked)
            return "none"
        const step = trackSeat.x - (trailCell.visible ? trailCell.x + trailCell.width : nameCell.x + nameCell.width)
        const past = nameLine.width - (trackSeat.x + trackSeat.width)
        if (Math.abs(step - picker.trackGap) < 0.5)
            return "after"
        return past < 0.5 && step > picker.trackGap ? "end" : "off"
    }

    // Both ends of the box are the theme's dense step rather than the style's own padding: this control is not one of
    // a set that has to keep its widths in step with the band's (`ActionButton.slack`), what it holds is a word rather
    // than a framed box, and three of them stand in a row where every step is paid twice — once by each neighbour
    // (デザイン規約 §余白).
    /// What this picker's chevron is short of the air the row's first one keeps. **The seat at the window's edge is
    /// a column** (`markSeat` = the ☰'s cell) and every other seat is the mark's own size, so the mark that opens a
    /// later name stands closer to what is before it than the first one does — and two marks doing the same job with
    /// different air either side read as two different marks. The row hands in the difference.
    property real markLeadIn: 0
    /// The air the box keeps after its last ink. The row decides: a picker the next one follows keeps the family's
    /// step, since the next one's own lead-in is the rest of the gap between them; **the one that ends the names keeps
    /// the air it opens with**, so its wash is as wide past the words as it is before the chevron — more reads as a
    /// box nobody shrank to what it holds, less as one cut short under the hand.
    property real endAir: picker.gap
    leftPadding: Math.max(0, Math.ceil(picker.gap - picker.markAir)) + picker.markLeadIn
    rightPadding: Math.ceil(picker.endAir)
    topPadding: 0
    bottomPadding: 0
    implicitHeight: Theme.controlHeight
    // The whole depth of the panel it stands in, so the hand's target and the wash it answers with are the band's own
    // — the way the ☰ and the window's buttons fill the band above. Square with it for the same reason
    // (`HoverToolButton.washRadius`): what washes beside a cell that reaches both edges is square.
    washRadius: 0
    // The whole name, wherever the row could not draw it (規約 §hover のツールチップ).
    tip: picker.nameGiven ? picker.shown : ""
    // The card this name opens keeps it lit while it stands, and the chevron turned (`opened`).
    standing: picker.opened

    TextMetrics {
        id: nameRuler
        font.family: Theme.uiFamily
        font.pixelSize: picker.pixelSize
        font.weight: picker.weight
        text: picker.shown
    }
    // The two runs after the name, each measured in the face it is drawn in.
    TextMetrics {
        id: trailRuler
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
        text: picker.trail
    }
    TextMetrics {
        id: noteRuler
        font.family: Theme.uiFamily
        // **A step under the name it stands for**, wherever that name is set: what the row is read for is the branch,
        // and an upstream written at the branch's own step would make the pair read as two names of equal weight.
        font.pixelSize: picker.noteSize
        text: picker.note
    }
    // Six `n` and the mark between them — the same reading a tab's floor is taken from, so the two floors in this
    // chrome are the same measurement rather than two numbers that happen to agree.
    TextMetrics {
        id: floorRuler
        font: nameRuler.font
        text: "nnn…nnn"
    }
    // The same floor for each run, measured in its own face: a run is a name too, and what tells one working copy
    // from another is both of its ends.
    TextMetrics {
        id: trailFloorRuler
        font: trailRuler.font
        text: "nnn…nnn"
    }
    // The upstream's floor is read in the same letters, but **this one is cut at the end** — the remote's name at the
    // head is what the run is read for (規約 §タイポグラフィ「upstream 名だけは末尾で切る」).
    TextMetrics {
        id: noteFloorRuler
        font: noteRuler.font
        text: "nnnnnn…"
    }
    // The line the letters stand on, and how tall the box around them is — what a shared baseline is worked out from.
    FontMetrics {
        id: nameMetrics
        font: nameRuler.font
    }
    FontMetrics {
        id: noteMetrics
        font: noteRuler.font
    }

    contentItem: RowLayout {
        // Each step written where it is paid, because the one after the mark is not the one after the word: the mark
        // gives its own air back (`markGap`).
        id: pickerRow
        spacing: 0

        Item {
            id: markSlot
            // **On the middle of everything this picker writes**, not on the first line's letters: the mark opens
            // the whole of what is written here, both lines of it.
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: picker.markSeat
            Layout.preferredHeight: picker.markSize
            NavIcon {
                id: mark
                anchors.centerIn: parent
                width: picker.markSize
                height: picker.markSize
                kind: "chevron"
                // Drawn pointing right, and turned down while what it opens is standing — the sidebar's own two
                // positions for this mark (`NameCell`).
                rotation: picker.opened ? 90 : 0
                tint: picker.enabled ? Theme.textSecondary : Theme.textMuted
            }
        }
        // **Two lines where the name has something it is measured against, one where it has not**
        //: the name on top, the upstream under it. The repository's own name and a branch
        // with no upstream are both the one-line shape — nothing but the upstream ever stands on the second line.
        ColumnLayout {
            id: nameStack
            Layout.leftMargin: picker.markGap
            // Centred as a pair, so a name written on two lines and one written on one come out on the same middle.
            Layout.alignment: Qt.AlignVCenter
            // As wide as the longer line, which is what the box was measured to (`textRoom`), so the first line can
            // reach the far end of the second.
            Layout.fillWidth: true
            spacing: 0

            RowLayout {
                id: nameLine
                Layout.fillWidth: true
                spacing: 0

                // **The two lines' marks stand in one column and their words begin on one line**
                //: the kind on the first, the remote on the second, each at the same seat,
                // so the name and the upstream under it start at the same x.
                Item {
                    id: kindSlot
                    // On the letters it is set in, not on the line box around them (`markMiddle`).
                    Layout.alignment: Qt.AlignTop
                    Layout.topMargin: picker.kindDrop
                    Layout.preferredWidth: picker.markColumn
                    Layout.preferredHeight: picker.kindSize
                    visible: picker.kind !== ""
                    NavIcon {
                        id: kindMark
                        anchors.centerIn: parent
                        width: picker.kindSize
                        height: picker.kindSize
                        kind: picker.kind === "" ? "chevron" : picker.kind
                        // The 16-grid scales with the seat and the line does not, so this mark drawn whole carries
                        // 4/3 the weight of the same mark in the left list — and weight is what the eye reads as
                        // size (`NavIcon.stroke`; measured, the branch's two rings closed into a blob at this seat).
                        stroke: Metrics.iconStroke * picker.kindSize / Theme.iconMd
                        tint: picker.enabled ? picker.kindTint : Theme.textMuted
                    }
                }
                CutName {
                    id: nameCell
                    Layout.leftMargin: picker.kind === "" ? 0 : picker.kindGap
                    // Laid out at what it was left, not at whatever is going spare: the order the words give way in
                    // is the row's to decide, and a cell that filled would take its neighbour's turn (§譲る順).
                    Layout.preferredWidth: Math.ceil(nameRuler.advanceWidth) - picker.nameGiven
                    Layout.preferredHeight: nameRuler.height
                    text: picker.shown
                    pixelSize: picker.pixelSize
                    weight: picker.weight
                    color: !picker.enabled ? Theme.textMuted
                         : picker.saying ? picker.tone
                         : Theme.textMuted
                }
                // The working copy, set after the name as one run: the mark, then the folder's name. **One step opens
                // the run and nothing opens inside it** — the mark is a letter of it, and what parts it from its own
                // name is the air the mark's box holds (§余白 / §タブの所作).
                NavIcon {
                    id: trailMark
                    Layout.leftMargin: picker.gap
                    Layout.alignment: Qt.AlignVCenter
                    Layout.preferredWidth: picker.trailSeat
                    Layout.preferredHeight: picker.trailSeat
                    visible: picker.trail !== ""
                    kind: "tree"
                    // The 16-grid scales with the seat and the line does not, so a mark this size beside a word would
                    // carry 4/3 the weight of the letters next to it (`NavIcon.stroke`).
                    stroke: Metrics.iconStroke * picker.trailSeat / Theme.iconMd
                    // **The ink of the word it is set in**, which is what the tab's own node
                    // does with the same mark (`TabTreeMark`): a mark written as a letter of a run reads as part of
                    // that run, and a letter that took the section's colour would read as two things.
                    tint: Theme.textSecondary
                }
                CutName {
                    id: trailCell
                    Layout.preferredWidth: Math.ceil(trailRuler.advanceWidth) - picker.trailGiven
                    Layout.preferredHeight: trailRuler.height
                    Layout.alignment: Qt.AlignVCenter
                    visible: picker.trail !== ""
                    text: picker.trail
                    pixelSize: Theme.fontSm
                    color: Theme.textSecondary
                }
                // **The counts end the longer of the two lines.** Where the upstream under the name runs further than
                // the name and its counts do, the counts are carried out to where it ends, so the pair closes on one
                // right edge; where it does not, they follow the name at a word's step (`trackGap`) — the step is this
                // item's floor, and what the second line has over the first is what it grows by.
                Item {
                    visible: picker.tracked
                    Layout.fillWidth: true
                    Layout.minimumWidth: picker.trackGap
                    Layout.preferredWidth: picker.trackGap
                }
                // **Where this branch stands against what it follows**, in the mark the left panel counts it with
                // (`HeadTrack`). Nothing at all where there is nothing to count: level with
                // the upstream, or no upstream to measure against (§左メニューの所作 — 数えない 0 に列は要らない).
                Loader {
                    id: trackSeat
                    active: picker.tracked
                    visible: trackSeat.active
                    Layout.alignment: Qt.AlignVCenter
                    sourceComponent: HeadTrack {
                        ahead: picker.ahead
                        behind: picker.behind
                    }
                }
            }
            // **The second line is the upstream's and nothing else's.** The mark that says it is a remote, then the
            // whole ref — **no brackets**, because the mark has already said what kind of name follows and a pair of
            // them round a name set apart by its own ink is punctuation nobody needs (規約 §ref の種別).
            RowLayout {
                id: noteLine
                spacing: 0
                visible: picker.note !== ""

                Item {
                    id: noteSlot
                    // The same, on its own line's letters (`markMiddle`).
                    Layout.alignment: Qt.AlignTop
                    Layout.topMargin: picker.noteMarkDrop
                    Layout.preferredWidth: picker.markColumn
                    Layout.preferredHeight: picker.noteSeat
                    NavIcon {
                        id: noteMark
                        anchors.centerIn: parent
                        width: picker.noteSeat
                        height: picker.noteSeat
                        kind: "remote"
                        stroke: Metrics.iconStroke * picker.noteSeat / Theme.iconMd
                        tint: picker.noteTone
                    }
                }
                CutName {
                    id: noteCell
                    Layout.leftMargin: picker.kind === "" ? 0 : picker.kindGap
                    // **Cut like every other name in a column, not left to run on** — a run that kept its whole width
                    // when the row lost its own walked out under the buttons beside it (observed at 560px).
                    Layout.preferredWidth: Math.ceil(noteRuler.advanceWidth) - picker.noteGiven
                    Layout.preferredHeight: noteRuler.height
                    Layout.alignment: Qt.AlignVCenter
                    // **Cut at the end, alone among the names here** — the remote's own name stands at the head of
                    // this one and is what the run is read for, so a middle cut would take it out first
                    // (規約 §タイポグラフィ「upstream 名(remote-tracking 名)だけは末尾で切る」).
                    cutAt: "end"
                    text: picker.note
                    pixelSize: picker.noteSize
                    weight: Font.Normal
                    color: picker.noteTone
                }
            }
        }
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A name in the ops panel and the door to its card (デザイン規約 §操作パネル): the chevron (turned like `NameCell`'s),
// an optional kind mark, the name cut in the middle (`CutName`), and the upstream under it where there is one. How
// loudly the name is said is the caller's; what a press opens is `TopBar`'s — nothing here is bound to a repository.
HoverToolButton {
    id: picker

    property string name: ""
    /// Shown in the muted ink while there is no name (a repository not yet answered, an unborn HEAD).
    property string blank: ""
    /// The upstream, on its own line under the name. **Never fold it into `name`** — it would take the name's weight
    /// and colour. Empty = one line.
    property string note: ""
    /// The name's ink — a detached HEAD takes the state's colour, as its chip does (規約 §ref の種別).
    property color tone: Theme.textPrimary
    /// Whether the card this opens is standing: turns the chevron down (`NameCell.folded`, inverted) and keeps the
    /// name lit.
    property bool opened: false
    /// The name's kind as a `NavIcon` kind, drawn between the chevron and the name a step under the chevron's size.
    /// Empty draws none.
    property string kind: ""
    property int kindSize: Theme.iconSm
    /// The kind mark's ink — the left panel's for the same kind (規約 §ref の種別).
    property color kindTint: Theme.textSecondary
    /// The upstream's ink — `warning` where it has gone from the remote, as the left panel does
    /// (§左メニューの所作「消えていれば段ごと `warning`」).
    property color noteTone: Theme.textMuted
    /// The upstream's step. Fixed rather than following `pixelSize`: raising the name must not raise the upstream.
    property int noteSize: Theme.fontMd
    /// Counts against the upstream, drawn as `HeadTrack`; both zero draws nothing.
    property int ahead: 0
    property int behind: 0
    /// The working copy, set after the name as one run — tree mark + folder name, a step down and quieter, as the tab
    /// draws it (デザイン規約 §タブの所作). Empty draws none.
    property string trail: ""
    /// How many pixels narrower than whole the row asks this picker to be (the order between pickers is the row's —
    /// 規約 §操作パネル の譲る順). Taken off the longer line only: cutting the shorter one frees nothing. Runs are cut,
    /// never dropped.
    property real given: 0
    /// What each run can give up before its floor (both ends of a name and the `…`).
    readonly property real trailSlack:
        picker.trail === "" ? 0
        : Math.max(0, Math.ceil(trailRuler.advanceWidth) - Math.ceil(trailFloorRuler.advanceWidth))
    readonly property real nameSlack:
        Math.max(0, Math.ceil(nameRuler.advanceWidth) - Math.ceil(floorRuler.advanceWidth))
    readonly property real noteSlack:
        picker.note === "" ? 0
        : Math.max(0, Math.ceil(noteRuler.advanceWidth) - Math.ceil(noteFloorRuler.advanceWidth))
    /// The width the lines are cut to: the longer line less what was asked, never under the wider floor.
    readonly property real textRoom: picker.textWhole - Math.min(Math.max(0, picker.given), picker.slack)
    /// …and what that takes off each run: the first line gives the copy's run before the name
    /// (規約 §操作パネル の譲る順).
    readonly property real lineOneShort: Math.max(0, picker.lineOneWhole - picker.textRoom)
    readonly property real trailGiven: Math.min(picker.lineOneShort, picker.trailSlack)
    readonly property real nameGiven: Math.min(picker.lineOneShort - picker.trailGiven, picker.nameSlack)
    readonly property real noteGiven:
        Math.min(Math.max(0, picker.lineTwoWhole - picker.textRoom), picker.noteSlack)
    /// The width once `given` is taken off.
    readonly property real drawnWidth: picker.bareBox + picker.textRoom

    // ---- register: each use site sets its own size, weight and ink ------
    /// The step from the chevron's **ink** (not its box) to the word (デザイン規約 §余白); the layout gets `markGap`.
    property int gap: Theme.spaceXs
    /// The step after the chevron's ink, where a row needs it to differ from `gap` (`TopBar`).
    property real markStep: picker.gap
    /// The step from the kind mark's ink to the name; the mark's own air counts towards it (デザイン規約 §余白「印が
    /// 自分で持っている余白は、隣の詰めに数える」). Zero sets the mark as a letter of the name: its own air is the whole
    /// step.
    property real kindStep: 0
    /// The chevron's size — the larger of the two marks, since it is what a hand goes for.
    property int markSize: Theme.iconMd
    /// The chevron's slot, centred. Wider than the mark where it stands in a column (the picker at the window's edge
    /// takes the ☰'s cell); `markGap` then comes out zero because the seat already holds more air than a step.
    property int markSeat: picker.markSize
    property int pixelSize: Theme.fontMd
    property int weight: Theme.fontWeightStrong
    /// Both lines' height **whatever is written now**, read off the faces — the panel's buttons are drawn this deep
    /// even for a one-line branch or a page still loading.
    readonly property real pairHeight: nameMetrics.height + noteMetrics.height
    /// Top offset for a mark set among letters: on the baseline, centred on the capitals where it is taller than them
    /// (デザイン規約 §操作パネル). Measured on the mark's ink (`NavIcon.inkTallGrid`), in the face it stands in.
    function markMiddle(seat, metrics, mark) {
        const ink = mark.inkTallGrid / 16 * mark.height + mark.stroke
        const stands = Math.min(Math.max(ink, metrics.xHeight), metrics.capitalHeight)
        return Math.max(0, Math.round(metrics.ascent - stands / 2 - seat / 2))
    }
    readonly property real kindDrop: picker.markMiddle(picker.kindSize, nameMetrics, kindMark)
    readonly property real noteMarkDrop: picker.markMiddle(picker.noteSeat, noteMetrics, noteMark)

    readonly property bool saying: picker.name !== ""
    readonly property string shown: picker.saying ? picker.name : picker.blank

    readonly property real bareBox:
        picker.leftPadding + picker.rightPadding + picker.markSeat + picker.markGap
    /// The column the kind mark stands in at the head of each line, and the step after it.
    readonly property real kindColumn: picker.kind === "" ? 0 : picker.markColumn + picker.kindGap
    readonly property real trailWidth:
        picker.trail === "" ? 0 : picker.gap + picker.trailSeat + Math.ceil(trailRuler.advanceWidth)
    /// Counts beside the name, and what they cost. The step is a word's, not a letter's (規約 §操作パネル).
    readonly property bool tracked: picker.ahead > 0 || picker.behind > 0
    property int trackGap: Theme.spaceSm
    readonly property real trackRun: picker.tracked ? picker.trackGap + Math.ceil(trackSeat.implicitWidth) : 0
    /// Each line measured on its own: the box is the longer of the two, never both end to end.
    readonly property real lineOneWhole:
        picker.kindColumn + Math.ceil(nameRuler.advanceWidth) + picker.trailWidth + picker.trackRun
    readonly property real lineTwoWhole:
        picker.note === "" ? 0 : picker.kindColumn + Math.ceil(noteRuler.advanceWidth)
    readonly property real textWhole: Math.max(picker.lineOneWhole, picker.lineTwoWhole)
    /// …and each at its floor: the runs and the name cut down as far as they go, the counts whole.
    readonly property real lineOneFloor: picker.lineOneWhole - picker.trailSlack - picker.nameSlack
    readonly property real lineTwoFloor: picker.lineTwoWhole - picker.noteSlack
    /// The width asked of the row before anything gives way. **Must not read `given`** — the row decides from this,
    /// and a figure that counted the answer closes the loop.
    readonly property real wholeWidth: picker.bareBox + picker.textWhole
    /// …and the narrowest: every name at both ends, three characters each side of `…`
    /// (規約 §レイアウト初期値「両端に 3 文字ずつ」).
    readonly property real foldWidth: picker.bareBox + Math.max(picker.lineOneFloor, picker.lineTwoFloor)
    /// What lies between the two — the most this picker can give.
    readonly property real slack: picker.wholeWidth - picker.foldWidth
    /// The two runs' marks, each at its own words' step (規約 §寸法「語の中に組む印は、その語の段に従う」).
    readonly property int trailSeat: Theme.iconXs
    readonly property int noteSeat: Theme.iconSm
    /// The one column both lines' marks stand in, as wide as the wider mark, so the words after them start on one x.
    readonly property int markColumn: Math.max(picker.kindSize, picker.noteSeat)
    /// The air the chevron keeps inside its square on each side, **measured at rest** — measured turned, the steps
    /// would change as the card opens and the names would move under the hand. From the middle out: the ink is centred
    /// in its square.
    readonly property real markAir:
        (picker.markSeat - (mark.inkGrid / 16 * mark.width + mark.stroke)) / 2
    /// The air between the box's leading edge and the chevron's ink.
    readonly property real leadAir: picker.leftPadding + picker.markAir
    readonly property real kindAir: picker.kind === "" ? 0 : (kindMark.width - kindMark.inkWidth) / 2
    /// Whole pixels: a fractional ask is laid out a fraction short, and the name gets cut. Both marks' airs come off
    /// the step, or the pair would stand further apart than either stands from the name.
    readonly property real markGap:
        Math.max(0, Math.ceil(picker.markStep - picker.markAir - picker.kindAir))
    readonly property real kindGap:
        picker.kind === "" ? 0 : Math.max(0, Math.ceil(picker.kindStep - picker.kindAir))
    /// Automation: whether the name was cut, and the chevron's turn — neither can be read off a picture.
    readonly property bool nameCut: nameCell.cutting
    readonly property real foldTurn: mark.rotation
    /// Automation: where the counts came out — `after` the name at `trackGap`, at the `end` of a longer second line,
    /// `off` both, or `none`. Read off the laid-out items, not the arithmetic.
    readonly property string trackPlace: {
        if (!picker.tracked)
            return "none"
        const step = trackSeat.x - (trailCell.visible ? trailCell.x + trailCell.width : nameCell.x + nameCell.width)
        const past = nameLine.width - (trackSeat.x + trackSeat.width)
        if (Math.abs(step - picker.trackGap) < 0.5)
            return "after"
        return past < 0.5 && step > picker.trackGap ? "end" : "off"
    }

    // The paddings are the theme's step, not the style's: this holds a word rather than a framed box (デザイン規約 §余白).
    /// What this chevron is short of the air the row's first one keeps in its ☰-wide seat — handed in by the row, so
    /// both chevrons stand with the same air (規約 §操作パネル).
    property real markLeadIn: 0
    /// The air after the last ink. The row decides: a picker another follows keeps `gap`; the last one keeps its
    /// `leadAir`, so its wash is even either side (規約 §操作パネル).
    property real endAir: picker.gap
    leftPadding: Math.max(0, Math.ceil(picker.gap - picker.markAir)) + picker.markLeadIn
    rightPadding: Math.ceil(picker.endAir)
    topPadding: 0
    bottomPadding: 0
    implicitHeight: Theme.controlHeight
    // Square: the wash fills the panel's whole depth, and a wash reaching both edges is square
    // (`HoverToolButton.washRadius`).
    washRadius: 0
    // The whole name, wherever the row could not draw it (規約 §hover のツールチップ).
    tip: picker.nameGiven ? picker.shown : ""
    standing: picker.opened

    TextMetrics {
        id: nameRuler
        font.family: Theme.uiFamily
        font.pixelSize: picker.pixelSize
        font.weight: picker.weight
        text: picker.shown
    }
    // Each run measured in the face it is drawn in.
    TextMetrics {
        id: trailRuler
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
        text: picker.trail
    }
    TextMetrics {
        id: noteRuler
        font.family: Theme.uiFamily
        font.pixelSize: picker.noteSize
        text: picker.note
    }
    // The same reading a tab's floor is taken from (規約 §レイアウト初期値).
    TextMetrics {
        id: floorRuler
        font: nameRuler.font
        text: "nnn…nnn"
    }
    TextMetrics {
        id: trailFloorRuler
        font: trailRuler.font
        text: "nnn…nnn"
    }
    // The upstream is cut at the end (`noteCell`), so its floor keeps the head.
    TextMetrics {
        id: noteFloorRuler
        font: noteRuler.font
        text: "nnnnnn…"
    }
    FontMetrics {
        id: nameMetrics
        font: nameRuler.font
    }
    FontMetrics {
        id: noteMetrics
        font: noteRuler.font
    }

    contentItem: RowLayout {
        // Each step is a margin where it is paid, not `spacing`: the one after the mark gives its air back (`markGap`).
        id: pickerRow
        spacing: 0

        Item {
            id: markSlot
            // Centred on both lines, not on the first line's letters.
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: picker.markSeat
            Layout.preferredHeight: picker.markSize
            NavIcon {
                id: mark
                anchors.centerIn: parent
                width: picker.markSize
                height: picker.markSize
                kind: "chevron"
                rotation: picker.opened ? 90 : 0
                tint: picker.enabled ? Theme.textSecondary : Theme.textMuted
            }
        }
        ColumnLayout {
            id: nameStack
            Layout.leftMargin: picker.markGap
            Layout.alignment: Qt.AlignVCenter
            // As wide as `textRoom`, so the first line's counts can reach the second line's end.
            Layout.fillWidth: true
            spacing: 0

            RowLayout {
                id: nameLine
                Layout.fillWidth: true
                spacing: 0

                Item {
                    id: kindSlot
                    // On the letters, not the line box (`markMiddle`).
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
                        // Scaled with the seat: the 16-grid shrinks and the line does not, so at full stroke the
                        // branch's two rings close into a blob (`NavIcon.stroke`).
                        stroke: Metrics.iconStroke * picker.kindSize / Theme.iconMd
                        tint: picker.enabled ? picker.kindTint : Theme.textMuted
                    }
                }
                CutName {
                    id: nameCell
                    Layout.leftMargin: picker.kind === "" ? 0 : picker.kindGap
                    // Exactly what it was left, never filling: a filling cell would take its neighbour's turn
                    // (規約 §操作パネル の譲る順).
                    Layout.preferredWidth: Math.ceil(nameRuler.advanceWidth) - picker.nameGiven
                    Layout.preferredHeight: nameRuler.height
                    text: picker.shown
                    pixelSize: picker.pixelSize
                    weight: picker.weight
                    color: !picker.enabled ? Theme.textMuted
                         : picker.saying ? picker.tone
                         : Theme.textMuted
                }
                // The working copy's run (`trail`): one step before the mark, none after it.
                NavIcon {
                    id: trailMark
                    Layout.leftMargin: picker.gap
                    Layout.alignment: Qt.AlignVCenter
                    Layout.preferredWidth: picker.trailSeat
                    Layout.preferredHeight: picker.trailSeat
                    visible: picker.trail !== ""
                    kind: "tree"
                    // Scaled as `kindMark`'s.
                    stroke: Metrics.iconStroke * picker.trailSeat / Theme.iconMd
                    // The run's own ink, as `TabTreeMark` — the section's colour would split the run in two.
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
                // The counts end the longer line: `trackGap` at least, grown by what the second line has over the
                // first (規約 §操作パネル).
                Item {
                    visible: picker.tracked
                    Layout.fillWidth: true
                    Layout.minimumWidth: picker.trackGap
                    Layout.preferredWidth: picker.trackGap
                }
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
            // The upstream: the remote mark, then the whole ref — no brackets, the mark already says what kind of
            // name follows (規約 §ref の種別).
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
                    // Cut, not left to run on: kept whole, it walks out under the buttons beside it.
                    Layout.preferredWidth: Math.ceil(noteRuler.advanceWidth) - picker.noteGiven
                    Layout.preferredHeight: noteRuler.height
                    Layout.alignment: Qt.AlignVCenter
                    // At the end: the remote's name at the head is what the run is read for
                    // (規約 §寸法「upstream 名(remote-tracking 名)だけは末尾で切る」).
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

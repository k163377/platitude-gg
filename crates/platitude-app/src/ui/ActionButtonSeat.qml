import QtQuick
import platitude.ui

// The seat a toolbar action's icon sits in; no icon, no width. A button that swaps icon and hold mark keeps the wider
// seat in every state, so the toolbar holds still under a resting pointer; one that only wears the mark fits it.
Item {
    id: seat

    /// The icon's `NavIcon` kind; empty draws none.
    property string kind: ""
    property int holdPresses: 0
    /// A mark standing next to the word (`ActionButton.besideWord`).
    property bool besideWord: false
    /// 0 to 1.
    property real holdProgress: 0
    /// git is out on the network for this button: the ring turns here in place of the seat's marks, so a button that
    /// can wait needs a seat (デザイン規約 §進行中・回数押しの定数).
    property bool busy: false
    /// The button's own `markFg`.
    property color tint: Theme.textPrimary
    /// The hold mark's own colour (`ActionButton.holdInk`): at rest what pressing through costs — a step down while
    /// [`holdOff`] — and the seat's over the fill and while waiting.
    property color holdTint: seat.tint
    property bool holdOff: false
    /// The failure mark for a button with no word left (`ActionButton.folded`), at the icon's upper corner with the
    /// raise and half gap it keeps from a word (デザイン規約 §操作パネル).
    property bool cornerAlert: false
    property color cornerAlertTone: seat.tint
    /// The pixel size of the word the seat heads on its line; zero where it heads none (stacked under the word,
    /// folded alone, or a phrase that carries its own marks). Set, the marks are drawn to its letters and stand on
    /// their capitals' middle: the icon and the ring head the word (デザイン規約 §アイコンの席 の文頭記号), and the hold
    /// mark alone is a letter of it (の名指し).
    property real wordSize: 0
    /// Where that word's baseline runs, in the seat's y.
    property real wordBase: 0
    /// How far that word's ground reaches before its letters: a code word's chip hangs half a step (`CodeChip`).
    property real wordHang: 0
    /// Whether the seat heads a word.
    readonly property bool leads: seat.wordSize > 0
    /// Where the word's capitals' middle stands, in the seat's y: what a round mark (the hold mark, the ring) centres
    /// on.
    readonly property real capitalsMiddle: seat.wordBase - WordFace.capitalMiddle(seat.wordSize)

    readonly property bool paired: seat.kind !== "" && seat.holdPresses > 0
    /// How much wider than one half the seat is (`implicitWidth`): the two halves' ink stands inside it.
    readonly property int spread: Theme.iconMd - Theme.spaceXs
    /// How far along the slash sets the two apart: `spread` and half a gap more — the air each half carries inside its
    /// box, so the ink of push's mark and the ring fits the seat with the boxes a pixel past it at either end
    /// (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」).
    readonly property int pairAlong: seat.spread + Theme.spaceXs / 2
    /// How far down: the pair stands no deeper than a lone mark at the panel's step (`iconLg` — moves with
    /// `ActionButton.stackStep`), so a frame held to the word over the mark (`ActionButton.stackDepth`, which macOS's
    /// short lines make the frame's depth) keeps air under the ring.
    readonly property int pairDown: Theme.iconLg - Theme.iconSm
    /// The slash's length, end to end: as far as a `fontMd` `/` reaches.
    readonly property real slashReach: Theme.iconSm + Theme.spaceXs / 2
    /// The grid's stroke scaled to the halves' step, lighter than the push mark's (`NavIcon` keeps its stroke whole at
    /// `iconSm`): the slash parts the two halves and is neither of them.
    readonly property real slashStroke: Metrics.iconStroke * Theme.iconSm / 16
    /// The icon's step. A held button keeps `iconMd`: the fraction is two `iconSm` halves with no room to give. A
    /// caller may raise it where a folded two-line button's mark grows into the word's line (`ActionButton.stacked`).
    property int step: seat.besideWord && seat.holdPresses <= 0 ? Theme.iconSm : Theme.iconMd
    /// The air inside the mark's box, given back so the padding and the row's spacing land on the ink; the icon
    /// overflows the seat by this much either side (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」).
    /// One number for the family, not each mark's own ink, so every mark at this step takes one seat; a wide mark's
    /// ink runs a little past it.
    readonly property int markAir: seat.step === Theme.iconSm ? Theme.spaceXs / 2 : 0

    /// How deep the seat is at a step: the mark and the air the seat carries round it. Asked by a button that holds
    /// its depth across a step it is not drawn at now (`ActionButton.stackDepth`).
    function depthAt(step) {
        return step + Theme.spaceXs
    }

    visible: seat.kind !== "" || seat.holdPresses > 0
    implicitWidth: !visible ? 0
                   : seat.kind !== ""
                     ? (seat.step === Theme.iconSm
                        ? Theme.iconSm - 2 * seat.markAir
                        : Math.max(Theme.iconMd, Theme.iconSm + seat.spread))
                     // Heading a word, the hold mark alone is a letter of the phrase (デザイン規約 §アイコンの席 の
                     // 名指し): the seat is its ink, and the row's step parts it from the word — from a code word's
                     // ground, which hangs half a step before its letters, too.
                     : seat.leads ? Math.round(loneHold.inkWidth) + seat.wordHang
                     : Theme.iconSm
    implicitHeight: seat.depthAt(seat.step)
    // Heading a word, its box runs a gap past the seat either way: the word's capitals stand where they stand, and the
    // canvas draws only inside its box.
    NavIcon {
        width: seat.step
        height: seat.leads ? seat.height + 2 * Theme.spaceXs : seat.step
        x: Math.round((seat.width - width) / 2)
        y: seat.leads ? -Theme.spaceXs : Math.round((seat.height - height) / 2)
        kind: seat.kind
        set: seat.leads ? "lead" : ""
        size: seat.wordSize
        wordBase: seat.wordBase + Theme.spaceXs
        tint: seat.tint
        // Whole at `iconMd`, and the grid ratio under it.
        stroke: Metrics.iconStroke * seat.step / 16
        visible: seat.holdPresses <= 0 && !seat.busy
    }
    // At the seat's step so the width holds, swapping with the hold mark
    // (デザイン規約 §進行中・回数押しの定数「回るリングと回数押しの印は同じ席で入れ替わる」). Its own item: an animator
    // leaves the angle where it stopped, and the icon has to come back upright — so it turns about its own middle, which
    // stands on the word's capitals' middle.
    SpinnerIcon {
        box: seat.step
        wordSize: seat.leads ? seat.wordSize : 0
        x: Math.round((seat.width - width) / 2)
        y: seat.leads ? Math.round(seat.capitalsMiddle - height / 2) : Math.round((seat.height - height) / 2)
        tint: seat.tint
        spinning: seat.busy
    }
    // Over the seat, so the icon under it keeps the step and centre it had with the word. On the fraction's box while
    // that is drawn: the folded seat is deeper than the pair, and on the seat's corner the mark floats clear of it.
    NavIcon {
        visible: seat.cornerAlert
        kind: "bang"
        tint: seat.cornerAlertTone
        width: Theme.iconSm
        height: Theme.iconSm
        x: (pair.visible ? pair.x + pair.width : parent.width) - width + Theme.spaceXs / 2
        y: (pair.visible ? pair.y : 0) - Theme.spaceXs
    }
    // A held button with no icon wears only the hold mark, at the row's start (デザイン規約 §回数押し) — heading a word,
    // as tall as its capitals and on their middle, its disc on the seat's start and its box's air past it.
    HoldIcon {
        id: loneHold
        x: seat.leads ? -Math.round((loneHold.width - loneHold.inkWidth) / 2) : 0
        y: seat.leads ? Math.round(seat.capitalsMiddle - loneHold.height / 2)
                      : Math.round((seat.height - loneHold.height) / 2)
        size: seat.wordSize
        progress: seat.holdProgress
        presses: seat.holdPresses
        tint: seat.holdTint
        off: seat.holdOff
        visible: seat.holdPresses > 0 && !seat.paired && !seat.busy
    }
    // A held button with an icon wears both as a fraction across a slash: the hold mark alone would cost `push -f` the
    // icon that says it is a push (デザイン規約 §回数押し).
    // Centred at its own size whatever the step: the folded seat is deeper, and the halves keep their distance in it.
    Item {
        id: pair
        width: Theme.iconSm + seat.pairAlong
        height: Theme.iconSm + seat.pairDown
        anchors.centerIn: parent
        visible: seat.paired && !seat.busy
        NavIcon {
            kind: seat.kind
            tint: seat.tint
            width: Theme.iconSm
            height: Theme.iconSm
            x: 0
            y: 0
        }
        // Drawn, not a glyph: square to the line between the halves' centres and through its middle, so it parts the
        // two evenly whatever `pairAlong` and `pairDown` are — a font's `/` keeps its own slant and sits on its
        // baseline.
        InkCanvas {
            id: slash

            property color tint: seat.tint

            anchors.fill: parent
            onTintChanged: slash.requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                const run = Math.hypot(seat.pairAlong, seat.pairDown)
                const half = seat.slashReach / 2
                const dx = half * seat.pairDown / run
                const dy = half * seat.pairAlong / run
                ctx.clearRect(0, 0, width, height)
                ctx.strokeStyle = slash.tint
                ctx.lineWidth = seat.slashStroke
                ctx.lineCap = "round"
                ctx.beginPath()
                ctx.moveTo(width / 2 - dx, height / 2 + dy)
                ctx.lineTo(width / 2 + dx, height / 2 - dy)
                ctx.stroke()
            }
        }
        HoldIcon {
            progress: seat.holdProgress
            presses: seat.holdPresses
            tint: seat.holdTint
            off: seat.holdOff
            width: Theme.iconSm
            height: Theme.iconSm
            x: seat.pairAlong
            y: seat.pairDown
        }
    }
}

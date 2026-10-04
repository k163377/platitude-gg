import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The seat a toolbar action's icon sits in; no icon, no width. A button that swaps icon and hold mark keeps the wider
// seat in every state, so the toolbar holds still under a resting pointer; one that only wears the mark fits it.
Item {
    id: seat

    /// The icon's `NavIcon` kind; empty draws none.
    property string kind: ""
    property int holdMs: 0
    /// A mark standing next to the word (`ActionButton.besideWord`).
    property bool besideWord: false
    /// 0 to 1.
    property real holdProgress: 0
    /// git is out on the network for this button: the ring turns here in place of the seat's marks, so a button that
    /// can wait needs a seat (デザイン規約 §進行中・長押しの定数).
    property bool busy: false
    /// The button's own `markFg`.
    property color tint: Theme.textPrimary
    /// The failure mark for a button with no word left (`ActionButton.folded`), at the icon's upper corner with the
    /// raise and half gap it keeps from a word (デザイン規約 §ウィンドウの縁).
    property bool cornerAlert: false
    property color cornerAlertTone: seat.tint

    readonly property bool paired: seat.kind !== "" && seat.holdMs > 0
    /// How far the two are set apart across the slash.
    readonly property int spread: Theme.iconMd - Theme.spaceXs
    /// The icon's step. A held button keeps `iconMd`: the fraction is two `iconSm` halves with no room to give. A
    /// caller may raise it where a folded two-line button's mark grows into the word's line (`ActionButton.stacked`).
    property int step: seat.besideWord && seat.holdMs <= 0 ? Theme.iconSm : Theme.iconMd
    /// The air inside the mark's box, given back so the padding and the row's spacing land on the ink; the icon
    /// overflows the seat by this much either side (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」).
    /// One number for the family, from its widest mark (8–10px of ink in the 12px box).
    readonly property int markAir: seat.step === Theme.iconSm ? Theme.spaceXs / 2 : 0

    /// How deep the seat is at a step: the mark and the air the seat carries round it. Asked by a button that holds
    /// its depth across a step it is not drawn at now (`ActionButton.stackDepth`).
    function depthAt(step) {
        return step + Theme.spaceXs
    }

    visible: seat.kind !== "" || seat.holdMs > 0
    implicitWidth: !visible ? 0
                   : seat.kind !== ""
                     ? (seat.step === Theme.iconSm
                        ? Theme.iconSm - 2 * seat.markAir
                        : Math.max(Theme.iconMd, Theme.iconSm + seat.spread))
                     : Theme.iconSm
    implicitHeight: seat.depthAt(seat.step)
    NavIcon {
        width: seat.step
        height: seat.step
        anchors.centerIn: parent
        kind: seat.kind
        tint: seat.tint
        // Whole at `iconMd`, and the grid ratio under it.
        stroke: Metrics.iconStroke * seat.step / 16
        visible: seat.holdMs <= 0 && !seat.busy
    }
    // At the seat's step so the width holds, swapping with the hold mark
    // (デザイン規約 §進行中・長押しの定数「回るリングと長押しの印は同じ席で入れ替わる」). Its own item: an animator
    // leaves the angle where it stopped, and the icon has to come back upright.
    SpinnerIcon {
        width: seat.step
        height: seat.step
        anchors.centerIn: parent
        tint: seat.tint
        spinning: seat.busy
    }
    // Over the seat, so the icon under it keeps the step and centre it had with the word.
    NavIcon {
        visible: seat.cornerAlert
        kind: "bang"
        tint: seat.cornerAlertTone
        width: Theme.iconSm
        height: Theme.iconSm
        x: parent.width - width + Theme.spaceXs / 2
        y: -Theme.spaceXs
    }
    // A held button with no icon wears only the hold mark, at the row's start (デザイン規約 §長押し).
    HoldIcon {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        progress: seat.holdProgress
        tint: seat.tint
        visible: seat.holdMs > 0 && !seat.paired && !seat.busy
    }
    // A held button with an icon wears both as a fraction across a slash: the hold mark alone would cost `push -f` the
    // icon that says it is a push (デザイン規約 §長押し).
    Item {
        anchors.fill: parent
        visible: seat.paired && !seat.busy
        NavIcon {
            kind: seat.kind
            tint: seat.tint
            width: Theme.iconSm
            height: Theme.iconSm
            x: 0
            y: 0
        }
        Label {
            text: "/"
            color: seat.tint
            font.pixelSize: Theme.fontMd
            anchors.centerIn: parent
        }
        HoldIcon {
            progress: seat.holdProgress
            tint: seat.tint
            width: Theme.iconSm
            height: Theme.iconSm
            x: seat.spread
            y: parent.height - Theme.iconSm
        }
    }
}

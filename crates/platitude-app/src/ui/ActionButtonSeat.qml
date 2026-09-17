import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The seat a toolbar action's icon sits in. A button with no icon to name it spends no width on one — the word is the
// whole of it (the hunk header's buttons). A button that swaps between a named icon and the mark keeps the wider of the
// two seats whichever it is wearing, so its width holds across its states and the toolbar stays put under a
// pointer already resting on it. One that only ever wears the mark fits it, and its words sit as close to it as a menu
// row's do.
Item {
    id: seat

    /// What the button does, named as a shape. Empty draws no icon.
    property string kind: ""
    /// How long the button has to be held; zero for an ordinary one.
    property int holdMs: 0
    /// The icon is a mark standing next to the word
    /// (`ActionButton.besideWord`).
    property bool besideWord: false
    /// How far into the hold the press has got, 0 to 1.
    property real holdProgress: 0
    /// git is out on the network for this button. The seat is the one place on it already measured for a mark, so the
    /// ring turns here and whatever the seat was wearing steps aside for it — the word beside it stays and says which
    /// action is out (デザイン規約 §進行中・長押しの定数). A button that can wait therefore needs a seat to wait in.
    property bool busy: false
    /// The colour both marks are drawn in — the button's own `markFg`.
    property color tint: Theme.textPrimary
    /// The last go at what this button does did not work, on a button with no word left to say it after
    /// (`ActionButton.folded`). The mark comes to the icon's own upper corner — raised and set out by the same half
    /// gap it keeps from the end of a word, so it reads the same wherever it is met (デザイン規約 §ウィンドウの縁).
    property bool cornerAlert: false
    property color cornerAlertTone: seat.tint

    /// Both marks to wear at once: what the button does, and that it is held.
    readonly property bool paired: seat.kind !== "" && seat.holdMs > 0
    /// How far the two are set apart across the slash.
    readonly property int spread: Theme.iconMd - Theme.spaceXs
    /// The step this button's icon is drawn at (see `besideWord`). A button that can pair keeps the band's step
    /// whatever it is asked for: the fraction is built out of two `iconSm` halves and has no room to give.
    readonly property int step: seat.besideWord && seat.holdMs <= 0 ? Theme.iconSm : Theme.iconMd
    /// The air the mark keeps inside its own box, which the seat gives back so that the padding on one side and the
    /// row's spacing on the other land on the **ink** (デザイン規約 §余白「印が自分で持っている余白は、隣の 詰めに数える」). The icon overflows the
    /// seat by this much either side.
    ///
    /// One number for the family, from the widest of these marks: 8–10px of ink inside the 12px box.
    readonly property int markAir: seat.step === Theme.iconSm ? Theme.spaceXs / 2 : 0

    visible: seat.kind !== "" || seat.holdMs > 0
    implicitWidth: !visible ? 0
                   : seat.kind !== ""
                     ? (seat.step === Theme.iconSm
                        ? Theme.iconSm - 2 * seat.markAir
                        : Math.max(Theme.iconMd, Theme.iconSm + seat.spread))
                     : Theme.iconSm
    implicitHeight: seat.step + Theme.spaceXs
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
    // The wait, in the seat the shape kept warm. Drawn at the seat's own step so the button's width holds, and
    // in the hold mark's place: the press is over by the time git is out, so the ring and the mark swap
    // (デザイン規約 §進行中・長押しの定数「回るリングと長押しの印は同じボタンの上で入れ替わり、同時には出ない」).
    //
    // Its own item: an animator leaves the angle where it stopped, and the
    // icon that comes back when the wait is over comes back upright.
    SpinnerIcon {
        width: seat.step
        height: seat.step
        anchors.centerIn: parent
        tint: seat.tint
        spinning: seat.busy
    }
    // The mark a folded button has nowhere else to put. Drawn over the seat, so the icon under
    // it keeps the step and the centre it had while the word was still there.
    NavIcon {
        visible: seat.cornerAlert
        kind: "bang"
        tint: seat.cornerAlertTone
        width: Theme.iconSm
        height: Theme.iconSm
        x: parent.width - width + Theme.spaceXs / 2
        y: -Theme.spaceXs
    }
    // A held button with nothing to name it says only how it is worked, where the eye starts the row (デザイン規約 §長押し).
    HoldIcon {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: Metrics.opticalDrop
        progress: seat.holdProgress
        tint: seat.tint
        visible: seat.holdMs > 0 && !seat.paired && !seat.busy
    }
    // A held button that also has a name wears both, set as a fraction: each a size down, the slash between them, each
    // pushed off the middle line. Letting the hold mark take the seat on its own would cost the button the one thing
    // that says what it does — `push -f` would stop being a push at a glance (デザイン規約 §長押し).
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

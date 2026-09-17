import QtQuick
import QtQuick.Layouts
import platitude.ui

// One badge's box in the band's state group: natural width until the
// group hands down a cap, and the word elides into whatever is left.
Rectangle {
    id: badge
    property bool filled: false
    /// Which state this badge is standing for (規約 §状態). **The frame is what carries it**
    /// (規約 §進行中・長押しの定数「警告の色は枠と印が持つ」) — a badge whose word is one colour and whose frame is
    /// another says two things at once, and the frame is the half a reader takes in first.
    property color tint: Theme.warning
    /// What this badge is drawn at with nothing narrowed. Handed in from
    /// the hidden measurement: that
    /// row is not laid out while the group is folded (`BadgeWord`).
    property real naturalW: 0
    /// What the group narrowed all of its badges to, together.
    /// `Number.MAX_VALUE` is "nothing is narrowed" (`BandStateGroup.cap`).
    property real cap: Number.MAX_VALUE
    /// Whether this badge is also a way somewhere. Only
    /// the identity one is, narrowed or not (規約 §identity).
    property bool pressable: false
    signal pressed()
    default property alias content: badgeRowInner.data

    implicitWidth: badge.naturalW
    width: Math.min(badge.naturalW, badge.cap)
    height: Theme.iconLg
    radius: Theme.radiusSm
    color: badge.filled ? Theme.danger : badge.pressable && badgeHover.hovered ? Theme.bgHover : "transparent"
    border.color: badge.filled ? "transparent" : badge.tint
    border.width: badge.filled ? 0 : Theme.borderWidth
    RowLayout {
        id: badgeRowInner
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        // Handlers: an `Item` handed to
        // a layout is given a seat in it, and the word
        // beside it loses that much room (measured here as
        // `SET IDENT…` in a window with 800px going spare).
        HoverHandler {
            id: badgeHover
            enabled: badge.pressable
        }
        TapHandler {
            enabled: badge.pressable
            onTapped: badge.pressed()
        }
    }
}
